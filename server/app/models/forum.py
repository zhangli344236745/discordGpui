from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Index, Integer, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class ForumPost(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "forum_posts"

    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), index=True)
    author_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    title: Mapped[str] = mapped_column(String(200), nullable=False)
    body: Mapped[str] = mapped_column(Text, default="", nullable=False)
    pinned: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    locked: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    reply_count: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    last_activity_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)

    __table_args__ = (Index("ix_forum_posts_channel_activity", "channel_id", "last_activity_at"),)


class ForumReply(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "forum_replies"

    post_id: Mapped[str] = mapped_column(ForeignKey("forum_posts.id", ondelete="CASCADE"), index=True)
    author_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"))
    body: Mapped[str] = mapped_column(Text, nullable=False)


class ForumTag(Base):
    __tablename__ = "forum_tags"

    post_id: Mapped[str] = mapped_column(ForeignKey("forum_posts.id", ondelete="CASCADE"), primary_key=True)
    tag: Mapped[str] = mapped_column(String(24), primary_key=True)
