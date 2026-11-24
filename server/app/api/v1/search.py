"""全局 / 会话内搜索与补全。"""

from fastapi import APIRouter, Depends, Query
from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import current_user
from app.errors import ApiError, ErrorCode
from app.models import Channel, Friendship, ServerMember, User
from app.services import search

router = APIRouter(tags=["search"])

COMMANDS = (
    {"value": "/gif", "label": "搜索 GIF"},
    {"value": "/sticker", "label": "发送贴纸"},
    {"value": "/poll", "label": "发起投票"},
    {"value": "/invite", "label": "创建邀请"},
)


@router.get("/search")
async def global_search(
    q: str = "",
    scope: str = Query(default="all", pattern="^(server|all|dm|everywhere)$"),
    server_id: str = "",
    before: str | None = None,
    limit: int = Query(default=25, le=50),
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    if not q.strip():
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "搜索词不能为空", "q")

    visible = await search.visible_channels(db, user.id)
    if scope == "server" and server_id:
        ids = visible.get(server_id, set())
    elif scope == "dm":
        ids = set()
    else:
        ids = set().union(*visible.values()) if visible else set()

    items: list[dict] = []
    if ids:
        items += await search.search_messages(db, user, q, channel_ids=ids, before=before, limit=limit)
    if scope in ("dm", "everywhere"):
        dm_ids = await search.dm_ids_for(db, user.id)
        items += await search.search_dms(db, user, q, dm_ids=dm_ids, before=before, limit=limit)

    items.sort(key=lambda item: item["created_at"], reverse=True)
    return {"items": items[:limit], "has_more": len(items) > limit}


@router.get("/channels/{channel_id}/search")
async def channel_search(
    channel_id: str,
    q: str = "",
    before: str | None = None,
    limit: int = Query(default=25, le=50),
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    if not q.strip():
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "搜索词不能为空", "q")
    await search.require_visible_channel(db, user, channel_id)
    items = await search.search_messages(db, user, q, channel_ids={channel_id}, before=before, limit=limit)
    return {"items": items, "has_more": len(items) >= limit}


@router.get("/search/suggest")
async def suggest(
    q: str = "",
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> list[dict]:
    needle = q.strip()
    if not needle:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "搜索词不能为空", "q")

    visible = await search.visible_channels(db, user.id)
    channel_ids = set().union(*visible.values()) if visible else set()
    channels = (
        await db.scalars(
            select(Channel).where(
                Channel.id.in_(channel_ids or {""}),
                func.lower(Channel.name).like(f"%{needle.lower()}%"),
            )
        )
    ).all() if channel_ids else []

    friend_ids = set(
        (await db.scalars(select(Friendship.friend_id).where(Friendship.user_id == user.id))).all()
    )
    member_ids = set(
        (await db.scalars(select(ServerMember.user_id).where(ServerMember.server_id.in_(visible or {""})))).all()
    ) if visible else set()
    users = (
        await db.scalars(
            select(User).where(
                User.id.in_(friend_ids | member_ids or {""}),
                (func.lower(User.username).like(f"%{needle.lower()}%"))
                | (func.lower(User.display_name).like(f"%{needle.lower()}%")),
            )
        )
    ).all()

    items: list[dict] = []
    for channel in channels:
        items.append(
            {
                "type": "channel",
                "id": channel.id,
                "label": f"#{channel.name}",
                "subtitle": channel.kind,
                "weight": 0 if channel.name.lower().startswith(needle.lower()) else 1,
            }
        )
    for member in users:
        items.append(
            {
                "type": "user",
                "id": member.id,
                "label": f"{member.username}#{member.tag}",
                "subtitle": member.display_name,
                "weight": 2 if not member.username.lower().startswith(needle.lower()) else 1,
            }
        )
    for command in COMMANDS:
        if needle.lower() in command["value"] or needle.lower() in command["label"].lower():
            items.append(
                {
                    "type": "command",
                    "id": command["value"],
                    "label": command["value"],
                    "subtitle": command["label"],
                    "weight": 3,
                }
            )
    items.sort(key=lambda item: (item["weight"], len(item["label"])))
    return items[:20]
