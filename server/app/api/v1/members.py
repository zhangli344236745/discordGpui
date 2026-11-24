"""成员、角色、权限矩阵、频道覆盖与封禁。"""

from datetime import UTC, datetime

from fastapi import APIRouter, Depends
from sqlalchemy import delete, func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ChannelContext, ServerContext, channel_context, current_user, server_context
from app.errors import ApiError, ErrorCode, forbidden, not_found
from app.models import (
    Ban,
    ChannelOverride,
    MemberRole,
    PermissionGrant,
    Role,
    Server,
    ServerMember,
    User,
)
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.guild_admin import (
    BanIn,
    MemberPatch,
    PermissionMatrixIn,
    RoleAssignIn,
    RoleCreate,
    RolePatch,
)
from app.services import audit, guild_admin
from app.services.permissions import ALL_PERMISSIONS, assert_can_act, effective_permissions

router = APIRouter(tags=["members"])


def _role_out(role: Role) -> dict:
    return {
        "id": role.id,
        "server_id": role.server_id,
        "name": role.name,
        "color": role.color,
        "position": role.position,
        "is_everyone": role.is_everyone,
    }


def _member_event(server_id: str, user_id: str) -> dict:
    return {"server_id": server_id, "user_id": user_id}


async def _member_out(db: AsyncSession, server_id: str, user_id: str, owner_id: str) -> dict:
    payload = await guild_admin.members_payload(db, server_id, owner_id)
    found = next((m for m in payload if m["user_id"] == user_id), None)
    if found is None:
        raise not_found("成员")
    return found


async def _role_scope(
    db: AsyncSession, role_id: str, user: User
) -> tuple[Role, Server, set[str]]:
    role = await guild_admin.get_role(db, role_id)
    server = await db.get(Server, role.server_id)
    if server is None:
        raise not_found("服务器")
    member = await db.get(ServerMember, (server.id, user.id))
    if member is None:
        raise not_found("角色")
    perms = await effective_permissions(db, server.id, user.id, None)
    return role, server, perms


# ---------------------------------------------------------------------------
# 成员
# ---------------------------------------------------------------------------


@router.get("/servers/{server_id}/members")
async def list_members(ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)) -> list[dict]:
    return await guild_admin.members_payload(db, ctx.server.id, ctx.server.owner_id)


@router.patch("/servers/{server_id}/members/{user_id}")
async def patch_member(
    server_id: str,
    user_id: str,
    body: MemberPatch,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    if user_id != ctx.user.id:
        ctx.require("MANAGE_SERVER")
    member = await guild_admin.get_member(db, server_id, user_id)
    member.nickname = body.nickname.strip()
    audit.log(db, server_id, ctx.user.id, audit.KIND_MEMBER, f"修改了成员昵称 / {user_id}")
    payload = await _member_out(db, server_id, user_id, ctx.server.owner_id)
    outbox.queue(db, "server", server_id, Event.MEMBER_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/servers/{server_id}/members/{user_id}")
async def kick_member(
    server_id: str,
    user_id: str,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("KICK_MEMBERS")
    target = await guild_admin.get_member(db, server_id, user_id)
    if user_id == ctx.server.owner_id:
        raise forbidden("KICK_MEMBERS")
    actor_top = await guild_admin.member_top_position(db, server_id, ctx.user.id, ctx.server.owner_id)
    target_top = await guild_admin.member_top_position(db, server_id, user_id, ctx.server.owner_id)
    assert_can_act(actor_top, target_top, "移出")

    await db.delete(target)
    await db.execute(delete(MemberRole).where(MemberRole.server_id == server_id, MemberRole.user_id == user_id))
    ctx.server.member_count = max(0, ctx.server.member_count - 1)
    audit.log(db, server_id, ctx.user.id, audit.KIND_MEMBER, f"移出了成员 / {user_id}")
    outbox.queue(db, "server", server_id, Event.MEMBER_LEAVE, _member_event(server_id, user_id))
    await db.commit()
    return {"removed": True, "user_id": user_id}


# ---------------------------------------------------------------------------
# 角色
# ---------------------------------------------------------------------------


@router.get("/servers/{server_id}/roles")
async def list_roles(ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)) -> list[dict]:
    rows = (
        await db.scalars(
            select(Role).where(Role.server_id == ctx.server.id).order_by(Role.position.desc(), Role.created_at)
        )
    ).all()
    return [_role_out(role) for role in rows]


@router.post("/servers/{server_id}/roles", status_code=201)
async def create_role(
    body: RoleCreate,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_ROLES")
    max_position = await db.scalar(
        select(func.max(Role.position)).where(Role.server_id == ctx.server.id)
    ) or 0
    position = body.position if body.position is not None else int(max_position) + 1
    actor_top = await guild_admin.member_top_position(db, ctx.server.id, ctx.user.id, ctx.server.owner_id)
    if position >= actor_top:
        raise ApiError(403, ErrorCode.FORBIDDEN, "不能创建不低于自己最高角色的角色", field="position")

    role = Role(
        server_id=ctx.server.id,
        name=body.name.strip(),
        color=body.color,
        position=position,
        is_everyone=False,
    )
    db.add(role)
    await db.flush()
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_ROLE, f"创建了角色 / {role.name}")
    payload = _role_out(role)
    outbox.queue(db, "server", ctx.server.id, Event.ROLE_CREATE, payload)
    await db.commit()
    return payload


@router.patch("/roles/{role_id}")
async def patch_role(
    role_id: str,
    body: RolePatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    role, server, perms = await _role_scope(db, role_id, user)
    if "MANAGE_ROLES" not in perms:
        raise forbidden("MANAGE_ROLES")
    actor_top = await guild_admin.member_top_position(db, server.id, user.id, server.owner_id)
    guild_admin.assert_role_manageable(actor_top, role)

    if body.position is not None:
        if role.is_everyone:
            raise ApiError(403, ErrorCode.FORBIDDEN, "@everyone 的位置固定为最低", field="position")
        if body.position >= actor_top:
            raise ApiError(403, ErrorCode.FORBIDDEN, "不能把角色移到不低于自己的位置", field="position")
        role.position = body.position
    if body.name is not None:
        if role.is_everyone:
            raise ApiError(403, ErrorCode.FORBIDDEN, "@everyone 不能改名", field="name")
        role.name = body.name.strip()
    if body.color is not None:
        role.color = body.color

    audit.log(db, server.id, user.id, audit.KIND_ROLE, f"更新了角色 / {role.name}")
    payload = _role_out(role)
    outbox.queue(db, "server", server.id, Event.ROLE_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/roles/{role_id}")
async def delete_role(
    role_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    role, server, perms = await _role_scope(db, role_id, user)
    if "MANAGE_ROLES" not in perms:
        raise forbidden("MANAGE_ROLES")
    if role.is_everyone:
        raise ApiError(409, ErrorCode.CONFLICT, "@everyone 不能删除", field="role_id")
    actor_top = await guild_admin.member_top_position(db, server.id, user.id, server.owner_id)
    guild_admin.assert_role_manageable(actor_top, role)

    await db.execute(delete(MemberRole).where(MemberRole.role_id == role.id))
    await db.execute(delete(PermissionGrant).where(PermissionGrant.role_id == role.id))
    await db.execute(delete(ChannelOverride).where(ChannelOverride.role_id == role.id))
    await db.delete(role)
    audit.log(db, server.id, user.id, audit.KIND_ROLE, f"删除了角色 / {role.name}")
    outbox.queue(db, "server", server.id, Event.ROLE_DELETE, {"id": role_id, "server_id": server.id})
    await db.commit()
    return {"deleted": True}


@router.put("/servers/{server_id}/members/{user_id}/roles/{role_id}")
async def assign_role(
    server_id: str,
    user_id: str,
    role_id: str,
    body: RoleAssignIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_ROLES")
    await guild_admin.get_member(db, server_id, user_id)
    role = await guild_admin.get_role(db, role_id)
    if role.server_id != server_id:
        raise not_found("角色")
    actor_top = await guild_admin.member_top_position(db, server_id, ctx.user.id, ctx.server.owner_id)
    guild_admin.assert_role_manageable(actor_top, role)

    existing = await db.get(MemberRole, (server_id, user_id, role_id))
    if body.assigned and existing is None:
        db.add(MemberRole(server_id=server_id, user_id=user_id, role_id=role_id))
    elif not body.assigned and existing is not None:
        await db.delete(existing)

    audit.log(
        db,
        server_id,
        ctx.user.id,
        audit.KIND_ROLE,
        f"{'授予' if body.assigned else '移除'}角色 / {role.name} → {user_id}",
    )
    payload = await _member_out(db, server_id, user_id, ctx.server.owner_id)
    outbox.queue(db, "server", server_id, Event.MEMBER_UPDATE, payload)
    await db.commit()
    return payload


# ---------------------------------------------------------------------------
# 权限矩阵与频道覆盖
# ---------------------------------------------------------------------------


@router.get("/servers/{server_id}/permissions")
async def get_permission_matrix(
    ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)
) -> dict:
    ctx.require("MANAGE_ROLES")
    rows = (
        await db.scalars(select(PermissionGrant).where(PermissionGrant.server_id == ctx.server.id))
    ).all()
    return {
        "permissions": list(ALL_PERMISSIONS),
        "items": [
            {"role_id": row.role_id, "permission": row.permission, "state": row.state} for row in rows
        ],
    }


@router.put("/servers/{server_id}/permissions")
async def write_permission_matrix(
    body: PermissionMatrixIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_ROLES")
    actor_top = await guild_admin.member_top_position(db, ctx.server.id, ctx.user.id, ctx.server.owner_id)
    roles = {
        role.id: role
        for role in (
            await db.scalars(select(Role).where(Role.server_id == ctx.server.id))
        ).all()
    }
    for item in body.items:
        if item.permission not in ALL_PERMISSIONS:
            raise ApiError(400, ErrorCode.VALIDATION_ERROR, f"未知权限 {item.permission}", "permission")
        role = roles.get(item.role_id)
        if role is None:
            raise not_found("角色")
        guild_admin.assert_role_manageable(actor_top, role)

    role_ids = {item.role_id for item in body.items}
    if role_ids:
        await db.execute(
            delete(PermissionGrant).where(
                PermissionGrant.server_id == ctx.server.id, PermissionGrant.role_id.in_(role_ids)
            )
        )
    for item in body.items:
        if item.state != "inherit":
            db.add(
                PermissionGrant(
                    server_id=ctx.server.id,
                    role_id=item.role_id,
                    permission=item.permission,
                    state=item.state,
                )
            )
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_ROLE, "更新了权限矩阵")
    outbox.queue(db, "server", ctx.server.id, Event.PERMISSIONS_UPDATE, {"server_id": ctx.server.id})
    await db.commit()
    return {"updated": len(body.items)}


@router.get("/channels/{channel_id}/overrides")
async def get_channel_overrides(
    ctx: ChannelContext = Depends(channel_context), db: AsyncSession = Depends(get_db)
) -> dict:
    rows = list(
        (await db.scalars(select(ChannelOverride).where(ChannelOverride.channel_id == ctx.channel.id))).all()
    )
    return {
        "channel_id": ctx.channel.id,
        "items": [
            {"role_id": row.role_id, "permission": row.permission, "state": row.state} for row in rows
        ],
    }


@router.put("/channels/{channel_id}/overrides")
async def write_channel_overrides(
    body: PermissionMatrixIn,
    ctx: ChannelContext = Depends(channel_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_CHANNELS")
    roles = {
        role.id: role
        for role in (
            await db.scalars(select(Role).where(Role.server_id == ctx.server.id))
        ).all()
    }
    actor_top = await guild_admin.member_top_position(db, ctx.server.id, ctx.user.id, ctx.server.owner_id)
    for item in body.items:
        if item.permission not in ALL_PERMISSIONS:
            raise ApiError(400, ErrorCode.VALIDATION_ERROR, f"未知权限 {item.permission}", "permission")
        role = roles.get(item.role_id)
        if role is None:
            raise not_found("角色")
        guild_admin.assert_role_manageable(actor_top, role)

    for item in body.items:
        await db.execute(
            delete(ChannelOverride).where(
                ChannelOverride.channel_id == ctx.channel.id,
                ChannelOverride.role_id == item.role_id,
                ChannelOverride.permission == item.permission,
            )
        )
        if item.state != "inherit":
            db.add(
                ChannelOverride(
                    channel_id=ctx.channel.id,
                    role_id=item.role_id,
                    permission=item.permission,
                    state=item.state,
                )
            )
    outbox.queue(db, "server", ctx.server.id, Event.PERMISSIONS_UPDATE, {"server_id": ctx.server.id})
    await db.commit()
    return {"updated": len(body.items)}


# ---------------------------------------------------------------------------
# 封禁
# ---------------------------------------------------------------------------


@router.get("/servers/{server_id}/bans")
async def list_bans(ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)) -> list[dict]:
    ctx.require("BAN_MEMBERS")
    rows = (
        await db.execute(
            select(Ban, User).join(User, User.id == Ban.user_id).where(Ban.server_id == ctx.server.id)
        )
    ).all()
    return [
        {
            "user_id": ban.user_id,
            "username": user.username,
            "tag": user.tag,
            "display_name": user.display_name,
            "reason": ban.reason,
            "banned_by": ban.banned_by,
            "created_at": guild_admin.iso(ban.created_at),
        }
        for ban, user in rows
    ]


@router.post("/servers/{server_id}/bans", status_code=201)
async def create_ban(
    body: BanIn,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("BAN_MEMBERS")
    if body.user_id == ctx.server.owner_id:
        raise forbidden("BAN_MEMBERS")
    target = await db.get(User, body.user_id)
    if target is None:
        raise not_found("用户")
    actor_top = await guild_admin.member_top_position(db, ctx.server.id, ctx.user.id, ctx.server.owner_id)
    target_top = await guild_admin.member_top_position(db, ctx.server.id, body.user_id, ctx.server.owner_id)
    assert_can_act(actor_top, target_top, "封禁")

    existing = await db.get(Ban, (ctx.server.id, body.user_id))
    if existing is None:
        db.add(
            Ban(
                server_id=ctx.server.id,
                user_id=body.user_id,
                reason=body.reason.strip(),
                banned_by=ctx.user.id,
            )
        )
    else:
        existing.reason = body.reason.strip()
        existing.banned_by = ctx.user.id

    member = await db.get(ServerMember, (ctx.server.id, body.user_id))
    if member is not None:
        await db.delete(member)
        await db.execute(
            delete(MemberRole).where(MemberRole.server_id == ctx.server.id, MemberRole.user_id == body.user_id)
        )
        ctx.server.member_count = max(0, ctx.server.member_count - 1)
    deleted = await guild_admin.soft_delete_recent_messages(
        db, ctx.server.id, body.user_id, body.delete_days
    )
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_MEMBER, f"封禁了成员 / {target.username}")
    outbox.queue(db, "server", ctx.server.id, Event.MEMBER_LEAVE, _member_event(ctx.server.id, body.user_id))
    await db.commit()
    return {
        "user_id": body.user_id,
        "reason": body.reason,
        "delete_days": body.delete_days,
        "messages_deleted": deleted,
        "created_at": guild_admin.iso(datetime.now(UTC)),
    }


@router.delete("/servers/{server_id}/bans/{user_id}")
async def remove_ban(
    server_id: str,
    user_id: str,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("BAN_MEMBERS")
    ban = await db.get(Ban, (server_id, user_id))
    if ban is not None:
        await db.delete(ban)
        audit.log(db, server_id, ctx.user.id, audit.KIND_MEMBER, f"撤销封禁 / {user_id}")
        await db.commit()
    return {"user_id": user_id, "banned": False}
