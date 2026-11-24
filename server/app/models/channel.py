from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Index, Integer, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class Category(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "categories"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    name: Mapped[str] = mapped_column(String(64), nullable=False)
    position: Mapped[int] = mapped_column(Integer, default=0, nullable=False)


class Channel(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "channels"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    category_id: Mapped[str | None] = mapped_column(
        ForeignKey("categories.id", ondelete="SET NULL"), nullable=True, index=True
    )
    name: Mapped[str] = mapped_column(String(64), nullable=False)
    kind: Mapped[str] = mapped_column(String(16), default="text", nullable=False)  # text/voice/forum
    topic: Mapped[str] = mapped_column(Text, default="", nullable=False)
    position: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    nsfw: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )

    __table_args__ = (Index("ix_channels_server_pos", "server_id", "position"),)


class ChannelUserState(Base):
    """用户级视图状态：折叠、通知覆盖、已读位置。跨设备同步靠这张表。"""

    __tablename__ = "channel_user_state"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), primary_key=True)
    collapsed: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    notify_level: Mapped[str] = mapped_column(String(16), default="inherit", nullable=False)
    last_read_message_id: Mapped[str | None] = mapped_column(String(24), nullable=True)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )
