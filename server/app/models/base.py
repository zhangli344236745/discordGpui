from datetime import datetime

from sqlalchemy import DateTime, String
from sqlalchemy.orm import Mapped, mapped_column

from app.db import Base, utcnow
from app.ids import new_id


class CreatedAtMixin:
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)


class IdMixin:
    id: Mapped[str] = mapped_column(String(24), primary_key=True, default=new_id)


__all__ = ["Base", "CreatedAtMixin", "IdMixin"]
