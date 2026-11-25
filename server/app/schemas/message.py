from pydantic import BaseModel, ConfigDict, Field

from app.schemas.user import AuthorOut


class CategoryOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    server_id: str
    name: str
    position: int
    collapsed: bool = False


class ChannelOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    server_id: str
    category_id: str | None
    name: str
    kind: str
    topic: str
    position: int
    notify_level: str = "inherit"
    last_read_message_id: str | None = None


class ChannelCreate(BaseModel):
    name: str = Field(min_length=1, max_length=64)
    kind: str = Field(default="text", pattern="^(text|voice|forum)$")
    category_id: str | None = None
    topic: str = Field(default="", max_length=2000)


class ChannelPatch(BaseModel):
    name: str | None = Field(default=None, min_length=1, max_length=64)
    topic: str | None = Field(default=None, max_length=2000)
    category_id: str | None = None
    position: int | None = None


class ChannelNotifyPatch(BaseModel):
    notify_level: str = Field(pattern="^(all|mentions|none|inherit)$")


class CategoryCreate(BaseModel):
    name: str = Field(min_length=1, max_length=64)


class CategoryPatch(BaseModel):
    name: str | None = Field(default=None, min_length=1, max_length=64)


class ServerTreeOut(BaseModel):
    categories: list[CategoryOut]
    channels: list[ChannelOut]


class AttachmentOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    filename: str
    mime: str
    size: int
    url: str = ""
    thumbnail_url: str | None = None


class MessageOut(BaseModel):
    id: str
    channel_id: str
    author: AuthorOut
    content: str
    type: str
    payload: dict | None = None
    reply_to: str | None = None
    client_nonce: str | None = None
    pinned: bool
    edited_at: str | None
    created_at: str
    reactions: list[dict] = []
    attachments: list[AttachmentOut] = []
    poll: dict | None = None
    warnings: list[dict] = []


class MessageCreate(BaseModel):
    content: str = Field(default="", max_length=4000)
    reply_to: str | None = None
    attachment_ids: list[str] = Field(default_factory=list)
    client_nonce: str | None = Field(default=None, max_length=64)


class MessagePatch(BaseModel):
    content: str = Field(min_length=1, max_length=4000)
    version: int | None = None


class ReactionIn(BaseModel):
    emoji: str = Field(min_length=1, max_length=32)


class Page(BaseModel):
    items: list
    has_more: bool = False


class UploadIn(BaseModel):
    filename: str = Field(min_length=1, max_length=255)
    size: int = Field(ge=0)
    mime: str = Field(default="application/octet-stream", max_length=120)


class UploadOut(BaseModel):
    file_id: str
    upload_url: str
    expires_in: int = 900
