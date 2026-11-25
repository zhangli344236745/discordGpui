from pydantic import BaseModel, ConfigDict, Field


class UserOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    username: str
    tag: str
    display_name: str
    avatar_color: int
    about: str
    status: str
    email: str | None = None


class AuthorOut(BaseModel):
    """消息 / 成员里内嵌的轻量作者信息，避免前端再查一次。"""

    id: str
    name: str
    color: int
    bot: bool = False


class UserPatch(BaseModel):
    display_name: str | None = Field(default=None, max_length=32)
    username: str | None = Field(default=None, min_length=2, max_length=32)
    email: str | None = None
    about: str | None = Field(default=None, max_length=190)
    avatar_color: int | None = Field(default=None, ge=0, le=0xFFFFFF)


class StatusIn(BaseModel):
    status: str = Field(pattern="^(online|idle|dnd|invisible|offline)$")


class SettingsOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    theme: str
    locale: str
    volume: int
    show_timestamps: bool
    show_emoji: bool
    compact_messages: bool
    notify_level: str


class SettingsPatch(BaseModel):
    theme: str | None = Field(default=None, pattern="^(dark|light|system)$")
    locale: str | None = None
    volume: int | None = Field(default=None, ge=0, le=100)
    show_timestamps: bool | None = None
    show_emoji: bool | None = None
    compact_messages: bool | None = None
    notify_level: str | None = Field(default=None, pattern="^(all|mentions|none)$")
