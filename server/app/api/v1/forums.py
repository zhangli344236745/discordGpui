"""论坛频道：帖子、回帖与标签。"""

from datetime import UTC, datetime

from fastapi import APIRouter, Depends, Query
from sqlalchemy import delete, func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ChannelContext, channel_context, current_user
from app.errors import bad_request, forbidden, not_found
from app.models import Channel, ForumPost, ForumReply, ForumTag, User
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.content import ForumPostCreate, ForumPostPatch, ForumReplyIn
from app.services import content as content_service
from app.services.permissions import effective_permissions

router = APIRouter(tags=["forums"])


def _forum_channel(ctx: ChannelContext) -> None:
    if ctx.channel.kind != "forum":
        raise not_found("论坛频道")


async def _post_scope(
    db: AsyncSession, post_id: str, user: User
) -> tuple[ForumPost, Channel, set[str]]:
    post = await db.get(ForumPost, post_id)
    if post is None:
        raise not_found("帖子")
    channel = await db.get(Channel, post.channel_id)
    if channel is None:
        raise not_found("帖子")
    perms = await effective_permissions(db, channel.server_id, user.id, channel.id)
    if "VIEW_CHANNEL" not in perms:
        raise not_found("帖子")
    return post, channel, perms


async def _tags(db: AsyncSession, post_id: str) -> list[str]:
    return list(
        (await db.scalars(select(ForumTag.tag).where(ForumTag.post_id == post_id).order_by(ForumTag.tag))).all()
    )


async def _post_out(db: AsyncSession, post: ForumPost, tags: list[str] | None = None) -> dict:
    author = await db.get(User, post.author_id)
    return {
        "id": post.id,
        "channel_id": post.channel_id,
        "title": post.title,
        "body": post.body,
        "pinned": post.pinned,
        "locked": post.locked,
        "reply_count": post.reply_count,
        "tags": tags if tags is not None else await _tags(db, post.id),
        "author": {
            "id": post.author_id,
            "name": author.display_name if author else "已注销用户",
            "color": author.avatar_color if author else 0x808080,
        },
        "created_at": content_service.iso(post.created_at),
        "last_activity_at": content_service.iso(post.last_activity_at),
    }


@router.get("/channels/{channel_id}/posts")
async def list_posts(
    q: str = "",
    sort: str = Query(default="recent", pattern="^(recent|popular|unanswered)$"),
    before: str | None = None,
    limit: int = Query(default=20, le=50),
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    _forum_channel(ctx)
    stmt = select(ForumPost).where(ForumPost.channel_id == ctx.channel.id)
    if q:
        needle = f"%{q.lower()}%"
        stmt = stmt.where(
            func.lower(ForumPost.title).like(needle) | func.lower(ForumPost.body).like(needle)
        )
    if sort == "unanswered":
        stmt = stmt.where(ForumPost.reply_count == 0)
    if before:
        stmt = stmt.where(ForumPost.id < before)

    order = [ForumPost.pinned.desc()]
    if sort == "popular":
        order += [ForumPost.reply_count.desc(), ForumPost.last_activity_at.desc()]
    else:
        order += [ForumPost.last_activity_at.desc()]
    rows = list((await db.scalars(stmt.order_by(*order, ForumPost.id.desc()).limit(limit + 1))).all())
    has_more = len(rows) > limit
    rows = rows[:limit]

    tags_rows = (
        await db.execute(
            select(ForumTag.post_id, ForumTag.tag).where(ForumTag.post_id.in_([p.id for p in rows] or [""]))
        )
    ).all()
    tags_by_post: dict[str, list[str]] = {}
    for post_id, tag in tags_rows:
        tags_by_post.setdefault(post_id, []).append(tag)
    authors = {
        u.id: u
        for u in (await db.scalars(select(User).where(User.id.in_({p.author_id for p in rows} or {""})))).all()
    }
    items = []
    for post in rows:
        author = authors.get(post.author_id)
        items.append(
            {
                "id": post.id,
                "channel_id": post.channel_id,
                "title": post.title,
                "body": post.body,
                "pinned": post.pinned,
                "locked": post.locked,
                "reply_count": post.reply_count,
                "tags": tags_by_post.get(post.id, []),
                "author": {
                    "id": post.author_id,
                    "name": author.display_name if author else "已注销用户",
                    "color": author.avatar_color if author else 0x808080,
                },
                "created_at": content_service.iso(post.created_at),
                "last_activity_at": content_service.iso(post.last_activity_at),
            }
        )
    return {"items": items, "has_more": has_more}


@router.post("/channels/{channel_id}/posts", status_code=201)
async def create_post(
    body: ForumPostCreate,
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    _forum_channel(ctx)
    ctx.require("SEND_MESSAGES")
    tags: list[str] = []
    for raw in body.tags:
        tag = raw.strip()
        if tag and tag not in tags:
            tags.append(tag)
    if len(tags) > 5:
        raise bad_request("标签最多 5 个", "tags")

    post = ForumPost(
        channel_id=ctx.channel.id,
        author_id=ctx.user.id,
        title=body.title.strip(),
        body=body.body,
        pinned=False,
        locked=False,
        reply_count=0,
        last_activity_at=datetime.now(UTC),
    )
    db.add(post)
    await db.flush()
    for tag in tags:
        db.add(ForumTag(post_id=post.id, tag=tag))
    payload = await _post_out(db, post, tags)
    outbox.queue(db, "server", ctx.server_id, Event.FORUM_POST_CREATE, payload)
    await db.commit()
    return payload


@router.get("/posts/{post_id}")
async def get_post(
    post_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> dict:
    post, channel, _perms = await _post_scope(db, post_id, user)
    replies = list(
        (await db.scalars(select(ForumReply).where(ForumReply.post_id == post.id).order_by(ForumReply.id))).all()
    )
    authors = {
        u.id: u
        for u in (await db.scalars(select(User).where(User.id.in_({r.author_id for r in replies} or {""})))).all()
    }
    return {
        "post": await _post_out(db, post),
        "channel_id": channel.id,
        "replies": [
            {
                "id": reply.id,
                "post_id": reply.post_id,
                "body": reply.body,
                "author": {
                    "id": reply.author_id,
                    "name": authors[reply.author_id].display_name if reply.author_id in authors else "已注销用户",
                    "color": authors[reply.author_id].avatar_color if reply.author_id in authors else 0x808080,
                },
                "created_at": content_service.iso(reply.created_at),
            }
            for reply in replies
        ],
    }


@router.patch("/posts/{post_id}")
async def patch_post(
    post_id: str,
    body: ForumPostPatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    post, channel, perms = await _post_scope(db, post_id, user)
    if post.author_id != user.id and "MANAGE_MESSAGES" not in perms:
        raise forbidden("MANAGE_MESSAGES")
    if body.title is not None:
        post.title = body.title.strip()
    if body.body is not None:
        post.body = body.body
    if body.pinned is not None:
        post.pinned = body.pinned
    if body.locked is not None:
        post.locked = body.locked
    payload = await _post_out(db, post)
    outbox.queue(db, "server", channel.server_id, Event.FORUM_POST_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/posts/{post_id}")
async def delete_post(
    post_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    post, channel, perms = await _post_scope(db, post_id, user)
    if post.author_id != user.id and "MANAGE_MESSAGES" not in perms:
        raise forbidden("MANAGE_MESSAGES")
    await db.execute(delete(ForumReply).where(ForumReply.post_id == post.id))
    await db.execute(delete(ForumTag).where(ForumTag.post_id == post.id))
    await db.delete(post)
    outbox.queue(
        db,
        "server",
        channel.server_id,
        Event.FORUM_POST_DELETE,
        {"id": post_id, "channel_id": channel.id},
    )
    await db.commit()
    return {"deleted": True}


@router.post("/posts/{post_id}/replies", status_code=201)
async def create_forum_reply(
    post_id: str,
    body: ForumReplyIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    post, channel, perms = await _post_scope(db, post_id, user)
    if post.locked:
        raise forbidden("SEND_MESSAGES")
    if "SEND_MESSAGES" not in perms:
        raise forbidden("SEND_MESSAGES")
    reply = ForumReply(post_id=post.id, author_id=user.id, body=body.body.strip())
    db.add(reply)
    post.reply_count += 1
    post.last_activity_at = datetime.now(UTC)
    await db.flush()
    payload = {
        "id": reply.id,
        "post_id": post.id,
        "body": reply.body,
        "author": {
            "id": user.id,
            "name": user.display_name,
            "color": user.avatar_color,
        },
        "created_at": content_service.iso(reply.created_at),
        "reply_count": post.reply_count,
    }
    outbox.queue(db, "server", channel.server_id, Event.FORUM_REPLY_CREATE, payload)
    await db.commit()
    return payload


@router.delete("/replies/{reply_id}")
async def delete_forum_reply(
    reply_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    reply = await db.get(ForumReply, reply_id)
    if reply is None:
        raise not_found("回帖")
    post, channel, perms = await _post_scope(db, reply.post_id, user)
    if reply.author_id != user.id and "MANAGE_MESSAGES" not in perms:
        raise forbidden("MANAGE_MESSAGES")
    await db.delete(reply)
    post.reply_count = max(0, post.reply_count - 1)
    outbox.queue(
        db,
        "server",
        channel.server_id,
        Event.FORUM_REPLY_DELETE,
        {"id": reply_id, "post_id": post.id, "reply_count": post.reply_count},
    )
    await db.commit()
    return {"deleted": True}


@router.get("/channels/{channel_id}/tags")
async def channel_tags(
    ctx: ChannelContext = Depends(channel_context), db: AsyncSession = Depends(get_db)
) -> list[dict]:
    _forum_channel(ctx)
    rows = (
        await db.execute(
            select(ForumTag.tag, func.count())
            .join(ForumPost, ForumPost.id == ForumTag.post_id)
            .where(ForumPost.channel_id == ctx.channel.id)
            .group_by(ForumTag.tag)
            .order_by(func.count().desc(), ForumTag.tag)
            .limit(20)
        )
    ).all()
    return [{"tag": tag, "count": int(count)} for tag, count in rows]
