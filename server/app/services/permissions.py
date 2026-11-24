"""权限矩阵求解。界面里的 13 项权限在这里第一次变成真正的门禁。"""

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.models import MemberRole, PermissionGrant, Role, Server, ServerMember

PERMISSIONS: tuple[str, ...] = (
    "VIEW_CHANNEL",
    "SEND_MESSAGES",
    "MANAGE_MESSAGES",
    "MANAGE_CHANNELS",
    "MANAGE_ROLES",
    "MANAGE_SERVER",
    "KICK_MEMBERS",
    "BAN_MEMBERS",
    "CREATE_INVITE",
    "MENTION_EVERYONE",
    "CONNECT_VOICE",
    "SPEAK",
    "ADMINISTRATOR",
)

# @everyone 基线：能看到、能说话、能建邀请、能进语音
BASE_PERMISSIONS: frozenset[str] = frozenset(
    {"VIEW_CHANNEL", "SEND_MESSAGES", "CREATE_INVITE", "CONNECT_VOICE", "SPEAK"}
)

ALL_PERMISSIONS: frozenset[str] = frozenset(PERMISSIONS)


async def is_member(db: AsyncSession, server_id: str, user_id: str) -> bool:
    row = await db.scalar(
        select(ServerMember).where(ServerMember.server_id == server_id, ServerMember.user_id == user_id)
    )
    return row is not None


async def is_owner(db: AsyncSession, server_id: str, user_id: str) -> bool:
    owner = await db.scalar(select(Server.owner_id).where(Server.id == server_id))
    return owner == user_id


async def effective_permissions(db: AsyncSession, server_id: str, user_id: str) -> set[str]:
    """owner / ADMINISTRATOR 短路为全部；否则 基线 ∪ allow − deny。"""
    if await is_owner(db, server_id, user_id):
        return set(ALL_PERMISSIONS)

    role_ids = list(
        (
            await db.scalars(
                select(MemberRole.role_id).where(MemberRole.server_id == server_id, MemberRole.user_id == user_id)
            )
        ).all()
    )
    if not role_ids:
        # 非成员：什么都没有，交给路由返 404，避免泄露服务器是否存在
        return set()

    grants = list(
        (
            await db.scalars(
                select(PermissionGrant).where(
                    PermissionGrant.server_id == server_id,
                    PermissionGrant.role_id.in_(role_ids),
                )
            )
        ).all()
    )
    allowed = set(BASE_PERMISSIONS)
    denied: set[str] = set()
    for grant in grants:
        if grant.state == "allow":
            allowed.add(grant.permission)
        elif grant.state == "deny":
            denied.add(grant.permission)

    # 角色 position 越高越优先：简单地让 deny 覆盖 allow（保守，宁可误拒）
    result = allowed - denied
    if "ADMINISTRATOR" in result:
        return set(ALL_PERMISSIONS)
    return result


async def has_permission(db: AsyncSession, server_id: str, user_id: str, permission: str) -> bool:
    return permission in await effective_permissions(db, server_id, user_id)


async def top_role_position(db: AsyncSession, server_id: str, user_id: str) -> int:
    """用于「只能操作比自己低的角色」这类校验；owner 返回极大值。"""
    if await is_owner(db, server_id, user_id):
        return 1_000_000
    role_ids = list(
        (
            await db.scalars(
                select(MemberRole.role_id).where(MemberRole.server_id == server_id, MemberRole.user_id == user_id)
            )
        ).all()
    )
    if not role_ids:
        return -1
    positions = list((await db.scalars(select(Role.position).where(Role.id.in_(role_ids)))).all())
    return max(positions, default=-1)
