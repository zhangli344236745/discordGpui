from datetime import datetime

from sqlalchemy import DateTime, ForeignKey, Index, Integer, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class Friendship(Base):
    """单向存两行（A→B、B→A），查询「我的好友」只需一个条件。"""

    __tablename__ = "friendships"

    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    friend_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    group_id: Mapped[str | None] = mapped_column(String(24), nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)


class FriendRequest(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "friend_requests"

    from_user: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    to_user: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    # pending / accepted / rejected
    state: Mapped[str] = mapped_column(String(16), default="pending", nullable=False)
    message: Mapped[str] = mapped_column(String(190), default="", nullable=False)

    __table_args__ = (Index("ix_friend_requests_to_state", "to_user", "state"),)


class FriendGroup(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "friend_groups"

    owner_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), index=True)
    name: Mapped[str] = mapped_column(String(32), nullable=False)
    position: Mapped[int] = mapped_column(Integer, default=0, nullable=False)


class Block(Base):
    __tablename__ = "blocks"

    owner_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    target_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)
    reason: Mapped[str] = mapped_column(Text, default="", nullable=False)
