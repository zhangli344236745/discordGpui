from pydantic import BaseModel, EmailStr, Field


class RegisterIn(BaseModel):
    email: EmailStr
    password: str = Field(min_length=8, max_length=128)
    username: str = Field(min_length=2, max_length=32)
    display_name: str | None = Field(default=None, max_length=32)


class LoginIn(BaseModel):
    email: EmailStr
    password: str
    totp_code: str | None = Field(default=None, max_length=32)


class RefreshIn(BaseModel):
    refresh_token: str


class TokenOut(BaseModel):
    access_token: str
    refresh_token: str
    token_type: str = "Bearer"
    expires_in: int


class PasswordChangeIn(BaseModel):
    old_password: str
    new_password: str = Field(min_length=8, max_length=128)


class SessionOut(BaseModel):
    id: str
    device: str
    ip: str
    created_at: str


class EmailVerifyIn(BaseModel):
    token: str = Field(min_length=1, max_length=64)


class ForgotPasswordIn(BaseModel):
    email: EmailStr


class PasswordResetIn(BaseModel):
    token: str = Field(min_length=1, max_length=64)
    new_password: str = Field(min_length=8, max_length=128)


class TwoFactorCodeIn(BaseModel):
    code: str = Field(min_length=6, max_length=32)
