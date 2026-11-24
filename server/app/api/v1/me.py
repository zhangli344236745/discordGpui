"""用户级视图状态与跨设备草稿。"""

from datetime import UTC, datetime

from fastapi import APIRouter, Depends
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import current_user
from app.errors import bad_request, not_found
from app.models import Channel, ChannelUserState, DmChannel, Draft, ReadState, User
from app.schemas.me import ChannelStatePatch, DraftIn
from app.services.permissions import effective_permissions

router = APIRouter(tags=["me"])


def _iso(value) -> str | None:  # noqa: ANN001
    return value.isoformat().replace("+00:00", "Z") if value else None


async def _visible_channel(db: AsyncSession, user: User, channel_id: str) -> Channel:
    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")
    return channel


async def _channel_state(db: AsyncSession, user_id: str, channel_id: str) -> ChannelUserState:
    state = await db.get(ChannelUserState, (user_id, channel_id))
    if state is None:
        state = ChannelUserState(user_id=user_id, channel_id=channel_id)
        db.add(state)
    return state


@router.get("/me/channel-state")
async def list_channel_state(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    rows = list((await db.scalars(select(ChannelUserState).where(ChannelUserState.user_id == user.id))).all())
    return [
        {
            "channel_id": row.channel_id,
            "collapsed": row.collapsed,
            "notify_level": row.notify_level,
            "last_read_message_id": row.last_read_message_id,
            "updated_at": _iso(row.updated_at),
        }
        for row in rows
    ]


@router.put("/me/channel-state/{channel_id}")
async def update_channel_state(
    channel_id: str,
    body: ChannelStatePatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    await _visible_channel(db, user, channel_id)
    state = await _channel_state(db, user.id, channel_id)
    if body.collapsed is not None:
        state.collapsed = body.collapsed
    if body.notify_level is not None:
        state.notify_level = body.notify_level
    if body.last_read_message_id is not None:
        current = state.last_read_message_id
        if current is None or body.last_read_message_id > current:
            state.last_read_message_id = body.last_read_message_id
            read = await db.get(ReadState, (channel_id, user.id))
            if read is None:
                db.add(
                    ReadState(
                        channel_id=channel_id,
                        user_id=user.id,
                        last_read_message_id=body.last_read_message_id,
                    )
                )
            else:
                read.last_read_message_id = body.last_read_message_id
    state.updated_at = datetime.now(UTC)
    await db.commit()
    return {
        "channel_id": channel_id,
        "collapsed": state.collapsed,
        "notify_level": state.notify_level,
        "last_read_message_id": state.last_read_message_id,
    }


@router.get("/me/drafts")
async def list_drafts(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    rows = list(
        (await db.scalars(select(Draft).where(Draft.user_id == user.id).order_by(Draft.updated_at.desc()))).all()
    )
    return [
        {
            "scope": row.scope,
            "content": row.content,
            "attachments": row.attachments or [],
            "updated_at": _iso(row.updated_at),
        }
        for row in rows
    ]


async def _validate_scope(db: AsyncSession, user: User, scope: str) -> None:
    if scope.startswith("channel:"):
        await _visible_channel(db, user, scope.split(":", 1)[1])
        return
    if scope.startswith("dm:"):
        dm = await db.get(DmChannel, scope.split(":", 1)[1])
        if dm is None or user.id not in (dm.user_a, dm.user_b):
            raise not_found("会话")
        return
    raise bad_request("scope 必须是 channel:<id> 或 dm:<id>", "scope")


@router.put("/me/drafts/{scope_id}")
async def save_draft(
    scope_id: str,
    body: DraftIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    await _validate_scope(db, user, scope_id)
    draft = await db.get(Draft, (user.id, scope_id))
    if not body.content.strip() and not body.attachments:
        if draft is not None:
            await db.delete(draft)
            await db.commit()
        return {"scope": scope_id, "cleared": True}

    if draft is None:
        draft = Draft(user_id=user.id, scope=scope_id, content=body.content, attachments=body.attachments)
        db.add(draft)
    else:
        draft.content = body.content
        draft.attachments = body.attachments
        draft.updated_at = datetime.now(UTC)
    await db.commit()
    return {"scope": scope_id, "cleared": False, "content": draft.content}
