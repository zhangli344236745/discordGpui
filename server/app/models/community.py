from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Index, Integer, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.db import utcnow
from app.models.base import Base, CreatedAtMixin, IdMixin


class ServerEvent(Base, IdMixin, CreatedAtMixin):
    __tablename__ = "server_events"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), index=True)
    title: Mapped[str] = mapped_column(String(120), nullable=False)
    description: Mapped[str] = mapped_column(Text, default="", nullable=False)
    category: Mapped[str] = mapped_column(String(16), default="other", nullable=False)
    channel_id: Mapped[str | None] = mapped_column(ForeignKey("channels.id", ondelete="SET NULL"), nullable=True)
    starts_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    duration_minutes: Mapped[int] = mapped_column(Integer, default=60, nullable=False)
    created_by: Mapped[str] = mapped_column(ForeignKey("users.id"), nullable=False)

    __table_args__ = (Index("ix_events_server_starts", "server_id", "starts_at"),)


class EventRsvp(Base):
    __tablename__ = "event_rsvps"

    event_id: Mapped[str] = mapped_column(ForeignKey("server_events.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)


class WelcomeScreen(Base):
    __tablename__ = "welcome_screens"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    headline: Mapped[str] = mapped_column(String(120), default="", nullable=False)
    description: Mapped[str] = mapped_column(Text, default="", nullable=False)
    require_rules: Mapped[bool] = mapped_column(Boolean, default=True, nullable=False)


class WelcomeRule(Base):
    __tablename__ = "welcome_rules"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    position: Mapped[int] = mapped_column(Integer, primary_key=True)
    title: Mapped[str] = mapped_column(String(120), nullable=False)
    body: Mapped[str] = mapped_column(Text, default="", nullable=False)


class WelcomeRecommended(Base):
    __tablename__ = "welcome_recommended"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    channel_id: Mapped[str] = mapped_column(ForeignKey("channels.id", ondelete="CASCADE"), primary_key=True)


class WelcomeAccept(Base):
    __tablename__ = "welcome_accepts"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    accepted_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)


class Boost(Base):
    __tablename__ = "boosts"

    server_id: Mapped[str] = mapped_column(ForeignKey("servers.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)


class VoiceParticipant(Base):
    """由 LiveKit webhook 回写；room 命名 srv:{server}:{channel} 或 dm:{call}。"""

    __tablename__ = "voice_participants"

    room: Mapped[str] = mapped_column(String(96), primary_key=True)
    user_id: Mapped[str] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    mic: Mapped[bool] = mapped_column(Boolean, default=True, nullable=False)
    camera: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    screen: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    joined_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), default=utcnow, nullable=False)
