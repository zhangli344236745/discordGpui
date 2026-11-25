"""邮件发送抽象。

- `log`（默认）：写日志并存入进程内 outbox，开发与测试可直接读取。
- `smtp`：生产用 SMTP；配置缺失时启动即报错，避免静默丢信。
"""

from __future__ import annotations

import logging
import smtplib
from email.message import EmailMessage
from typing import Protocol

from app.config import settings

log = logging.getLogger("server.mailer")

# 仅 log 后端写入；测试用它断言验证码/重置链接
outbox: list[dict[str, str]] = []


class Mailer(Protocol):
    def send(self, to: str, subject: str, body: str) -> None: ...


class LogMailer:
    def send(self, to: str, subject: str, body: str) -> None:
        outbox.append({"to": to, "subject": subject, "body": body})
        log.info("mail to=%s subject=%s body=%s", to, subject, body)


class SmtpMailer:
    def __init__(self) -> None:
        if not settings.smtp_host:
            raise RuntimeError("SMTP_HOST 未配置，无法启用 smtp 邮件后端")
        self._host = settings.smtp_host
        self._port = settings.smtp_port
        self._user = settings.smtp_user
        self._password = settings.smtp_password
        self._sender = settings.smtp_from or settings.smtp_user
        self._tls = settings.smtp_starttls

    def send(self, to: str, subject: str, body: str) -> None:
        message = EmailMessage()
        message["From"] = self._sender
        message["To"] = to
        message["Subject"] = subject
        message.set_content(body)
        with smtplib.SMTP(self._host, self._port, timeout=10) as client:
            if self._tls:
                client.starttls()
            if self._user:
                client.login(self._user, self._password)
            client.send_message(message)


mailer: Mailer = LogMailer()


def init_mailer() -> None:
    global mailer
    if settings.mail_backend == "smtp":
        mailer = SmtpMailer()
        log.info("smtp mailer enabled: %s:%s", settings.smtp_host, settings.smtp_port)
    else:
        mailer = LogMailer()


def send_verification(to: str, code: str) -> None:
    mailer.send(
        to,
        "验证你的邮箱",
        f"你的邮箱验证码是 {code}，30 分钟内有效。\n\n如果这不是你本人的操作，请忽略此邮件。",
    )


def send_password_reset(to: str, token: str) -> None:
    link = f"{settings.app_base_url.rstrip('/')}/reset-password?token={token}"
    mailer.send(
        to,
        "重置密码",
        f"点击链接重置密码（30 分钟内有效）：\n{link}\n\n如果这不是你本人的操作，请忽略此邮件。",
    )


__all__ = [
    "LogMailer",
    "Mailer",
    "SmtpMailer",
    "init_mailer",
    "mailer",
    "outbox",
    "send_password_reset",
    "send_verification",
]
