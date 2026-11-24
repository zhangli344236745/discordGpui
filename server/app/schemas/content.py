from datetime import datetime

from pydantic import BaseModel, Field


class PollCreate(BaseModel):
    question: str = Field(min_length=1, max_length=300)
    options: list[str] = Field(min_length=2, max_length=10)
    multi: bool = False


class PollVoteIn(BaseModel):
    option_id: str


class ThreadCreate(BaseModel):
    title: str = Field(default="", max_length=120)


class ThreadReplyIn(BaseModel):
    content: str = Field(min_length=1, max_length=4000)


class BookmarkIn(BaseModel):
    saved: bool = True
    kind: str | None = Field(default=None, pattern="^(channel|dm)$")


class ForumPostCreate(BaseModel):
    title: str = Field(min_length=1, max_length=200)
    body: str = Field(default="", max_length=8000)
    tags: list[str] = Field(default_factory=list)


class ForumPostPatch(BaseModel):
    title: str | None = Field(default=None, min_length=1, max_length=200)
    body: str | None = Field(default=None, max_length=8000)
    pinned: bool | None = None
    locked: bool | None = None


class ForumReplyIn(BaseModel):
    body: str = Field(min_length=1, max_length=4000)


class ServerEventIn(BaseModel):
    title: str = Field(min_length=1, max_length=120)
    description: str = Field(default="", max_length=2000)
    category: str = Field(default="other", max_length=16)
    channel_id: str | None = None
    starts_at: datetime
    duration_minutes: int = Field(default=60, ge=1, le=1440)


class ServerEventPatch(BaseModel):
    title: str | None = Field(default=None, min_length=1, max_length=120)
    description: str | None = Field(default=None, max_length=2000)
    category: str | None = Field(default=None, max_length=16)
    channel_id: str | None = None
    starts_at: datetime | None = None
    duration_minutes: int | None = Field(default=None, ge=1, le=1440)


class RsvpIn(BaseModel):
    going: bool = True


class WelcomeRuleIn(BaseModel):
    title: str = Field(min_length=1, max_length=120)
    body: str = Field(default="", max_length=2000)


class WelcomePatch(BaseModel):
    headline: str | None = Field(default=None, max_length=120)
    description: str | None = Field(default=None, max_length=2000)
    require_rules: bool | None = None
    rules: list[WelcomeRuleIn] | None = None
    recommended: list[str] | None = None


class WelcomeAcceptIn(BaseModel):
    rule_agreed: bool = True
    channels: list[str] = Field(default_factory=list)


class BoostToggleIn(BaseModel):
    active: bool = True
