"""自动审核规则与审核日志。"""

from fastapi import APIRouter, Depends, Query
from sqlalchemy import delete, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ServerContext, current_user, server_context
from app.errors import ApiError, ErrorCode, forbidden, not_found
from app.models import AuditEntry, AutoModKeyword, AutoModRule, Server, ServerMember, User
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.community import AutoModRuleIn, AutoModRulePatch, AutoModTestIn
from app.services import audit, automod, guild_admin
from app.services.permissions import effective_permissions

router = APIRouter(tags=["moderation"])


def _rule_out(rule: AutoModRule, keywords: list[str]) -> dict:
    return {
        "id": rule.id,
        "server_id": rule.server_id,
        "name": rule.name,
        "enabled": rule.enabled,
        "action": rule.action,
        "keywords": keywords,
        "created_at": guild_admin.iso(rule.created_at),
    }


async def _keywords(db: AsyncSession, rule_id: str) -> list[str]:
    return list(
        (await db.scalars(select(AutoModKeyword.keyword).where(AutoModKeyword.rule_id == rule_id))).all()
    )


async def _rule_scope(db: AsyncSession, rule_id: str, user: User) -> tuple[AutoModRule, Server, set[str]]:
    rule = await db.get(AutoModRule, rule_id)
    if rule is None:
        raise not_found("自动审核规则")
    server = await db.get(Server, rule.server_id)
    if server is None:
        raise not_found("自动审核规则")
    if await db.get(ServerMember, (server.id, user.id)) is None:
        raise not_found("自动审核规则")
    perms = await effective_permissions(db, server.id, user.id)
    return rule, server, perms


def _clean_keywords(values: list[str]) -> list[str]:
    cleaned: list[str] = []
    for value in values:
        word = value.strip()
        if word and word not in cleaned:
            cleaned.append(word)
    if not cleaned:
        raise ApiError(400, ErrorCode.VALIDATION_ERROR, "关键词不能为空", "keywords")
    return cleaned


@router.get("/servers/{server_id}/automod/rules")
async def list_automod_rules(
    ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)
) -> list[dict]:
    ctx.require("MANAGE_SERVER")
    rules = list(
        (await db.scalars(select(AutoModRule).where(AutoModRule.server_id == ctx.server.id))).all()
    )
    return [_rule_out(rule, await _keywords(db, rule.id)) for rule in rules]


@router.post("/servers/{server_id}/automod/rules", status_code=201)
async def create_automod_rule(
    body: AutoModRuleIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    keywords = _clean_keywords(body.keywords)
    rule = AutoModRule(
        server_id=ctx.server.id,
        name=body.name.strip(),
        enabled=body.enabled,
        action=body.action,
    )
    db.add(rule)
    await db.flush()
    for word in keywords:
        db.add(AutoModKeyword(rule_id=rule.id, keyword=word))
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_SERVER, f"创建了自动审核规则 / {rule.name}")
    payload = _rule_out(rule, keywords)
    outbox.queue(db, "server", ctx.server.id, Event.AUTOMOD_RULE_CREATE, payload)
    await db.commit()
    return payload


@router.patch("/automod/rules/{rule_id}")
async def patch_automod_rule(
    rule_id: str,
    body: AutoModRulePatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    rule, server, perms = await _rule_scope(db, rule_id, user)
    if "MANAGE_SERVER" not in perms:
        raise forbidden("MANAGE_SERVER")

    if body.name is not None:
        rule.name = body.name.strip()
    if body.action is not None:
        rule.action = body.action
    if body.enabled is not None:
        rule.enabled = body.enabled
    keywords = await _keywords(db, rule.id)
    if body.keywords is not None:
        keywords = _clean_keywords(body.keywords)
        await db.execute(delete(AutoModKeyword).where(AutoModKeyword.rule_id == rule.id))
        for word in keywords:
            db.add(AutoModKeyword(rule_id=rule.id, keyword=word))

    audit.log(db, server.id, user.id, audit.KIND_SERVER, f"更新了自动审核规则 / {rule.name}")
    payload = _rule_out(rule, keywords)
    outbox.queue(db, "server", server.id, Event.AUTOMOD_RULE_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/automod/rules/{rule_id}")
async def delete_automod_rule(
    rule_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    rule, server, perms = await _rule_scope(db, rule_id, user)
    if "MANAGE_SERVER" not in perms:
        raise forbidden("MANAGE_SERVER")
    await db.execute(delete(AutoModKeyword).where(AutoModKeyword.rule_id == rule.id))
    await db.delete(rule)
    audit.log(db, server.id, user.id, audit.KIND_SERVER, f"删除了自动审核规则 / {rule.name}")
    outbox.queue(db, "server", server.id, Event.AUTOMOD_RULE_DELETE, {"id": rule_id, "server_id": server.id})
    await db.commit()
    return {"deleted": True}


@router.post("/servers/{server_id}/automod/test")
async def test_automod(
    body: AutoModTestIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    result = await automod.scan(db, ctx.server.id, body.content)
    matches = [
        {"rule_id": hit.rule_id, "rule_name": hit.rule_name, "action": hit.action, "keyword": hit.keyword}
        for hit in ([result.blocked] if result.blocked else []) + result.warnings
    ]
    return {"blocked": result.blocked is not None, "matches": matches}


@router.get("/servers/{server_id}/audit")
async def list_audit(
    kind: str = "",
    before: str | None = None,
    limit: int = Query(default=50, le=100),
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    stmt = select(AuditEntry, User).join(User, User.id == AuditEntry.actor_id).where(
        AuditEntry.server_id == ctx.server.id
    )
    if kind:
        stmt = stmt.where(AuditEntry.kind == kind)
    if before:
        stmt = stmt.where(AuditEntry.id < before)
    rows = (await db.execute(stmt.order_by(AuditEntry.id.desc()).limit(limit + 1))).all()
    has_more = len(rows) > limit
    rows = rows[:limit]
    return {
        "items": [
            {
                "id": entry.id,
                "server_id": entry.server_id,
                "kind": entry.kind,
                "actor": {
                    "id": actor.id,
                    "display_name": actor.display_name,
                    "avatar_color": actor.avatar_color,
                },
                "detail": entry.detail,
                "target": entry.target,
                "created_at": guild_admin.iso(entry.created_at),
            }
            for entry, actor in rows
        ],
        "has_more": has_more,
    }


@router.delete("/servers/{server_id}/audit")
async def clear_audit(
    ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)
) -> dict:
    if not ctx.is_owner:
        raise forbidden("MANAGE_SERVER")
    result = await db.execute(delete(AuditEntry).where(AuditEntry.server_id == ctx.server.id))
    await db.commit()
    return {"deleted": result.rowcount or 0}
