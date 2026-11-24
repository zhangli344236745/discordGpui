"""Gateway 事件日志：写操作与事件同事务落库，提交后广播，支持 24 h 内 ?since 补偿。"""

from datetime import datetime
from typing import Any

from sqlalchemy import JSON, DateTime, Index, String
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base


class GatewayEvent(Base):
    __tablename__ = "gateway_events"

    # 雪花 id：有序，可直接作为 ?since 游标
    id: Mapped[str] = mapped_column(String(24), primary_key=True)
    # server / dm / user
    scope_kind: Mapped[str] = mapped_column(String(8), nullable=False)
    scope_id: Mapped[str] = mapped_column(String(24), nullable=False)
    event: Mapped[str] = mapped_column(String(40), nullable=False)
    payload: Mapped[dict[str, Any] | None] = mapped_column(JSON, nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)

    __table_args__ = (
        Index("ix_gateway_events_scope", "scope_kind", "scope_id", "id"),
        Index("ix_gateway_events_created", "created_at"),
    )
