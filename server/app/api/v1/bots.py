"""机器人与集成：Bot 账号、Webhook、斜杠命令。

- Bot 账号是 users 表里 is_bot=True 的用户，加入服务器后走与真人相同的
  消息 / 成员接口；鉴权用 `Authorization: Bot <token>`。
- Webhook 采用 Discord 风格 URL token：`POST /webhooks/{id}/{token}`，
  外部系统无需登录即可向绑定频道投递消息；撤销后立即 404。
- 斜杠命令：内置 help / shrug 处理器 + 服务器自定义静态模板；
  注册需要 MANAGE_SERVER，执行需要频道 SEND_MESSAGES。
"""

import hashlib
import secrets

from fastapi import APIRouter, Depends, status
from pydantic import BaseModel, Field
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db
from app.deps import ChannelContext, ServerContext, channel_context, server_context
from app.errors import bad_request, conflict, not_found
from app.ids import new_id, new_tag
from app.models import (
    BotToken,
    Channel,
    MemberRole,
    Message,
    Role,
    ServerMember,
    SlashCommand,
    User,
    Webhook,
)
from app.realtime.bus import bus
from app.realtime.events import Event
from app.schemas.message import MessageOut
from app.services import ratelimit

router = APIRouter(tags=["integrations"])


def _digest(token: str) -> str:
    return hashlib.sha256(token.encode()).hexdigest()


class BotCreateIn(BaseModel):
    name: str = Field(min_length=2, max_length=32)


class WebhookCreateIn(BaseModel):
    name: str = Field(min_length=1, max_length=80)
    channel_id: str = Field(min_length=1, max_length=24)


class WebhookPostIn(BaseModel):
    content: str = Field(min_length=1, max_length=4000)
    username: str | None = Field(default=None, max_length=32)


class CommandCreateIn(BaseModel):
    name: str = Field(min_length=1, max_length=32, pattern=r"^[a-z0-9-]+$")
    description: str = Field(default="", max_length=200)
    kind: str = Field(default="custom", pattern="^(builtin|custom)$")
    response: str = Field(default="", max_length=500)


class CommandRunIn(BaseModel):
    args: str = Field(default="", max_length=500)


BUILTIN_COMMANDS: dict[str, str] = {
    "help": "列出本服务器可用的斜杠命令",
    "shrug": "在消息末尾追加 ¯\\_(ツ)_/¯",
}


async def _create_bot_user(db: AsyncSession, name: str) -> User:
    user_id = new_id()
    for _ in range(10):
        tag = new_tag()
        if await db.scalar(select(User.id).where(User.username == name, User.tag == tag)) is None:
            break
    else:  # pragma: no cover - 极端冲突
        raise conflict("机器人名称太热门，请换一个", "name")
    user = User(
        id=user_id,
        username=name,
        tag=tag,
        email=f"bot-{user_id}@bots.local",
        password_hash="!",
        display_name=name[:32],
        about="",
        is_bot=True,
    )
    db.add(user)
    await db.flush()
    return user


async def _add_member_everyone(db: AsyncSession, server_id: str, user_id: str) -> None:
    if await db.get(ServerMember, (server_id, user_id)) is None:
        db.add(ServerMember(server_id=server_id, user_id=user_id))
    everyone = await db.scalar(
        select(Role).where(Role.server_id == server_id, Role.is_everyone.is_(True))
    )
    if everyone is not None and await db.get(MemberRole, (server_id, user_id, everyone.id)) is None:
        db.add(MemberRole(server_id=server_id, user_id=user_id, role_id=everyone.id))


# ---------------------------------------------------------------------------
# Bot 账号
# ---------------------------------------------------------------------------


@router.post("/servers/{server_id}/bots", status_code=status.HTTP_201_CREATED)
async def create_bot(
    body: BotCreateIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    user = await _create_bot_user(db, body.name)
    await _add_member_everyone(db, ctx.server.id, user.id)
    token = secrets.token_hex(32)
    db.add(
        BotToken(
            id=new_id(),
            bot_user_id=user.id,
            server_id=ctx.server.id,
            token_hash=_digest(token),
        )
    )
    await db.commit()
    # token 只在创建时返回一次
    return {"id": user.id, "name": user.display_name, "tag": user.tag, "token": token}


@router.get("/servers/{server_id}/bots")
async def list_bots(
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> list[dict]:
    tokens = (
        await db.scalars(
            select(BotToken).where(BotToken.server_id == ctx.server.id, BotToken.revoked.is_(False))
        )
    ).all()
    users = {
        u.id: u
        for u in (
            await db.scalars(select(User).where(User.id.in_([t.bot_user_id for t in tokens] or [""])))
        ).all()
    }
    return [
        {
            "id": token.bot_user_id,
            "name": users[token.bot_user_id].display_name if token.bot_user_id in users else "已注销",
            "tag": users[token.bot_user_id].tag if token.bot_user_id in users else "",
            "created_at": token.created_at.isoformat(),
        }
        for token in tokens
    ]


@router.delete("/servers/{server_id}/bots/{bot_user_id}", status_code=status.HTTP_204_NO_CONTENT)
async def revoke_bot(
    bot_user_id: str,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    ctx.require("MANAGE_SERVER")
    tokens = (await db.scalars(select(BotToken).where(BotToken.bot_user_id == bot_user_id))).all()
    if not any(token.server_id == ctx.server.id for token in tokens):
        raise not_found("机器人")
    for token in tokens:
        token.revoked = True
    await db.commit()


# ---------------------------------------------------------------------------
# Webhook
# ---------------------------------------------------------------------------


@router.post("/servers/{server_id}/webhooks", status_code=status.HTTP_201_CREATED)
async def create_webhook(
    body: WebhookCreateIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    channel = await db.get(Channel, body.channel_id)
    if channel is None or channel.server_id != ctx.server.id:
        raise not_found("频道")
    bot_user = await _create_bot_user(db, f"Webhook · {body.name}")
    token = secrets.token_hex(24)
    webhook = Webhook(
        id=new_id(),
        server_id=ctx.server.id,
        channel_id=channel.id,
        bot_user_id=bot_user.id,
        name=body.name,
        token_hash=_digest(token),
    )
    db.add(webhook)
    await db.commit()
    return {
        "id": webhook.id,
        "name": webhook.name,
        "channel_id": channel.id,
        "url": f"{settings.app_base_url.rstrip('/')}/api/v1/webhooks/{webhook.id}/{token}",
        "token": token,
    }


@router.get("/servers/{server_id}/webhooks")
async def list_webhooks(
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> list[dict]:
    rows = (
        await db.scalars(select(Webhook).where(Webhook.server_id == ctx.server.id, Webhook.revoked.is_(False)))
    ).all()
    return [
        {
            "id": row.id,
            "name": row.name,
            "channel_id": row.channel_id,
            "created_at": row.created_at.isoformat(),
        }
        for row in rows
    ]


@router.delete("/servers/{server_id}/webhooks/{webhook_id}", status_code=status.HTTP_204_NO_CONTENT)
async def revoke_webhook(
    webhook_id: str,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    ctx.require("MANAGE_SERVER")
    webhook = await db.get(Webhook, webhook_id)
    if webhook is None or webhook.server_id != ctx.server.id or webhook.revoked:
        raise not_found("Webhook")
    webhook.revoked = True
    await db.commit()


async def _webhook_by_token(db: AsyncSession, webhook_id: str, token: str) -> Webhook:
    webhook = await db.get(Webhook, webhook_id)
    if webhook is None or webhook.revoked or webhook.token_hash != _digest(token):
        # 不区分「不存在」与「令牌错误」，避免探测
        raise not_found("Webhook")
    return webhook


@router.get("/webhooks/{webhook_id}/{token}")
async def webhook_info(webhook_id: str, token: str, db: AsyncSession = Depends(get_db)) -> dict:
    webhook = await _webhook_by_token(db, webhook_id, token)
    return {"id": webhook.id, "name": webhook.name, "channel_id": webhook.channel_id}


@router.post("/webhooks/{webhook_id}/{token}", status_code=status.HTTP_201_CREATED)
async def webhook_post(
    webhook_id: str,
    token: str,
    body: WebhookPostIn,
    db: AsyncSession = Depends(get_db),
) -> dict:
    webhook = await _webhook_by_token(db, webhook_id, token)
    ratelimit.check(f"webhook:{webhook.id}", settings.rate_limit_messages, settings.rate_limit_window_seconds)
    channel = await db.get(Channel, webhook.channel_id)
    if channel is None:
        raise not_found("频道")
    bot_user = await db.get(User, webhook.bot_user_id)
    if bot_user is None:  # pragma: no cover - 级联删除保护
        raise not_found("Webhook")

    message = Message(
        id=new_id(),
        channel_id=channel.id,
        author_id=bot_user.id,
        content=body.content,
        msg_type="text",
        payload={"webhook_id": webhook.id, "webhook_name": body.username or webhook.name},
    )
    db.add(message)
    await db.commit()

    data = (
        await _serialize_channel_message(db, message, viewer_id=bot_user.id)
    ).model_dump()
    data["server_id"] = channel.server_id
    await bus.to_server(channel.server_id, Event.MESSAGE_CREATE, data)
    return data


async def _serialize_channel_message(
    db: AsyncSession, message: Message, viewer_id: str
) -> MessageOut:
    """复用消息序列化，避免 webhooks 依赖用户态查询。"""
    from app.api.v1.messages import serialize

    out = (await serialize(db, [message], viewer_id))[0]
    return out


# ---------------------------------------------------------------------------
# 斜杠命令
# ---------------------------------------------------------------------------


@router.post("/servers/{server_id}/commands", status_code=status.HTTP_201_CREATED)
async def create_command(
    body: CommandCreateIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    if body.kind == "builtin" and body.name not in BUILTIN_COMMANDS:
        raise bad_request(f"内置命令只有：{', '.join(BUILTIN_COMMANDS)}", "name")
    if body.kind == "custom" and not body.response.strip():
        raise bad_request("自定义命令需要填写 response 模板", "response")
    existing = await db.scalar(
        select(SlashCommand).where(SlashCommand.server_id == ctx.server.id, SlashCommand.name == body.name)
    )
    if existing is not None:
        raise conflict("同名命令已存在", "name")
    command = SlashCommand(
        id=new_id(),
        server_id=ctx.server.id,
        name=body.name,
        description=body.description,
        kind=body.kind,
        response=body.response,
        created_by=ctx.user.id,
    )
    db.add(command)
    await db.commit()
    return {
        "id": command.id,
        "name": command.name,
        "description": command.description,
        "kind": command.kind,
    }


@router.get("/servers/{server_id}/commands")
async def list_commands(
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> list[dict]:
    rows = (
        await db.scalars(
            select(SlashCommand).where(SlashCommand.server_id == ctx.server.id).order_by(SlashCommand.name)
        )
    ).all()
    out = [
        {
            "id": row.id,
            "name": row.name,
            "description": row.description,
            "kind": row.kind,
            "response": row.response,
        }
        for row in rows
    ]
    # 内置命令对所有服务器可见
    out += [
        {
            "id": f"builtin:{name}",
            "name": name,
            "description": description,
            "kind": "builtin",
            "response": "",
        }
        for name, description in BUILTIN_COMMANDS.items()
        if not any(item["name"] == name for item in out)
    ]
    return out


@router.delete("/servers/{server_id}/commands/{command_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_command(
    command_id: str,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    ctx.require("MANAGE_SERVER")
    command = await db.get(SlashCommand, command_id)
    if command is None or command.server_id != ctx.server.id:
        raise not_found("命令")
    await db.delete(command)
    await db.commit()


@router.post("/channels/{channel_id}/commands/{name}")
async def run_command(
    channel_id: str,
    name: str,
    body: CommandRunIn,
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("SEND_MESSAGES")
    ratelimit.check(f"msg:{ctx.user.id}", settings.rate_limit_messages, settings.rate_limit_window_seconds)

    command = await db.scalar(
        select(SlashCommand).where(
            SlashCommand.server_id == ctx.server.id, SlashCommand.name == name.lstrip("/")
        )
    )
    builtin = name.lstrip("/") in BUILTIN_COMMANDS

    if command is None and not builtin:
        raise not_found("命令")

    if builtin or (command is not None and command.kind == "builtin"):
        key = name.lstrip("/")
        if key == "help":
            items = await list_commands(ctx=ServerContext(
                server=ctx.server,
                member=ctx.member,
                perms=ctx.perms,
                user=ctx.user,
            ), db=db)
            listing = "\n".join(f"/{item['name']} — {item['description']}" for item in items)
            return {"type": "command", "name": "help", "response": listing or "暂无可用命令", "ephemeral": True}
        if key == "shrug":
            content = (body.args + " " if body.args.strip() else "") + "¯\\_(ツ)_/¯"
            message = Message(
                id=new_id(),
                channel_id=ctx.channel.id,
                author_id=ctx.user.id,
                content=content,
                msg_type="text",
            )
            db.add(message)
            await db.commit()
            data = (await _serialize_channel_message(db, message, ctx.user.id)).model_dump()
            data["server_id"] = ctx.server.id
            await bus.to_server(ctx.server.id, Event.MESSAGE_CREATE, data)
            return {"type": "command", "name": "shrug", "response": content, "message_id": message.id}

    assert command is not None
    response = command.response.replace("{args}", body.args) if "{args}" in command.response else command.response
    if body.args.strip() and "{args}" not in command.response:
        response = f"{response} {body.args}".strip()
    return {"type": "command", "name": command.name, "response": response, "ephemeral": True}


__all__ = ["BUILTIN_COMMANDS", "router"]
