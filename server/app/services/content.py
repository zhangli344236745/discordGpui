"""投票 / 话题 / 书签的序列化与共用查询。"""

from __future__ import annotations

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.models import Poll, PollOption, PollVote, Thread, ThreadReply, User


def iso(value):  # noqa: ANN001, ANN201
    return value.isoformat().replace("+00:00", "Z") if value else None


async def poll_payload(db: AsyncSession, poll: Poll, viewer_id: str) -> dict:
    options = list(
        (
            await db.scalars(
                select(PollOption).where(PollOption.poll_id == poll.id).order_by(PollOption.position)
            )
        ).all()
    )
    votes = list((await db.scalars(select(PollVote).where(PollVote.poll_id == poll.id))).all())
    counts: dict[str, int] = {}
    mine: set[str] = set()
    for vote in votes:
        counts[vote.option_id] = counts.get(vote.option_id, 0) + 1
        if vote.user_id == viewer_id:
            mine.add(vote.option_id)
    return {
        "id": poll.id,
        "message_id": poll.message_id,
        "question": poll.question,
        "multi": poll.multi,
        "closed": poll.closed,
        "created_by": poll.created_by,
        "total_votes": len(votes),
        "options": [
            {
                "id": option.id,
                "label": option.label,
                "position": option.position,
                "votes": counts.get(option.id, 0),
                "me": option.id in mine,
            }
            for option in options
        ],
    }


def thread_payload(thread: Thread) -> dict:
    return {
        "id": thread.id,
        "message_id": thread.message_id,
        "channel_id": thread.channel_id,
        "title": thread.title,
        "reply_count": thread.reply_count,
        "created_at": iso(thread.created_at),
    }


def reply_payload(reply: ThreadReply, author: User | None) -> dict:
    return {
        "id": reply.id,
        "thread_id": reply.thread_id,
        "content": reply.content,
        "author": {
            "id": reply.author_id,
            "name": author.display_name if author else "已注销用户",
            "color": author.avatar_color if author else 0x808080,
        },
        "created_at": iso(reply.created_at),
    }
