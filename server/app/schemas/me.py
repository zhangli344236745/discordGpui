from pydantic import BaseModel, Field


class ChannelStatePatch(BaseModel):
    collapsed: bool | None = None
    notify_level: str | None = Field(default=None, pattern="^(all|mentions|none|inherit)$")
    last_read_message_id: str | None = None


class DraftIn(BaseModel):
    content: str = Field(default="", max_length=4000)
    attachments: list | None = None


class NotificationReadIn(BaseModel):
    ids: list[str] = Field(default_factory=list)
    all: bool = False


class PushSubscribeIn(BaseModel):
    endpoint: str = Field(min_length=1, max_length=600)
    keys: dict | None = None
