import base64
import hashlib
import json

import jwt
from fastapi import APIRouter, Depends, Request, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db, utcnow
from app.deps import ChannelContext, channel_context, current_user
from app.errors import forbidden, not_found, unauthorized
from app.livekit import rooms as livekit_rooms
from app.livekit.token import build_token, server_room
from app.models import User, VoiceParticipant
from app.realtime.bus import bus
from app.realtime.events import Event

router = APIRouter(tags=["voice"])


async def _participants(db: AsyncSession, room: str) -> list[dict]:
    rows = (await db.scalars(select(VoiceParticipant).where(VoiceParticipant.room == room))).all()
    users = {
        u.id: u for u in (await db.scalars(select(User).where(User.id.in_([r.user_id for r in rows] or [""])))).all()
    }
    return [
        {
            "user_id": r.user_id,
            "name": users[r.user_id].display_name if r.user_id in users else "已注销",
            "color": users[r.user_id].avatar_color if r.user_id in users else 0x808080,
            "mic": r.mic,
            "camera": r.camera,
            "screen": r.screen,
            "joined_at": r.joined_at.isoformat(),
        }
        for r in rows
    ]


@router.post("/channels/{channel_id}/voice/join")
async def join_voice(ctx: ChannelContext = Depends(channel_context), db: AsyncSession = Depends(get_db)) -> dict:
    if ctx.channel.kind != "voice":
        raise not_found("语音频道")
    ctx.require("CONNECT_VOICE")

    room = server_room(ctx.server_id, ctx.channel.id)
    token = build_token(
        identity=ctx.user.id,
        name=ctx.user.display_name,
        room=room,
        metadata={"color": ctx.user.avatar_color},
    )

    row = await db.get(VoiceParticipant, (room, ctx.user.id))
    if row is None:
        db.add(VoiceParticipant(room=room, user_id=ctx.user.id))
        await db.commit()

    participants = await _participants(db, room)
    await bus.to_server(
        ctx.server_id,
        Event.VOICE_STATE_UPDATE,
        {"channel_id": ctx.channel.id, "room": room, "participants": participants},
    )
    return {
        "token": token,
        "url": settings.livekit_url,
        "room": room,
        "participants": participants,
    }


@router.post("/channels/{channel_id}/voice/leave", status_code=status.HTTP_204_NO_CONTENT)
async def leave_voice(ctx: ChannelContext = Depends(channel_context), db: AsyncSession = Depends(get_db)) -> None:
    room = server_room(ctx.server_id, ctx.channel.id)
    row = await db.get(VoiceParticipant, (room, ctx.user.id))
    if row is not None:
        await db.delete(row)
        await db.commit()
    participants = await _participants(db, room)
    await bus.to_server(
        ctx.server_id,
        Event.VOICE_STATE_UPDATE,
        {"channel_id": ctx.channel.id, "room": room, "participants": participants},
    )


@router.get("/channels/{channel_id}/voice/participants")
async def voice_participants(
    ctx: ChannelContext = Depends(channel_context), db: AsyncSession = Depends(get_db)
) -> list[dict]:
    room = server_room(ctx.server_id, ctx.channel.id)
    return await _participants(db, room)


@router.patch("/voice/{room}/participants/{identity}")
async def moderate_participant(
    room: str,
    identity: str,
    body: dict,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    # room 形如 srv:{server}:{channel}，从房间名反查服务器做权限校验
    parts = room.split(":")
    if len(parts) != 3 or parts[0] != "srv":
        raise not_found("房间")
    server_id = parts[1]
    from app.services.permissions import has_permission

    if not await has_permission(db, server_id, user.id, "MANAGE_CHANNELS"):
        raise forbidden("MANAGE_CHANNELS")

    row = await db.get(VoiceParticipant, (room, identity))
    if row is None:
        raise not_found("参与者")

    action = body.get("action", "mute")
    roomservice_ok = False
    if livekit_rooms.enabled():
        # 真实部署：先调 RoomService，成功后状态由 webhook 回写
        if action == "remove":
            roomservice_ok = await livekit_rooms.remove_participant(room, identity)
        elif action in {"mute", "unmute"}:
            roomservice_ok = await livekit_rooms.mute_published_track(room, identity, action == "mute")

    if not roomservice_ok:
        # 未启用或 RoomService 不可达：回退 Demo 投影
        if action == "remove":
            await db.delete(row)
        elif action in {"mute", "unmute"}:
            row.mic = action == "unmute"
        elif action in {"camera_on", "camera_off"}:
            row.camera = action == "camera_on"
        else:
            row.mic = not row.mic
    await db.commit()

    # 真实部署在此调用 LiveKit RoomService.MutePublishedTrack / RemoveParticipant，
    # 成功后再由 webhook 回写；Demo 直接改本地投影表。
    participants = await _participants(db, room)
    await bus.to_server(server_id, Event.VOICE_STATE_UPDATE, {"room": room, "participants": participants})
    return {"ok": True, "action": action}


@router.put("/voice/{room}/me", status_code=status.HTTP_204_NO_CONTENT)
async def update_my_voice_state(
    room: str, body: dict, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> None:
    row = await db.get(VoiceParticipant, (room, user.id))
    if row is None:
        raise not_found("参与者")
    if "mic" in body:
        row.mic = bool(body["mic"])
    if "camera" in body:
        row.camera = bool(body["camera"])
    if "screen" in body:
        row.screen = bool(body["screen"])
    await db.commit()
    parts = room.split(":")
    if len(parts) == 3 and parts[0] == "srv":
        await bus.to_server(
            parts[1],
            Event.VOICE_STATE_UPDATE,
            {"room": room, "participants": await _participants(db, room)},
        )


@router.post("/livekit/webhook", status_code=status.HTTP_200_OK)
async def livekit_webhook(request: Request, db: AsyncSession = Depends(get_db)) -> dict:
    """LiveKit 回调：验签 → 同步 voice_participants → 广播。"""
    auth = request.headers.get("authorization", "")
    token = auth.removeprefix("Bearer ").strip()
    raw = await request.body()
    if not token:
        raise unauthorized("缺少 webhook 签名")

    try:
        claims = jwt.decode(token, settings.livekit_api_secret, algorithms=["HS256"])
    except jwt.InvalidTokenError:
        raise unauthorized("webhook 签名无效") from None

    digest = base64.b64encode(hashlib.sha256(raw).digest()).decode()
    if claims.get("sha256") != digest:
        raise unauthorized("webhook 内容校验失败")

    event = json.loads(raw or b"{}")
    kind = event.get("event")
    room = (event.get("room") or {}).get("name", "")
    identity = (event.get("participant") or {}).get("identity", "")
    if not room or not identity:
        return {"ok": True, "ignored": True}

    if kind == "participant_joined":
        if await db.get(VoiceParticipant, (room, identity)) is None:
            db.add(VoiceParticipant(room=room, user_id=identity))
    elif kind == "participant_left":
        row = await db.get(VoiceParticipant, (room, identity))
        if row is not None:
            await db.delete(row)
    elif kind in {"track_published", "track_unpublished"}:
        row = await db.get(VoiceParticipant, (room, identity))
        track = (event.get("track") or {}).get("type", "")
        published = kind == "track_published"
        if row is not None:
            if track == "MICROPHONE":
                row.mic = published
            elif track == "CAMERA":
                row.camera = published
            elif track in {"SCREEN_SHARE", "SCREEN_SHARE_AUDIO"}:
                row.screen = published
    elif kind == "room_finished":
        for row in (await db.scalars(select(VoiceParticipant).where(VoiceParticipant.room == room))).all():
            await db.delete(row)
    else:
        return {"ok": True, "ignored": True}

    row_check = await db.scalar(select(VoiceParticipant).where(VoiceParticipant.room == room).limit(1))
    await db.commit()

    parts = room.split(":")
    if len(parts) == 3 and parts[0] == "srv":
        payload = {"room": room, "participants": await _participants(db, room), "at": utcnow().isoformat()}
        await bus.to_server(parts[1], Event.VOICE_STATE_UPDATE, payload)
    return {"ok": True, "empty": row_check is None and kind != "participant_joined"}
