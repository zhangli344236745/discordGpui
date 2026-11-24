from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Index, Integer, String
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base, CreatedAtMixin, IdMixin


class Invite(Base, CreatedAtMixin):
    __tablename__ = "invites"

    code: Mapped[str] = mapped_column(String(16), primary_key=True)
    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"))
    created_by: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)
    max_uses: Mapped[int | None] = mapped_column(Integer, nullable=True)  # NULL = 不限
    uses: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    expires_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    revoked: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)


class AuditEntry(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "audit_entries"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    actor_id: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)
    kind: Mapped[str] = mapped_column(String(24), nullable=False)  # server/channel/role/member/invite/emoji
    detail: Mapped[str] = mapped_column(String(300), default="", nullable=False)
    target: Mapped[str] = mapped_column(String(120), default="", nullable=False)

    __table_args__ = (Index("ix_audit_server_id_desc", "server_id", "id"),)


class AutoModRule(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "automod_rules"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    name: Mapped[str] = mapped_column(String(64), nullable=False)
    enabled: Mapped[bool] = mapped_column(Boolean, default=True, nullable=False)
    action: Mapped[str] = mapped_column(String(8), default="block", nullable=False)  # block/warn


class AutoModKeyword(Base):
    __tablename__ = "automod_keywords"

    rule_id: Mapped[str] = mapped_column(ForeignKey("automod_rules.id", ondelete="CASCADE"), primary_key=True)
    keyword: Mapped[str] = mapped_column(String(64), primary_key=True)
