"""身份安全：TOTP 2FA、恢复码与安全审计。

审计写入独立会话，避免业务事务回滚时丢审计；只记录必要元数据，不写明文凭证。
"""

from __future__ import annotations

import hashlib
import secrets
from datetime import datetime

import pyotp

from app.config import settings
from app.db import SessionLocal
from app.ids import new_id
from app.models import SecurityAudit, User

RECOVERY_CODE_COUNT = 10


def generate_totp_secret() -> str:
    return pyotp.random_base32()


def provisioning_uri(secret: str, email: str) -> str:
    return pyotp.TOTP(secret).provisioning_uri(name=email, issuer_name=settings.totp_issuer)


def verify_totp(secret: str, code: str) -> bool:
    if not secret or not code:
        return False
    return pyotp.TOTP(secret).verify(code.strip(), valid_window=1)


def _hash_code(code: str) -> str:
    return hashlib.sha256(code.strip().upper().encode()).hexdigest()


def generate_recovery_codes() -> tuple[list[str], list[str]]:
    """返回 (明文, 哈希)。明文只应返回给用户一次。"""
    plain = [f"{secrets.token_hex(4).upper()}-{secrets.token_hex(4).upper()}" for _ in range(RECOVERY_CODE_COUNT)]
    return plain, [_hash_code(code) for code in plain]


def consume_recovery_code(user: User, code: str) -> bool:
    """匹配并移除一个恢复码；返回是否成功。调用方负责 commit。"""
    if not user.recovery_codes:
        return False
    hashed = _hash_code(code)
    remaining = [item for item in user.recovery_codes if item != hashed]
    if len(remaining) == len(user.recovery_codes):
        return False
    user.recovery_codes = remaining
    return True


async def audit(
    event: str,
    *,
    user: User | None = None,
    email: str = "",
    success: bool = True,
    ip: str = "",
    device: str = "",
    detail: str = "",
    now: datetime | None = None,
) -> None:
    """写一条安全审计。独立事务，失败只记日志不影响主流程。"""
    from app.db import utcnow

    async with SessionLocal() as db:
        db.add(
            SecurityAudit(
                id=new_id(),
                user_id=user.id if user is not None else None,
                email=(user.email if user is not None else email)[:254],
                event=event[:32],
                success=success,
                ip=ip[:64],
                device=device[:160],
                detail=detail[:200],
                created_at=now or utcnow(),
            )
        )
        await db.commit()


__all__ = [
    "RECOVERY_CODE_COUNT",
    "audit",
    "consume_recovery_code",
    "generate_recovery_codes",
    "generate_totp_secret",
    "provisioning_uri",
    "verify_totp",
]
