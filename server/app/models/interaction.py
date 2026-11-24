from datetime import datetime

from sqlalchemy import (
    Boolean,
    DateTime,
    ForeignKey,
    Index,
    Integer,
    String,
    Text,
)
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class Poll(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "polls"

    message_id: Mapped[str] = mapped_column(ForeignKey("messages.id", ondelete="CASCADE"), unique=True, index=True)
    question: Mapped[str] = mapped_column(String(300), nullable=False)
    multi: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    closed: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    created_by: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)


class PollOption(Base, IdMixin):
    __tablename__ = "poll_options"

    poll_id: Mapped[str] = mapped_column(ForeignKey("polls.id", ondelete="CASCADE"), index=True)
    label: Mapped[str] = mapped_column(String(120), nullable=False)
    position: Mapped[int] = mapped_column(Integer, default=0, nullable=False)


class PollVote(Base):
    __tablename__ = "poll_votes"

    poll_id: Mapped[str] = mapped_column(ForeignKey("polls.id", ondelete="CASCADE"), primary_key=True)
    option_id: Mapped[str] = mapped_column(ForeignKey("poll_options.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)


class Thread(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "threads"

    message_id: Mapped[str] = mapped_column(ForeignKey("messages.id", ondelete="CASCADE"), unique=True, index=True)
    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), index=True)
    title: Mapped[str] = mapped_column(String(120), default="", nullable=False)
    reply_count: Mapped[int] = mapped_column(Integer, default=0, nullable=False)


class ThreadReply(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "thread_replies"

    thread_id: Mapped[str] = mapped_column(ForeignKey("threads.id", ondelete="CASCADE"), index=True)
    author_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"))
    content: Mapped[str] = mapped_column(Text, default="", nullable=False)


class Bookmark(Base):
    """存消息 id 引用而不是正文副本；target_kind 区分频道消息与私信消息。

    原消息删除后由查询决定展示墓碑，不在这里复制正文。
    """

    __tablename__ = "bookmarks"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    target_kind: Mapped[str] = mapped_column(String(8), default="channel", primary_key=True)
    target_id: Mapped[str] = mapped_column(String(24), primary_key=True)
    # 跳转用：频道 id 或 dm id
    scope: Mapped[str] = mapped_column(String(24), default="", nullable=False)
    note: Mapped[str] = mapped_column(String(190), default="", nullable=False)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)

    __table_args__ = (Index("ix_bookmarks_user_created", "user_id", "created_at"),)
