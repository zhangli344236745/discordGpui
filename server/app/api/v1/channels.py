from dataclasses import dataclass

from fastapi import APIRouter, Depends, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ChannelContext, ServerContext, channel_context, current_user, server_context
from app.errors import conflict, forbidden, not_found
from app.ids import new_id
from app.models import (
    Category,
    Channel,
    ChannelUserState,
    Message,
    ReadState,
    ServerMember,
    User,
)
from app.realtime.bus import bus
from app.realtime.events import Event
from app.schemas.message import (
    CategoryCreate,
    CategoryOut,
    CategoryPatch,
    ChannelCreate,
    ChannelNotifyPatch,
    ChannelOut,
    ChannelPatch,
    ServerTreeOut,
)
from app.services import audit
from app.services.permissions import effective_permissions

router = APIRouter(tags=["channels"])


async def _state(db: AsyncSession, user_id: str, channel_id: str) -> ChannelUserState:
    row = await db.get(ChannelUserState, (user_id, channel_id))
    if row is None:
        row = ChannelUserState(user_id=user_id, channel_id=channel_id)
        db.add(row)
        await db.flush()
    return row


@dataclass
class CategoryContext:
    category: Category
    perms: set[str]
    user: User

    def require(self, permission: str) -> None:
        if permission not in self.perms:
            raise forbidden(permission)


async def category_context(
    category_id: str,
    user: User = Depends(current_user),
    db: AsyncSession = Depends(get_db),
) -> CategoryContext:
    category = await db.get(Category, category_id)
    if category is None:
        raise not_found("分类")
    member = await db.scalar(
        select(ServerMember).where(ServerMember.server_id == category.server_id, ServerMember.user_id == user.id)
    )
    if member is None:
        raise not_found("分类")
    perms = await effective_permissions(db, category.server_id, user.id)
    return CategoryContext(category=category, perms=perms, user=user)


@router.get("/servers/{server_id}/channels", response_model=ServerTreeOut)
async def channel_tree(
    ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)
) -> ServerTreeOut:
    categories = (
        await db.scalars(
            select(Category).where(Category.server_id == ctx.server.id).order_by(Category.position, Category.id)
        )
    ).all()
    channels = (
        await db.scalars(
            select(Channel).where(Channel.server_id == ctx.server.id).order_by(Channel.position, Channel.id)
        )
    ).all()
    states = {
        s.channel_id: s
        for s in (
            await db.scalars(
                select(ChannelUserState).where(
                    ChannelUserState.user_id == ctx.user.id,
                    ChannelUserState.channel_id.in_([c.id for c in channels] or [""]),
                )
            )
        ).all()
    }

    # 分类折叠是纯客户端偏好（不落库，避免给 channels 表塞伪行），这里恒 false
    cat_out = [
        CategoryOut(
            id=c.id,
            server_id=c.server_id,
            name=c.name,
            position=c.position,
            collapsed=False,
        )
        for c in categories
    ]
    ch_out = []
    for ch in channels:
        st = states.get(ch.id)
        ch_out.append(
            ChannelOut(
                id=ch.id,
                server_id=ch.server_id,
                category_id=ch.category_id,
                name=ch.name,
                kind=ch.kind,
                topic=ch.topic,
                position=ch.position,
                notify_level=st.notify_level if st else "inherit",
                last_read_message_id=st.last_read_message_id if st else None,
            )
        )
    return ServerTreeOut(categories=cat_out, channels=ch_out)


@router.post("/servers/{server_id}/categories", response_model=CategoryOut, status_code=201)
async def create_category(
    body: CategoryCreate,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> CategoryOut:
    ctx.require("MANAGE_CHANNELS")
    if await db.scalar(
        select(Category.id).where(Category.server_id == ctx.server.id, Category.name == body.name.strip())
    ):
        raise conflict("已存在同名分类", "name")

    count = len((await db.scalars(select(Category.id).where(Category.server_id == ctx.server.id))).all())
    category = Category(id=new_id(), server_id=ctx.server.id, name=body.name.strip(), position=count)
    db.add(category)
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_CHANNEL, f"创建了分类 / {category.name}")
    await db.commit()
    await bus.to_server(ctx.server.id, Event.CATEGORY_CREATE, {"id": category.id, "name": category.name})
    return CategoryOut.model_validate(category)


@router.patch("/categories/{category_id}", response_model=CategoryOut)
async def patch_category(
    body: CategoryPatch,
    ctx: CategoryContext = Depends(category_context),
    db: AsyncSession = Depends(get_db),
) -> CategoryOut:
    ctx.require("MANAGE_CHANNELS")
    if body.name is not None and body.name.strip():
        old = ctx.category.name
        ctx.category.name = body.name.strip()
        audit.log(
            db,
            ctx.category.server_id,
            ctx.user.id,
            audit.KIND_CHANNEL,
            f"重命名分类 / {old} → {ctx.category.name}",
        )
    await db.commit()
    await bus.to_server(
        ctx.category.server_id,
        Event.CATEGORY_UPDATE,
        {"id": ctx.category.id, "name": ctx.category.name},
    )
    return CategoryOut.model_validate(ctx.category)


@router.delete("/categories/{category_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_category(ctx: CategoryContext = Depends(category_context), db: AsyncSession = Depends(get_db)) -> None:
    ctx.require("MANAGE_CHANNELS")
    for ch in (await db.scalars(select(Channel).where(Channel.category_id == ctx.category.id))).all():
        ch.category_id = None  # 频道转为未分类，不连带删除
    category_id = ctx.category.id
    server_id = ctx.category.server_id
    await db.delete(ctx.category)
    audit.log(db, server_id, ctx.user.id, audit.KIND_CHANNEL, "删除了分类")
    await db.commit()
    await bus.to_server(server_id, Event.CATEGORY_DELETE, {"id": category_id})


@router.post("/servers/{server_id}/channels", response_model=ChannelOut, status_code=201)
async def create_channel(
    body: ChannelCreate,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> ChannelOut:
    ctx.require("MANAGE_CHANNELS")
    name = body.name.strip()
    if await db.scalar(select(Channel.id).where(Channel.server_id == ctx.server.id, Channel.name == name)):
        raise conflict("已存在同名频道", "name")
    if body.category_id is not None:
        cat = await db.get(Category, body.category_id)
        if cat is None or cat.server_id != ctx.server.id:
            raise not_found("分类")

    count = len((await db.scalars(select(Channel.id).where(Channel.server_id == ctx.server.id))).all())
    channel = Channel(
        id=new_id(),
        server_id=ctx.server.id,
        category_id=body.category_id,
        name=name,
        kind=body.kind,
        topic=body.topic,
        position=count,
    )
    db.add(channel)
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_CHANNEL, f"创建了频道 / #{name}")
    await db.commit()
    await bus.to_server(
        ctx.server.id,
        Event.CHANNEL_CREATE,
        {"id": channel.id, "name": name, "kind": channel.kind, "category_id": channel.category_id},
    )
    return ChannelOut.model_validate(channel)


@router.patch("/channels/{channel_id}", response_model=ChannelOut)
async def patch_channel(
    body: ChannelPatch,
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> ChannelOut:
    ctx.require("MANAGE_CHANNELS")
    channel = ctx.channel
    if body.name is not None and body.name.strip() != channel.name:
        old = channel.name
        channel.name = body.name.strip()
        audit.log(db, ctx.server_id, ctx.user.id, audit.KIND_CHANNEL, f"重命名频道 / #{old} → #{channel.name}")
    if body.topic is not None:
        channel.topic = body.topic
        audit.log(db, ctx.server_id, ctx.user.id, audit.KIND_CHANNEL, f"更新了频道主题 / #{channel.name}")
    if body.category_id is not None:
        channel.category_id = body.category_id or None
    if body.position is not None:
        channel.position = body.position
    await db.commit()
    await bus.to_server(
        ctx.server_id,
        Event.CHANNEL_UPDATE,
        {"channel": ChannelOut.model_validate(channel).model_dump()},
    )
    return ChannelOut.model_validate(channel)


@router.delete("/channels/{channel_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_channel(ctx: ChannelContext = Depends(channel_context), db: AsyncSession = Depends(get_db)) -> None:
    ctx.require("MANAGE_CHANNELS")
    if ctx.channel.kind == "text":
        siblings = (
            await db.scalars(
                select(Channel.id).where(
                    Channel.server_id == ctx.server_id,
                    Channel.kind == "text",
                    Channel.id != ctx.channel.id,
                )
            )
        ).all()
        if not siblings:
            raise conflict("至少保留一个文字频道", "kind")

    name = ctx.channel.name
    channel_id = ctx.channel.id
    await db.delete(ctx.channel)
    audit.log(db, ctx.server_id, ctx.user.id, audit.KIND_CHANNEL, f"删除了频道 / #{name}")
    await db.commit()
    await bus.to_server(ctx.server_id, Event.CHANNEL_DELETE, {"id": channel_id})


@router.patch("/channels/{channel_id}/notify", status_code=status.HTTP_204_NO_CONTENT)
async def set_channel_notify(
    body: ChannelNotifyPatch,
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    state = await _state(db, ctx.user.id, ctx.channel.id)
    state.notify_level = body.notify_level
    await db.commit()


@router.post("/channels/{channel_id}/read", status_code=status.HTTP_204_NO_CONTENT)
async def mark_read(
    body: dict,
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    message_id = body.get("message_id") or body.get("last_read_message_id")
    if message_id is None:
        message_id = await db.scalar(
            select(Message.id).where(Message.channel_id == ctx.channel.id).order_by(Message.id.desc()).limit(1)
        )
    state = await _state(db, ctx.user.id, ctx.channel.id)
    state.last_read_message_id = message_id
    read = await db.get(ReadState, (ctx.channel.id, ctx.user.id))
    if read is None:
        db.add(ReadState(channel_id=ctx.channel.id, user_id=ctx.user.id, last_read_message_id=message_id))
    else:
        read.last_read_message_id = message_id
    from app.realtime import outbox
    from app.realtime.events import Event

    outbox.queue(
        db,
        "user",
        ctx.user.id,
        Event.CHANNEL_READ,
        {"channel_id": ctx.channel.id, "last_read_message_id": message_id},
    )
    await db.commit()


@router.patch("/servers/{server_id}/notify", status_code=status.HTTP_204_NO_CONTENT)
async def set_server_notify(
    body: dict,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    level = body.get("notify_level", "all")
    if level not in {"all", "mentions", "none"}:
        raise conflict("通知级别不合法", "notify_level")
    ctx.server.notify_level = level
    await db.commit()
