from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Index, Integer, String
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
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


class Report(Base, IdMixin, CreatedAtMixin):
    """用户举报：消息 / 用户 / 服务器。同一人重复举报同一目标合并计数。"""

    __tablename__ = "reports"

    server_id: Mapped[str | None] = mapped_column(
        ForeignKey("servers.id", ondelete="CASCADE"), nullable=True, index=True
    )
    reporter_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    target_kind: Mapped[str] = mapped_column(String(16), nullable=False)  # message/user/server
    target_id: Mapped[str] = mapped_column(String(24), nullable=False)
    reason: Mapped[str] = mapped_column(String(64), nullable=False)
    detail: Mapped[str] = mapped_column(String(500), default="", nullable=False)
    # pending → reviewing → resolved / rejected / escalated
    status: Mapped[str] = mapped_column(String(16), default="pending", nullable=False)
    count: Mapped[int] = mapped_column(Integer, default=1, nullable=False)

    __table_args__ = (Index("uq_reports_reporter_target", "reporter_id", "target_kind", "target_id", unique=True),)


class ReportAction(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "report_actions"

    report_id: Mapped[str] = mapped_column(ForeignKey("reports.id", ondelete="CASCADE"), index=True)
    actor_id: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)
    action: Mapped[str] = mapped_column(String(16), nullable=False)  # resolve/reject/escalate
    note: Mapped[str] = mapped_column(String(300), default="", nullable=False)
    deleted_content: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)


class RetentionRule(Base):
    """内容保留策略：0 = 永久保留。法务持有优先于策略。"""

    __tablename__ = "retention_rules"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    message_days: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    attachment_days: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    audit_days: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )


class LegalHold(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "legal_holds"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    target_kind: Mapped[str] = mapped_column(String(16), nullable=False)  # message/channel/user
    target_id: Mapped[str] = mapped_column(String(24), nullable=False)
    note: Mapped[str] = mapped_column(String(200), default="", nullable=False)

    __table_args__ = (Index("uq_legal_holds", "server_id", "target_kind", "target_id", unique=True),)
