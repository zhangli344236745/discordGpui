"""投票、话题与书签。"""

from fastapi import APIRouter, Depends, status
from sqlalchemy import delete, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.api.v1.messages import _load_message, serialize
from app.config import settings
from app.db import get_db
from app.deps import channel_context, current_user
from app.errors import ApiError, ErrorCode, conflict, forbidden, not_found
from app.ids import new_id
from app.models import (
    Bookmark,
    Channel,
    DmChannel,
    DmMessage,
    Message,
    Poll,
    PollOption,
    PollVote,
    Thread,
    ThreadReply,
    User,
)
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.content import BookmarkIn, PollCreate, PollVoteIn, ThreadCreate, ThreadReplyIn
from app.services import content as content_service
from app.services import ratelimit
from app.services.permissions import effective_permissions

router = APIRouter(tags=["content"])


async def _poll_scope(db: AsyncSession, poll_id: str, user: User):  # noqa: ANN201
    poll = await db.get(Poll, poll_id)
    if poll is None:
        raise not_found("投票")
    ctx = await _load_message(db, poll.message_id, user)
    return poll, ctx


@router.post("/channels/{channel_id}/polls", status_code=status.HTTP_201_CREATED)
async def create_poll(
    body: PollCreate,
    ctx=Depends(channel_context),  # noqa: ANN001
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
):
    ctx.require("SEND_MESSAGES")
    if ctx.channel.kind != "text":
        raise conflict("该频道不接受投票", "kind")
    labels: list[str] = []
    for raw in body.options:
        label = raw.strip()
        if label and label not in labels:
            labels.append(label)
    if len(labels) < 2:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "至少需要 2 个不同选项", "options")
    ratelimit.check(f"msg:{user.id}", settings.rate_limit_messages, settings.rate_limit_window_seconds)

    message = Message(
        id=new_id(),
        channel_id=ctx.channel.id,
        author_id=user.id,
        content=body.question.strip(),
        msg_type="poll",
    )
    db.add(message)
    await db.flush()
    poll = Poll(
        message_id=message.id,
        question=body.question.strip(),
        multi=body.multi,
        closed=False,
        created_by=user.id,
    )
    db.add(poll)
    await db.flush()
    for position, label in enumerate(labels):
        db.add(PollOption(poll_id=poll.id, label=label, position=position))
    await db.flush()
    message.payload = {"poll_id": poll.id}

    out = (await serialize(db, [message], user.id))[0]
    data = out.model_dump()
    data["server_id"] = ctx.server_id
    outbox.queue(db, "server", ctx.server_id, Event.MESSAGE_CREATE, data)
    await db.commit()
    return out


@router.post("/polls/{poll_id}/votes")
async def vote_poll(
    poll_id: str,
    body: PollVoteIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    poll, ctx = await _poll_scope(db, poll_id, user)
    ctx.require("SEND_MESSAGES")
    if poll.closed:
        raise conflict("投票已结束", "poll_id")
    option = await db.get(PollOption, body.option_id)
    if option is None or option.poll_id != poll.id:
        raise not_found("选项")

    if poll.multi:
        existing = await db.get(PollVote, (poll.id, option.id, user.id))
        if existing is not None:
            await db.delete(existing)
        else:
            db.add(PollVote(poll_id=poll.id, option_id=option.id, user_id=user.id))
    else:
        await db.execute(delete(PollVote).where(PollVote.poll_id == poll.id, PollVote.user_id == user.id))
        db.add(PollVote(poll_id=poll.id, option_id=option.id, user_id=user.id))

    await db.flush()
    payload = await content_service.poll_payload(db, poll, user.id)
    payload["channel_id"] = ctx.channel.id
    outbox.queue(db, "server", ctx.channel.server_id, Event.POLL_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/polls/{poll_id}/votes")
async def clear_poll_vote(
    poll_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    poll, ctx = await _poll_scope(db, poll_id, user)
    if poll.closed:
        raise conflict("投票已结束", "poll_id")
    await db.execute(delete(PollVote).where(PollVote.poll_id == poll.id, PollVote.user_id == user.id))
    await db.flush()
    payload = await content_service.poll_payload(db, poll, user.id)
    payload["channel_id"] = ctx.channel.id
    outbox.queue(db, "server", ctx.channel.server_id, Event.POLL_UPDATE, payload)
    await db.commit()
    return payload


@router.post("/polls/{poll_id}/close")
async def close_poll(
    poll_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    poll, ctx = await _poll_scope(db, poll_id, user)
    if poll.created_by != user.id:
        ctx.require("MANAGE_MESSAGES")
    if poll.closed:
        return await content_service.poll_payload(db, poll, user.id)
    poll.closed = True
    payload = await content_service.poll_payload(db, poll, user.id)
    payload["channel_id"] = ctx.channel.id
    outbox.queue(db, "server", ctx.channel.server_id, Event.POLL_UPDATE, payload)
    await db.commit()
    return payload


# ---------------------------------------------------------------------------
# 话题
# ---------------------------------------------------------------------------


async def _thread_scope(db: AsyncSession, thread_id: str, user: User):  # noqa: ANN201
    thread = await db.get(Thread, thread_id)
    if thread is None:
        raise not_found("话题")
    channel = await db.get(Channel, thread.channel_id)
    if channel is None:
        raise not_found("话题")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("话题")
    return thread, channel, perms


@router.post("/messages/{message_id}/threads", status_code=status.HTTP_201_CREATED)
async def create_thread(
    message_id: str,
    body: ThreadCreate,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    ctx = await _load_message(db, message_id, user)
    ctx.require("SEND_MESSAGES")
    existing = await db.scalar(select(Thread).where(Thread.message_id == message_id))
    if existing is not None:
        raise ApiError(
            409,
            ErrorCode.CONFLICT,
            "该消息已有话题",
            "message_id",
            extra={"thread_id": existing.id},
        )
    title = body.title.strip() or ctx.message.content.strip()[:60] or "话题"
    thread = Thread(
        message_id=message_id,
        channel_id=ctx.channel.id,
        title=title,
        reply_count=0,
    )
    db.add(thread)
    await db.flush()
    payload = content_service.thread_payload(thread)
    outbox.queue(db, "server", ctx.channel.server_id, Event.THREAD_CREATE, payload)
    await db.commit()
    return payload


@router.get("/threads/{thread_id}")
async def get_thread(
    thread_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    thread, channel, _perms = await _thread_scope(db, thread_id, user)
    original = await db.get(Message, thread.message_id)
    original_out = (await serialize(db, [original], user.id))[0].model_dump() if original else None
    replies = list(
        (
            await db.scalars(
                select(ThreadReply).where(ThreadReply.thread_id == thread.id).order_by(ThreadReply.id)
            )
        ).all()
    )
    authors = {
        u.id: u
        for u in (
            await db.scalars(select(User).where(User.id.in_({r.author_id for r in replies} or {""})))
        ).all()
    }
    return {
        "thread": content_service.thread_payload(thread),
        "channel_id": channel.id,
        "original": original_out,
        "replies": [content_service.reply_payload(reply, authors.get(reply.author_id)) for reply in replies],
    }


@router.post("/threads/{thread_id}/replies", status_code=status.HTTP_201_CREATED)
async def reply_thread(
    thread_id: str,
    body: ThreadReplyIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    thread, channel, perms = await _thread_scope(db, thread_id, user)
    if "SEND_MESSAGES" not in perms:
        raise forbidden("SEND_MESSAGES")
    reply = ThreadReply(thread_id=thread.id, author_id=user.id, content=body.content.strip())
    db.add(reply)
    thread.reply_count += 1
    await db.flush()
    payload = content_service.reply_payload(reply, user)
    outbox.queue(
        db,
        "server",
        channel.server_id,
        Event.THREAD_UPDATE,
        {"thread_id": thread.id, "reply_count": thread.reply_count, "channel_id": channel.id},
    )
    await db.commit()
    return payload


# ---------------------------------------------------------------------------
# 书签
# ---------------------------------------------------------------------------


async def _bookmark_item(
    db: AsyncSession, bookmark: Bookmark, user: User
) -> dict | None:
    base = {
        "target_id": bookmark.target_id,
        "kind": bookmark.target_kind,
        "scope": bookmark.scope,
        "note": bookmark.note,
        "saved_at": content_service.iso(bookmark.created_at),
    }
    if bookmark.target_kind == "channel":
        message = await db.get(Message, bookmark.target_id)
        channel = await db.get(Channel, bookmark.scope)
        if channel is None:
            return None
        server_id = channel.server_id
        if message is None or message.deleted_at is not None:
            return {
                **base,
                "server_id": server_id,
                "channel_id": channel.id,
                "channel_name": channel.name,
                "content": "",
                "author": None,
                "deleted": True,
            }
        author = await db.get(User, message.author_id)
        return {
            **base,
            "server_id": server_id,
            "channel_id": channel.id,
            "channel_name": channel.name,
            "content": message.content,
            "author": {
                "id": message.author_id,
                "name": author.display_name if author else "已注销用户",
                "color": author.avatar_color if author else 0x808080,
            },
            "message_created_at": content_service.iso(message.created_at),
            "deleted": False,
        }

    message = await db.get(DmMessage, bookmark.target_id)
    dm = await db.get(DmChannel, bookmark.scope)
    if dm is None:
        return None
    partner_id = dm.user_b if dm.user_a == user.id else dm.user_a
    partner = await db.get(User, partner_id)
    if message is None or message.deleted_at is not None:
        return {
            **base,
            "dm_id": dm.id,
            "peer": {"id": partner_id, "name": partner.display_name if partner else "已注销用户"},
            "content": "",
            "author": None,
            "deleted": True,
        }
    author = await db.get(User, message.author_id)
    return {
        **base,
        "dm_id": dm.id,
        "peer": {"id": partner_id, "name": partner.display_name if partner else "已注销用户"},
        "content": message.content,
        "author": {
            "id": message.author_id,
            "name": author.display_name if author else "已注销用户",
            "color": author.avatar_color if author else 0x808080,
        },
        "message_created_at": content_service.iso(message.created_at),
        "deleted": False,
    }


@router.get("/bookmarks")
async def list_bookmarks(
    q: str = "",
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> list[dict]:
    rows = list(
        (
            await db.scalars(
                select(Bookmark).where(Bookmark.user_id == user.id).order_by(Bookmark.created_at.desc())
            )
        ).all()
    )
    items = [item for bookmark in rows if (item := await _bookmark_item(db, bookmark, user)) is not None]
    if q:
        needle = q.lower()
        items = [item for item in items if needle in (item.get("content") or "").lower()]
    return items


@router.put("/bookmarks/{message_id}")
async def save_bookmark(
    message_id: str,
    body: BookmarkIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    kind = body.kind
    scope = ""

    async def resolve_channel() -> tuple[str, str] | None:
        message = await db.get(Message, message_id)
        if message is None or message.deleted_at is not None:
            return None
        channel = await db.get(Channel, message.channel_id)
        if channel is None:
            return None
        perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
        if "VIEW_CHANNEL" not in perms:
            return None
        return "channel", channel.id

    async def resolve_dm() -> tuple[str, str] | None:
        dm_message = await db.get(DmMessage, message_id)
        if dm_message is None or dm_message.deleted_at is not None:
            return None
        dm = await db.get(DmChannel, dm_message.dm_id)
        if dm is None or user.id not in (dm.user_a, dm.user_b):
            return None
        return "dm", dm.id

    resolved: tuple[str, str] | None = None
    if kind == "channel":
        resolved = await resolve_channel()
    elif kind == "dm":
        resolved = await resolve_dm()
    else:
        resolved = await resolve_channel() or await resolve_dm()
    if resolved is None:
        raise not_found("消息")
    kind, scope = resolved

    existing = await db.get(Bookmark, (user.id, kind, message_id))
    if body.saved and existing is None:
        db.add(Bookmark(user_id=user.id, target_kind=kind, target_id=message_id, scope=scope))
    elif not body.saved and existing is not None:
        await db.delete(existing)

    payload = {"message_id": message_id, "kind": kind, "saved": body.saved}
    outbox.queue(db, "user", user.id, Event.BOOKMARK_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/bookmarks/{bookmark_id}")
async def remove_bookmark(
    bookmark_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    await db.execute(
        delete(Bookmark).where(Bookmark.user_id == user.id, Bookmark.target_id == bookmark_id)
    )
    outbox.queue(db, "user", user.id, Event.BOOKMARK_UPDATE, {"message_id": bookmark_id, "saved": False})
    await db.commit()
    return {"removed": True, "message_id": bookmark_id}
