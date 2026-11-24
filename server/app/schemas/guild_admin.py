from pydantic import BaseModel, Field

PERMISSION_STATE = "^(allow|deny|inherit)$"


class MemberOut(BaseModel):
    user_id: str
    username: str
    tag: str
    display_name: str
    nickname: str
    avatar_color: int
    status: str
    role_ids: list[str] = []
    joined_at: str
    is_owner: bool = False


class MemberPatch(BaseModel):
    nickname: str = Field(default="", max_length=32)


class RoleCreate(BaseModel):
    name: str = Field(min_length=1, max_length=32)
    color: int = Field(default=0x99AAB5, ge=0, le=0xFFFFFF)
    position: int | None = Field(default=None, ge=0)


class RolePatch(BaseModel):
    name: str | None = Field(default=None, min_length=1, max_length=32)
    color: int | None = Field(default=None, ge=0, le=0xFFFFFF)
    position: int | None = Field(default=None, ge=0)


class RoleAssignIn(BaseModel):
    assigned: bool = True


class PermissionItem(BaseModel):
    role_id: str
    permission: str
    state: str = Field(pattern=PERMISSION_STATE)


class PermissionMatrixIn(BaseModel):
    items: list[PermissionItem] = Field(default_factory=list)


class BanIn(BaseModel):
    user_id: str
    reason: str = Field(default="", max_length=190)
    delete_days: int = Field(default=0, ge=0, le=7)
