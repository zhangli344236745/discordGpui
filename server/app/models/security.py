from sqlalchemy import Boolean, ForeignKey, Index, String
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base, CreatedAtMixin, IdMixin


class SecurityAudit(Base, IdMixin, CreatedAtMixin):
    """账号安全审计：登录、密码、2FA、会话事件；不含任何明文凭证。"""

    __tablename__ = "security_audit"

    user_id: Mapped[str | None] = mapped_column(
        ForeignKey("users.id", ondelete="SET NULL"), nullable=True, index=True
    )
    email: Mapped[str] = mapped_column(String(254), default="", nullable=False)
    event: Mapped[str] = mapped_column(String(32), nullable=False)
    success: Mapped[bool] = mapped_column(Boolean, default=True, nullable=False)
    ip: Mapped[str] = mapped_column(String(64), default="", nullable=False)
    device: Mapped[str] = mapped_column(String(160), default="", nullable=False)
    detail: Mapped[str] = mapped_column(String(200), default="", nullable=False)

    __table_args__ = (Index("ix_security_audit_user_created", "user_id", "created_at"),)


__all__ = ["SecurityAudit"]
