from pydantic import BaseModel, Field


class FriendRequestIn(BaseModel):
    username: str = Field(min_length=2, max_length=32)
    tag: str = Field(pattern=r"^\d{4}$")
    message: str = Field(default="", max_length=190)


class FriendRequestOut(BaseModel):
    id: str
    from_user: dict
    to_user: dict
    state: str
    message: str
    created_at: str


class FriendOut(BaseModel):
    id: str
    username: str
    tag: str
    display_name: str
    avatar_color: int
    status: str
    online: bool
    group_id: str | None = None
    created_at: str


class FriendGroupIn(BaseModel):
    name: str = Field(min_length=1, max_length=32)


class FriendGroupOrderIn(BaseModel):
    ids: list[str] = Field(min_length=1)


class FriendMoveIn(BaseModel):
    group_id: str | None = None


class BlockIn(BaseModel):
    user_id: str
    reason: str = Field(default="", max_length=190)


class BlockOut(BaseModel):
    user_id: str
    username: str
    tag: str
    display_name: str
    avatar_color: int
    reason: str
    created_at: str
