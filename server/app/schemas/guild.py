from pydantic import BaseModel, ConfigDict, Field


class ServerOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    name: str
    icon_color: int
    description: str
    owner_id: str
    is_public: bool
    category: str
    member_count: int
    notify_level: str


class ServerCreate(BaseModel):
    name: str = Field(min_length=1, max_length=100)
    icon_color: int = Field(default=0x5865F2, ge=0, le=0xFFFFFF)
    template_key: str | None = Field(default=None, max_length=32)
    is_public: bool = False


class ServerPatch(BaseModel):
    name: str | None = Field(default=None, min_length=1, max_length=100)
    description: str | None = Field(default=None, max_length=2000)
    icon_color: int | None = Field(default=None, ge=0, le=0xFFFFFF)
    is_public: bool | None = None
    notify_level: str | None = Field(default=None, pattern="^(all|mentions|none)$")


class MemberOut(BaseModel):
    user_id: str
    name: str
    color: int
    nickname: str
    roles: list[str]
    status: str
    joined_at: str


class MemberPatch(BaseModel):
    nickname: str | None = Field(default=None, max_length=32)


class RoleOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    server_id: str
    name: str
    color: int
    position: int
    is_everyone: bool


class RoleCreate(BaseModel):
    name: str = Field(min_length=1, max_length=32)
    color: int = Field(default=0x99AAB5, ge=0, le=0xFFFFFF)
    position: int = 0


class RolePatch(BaseModel):
    name: str | None = Field(default=None, min_length=1, max_length=32)
    color: int | None = Field(default=None, ge=0, le=0xFFFFFF)
    position: int | None = None


class PermissionItem(BaseModel):
    role_id: str
    permission: str
    state: str = Field(pattern="^(inherit|allow|deny)$")


class PermissionBulk(BaseModel):
    items: list[PermissionItem]


class BanCreate(BaseModel):
    user_id: str
    reason: str = Field(default="", max_length=190)
    delete_days: int = Field(default=0, ge=0, le=7)


class BanOut(BaseModel):
    user_id: str
    name: str
    reason: str
    banned_by: str
    created_at: str


class InviteCreate(BaseModel):
    channel_id: str
    max_uses: int | None = Field(default=None, ge=0, le=100)
    expires_in_minutes: int | None = Field(default=None, ge=0, le=60 * 24 * 30)


class InviteOut(BaseModel):
    code: str
    server_id: str
    channel_id: str
    uses: int
    max_uses: int | None
    expires_at: str | None
    revoked: bool
    valid: bool
