from pydantic import BaseModel, Field


class DmCreate(BaseModel):
    user_id: str
    first_message: str | None = Field(default=None, max_length=4000)


class DmPatch(BaseModel):
    pinned: bool | None = None
    closed: bool | None = None


class DmMessageCreate(BaseModel):
    content: str = Field(default="", max_length=4000)
    reply_to: str | None = None
    client_nonce: str | None = Field(default=None, max_length=64)
    type: str = Field(default="text", max_length=16)
    payload: dict | None = None


class DmReadIn(BaseModel):
    last_read_message_id: str | None = None


class MessageRequestQuery(BaseModel):
    state: str = Field(default="pending", pattern="^(pending|accepted|ignored|all)$")


class CallIn(BaseModel):
    kind: str = Field(default="voice", pattern="^(voice|video)$")
