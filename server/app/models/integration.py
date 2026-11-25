"""机器人与集成：Bot 令牌、Webhook、斜杠命令。

Bot 账号复用 users 表（is_bot=True），消息 / 成员接口无需特判；
Webhook 挂到一个频道并拥有自己的 bot 用户作为消息作者。
"""

from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Index, String
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class BotToken(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "bot_tokens"

    bot_user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    token_hash: Mapped[str] = mapped_column(String(128), nullable=False, unique=True)
    revoked: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)


class Webhook(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "webhooks"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), index=True)
    bot_user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"))
    name: Mapped[str] = mapped_column(String(80), nullable=False)
    # URL token（Discord 风格 /webhooks/{id}/{token}）的哈希
    token_hash: Mapped[str] = mapped_column(String(128), nullable=False, index=True)
    revoked: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)


class SlashCommand(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "slash_commands"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    name: Mapped[str] = mapped_column(String(32), nullable=False)
    description: Mapped[str] = mapped_column(String(200), default="", nullable=False)
    # builtin（内置处理器）/ custom（静态模板）
    kind: Mapped[str] = mapped_column(String(16), default="custom", nullable=False)
    response: Mapped[str] = mapped_column(String(500), default="", nullable=False)
    created_by: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )

    __table_args__ = (Index("uq_slash_commands_server_name", "server_id", "name", unique=True),)


__all__ = ["BotToken", "SlashCommand", "Webhook"]
