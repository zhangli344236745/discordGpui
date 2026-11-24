"""服务器欢迎屏幕：配置、同意状态与推荐频道已读。"""

from datetime import UTC, datetime

from fastapi import APIRouter, Depends
from sqlalchemy import delete, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ChannelContext, ServerContext, channel_context, server_context
from app.errors import ApiError, ErrorCode, not_found
from app.models import (
    Channel,
    ChannelUserState,
    Message,
    ReadState,
    WelcomeAccept,
    WelcomeRecommended,
    WelcomeRule,
    WelcomeScreen,
)
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.content import WelcomeAcceptIn, WelcomePatch
from app.services import audit
from app.services import content as content_service

router = APIRouter(tags=["welcome"])


async def _screen(db: AsyncSession, server_id: str) -> WelcomeScreen | None:
    return await db.get(WelcomeScreen, server_id)


async def _config(db: AsyncSession, server_id: str, viewer_id: str) -> dict:
    screen = await _screen(db, server_id)
    if screen is None:
        return {"welcome": None, "accepted": False}
    rules = list(
        (
            await db.scalars(
                select(WelcomeRule).where(WelcomeRule.server_id == server_id).order_by(WelcomeRule.position)
            )
        ).all()
    )
    recommended = list(
        (await db.scalars(select(WelcomeRecommended.channel_id).where(WelcomeRecommended.server_id == server_id))).all()
    )
    existing = {
        channel.id: channel
        for channel in (
            await db.scalars(select(Channel).where(Channel.id.in_(recommended or [""])))
        ).all()
    }
    recommended = [channel_id for channel_id in recommended if channel_id in existing]
    accepted = await db.get(WelcomeAccept, (server_id, viewer_id))
    return {
        "welcome": {
            "server_id": server_id,
            "headline": screen.headline,
            "description": screen.description,
            "require_rules": screen.require_rules,
            "rules": [{"title": rule.title, "body": rule.body} for rule in rules],
            "recommended": recommended,
        },
        "accepted": accepted is not None,
        "accepted_at": content_service.iso(accepted.accepted_at) if accepted else None,
    }


@router.get("/servers/{server_id}/welcome")
async def get_welcome(
    ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)
) -> dict:
    return await _config(db, ctx.server.id, ctx.user.id)


@router.patch("/servers/{server_id}/welcome")
async def patch_welcome(
    body: WelcomePatch,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    screen = await _screen(db, ctx.server.id)
    if screen is None:
        screen = WelcomeScreen(server_id=ctx.server.id, headline="", description="", require_rules=True)
        db.add(screen)
    if body.headline is not None:
        screen.headline = body.headline
    if body.description is not None:
        screen.description = body.description
    if body.require_rules is not None:
        screen.require_rules = body.require_rules

    if body.rules is not None:
        await db.execute(delete(WelcomeRule).where(WelcomeRule.server_id == ctx.server.id))
        for position, rule in enumerate(body.rules, start=1):
            db.add(
                WelcomeRule(
                    server_id=ctx.server.id,
                    position=position,
                    title=rule.title.strip(),
                    body=rule.body,
                )
            )
    if body.recommended is not None:
        channels = list(
            (await db.scalars(select(Channel).where(Channel.id.in_(body.recommended or [""])))).all()
        )
        by_id = {channel.id: channel for channel in channels if channel.server_id == ctx.server.id}
        if len(by_id) != len(set(body.recommended)):
            raise not_found("推荐频道")
        await db.execute(
            delete(WelcomeRecommended).where(WelcomeRecommended.server_id == ctx.server.id)
        )
        for channel_id in body.recommended:
            db.add(WelcomeRecommended(server_id=ctx.server.id, channel_id=channel_id))

    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_SERVER, "更新了欢迎屏幕")
    outbox.queue(db, "server", ctx.server.id, Event.WELCOME_UPDATE, {"server_id": ctx.server.id})
    await db.commit()
    return await _config(db, ctx.server.id, ctx.user.id)


@router.post("/servers/{server_id}/welcome/accept")
async def accept_welcome(
    body: WelcomeAcceptIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    screen = await _screen(db, ctx.server.id)
    if screen is None:
        raise not_found("欢迎屏幕")
    if screen.require_rules and not body.rule_agreed:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "需要先同意规则", "rule_agreed")

    if body.channels:
        channels = list(
            (await db.scalars(select(Channel).where(Channel.id.in_(body.channels)))).all()
        )
        if len(channels) != len(set(body.channels)) or any(c.server_id != ctx.server.id for c in channels):
            raise not_found("频道")

    accepted = await db.get(WelcomeAccept, (ctx.server.id, ctx.user.id))
    if accepted is None:
        accepted = WelcomeAccept(server_id=ctx.server.id, user_id=ctx.user.id, accepted_at=datetime.now(UTC))
        db.add(accepted)
    else:
        accepted.accepted_at = datetime.now(UTC)
    outbox.queue(db, "user", ctx.user.id, Event.WELCOME_UPDATE, {"server_id": ctx.server.id, "accepted": True})
    await db.commit()
    return {
        "accepted": True,
        "accepted_at": content_service.iso(accepted.accepted_at),
        "channels": body.channels,
    }


@router.get("/servers/{server_id}/welcome/state")
async def welcome_state(
    ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)
) -> dict:
    screen = await _screen(db, ctx.server.id)
    accepted = await db.get(WelcomeAccept, (ctx.server.id, ctx.user.id))
    return {
        "accepted": accepted is not None,
        "accepted_at": content_service.iso(accepted.accepted_at) if accepted else None,
        "require_rules": screen.require_rules if screen else False,
    }


@router.post("/channels/{channel_id}/welcome-read")
async def welcome_read(
    ctx: ChannelContext = Depends(channel_context), db: AsyncSession = Depends(get_db)
) -> dict:
    latest = await db.scalar(
        select(Message.id)
        .where(Message.channel_id == ctx.channel.id, Message.deleted_at.is_(None))
        .order_by(Message.id.desc())
        .limit(1)
    )
    if latest:
        state = await db.get(ChannelUserState, (ctx.user.id, ctx.channel.id))
        if state is None:
            db.add(ChannelUserState(user_id=ctx.user.id, channel_id=ctx.channel.id, last_read_message_id=latest))
        else:
            state.last_read_message_id = latest
        read = await db.get(ReadState, (ctx.channel.id, ctx.user.id))
        if read is None:
            db.add(ReadState(channel_id=ctx.channel.id, user_id=ctx.user.id, last_read_message_id=latest))
        else:
            read.last_read_message_id = latest
        await db.commit()
    return {"channel_id": ctx.channel.id, "last_read_message_id": latest}
