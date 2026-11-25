from datetime import datetime

from sqlalchemy import JSON, Boolean, DateTime, ForeignKey, Index, Integer, String
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class User(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "users"

    username: Mapped[str] = mapped_column(String(32), nullable=False)
    tag: Mapped[str] = mapped_column(String(4), nullable=False)
    email: Mapped[str] = mapped_column(String(254), nullable=False, unique=True, index=True)
    password_hash: Mapped[str] = mapped_column(String(255), nullable=False)
    display_name: Mapped[str] = mapped_column(String(32), nullable=False)
    avatar_color: Mapped[int] = mapped_column(Integer, default=0x5865F2, nullable=False)
    about: Mapped[str] = mapped_column(String(190), default="", nullable=False)
    # online / idle / dnd / invisible / offline
    status: Mapped[str] = mapped_column(String(16), default="online", nullable=False)
    email_verified: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )
    # TOTP 2FA：secret 仅在 setup 后写入，enabled 表示登录强制校验
    totp_secret: Mapped[str | None] = mapped_column(String(64), nullable=True)
    totp_enabled: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    # 恢复码只存哈希；明文仅在启用时返回一次
    recovery_codes: Mapped[list | None] = mapped_column(JSON, nullable=True)

    __table_args__ = (Index("ix_users_username_tag", "username", "tag", unique=True),)


class UserSettings(Base):
    __tablename__ = "user_settings"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    theme: Mapped[str] = mapped_column(String(16), default="dark", nullable=False)
    locale: Mapped[str] = mapped_column(String(16), default="zh-CN", nullable=False)
    volume: Mapped[int] = mapped_column(Integer, default=80, nullable=False)
    show_timestamps: Mapped[bool] = mapped_column(Boolean, default=True, nullable=False)
    show_emoji: Mapped[bool] = mapped_column(Boolean, default=True, nullable=False)
    compact_messages: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    notify_level: Mapped[str] = mapped_column(String(16), default="all", nullable=False)
    # WebPush / APNs 端点：[{endpoint, keys, created_at}]；Demo 只存不投递
    push_endpoints: Mapped[list | None] = mapped_column(JSON, nullable=True)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )


class Session(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "sessions"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    refresh_hash: Mapped[str] = mapped_column(String(128), nullable=False)
    device: Mapped[str] = mapped_column(String(120), default="", nullable=False)
    ip: Mapped[str] = mapped_column(String(64), default="", nullable=False)
    expires_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    revoked: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)


class AuthToken(Base, IdMixin, CreatedAtMixin):
    """邮箱验证 / 密码重置的一次性凭证。"""

    __tablename__ = "auth_tokens"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    kind: Mapped[str] = mapped_column(String(16), nullable=False)  # verify_email / reset_password
    code_hash: Mapped[str] = mapped_column(String(128), nullable=False)
    expires_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    used: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
