from datetime import UTC, datetime, timedelta
from typing import Any

import jwt
from argon2 import PasswordHasher
from argon2.exceptions import VerifyMismatchError

from app.config import settings

_hasher = PasswordHasher()


def hash_password(raw: str) -> str:
    return _hasher.hash(raw)


def verify_password(raw: str, hashed: str) -> bool:
    try:
        return _hasher.verify(hashed, raw)
    except (VerifyMismatchError, ValueError):
        return False


def create_token(subject: str, kind: str, ttl: timedelta) -> str:
    now = datetime.now(UTC)
    payload = {
        "sub": subject,
        "kind": kind,
        "iat": int(now.timestamp()),
        "exp": int((now + ttl).timestamp()),
    }
    return jwt.encode(payload, settings.jwt_secret, algorithm=settings.jwt_algorithm)


def create_access_token(user_id: str) -> str:
    return create_token(user_id, "access", timedelta(minutes=settings.access_ttl_minutes))


def create_refresh_token(user_id: str) -> str:
    return create_token(user_id, "refresh", timedelta(days=settings.refresh_ttl_days))


def decode_token(token: str, expected_kind: str) -> dict[str, Any]:
    """解码并校验 kind。过期抛 jwt.ExpiredSignatureError，交给 deps 转 401。"""
    payload = jwt.decode(token, settings.jwt_secret, algorithms=[settings.jwt_algorithm])
    if payload.get("kind") != expected_kind:
        raise jwt.InvalidTokenError("token kind mismatch")
    return payload
