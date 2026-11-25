from dataclasses import dataclass

from fastapi import APIRouter, Depends, Query, status
from sqlalchemy import select
from sqlalchemy.exc import IntegrityError
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db, utcnow
from app.deps import current_user, principal
from app.errors import ApiError, ErrorCode, conflict, forbidden, not_found
from app.ids import new_id
from app.models import (
    Attachment,
    Channel,
    ChannelUserState,
    DmChannel,
    Message,
    MessageEdit,
    Poll,
    PollOption,
    PollVote,
    Reaction,
    ReadState,
    ServerMember,
    User,
)
from app.ops import MESSAGES_TOTAL
from app.realtime.bus import bus
from app.realtime.events import Event
from app.schemas.message import MessageCreate, MessageOut, MessagePatch, ReactionIn
from app.services import automod, notifications, ratelimit
from app.services.permissions import effective_permissions
from app.services.storage import storage

router = APIRouter(tags=["messages"])


@dataclass
class MessageContext:
    message: Message
    channel: Channel
    perms: set[str]
    user: User

    def require(self, permission: str) -> None:
        if permission not in self.perms:
            raise forbidden(permission)

    @property
    def is_author(self) -> bool:
        return self.message.author_id == self.user.id


async def _load_message(db: AsyncSession, message_id: str, user: User) -> MessageContext:
    msg = await db.get(Message, message_id)
    if msg is None or msg.deleted_at is not None:
        raise not_found("消息")
    channel = await db.get(Channel, msg.channel_id)
    if channel is None:
        raise not_found("消息")
    member = await db.scalar(
        select(ServerMember).where(ServerMember.server_id == channel.server_id, ServerMember.user_id == user.id)
    )
    if member is None:
        raise not_found("消息")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("消息")
    return MessageContext(message=msg, channel=channel, perms=perms, user=user)


async def serialize(db: AsyncSession, msgs: list[Message], viewer_id: str) -> list[MessageOut]:
    """批量序列化：作者 / 回应 / 附件各查一次，避免 N+1。"""
    if not msgs:
        return []

    author_ids = {m.author_id for m in msgs}
    users = {u.id: u for u in (await db.scalars(select(User).where(User.id.in_(author_ids)))).all()}
    msg_ids = [m.id for m in msgs]
    reactions = (await db.scalars(select(Reaction).where(Reaction.message_id.in_(msg_ids)))).all()
    attachments = (await db.scalars(select(Attachment).where(Attachment.message_id.in_(msg_ids)))).all()
    polls = list((await db.scalars(select(Poll).where(Poll.message_id.in_(msg_ids)))).all())
    poll_options = (
        list(
            (
                await db.scalars(
                    select(PollOption)
                    .where(PollOption.poll_id.in_([p.id for p in polls] or [""]))
                    .order_by(PollOption.position)
                )
            ).all()
        )
        if polls
        else []
    )
    poll_votes = (
        list((await db.scalars(select(PollVote).where(PollVote.poll_id.in_([p.id for p in polls])))).all())
        if polls
        else []
    )
    votes_by_option: dict[str, int] = {}
    mine: set[str] = set()
    for vote in poll_votes:
        votes_by_option[vote.option_id] = votes_by_option.get(vote.option_id, 0) + 1
        if vote.user_id == viewer_id:
            mine.add(vote.option_id)
    poll_by_message = {p.message_id: p for p in polls}
    poll_payload: dict[str, dict] = {}
    for poll in polls:
        options = [
            {
                "id": option.id,
                "label": option.label,
                "position": option.position,
                "votes": votes_by_option.get(option.id, 0),
                "me": option.id in mine,
            }
            for option in poll_options
            if option.poll_id == poll.id
        ]
        poll_payload[poll.message_id] = {
            "id": poll.id,
            "question": poll.question,
            "multi": poll.multi,
            "closed": poll.closed,
            "created_by": poll.created_by,
            "total_votes": sum(option["votes"] for option in options),
            "options": options,
        }

    reaction_map: dict[str, dict[str, dict]] = {}
    for r in reactions:
        bucket = reaction_map.setdefault(r.message_id, {})
        item = bucket.setdefault(r.emoji, {"emoji": r.emoji, "count": 0, "me": False})
        item["count"] += 1
        if r.user_id == viewer_id:
            item["me"] = True

    attachment_map: dict[str, list[dict]] = {}
    for a in attachments:
        if not a.message_id:
            continue
        item = {
            "id": a.id,
            "filename": a.filename,
            "mime": a.mime,
            "size": a.size,
            "url": storage.url_for(a.storage_key),
        }
        if a.thumb_storage_key:
            item["thumbnail_url"] = storage.url_for(a.thumb_storage_key)
        attachment_map.setdefault(a.message_id, []).append(item)

    out: list[MessageOut] = []
    for m in msgs:
        author = users.get(m.author_id)
        out.append(
            MessageOut(
                id=m.id,
                channel_id=m.channel_id,
                author={
                    "id": m.author_id,
                    "name": author.display_name if author else "已注销用户",
                    "color": author.avatar_color if author else 0x808080,
                    "bot": bool(author.is_bot) if author else False,
                },
                content=m.content,
                type=m.msg_type,
                payload=m.payload,
                reply_to=m.reply_to,
                client_nonce=m.client_nonce,
                pinned=m.pinned,
                edited_at=m.edited_at.isoformat() if m.edited_at else None,
                created_at=m.created_at.isoformat(),
                reactions=list(reaction_map.get(m.id, {}).values()),
                attachments=attachment_map.get(m.id, []),
                poll=poll_payload.get(m.id) if m.id in poll_by_message else None,
            )
        )
    return out


@router.get("/channels/{channel_id}/messages")
async def list_messages(
    channel_id: str,
    before: str | None = None,
    after: str | None = None,
    limit: int = Query(default=50, le=100),
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")

    stmt = select(Message).where(Message.channel_id == channel_id, Message.deleted_at.is_(None))
    if before:
        stmt = stmt.where(Message.id < before)
    if after:
        stmt = stmt.where(Message.id > after)
    ordering = Message.id.asc() if after else Message.id.desc()
    rows = list((await db.scalars(stmt.order_by(ordering).limit(limit + 1))).all())
    has_more = len(rows) > limit
    rows = rows[:limit]
    if not after:
        rows.reverse()  # 统一按时间正序返回

    return {"items": await serialize(db, rows, user.id), "has_more": has_more}


@router.post("/channels/{channel_id}/messages", status_code=status.HTTP_201_CREATED)
async def send_message(
    channel_id: str,
    body: MessageCreate,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(principal),
) -> MessageOut:
    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")
    if "SEND_MESSAGES" not in perms:
        raise forbidden("SEND_MESSAGES")
    if channel.kind != "text":
        raise conflict("该频道不接受消息", "kind")

    ratelimit.check(
        f"msg:{user.id}", settings.rate_limit_messages, settings.rate_limit_window_seconds
    )
    if body.client_nonce:
        existing = await db.scalar(select(Message).where(Message.client_nonce == body.client_nonce))
        if existing is not None:
            return (await serialize(db, [existing], user.id))[0]

    if not body.content.strip() and not body.attachment_ids:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "消息不能为空", "content")

    # 自动审核：block 直接拒绝且不落库
    scan = await automod.scan(db, channel.server_id, body.content)
    if scan.is_blocked:
        assert scan.blocked is not None
        raise ApiError(
            422,
            ErrorCode.AUTOMOD_BLOCKED,
            f"消息被自动审核拦截：{scan.blocked.rule_name}",
            "content",
            extra={"rule_id": scan.blocked.rule_id, "matched_keyword": scan.blocked.keyword},
        )

    msg_type = "text"
    payload: dict | None = None
    if body.attachment_ids:
        msg_type = "attachment"
        payload = {"attachment_ids": body.attachment_ids}

    viewer_id = user.id
    message = Message(
        id=new_id(),
        channel_id=channel.id,
        author_id=user.id,
        content=body.content,
        reply_to=body.reply_to,
        msg_type=msg_type,
        payload=payload,
        client_nonce=body.client_nonce,
    )
    db.add(message)
    try:
        await db.flush()
    except IntegrityError:
        await db.rollback()
        existing = (
            await db.scalar(select(Message).where(Message.client_nonce == body.client_nonce))
            if body.client_nonce
            else None
        )
        if existing is None:
            raise
        return (await serialize(db, [existing], viewer_id))[0]

    for att_id in body.attachment_ids:
        att = await db.get(Attachment, att_id)
        if att is not None and att.owner_id == user.id and att.message_id is None:
            att.message_id = message.id

    # 发消息顺带把自己读到最新
    state = await db.get(ChannelUserState, (user.id, channel.id))
    if state is None:
        db.add(ChannelUserState(user_id=user.id, channel_id=channel.id, last_read_message_id=message.id))
    else:
        state.last_read_message_id = message.id
    read = await db.get(ReadState, (channel.id, user.id))
    if read is not None:
        read.last_read_message_id = message.id

    await notifications.channel_mentions(
        db, message, channel, user, can_everyone="MENTION_EVERYONE" in perms
    )
    try:
        await db.commit()
    except IntegrityError:
        # 并发同 client_nonce：唯一索引兜底，回滚后返回已存在那条
        await db.rollback()
        existing = (
            await db.scalar(select(Message).where(Message.client_nonce == body.client_nonce))
            if body.client_nonce
            else None
        )
        if existing is None:
            raise
        return (await serialize(db, [existing], viewer_id))[0]

    MESSAGES_TOTAL.labels("channel").inc()
    out = (await serialize(db, [message], user.id))[0]
    if scan.warnings:
        out.warnings = [{"rule": w.rule_name, "keyword": w.keyword, "rule_id": w.rule_id} for w in scan.warnings]
    data = out.model_dump()
    data["server_id"] = channel.server_id
    await bus.to_server(channel.server_id, Event.MESSAGE_CREATE, data)
    return out


@router.get("/messages/{message_id}", response_model=MessageOut)
async def get_message(
    message_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> MessageOut:
    ctx = await _load_message(db, message_id, user)
    return (await serialize(db, [ctx.message], user.id))[0]


@router.patch("/messages/{message_id}", response_model=MessageOut)
async def edit_message(
    message_id: str,
    body: MessagePatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> MessageOut:
    ctx = await _load_message(db, message_id, user)
    if not ctx.is_author and "MANAGE_MESSAGES" not in ctx.perms:
        raise forbidden("MANAGE_MESSAGES")
    if body.version is not None and body.version != ctx.message.version:
        raise conflict("消息已被修改，请刷新", "version")

    scan = await automod.scan(db, ctx.channel.server_id, body.content)
    if scan.is_blocked:
        assert scan.blocked is not None
        raise ApiError(
            422,
            ErrorCode.AUTOMOD_BLOCKED,
            f"消息被自动审核拦截：{scan.blocked.rule_name}",
            "content",
            extra={"rule_id": scan.blocked.rule_id, "matched_keyword": scan.blocked.keyword},
        )

    if body.content != ctx.message.content:
        db.add(
            MessageEdit(
                id=new_id(),
                message_id=ctx.message.id,
                content=ctx.message.content,
                edited_by=user.id,
            )
        )
        ctx.message.content = body.content
        ctx.message.edited_at = utcnow()
        ctx.message.version += 1
        await db.commit()

    out = (await serialize(db, [ctx.message], user.id))[0]
    if scan.warnings:
        out.warnings = [
            {"rule": w.rule_name, "keyword": w.keyword, "rule_id": w.rule_id} for w in scan.warnings
        ]
    await bus.to_server(
        ctx.channel.server_id,
        Event.MESSAGE_UPDATE,
        {"message": out.model_dump(), "channel_id": ctx.channel.id},
    )
    return out


@router.delete("/messages/{message_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_message(
    message_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> None:
    ctx = await _load_message(db, message_id, user)
    if not ctx.is_author and "MANAGE_MESSAGES" not in ctx.perms:
        raise forbidden("MANAGE_MESSAGES")
    ctx.message.deleted_at = utcnow()
    await db.commit()
    await bus.to_server(
        ctx.channel.server_id,
        Event.MESSAGE_DELETE,
        {"id": message_id, "channel_id": ctx.channel.id},
    )


@router.get("/messages/{message_id}/edits")
async def edit_history(
    message_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> list[dict]:
    await _load_message(db, message_id, user)  # 权限门禁
    rows = (
        await db.scalars(
            select(MessageEdit).where(MessageEdit.message_id == message_id).order_by(MessageEdit.id.desc())
        )
    ).all()
    return [{"content": r.content, "edited_by": r.edited_by, "created_at": r.created_at.isoformat()} for r in rows]


@router.put("/messages/{message_id}/pin", response_model=MessageOut)
async def toggle_pin(
    message_id: str,
    pinned: bool = True,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> MessageOut:
    ctx = await _load_message(db, message_id, user)
    ctx.require("MANAGE_MESSAGES")
    ctx.message.pinned = pinned
    await db.commit()
    out = (await serialize(db, [ctx.message], user.id))[0]
    await bus.to_server(
        ctx.channel.server_id,
        Event.MESSAGE_UPDATE,
        {"message": out.model_dump(), "channel_id": ctx.channel.id},
    )
    return out


@router.get("/channels/{channel_id}/pins")
async def list_pins(
    channel_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> list[dict]:
    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")
    rows = (
        await db.scalars(
            select(Message)
            .where(
                Message.channel_id == channel_id,
                Message.pinned.is_(True),
                Message.deleted_at.is_(None),
            )
            .order_by(Message.id.desc())
        )
    ).all()
    return [m.model_dump() for m in await serialize(db, list(rows), user.id)]


@router.post("/messages/{message_id}/reactions", status_code=status.HTTP_204_NO_CONTENT)
async def add_reaction(
    message_id: str,
    body: ReactionIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    ctx = await _load_message(db, message_id, user)
    existing = await db.get(Reaction, (message_id, user.id, body.emoji))
    if existing is None:
        db.add(Reaction(message_id=message_id, user_id=user.id, emoji=body.emoji))
        await db.commit()
        await bus.to_server(
            ctx.channel.server_id,
            Event.MESSAGE_REACTION_ADD,
            {"message_id": message_id, "emoji": body.emoji, "user_id": user.id},
        )


@router.delete("/messages/{message_id}/reactions/{emoji}", status_code=status.HTTP_204_NO_CONTENT)
async def remove_reaction(
    message_id: str,
    emoji: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    ctx = await _load_message(db, message_id, user)
    existing = await db.get(Reaction, (message_id, user.id, emoji))
    if existing is not None:
        await db.delete(existing)
        await db.commit()
        await bus.to_server(
            ctx.channel.server_id,
            Event.MESSAGE_REACTION_REMOVE,
            {"message_id": message_id, "emoji": emoji, "user_id": user.id},
        )


@router.post("/messages/{message_id}/forward", response_model=MessageOut, status_code=201)
async def forward_message(
    message_id: str,
    body: dict,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> MessageOut:
    ctx = await _load_message(db, message_id, user)
    target_id = body.get("target_channel_id") or body.get("target_dm_id")
    if not target_id:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "缺少目标会话", "target_channel_id")

    origin = {
        "channel_id": ctx.channel.id,
        "channel_name": ctx.channel.name,
        "server_id": ctx.channel.server_id,
        "author": (await db.get(User, ctx.message.author_id)).display_name
        if await db.get(User, ctx.message.author_id)
        else "",
    }

    target_channel = await db.get(Channel, target_id)
    dm = None if target_channel else await db.get(DmChannel, target_id)
    if target_channel is None and dm is None:
        raise not_found("目标会话")

    if target_channel is not None:
        perms = await effective_permissions(db, target_channel.server_id, user.id, target_channel.id)
        if "SEND_MESSAGES" not in perms:
            raise forbidden("SEND_MESSAGES")
        forwarded = Message(
            id=new_id(),
            channel_id=target_channel.id,
            author_id=user.id,
            content=ctx.message.content,
            msg_type="forward",
            payload={"forward_from": origin},
        )
        db.add(forwarded)
        await db.commit()
        out = (await serialize(db, [forwarded], user.id))[0]
        data = out.model_dump()
        data["server_id"] = target_channel.server_id
        await bus.to_server(target_channel.server_id, Event.MESSAGE_CREATE, data)
        return out

    # 私信转发：Demo 阶段先拒绝，DM 消息表结构不同，留给 P1 的 dms 路由
    raise ApiError(501, ErrorCode.NOT_IMPLEMENTED, "私信转发尚未接入（P1）", "target_dm_id")


@router.get("/channels/{channel_id}/read-states")
async def read_states(
    channel_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> list[dict]:
    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")
    rows = (await db.scalars(select(ReadState).where(ReadState.channel_id == channel_id))).all()
    return [
        {
            "user_id": r.user_id,
            "last_read_message_id": r.last_read_message_id,
            "updated_at": r.updated_at.isoformat(),
        }
        for r in rows
    ]


@router.post("/channels/{channel_id}/typing", status_code=status.HTTP_202_ACCEPTED)
async def typing(channel_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    channel = await db.get(Channel, channel_id)
    if channel is None:
        raise not_found("频道")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("频道")
    await bus.to_server(
        channel.server_id,
        Event.TYPING_START,
        {"channel_id": channel_id, "user_id": user.id, "name": user.display_name},
    )
    return {"ok": True}
