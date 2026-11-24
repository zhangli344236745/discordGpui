from datetime import datetime

from sqlalchemy import (
    Boolean,
    DateTime,
    ForeignKey,
    Index,
    Integer,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class Server(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "servers"

    name: Mapped[str] = mapped_column(String(100), nullable=False)
    icon_color: Mapped[int] = mapped_column(Integer, default=0x5865F2, nullable=False)
    description: Mapped[str] = mapped_column(Text, default="", nullable=False)
    owner_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    is_public: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    category: Mapped[str] = mapped_column(String(32), default="other", nullable=False)
    template_key: Mapped[str | None] = mapped_column(String(32), nullable=True)
    member_count: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    notify_level: Mapped[str] = mapped_column(String(16), default="all", nullable=False)
    position: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )


class ServerMember(Base):
    __tablename__ = "server_members"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    nickname: Mapped[str] = mapped_column(String(32), default="", nullable=False)
    joined_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)

    __table_args__ = (Index("ix_server_members_user", "user_id"),)


class Role(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "roles"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    name: Mapped[str] = mapped_column(String(32), nullable=False)
    color: Mapped[int] = mapped_column(Integer, default=0x99AAB5, nullable=False)
    position: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    is_everyone: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)


class MemberRole(Base):
    __tablename__ = "member_roles"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    role_id: Mapped[str] = mapped_column(ForeignKey("roles.id", ondelete="CASCADE"), primary_key=True)


class PermissionGrant(Base):
    """角色 × 权限位的三态：inherit / allow / deny。只存非 inherit 的行。"""

    __tablename__ = "permission_grants"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    role_id: Mapped[str] = mapped_column(ForeignKey("roles.id", ondelete="CASCADE"), primary_key=True)
    permission: Mapped[str] = mapped_column(String(32), primary_key=True)
    state: Mapped[str] = mapped_column(String(8), nullable=False)


class ChannelOverride(Base):
    __tablename__ = "channel_overrides"

    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), primary_key=True)
    role_id: Mapped[str] = mapped_column(ForeignKey("roles.id", ondelete="CASCADE"), primary_key=True)
    permission: Mapped[str] = mapped_column(String(32), primary_key=True)
    state: Mapped[str] = mapped_column(String(8), nullable=False)


class Ban(Base, CreatedAtMixin):
    __tablename__ = "bans"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    reason: Mapped[str] = mapped_column(String(190), default="", nullable=False)
    banned_by: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)

    __table_args__ = (UniqueConstraint("server_id", "user_id", name="ux_bans_server_user"),)
