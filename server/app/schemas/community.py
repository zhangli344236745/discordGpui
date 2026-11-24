from pydantic import BaseModel, Field


class InviteCreate(BaseModel):
    expiry: str = Field(default="7d", pattern="^(1h|1d|7d|never)$")
    max_uses: int | None = Field(default=None, ge=1, le=1000)


class AutoModRuleIn(BaseModel):
    name: str = Field(min_length=1, max_length=64)
    action: str = Field(default="block", pattern="^(block|warn)$")
    keywords: list[str] = Field(min_length=1, max_length=50)
    enabled: bool = True


class AutoModRulePatch(BaseModel):
    name: str | None = Field(default=None, min_length=1, max_length=64)
    action: str | None = Field(default=None, pattern="^(block|warn)$")
    keywords: list[str] | None = Field(default=None, max_length=50)
    enabled: bool | None = None


class AutoModTestIn(BaseModel):
    content: str = Field(min_length=1, max_length=4000)
