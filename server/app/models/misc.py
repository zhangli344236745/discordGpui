from datetime import datetime

from sqlalchemy import JSON, Boolean, DateTime, ForeignKey, Index, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class Notification(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "notifications"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    kind: Mapped[str] = mapped_column(String(24), nullable=False)  # mention/unread/dm/friend_request
    target_kind: Mapped[str] = mapped_column(String(16), default="channel", nullable=False)
    target_id: Mapped[str] = mapped_column(String(64), default="", nullable=False)
    title: Mapped[str] = mapped_column(String(190), default="", nullable=False)
    body: Mapped[str] = mapped_column(Text, default="", nullable=False)
    read: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)

    __table_args__ = (Index("ix_notifications_user_read", "user_id", "read", "id"),)


class MessageRequest(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "message_requests"

    from_user: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    to_user: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    state: Mapped[str] = mapped_column(String(16), default="pending", nullable=False)
    first_message: Mapped[str] = mapped_column(Text, default="", nullable=False)


class CallSession(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "call_sessions"

    dm_id: Mapped[str] = mapped_column(ForeignKey("dm_channels.id", ondelete="CASCADE"), index=True)
    kind: Mapped[str] = mapped_column(String(8), default="voice", nullable=False)  # voice/video
    # ringing / active / declined / ended
    state: Mapped[str] = mapped_column(String(16), default="ringing", nullable=False)
    caller_id: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)
    callee_id: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)
    ended_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)


class Draft(Base):
    """会话草稿：scope = channel:{id} / dm:{id}。"""

    __tablename__ = "drafts"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    scope: Mapped[str] = mapped_column(String(48), primary_key=True)
    content: Mapped[str] = mapped_column(Text, default="", nullable=False)
    attachments: Mapped[list | None] = mapped_column(JSON, nullable=True)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )
