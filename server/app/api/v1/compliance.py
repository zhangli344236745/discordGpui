"""举报与合规：举报状态机、内容保留策略、法务持有、数据导出与账号删除。"""


from fastapi import APIRouter, Depends, Query, status
from pydantic import BaseModel, Field
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db, utcnow
from app.deps import ServerContext, current_user, server_context
from app.errors import bad_request, not_found
from app.ids import new_id
from app.models import (
    Attachment,
    AuditEntry,
    Channel,
    LegalHold,
    Message,
    Report,
    ReportAction,
    RetentionRule,
    Server,
    ServerMember,
    Session,
    User,
)
from app.realtime.bus import bus
from app.realtime.events import Event
from app.security import verify_password
from app.services import notifications
from app.services.permissions import effective_permissions

router = APIRouter(tags=["compliance"])

VALID_TARGET_KINDS = ("message", "user", "server")
VALID_ACTIONS = ("resolve", "reject", "escalate")


class ReportIn(BaseModel):
    target_kind: str = Field(pattern="^(message|user|server)$")
    target_id: str = Field(min_length=1, max_length=24)
    reason: str = Field(min_length=2, max_length=64)
    detail: str = Field(default="", max_length=500)
    server_id: str | None = Field(default=None, max_length=24)


class ReportActionIn(BaseModel):
    action: str = Field(pattern="^(resolve|reject|escalate)$")
    note: str = Field(default="", max_length=300)
    delete_content: bool = False


class RetentionRuleIn(BaseModel):
    message_days: int = Field(default=0, ge=0, le=3650)
    attachment_days: int = Field(default=0, ge=0, le=3650)
    audit_days: int = Field(default=0, ge=0, le=3650)


class LegalHoldIn(BaseModel):
    target_kind: str = Field(pattern="^(message|channel|user)$")
    target_id: str = Field(min_length=1, max_length=24)
    note: str = Field(default="", max_length=200)


class AccountDeleteIn(BaseModel):
    password: str = Field(min_length=1, max_length=128)


async def _moderates(db: AsyncSession, server_id: str, user_id: str) -> bool:
    perms = await effective_permissions(db, server_id, user_id)
    return "MANAGE_MESSAGES" in perms


@router.post("/reports", status_code=status.HTTP_201_CREATED)
async def create_report(
    body: ReportIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    # 目标必须真实存在，避免凭空举报刷屏
    if body.target_kind == "message":
        target = await db.get(Message, body.target_id)
        if target is None or target.deleted_at is not None:
            raise not_found("消息")
        target_channel = await db.get(Channel, target.channel_id)
        server_id = target_channel.server_id if target_channel is not None else None
    elif body.target_kind == "user":
        target_user = await db.get(User, body.target_id)
        if target_user is None:
            raise not_found("用户")
        server_id = body.server_id
    else:
        server = await db.get(Server, body.target_id)
        if server is None:
            raise not_found("服务器")
        server_id = server.id

    if server_id:
        member = await db.scalar(
            select(ServerMember).where(ServerMember.server_id == server_id, ServerMember.user_id == user.id)
        )
        if member is None:
            raise not_found("服务器")
    else:
        server_id = body.server_id

    existing = await db.scalar(
        select(Report).where(
            Report.reporter_id == user.id,
            Report.target_kind == body.target_kind,
            Report.target_id == body.target_id,
        )
    )
    if existing is not None:
        # 重复举报合并计数，不生成新的处理项
        existing.count += 1
        if body.detail:
            existing.detail = body.detail[:500]
        await db.commit()
        return {"id": existing.id, "merged": True, "count": existing.count}

    report = Report(
        id=new_id(),
        server_id=server_id,
        reporter_id=user.id,
        target_kind=body.target_kind,
        target_id=body.target_id,
        reason=body.reason,
        detail=body.detail,
    )
    db.add(report)
    await db.flush()

    # 通知该服务器有处理权限的成员（最多提醒 20 人，避免大群风暴）
    if server_id:
        members = (
            await db.scalars(
                select(ServerMember.user_id).where(ServerMember.server_id == server_id).limit(500)
            )
        ).all()
        notified = 0
        for member_id in members:
            if notified >= 20:
                break
            if await _moderates(db, server_id, member_id):
                notifications.create(
                    db,
                    member_id,
                    kind="report",
                    target_kind="report",
                    target_id=report.id,
                    title="收到新的举报",
                    body=f"{body.reason}（{body.target_kind}）",
                    actor_id=user.id,
                )
                notified += 1
    await db.commit()
    return {"id": report.id, "merged": False, "count": 1}


@router.get("/servers/{server_id}/reports")
async def list_reports(
    status_filter: str = Query(default="pending", alias="status"),
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> list[dict]:
    ctx.require("MANAGE_MESSAGES")
    stmt = select(Report).where(Report.server_id == ctx.server.id)
    if status_filter != "all":
        stmt = stmt.where(Report.status == status_filter)
    rows = (await db.scalars(stmt.order_by(Report.id.desc()).limit(200))).all()
    reporters = {
        u.id: u
        for u in (
            await db.scalars(select(User).where(User.id.in_([r.reporter_id for r in rows] or [""])))
        ).all()
    }
    return [
        {
            "id": row.id,
            "target_kind": row.target_kind,
            "target_id": row.target_id,
            "reason": row.reason,
            "detail": row.detail,
            "status": row.status,
            "count": row.count,
            "reporter": reporters[row.reporter_id].display_name if row.reporter_id in reporters else "已注销",
            "created_at": row.created_at.isoformat(),
        }
        for row in rows
    ]


@router.get("/me/reports")
async def my_reports(
    db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> list[dict]:
    rows = (
        await db.scalars(
            select(Report).where(Report.reporter_id == user.id).order_by(Report.id.desc()).limit(100)
        )
    ).all()
    return [
        {
            "id": row.id,
            "target_kind": row.target_kind,
            "target_id": row.target_id,
            "reason": row.reason,
            "status": row.status,
            "count": row.count,
            "created_at": row.created_at.isoformat(),
        }
        for row in rows
    ]


@router.post("/reports/{report_id}/actions", status_code=status.HTTP_201_CREATED)
async def act_on_report(
    report_id: str,
    body: ReportActionIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    report = await db.get(Report, report_id)
    if report is None or report.server_id is None:
        raise not_found("举报")
    server = await db.get(Server, report.server_id)
    if server is None:
        raise not_found("服务器")
    member = await db.scalar(
        select(ServerMember).where(
            ServerMember.server_id == server.id, ServerMember.user_id == user.id
        )
    )
    if member is None or not await _moderates(db, server.id, user.id):
        raise not_found("举报")
    ctx = ServerContext(server=server, member=member, perms={"MANAGE_MESSAGES"}, user=user)

    deleted = False
    if body.delete_content and report.target_kind == "message":
        message = await db.get(Message, report.target_id)
        message_channel = await db.get(Channel, message.channel_id) if message is not None else None
        if message is not None and message_channel is not None and message_channel.server_id == server.id:
            hold = await db.scalar(
                select(LegalHold).where(
                    LegalHold.server_id == ctx.server.id,
                    LegalHold.target_kind == "message",
                    LegalHold.target_id == message.id,
                )
            )
            if hold is not None:
                raise bad_request("该消息处于法务持有，不能删除", "target_id")
            message.deleted_at = utcnow()
            deleted = True
            await bus.to_server(
                ctx.server.id,
                Event.MESSAGE_DELETE,
                {"id": message.id, "channel_id": message.channel_id},
            )

    status_map = {"resolve": "resolved", "reject": "rejected", "escalate": "escalated"}
    report.status = status_map[body.action]
    db.add(
        ReportAction(
            id=new_id(),
            report_id=report.id,
            actor_id=ctx.user.id,
            action=body.action,
            note=body.note,
            deleted_content=deleted,
        )
    )
    notifications.create(
        db,
        report.reporter_id,
        kind="report",
        target_kind="report",
        target_id=report.id,
        title="举报已处理",
        body={
            "resolve": "你的举报已被处理",
            "reject": "你的举报被驳回",
            "escalate": "你的举报已升级处理",
        }[body.action] + (f"：{body.note}" if body.note else ""),
        actor_id=ctx.user.id,
    )
    db.add(
        AuditEntry(
            id=new_id(),
            server_id=ctx.server.id,
            actor_id=ctx.user.id,
            kind="report",
            target=f"{report.target_kind}:{report.target_id}",
            detail=f"{body.action}{'（删除内容）' if deleted else ''}",
        )
    )
    await db.commit()
    return {"report_id": report.id, "status": report.status, "deleted_content": deleted}


# ---------------------------------------------------------------------------
# 内容保留策略与法务持有
# ---------------------------------------------------------------------------


@router.get("/servers/{server_id}/retention")
async def get_retention(
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    rule = await db.get(RetentionRule, ctx.server.id)
    if rule is None:
        return {"message_days": 0, "attachment_days": 0, "audit_days": 0}
    return {
        "message_days": rule.message_days,
        "attachment_days": rule.attachment_days,
        "audit_days": rule.audit_days,
    }


@router.put("/servers/{server_id}/retention")
async def set_retention(
    body: RetentionRuleIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    rule = await db.get(RetentionRule, ctx.server.id)
    if rule is None:
        rule = RetentionRule(server_id=ctx.server.id)
        db.add(rule)
    rule.message_days = body.message_days
    rule.attachment_days = body.attachment_days
    rule.audit_days = body.audit_days
    await db.commit()
    return {
        "message_days": rule.message_days,
        "attachment_days": rule.attachment_days,
        "audit_days": rule.audit_days,
    }


@router.get("/servers/{server_id}/legal-holds")
async def list_legal_holds(
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> list[dict]:
    ctx.require("MANAGE_SERVER")
    rows = (
        await db.scalars(select(LegalHold).where(LegalHold.server_id == ctx.server.id))
    ).all()
    return [
        {
            "id": row.id,
            "target_kind": row.target_kind,
            "target_id": row.target_id,
            "note": row.note,
            "created_at": row.created_at.isoformat(),
        }
        for row in rows
    ]


@router.post("/servers/{server_id}/legal-holds", status_code=status.HTTP_201_CREATED)
async def create_legal_hold(
    body: LegalHoldIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    existing = await db.scalar(
        select(LegalHold).where(
            LegalHold.server_id == ctx.server.id,
            LegalHold.target_kind == body.target_kind,
            LegalHold.target_id == body.target_id,
        )
    )
    if existing is not None:
        existing.note = body.note
        await db.commit()
        return {"id": existing.id, "merged": True}
    hold = LegalHold(
        id=new_id(),
        server_id=ctx.server.id,
        target_kind=body.target_kind,
        target_id=body.target_id,
        note=body.note,
    )
    db.add(hold)
    await db.commit()
    return {"id": hold.id, "merged": False}


@router.delete("/servers/{server_id}/legal-holds/{hold_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_legal_hold(
    hold_id: str,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    ctx.require("MANAGE_SERVER")
    hold = await db.get(LegalHold, hold_id)
    if hold is None or hold.server_id != ctx.server.id:
        raise not_found("法务持有")
    await db.delete(hold)
    await db.commit()


# ---------------------------------------------------------------------------
# 用户数据导出与账号删除
# ---------------------------------------------------------------------------


@router.get("/me/data-export")
async def data_export(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    server_rows = (
        await db.execute(
            select(Server.id, Server.name)
            .join(ServerMember, ServerMember.server_id == Server.id)
            .where(ServerMember.user_id == user.id)
        )
    ).all()

    messages = (
        await db.scalars(
            select(Message)
            .where(Message.author_id == user.id, Message.deleted_at.is_(None))
            .order_by(Message.id.desc())
            .limit(1000)
        )
    ).all()

    attachments = (
        await db.scalars(
            select(Attachment).where(Attachment.owner_id == user.id).limit(500)
        )
    ).all()

    return {
        "profile": {
            "id": user.id,
            "username": user.username,
            "tag": user.tag,
            "email": user.email,
            "display_name": user.display_name,
            "created_at": user.created_at.isoformat(),
        },
        "servers": [{"id": server_id, "name": name} for server_id, name in server_rows],
        "messages": [
            {
                "id": m.id,
                "channel_id": m.channel_id,
                "content": m.content,
                "created_at": m.created_at.isoformat(),
            }
            for m in reversed(messages)
        ],
        "attachments": [
            {"id": a.id, "filename": a.filename, "size": a.size, "mime": a.mime} for a in attachments
        ],
        "generated_at": utcnow().isoformat(),
    }


@router.post("/me/account/delete", status_code=status.HTTP_204_NO_CONTENT)
async def delete_account(
    body: AccountDeleteIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    if not verify_password(body.password, user.password_hash):
        raise bad_request("密码不正确", "password")
    if user.is_bot:  # pragma: no cover - bot 不走用户接口
        raise bad_request("机器人账号不支持自助删除", "password")

    now = utcnow()
    # 个人内容软删；审计与举报记录保留但作者匿名化
    await db.execute(
        Message.__table__.update()
        .where(Message.author_id == user.id, Message.deleted_at.is_(None))
        .values(deleted_at=now)
    )
    user.username = f"deleted-{user.id[:8]}"
    user.display_name = "已注销用户"
    user.email = f"deleted-{user.id[:12]}@deleted.local"
    user.password_hash = "!"
    user.about = ""
    user.deleted_at = now
    for session in (await db.scalars(select(Session).where(Session.user_id == user.id))).all():
        session.revoked = True
    await db.commit()
    await bus.disconnect_user(user.id)


__all__ = ["router"]
