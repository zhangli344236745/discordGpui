from datetime import datetime
from typing import Any

from sqlalchemy import (
    JSON,
    BigInteger,
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


class Message(Base, IdMixin, CreatedAtMixin):
    """频道消息。贴纸 / GIF / 投票 / 转发的差异全部塞 payload，避免表爆炸。"""

    __tablename__ = "messages"

    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), index=True)
    author_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    content: Mapped[str] = mapped_column(Text, default="", nullable=False)
    reply_to: Mapped[str | None] = mapped_column(String(24), nullable=True)
    msg_type: Mapped[str] = mapped_column(String(16), default="text", nullable=False)
    payload: Mapped[dict[str, Any] | None] = mapped_column(JSON, nullable=True)
    pinned: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    edited_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    deleted_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    version: Mapped[int] = mapped_column(Integer, default=1, nullable=False)
    client_nonce: Mapped[str | None] = mapped_column(String(64), nullable=True)

    __table_args__ = (
        Index("ix_messages_channel_id_desc", "channel_id", "id"),
        Index("ux_messages_nonce", "client_nonce", unique=True),
        Index("ix_messages_pinned", "channel_id", "pinned"),
    )


class DmChannel(Base, IdMixin, CreatedAtMixin):
    """一对一会话。user_a < user_b 规范化，避免重复建会话。"""

    __tablename__ = "dm_channels"

    user_a: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    user_b: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    last_message_id: Mapped[str | None] = mapped_column(String(24), nullable=True)
    closed: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    # 双方各自的已读位置与置顶：user_a 用 *_a 列，user_b 用 *_b 列
    last_read_a: Mapped[str | None] = mapped_column(String(24), nullable=True)
    last_read_b: Mapped[str | None] = mapped_column(String(24), nullable=True)
    pinned_a: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    pinned_b: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)

    __table_args__ = (Index("ux_dm_pair", "user_a", "user_b", unique=True),)


class DmMessage(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "dm_messages"

    dm_id: Mapped[str] = mapped_column(ForeignKey("dm_channels.id", ondelete="CASCADE"), index=True)
    author_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    content: Mapped[str] = mapped_column(Text, default="", nullable=False)
    reply_to: Mapped[str | None] = mapped_column(String(24), nullable=True)
    msg_type: Mapped[str] = mapped_column(String(16), default="text", nullable=False)
    payload: Mapped[dict[str, Any] | None] = mapped_column(JSON, nullable=True)
    edited_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    deleted_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    version: Mapped[int] = mapped_column(Integer, default=1, nullable=False)
    client_nonce: Mapped[str | None] = mapped_column(String(64), nullable=True)

    __table_args__ = (
        Index("ix_dm_messages_dm_id_desc", "dm_id", "id"),
        Index("ux_dm_messages_nonce", "client_nonce", unique=True),
    )


class MessageEdit(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "message_edits"

    message_id: Mapped[str] = mapped_column(ForeignKey("messages.id", ondelete="CASCADE"), index=True)
    content: Mapped[str] = mapped_column(Text, nullable=False)
    edited_by: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)


class Attachment(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "attachments"

    owner_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    message_id: Mapped[str | None] = mapped_column(String(24), nullable=True, index=True)
    filename: Mapped[str] = mapped_column(String(255), nullable=False)
    mime: Mapped[str] = mapped_column(String(120), default="application/octet-stream", nullable=False)
    size: Mapped[int] = mapped_column(BigInteger, default=0, nullable=False)
    storage_key: Mapped[str] = mapped_column(String(255), nullable=False)
    uploaded: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)


class Reaction(Base):
    __tablename__ = "reactions"

    message_id: Mapped[str] = mapped_column(ForeignKey("messages.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    emoji: Mapped[str] = mapped_column(String(32), primary_key=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)


class ReadState(Base):
    __tablename__ = "read_states"

    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    last_read_message_id: Mapped[str | None] = mapped_column(String(24), nullable=True)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )
