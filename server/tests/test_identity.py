import re
from datetime import timedelta

import pyotp
from httpx import AsyncClient
from sqlalchemy import select

from app.db import SessionLocal, utcnow
from app.models import AuthToken, SecurityAudit, Session, User
from app.services import mailer
from tests.conftest import auth_headers, register


def _last_mail(to: str) -> dict[str, str]:
    for item in reversed(mailer.outbox):
        if item["to"] == to:
            return item
    raise AssertionError(f"no mail sent to {to}")


def _mail_token(to: str) -> str:
    body = _last_mail(to)["body"]
    match = re.search(r"token=([A-Za-z0-9]+)", body)
    if match:
        return match.group(1)
    return body.split("验证码是 ")[1].split("，")[0]


async def test_register_sends_verification_and_verify_is_idempotent(client: AsyncClient) -> None:
    mailer.outbox.clear()
    await register(client)

    token = _mail_token("a@example.com")
    first = await client.post("/api/v1/auth/verify-email", json={"token": token})
    assert first.status_code == 202, first.text
    assert first.json()["verified"] is True

    second = await client.post("/api/v1/auth/verify-email", json={"token": token})
    assert second.status_code == 202
    assert second.json()["verified"] is True

    async with SessionLocal() as db:
        user = await db.scalar(select(User).where(User.email == "a@example.com"))
        assert user is not None and user.email_verified is True


async def test_expired_verification_token_rejected(client: AsyncClient) -> None:
    mailer.outbox.clear()
    await register(client)
    token = _mail_token("a@example.com")

    async with SessionLocal() as db:
        row = await db.scalar(select(AuthToken).where(AuthToken.kind == "verify_email"))
        assert row is not None
        row.expires_at = utcnow() - timedelta(minutes=1)
        await db.commit()

    resp = await client.post("/api/v1/auth/verify-email", json={"token": token})
    assert resp.status_code == 400
    assert resp.json()["error"]["code"] == "VALIDATION_ERROR"


async def test_password_reset_revokes_sessions(client: AsyncClient) -> None:
    mailer.outbox.clear()
    tokens = await register(client)
    refresh_token = tokens["refresh_token"]

    forgot = await client.post("/api/v1/auth/password/forgot", json={"email": "a@example.com"})
    assert forgot.status_code == 202
    reset_token = _mail_token("a@example.com")

    reset = await client.post(
        "/api/v1/auth/password/reset",
        json={"token": reset_token, "new_password": "newpassword123"},
    )
    assert reset.status_code == 204, reset.text

    old_login = await client.post(
        "/api/v1/auth/login", json={"email": "a@example.com", "password": "password123"}
    )
    assert old_login.status_code == 401

    new_login = await client.post(
        "/api/v1/auth/login", json={"email": "a@example.com", "password": "newpassword123"}
    )
    assert new_login.status_code == 200

    refreshed = await client.post("/api/v1/auth/refresh", json={"refresh_token": refresh_token})
    assert refreshed.status_code == 401


async def test_totp_enable_login_and_recovery_code(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    setup = await client.post("/api/v1/auth/2fa/setup", headers=headers)
    assert setup.status_code == 200, setup.text
    secret = setup.json()["secret"]
    assert setup.json()["otpauth_url"].startswith("otpauth://totp/")

    code = pyotp.TOTP(secret).now()
    enabled = await client.post("/api/v1/auth/2fa/enable", headers=headers, json={"code": code})
    assert enabled.status_code == 200, enabled.text
    recovery = enabled.json()["recovery_codes"]
    assert len(recovery) == 10

    missing = await client.post(
        "/api/v1/auth/login", json={"email": "a@example.com", "password": "password123"}
    )
    assert missing.status_code == 401
    assert missing.json()["error"]["code"] == "MFA_REQUIRED"

    wrong = await client.post(
        "/api/v1/auth/login",
        json={"email": "a@example.com", "password": "password123", "totp_code": "000000"},
    )
    assert wrong.status_code == 401
    assert wrong.json()["error"]["code"] == "MFA_INVALID"

    good = await client.post(
        "/api/v1/auth/login",
        json={"email": "a@example.com", "password": "password123", "totp_code": pyotp.TOTP(secret).now()},
    )
    assert good.status_code == 200

    recovery_login = await client.post(
        "/api/v1/auth/login",
        json={"email": "a@example.com", "password": "password123", "totp_code": recovery[0]},
    )
    assert recovery_login.status_code == 200

    reused = await client.post(
        "/api/v1/auth/login",
        json={"email": "a@example.com", "password": "password123", "totp_code": recovery[0]},
    )
    assert reused.status_code == 401


async def test_logout_all_revokes_refresh_tokens(client: AsyncClient) -> None:
    tokens = await register(client)
    headers = {"Authorization": f"Bearer {tokens['access_token']}"}

    resp = await client.post("/api/v1/auth/logout-all", headers=headers)
    assert resp.status_code == 204

    refreshed = await client.post("/api/v1/auth/refresh", json={"refresh_token": tokens["refresh_token"]})
    assert refreshed.status_code == 401

    sessions = await client.get("/api/v1/auth/sessions", headers=headers)
    assert sessions.json() == []


async def test_security_audit_records_events_without_secrets(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    await client.post("/api/v1/auth/login", json={"email": "a@example.com", "password": "wrong-password"})

    resp = await client.get("/api/v1/auth/security-audit", headers=headers)
    assert resp.status_code == 200, resp.text
    events = [item["event"] for item in resp.json()]
    assert "login_failed" in events
    assert "account_registered" in events
    assert "wrong-password" not in resp.text
    assert "password123" not in resp.text

    csv_resp = await client.get("/api/v1/auth/security-audit?format=csv", headers=headers)
    assert csv_resp.status_code == 200
    assert csv_resp.headers["content-type"].startswith("text/csv")
    assert "login_failed" in csv_resp.text

    async with SessionLocal() as db:
        rows = (await db.scalars(select(SecurityAudit))).all()
        assert rows
        assert all("password" not in (row.detail or "").lower() for row in rows)
        sessions = (await db.scalars(select(Session))).all()
        assert all(session.refresh_hash for session in sessions)
