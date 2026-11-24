"""服务器加成：次数、等级、特权与加成撤销。"""

from fastapi import APIRouter, Depends
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ServerContext, server_context
from app.errors import ApiError, ErrorCode
from app.models import Boost, User
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.content import BoostToggleIn
from app.services import audit, boost
from app.services import content as content_service

router = APIRouter(tags=["boosts"])

TIERS = (
    {
        "level": 1,
        "threshold": boost.BOOST_TIERS[0],
        "title": "等级 1",
        "perks": ["150 个自定义表情", "语音音质提升", "专属加成徽标"],
    },
    {
        "level": 2,
        "threshold": boost.BOOST_TIERS[1],
        "title": "等级 2",
        "perks": ["服务器横幅", "上传上限 50MB", "自定义邀请背景"],
    },
    {
        "level": 3,
        "threshold": boost.BOOST_TIERS[2],
        "title": "等级 3",
        "perks": ["自定义服务器 URL", "上传上限 100MB", "最高语音音质"],
    },
)


async def _boosters(db: AsyncSession, server_id: str) -> list[dict]:
    rows = (
        await db.execute(select(Boost, User).join(User, User.id == Boost.user_id).where(Boost.server_id == server_id))
    ).all()
    return [
        {
            "user_id": user.id,
            "display_name": user.display_name,
            "avatar_color": user.avatar_color,
            "created_at": content_service.iso(row.created_at),
        }
        for row, user in rows
    ]


async def _payload(db: AsyncSession, server_id: str, viewer_id: str) -> dict:
    boosts = await _boosters(db, server_id)
    stats = boost.boost_stats(len(boosts))
    return {
        "server_id": server_id,
        "count": stats.count,
        "level": stats.level,
        "next_threshold": stats.next_threshold,
        "remaining": stats.remaining,
        "progress": stats.progress,
        "perks_unlocked": stats.perks_unlocked,
        "boosters": boosts,
        "boosted": viewer_id in {item["user_id"] for item in boosts},
        "tiers": list(TIERS),
    }


@router.get("/servers/{server_id}/boosts")
async def get_boosts(ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)) -> dict:
    return await _payload(db, ctx.server.id, ctx.user.id)


@router.put("/servers/{server_id}/boosts/me")
async def toggle_boost(
    body: BoostToggleIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    row = await db.get(Boost, (ctx.server.id, ctx.user.id))
    if body.active and row is None:
        db.add(Boost(server_id=ctx.server.id, user_id=ctx.user.id))
    elif not body.active and row is not None:
        await db.delete(row)
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_BOOST, "加成" if body.active else "撤销加成")
    await db.flush()
    payload = await _payload(db, ctx.server.id, ctx.user.id)
    outbox.queue(db, "server", ctx.server.id, Event.BOOST_UPDATE, payload)
    await db.commit()
    return payload


@router.get("/servers/{server_id}/boost/tiers")
async def boost_tiers(ctx: ServerContext = Depends(server_context)) -> list[dict]:
    return list(TIERS)


@router.post("/servers/{server_id}/boost/subscribe")
async def subscribe_boost(ctx: ServerContext = Depends(server_context)) -> dict:
    raise ApiError(
        501,
        ErrorCode.NOT_IMPLEMENTED,
        "订阅计费在 Demo 阶段未实现，请使用 /boosts/me 直接加成",
    )
