"""权限矩阵求解。界面里的 13 项权限在这里变成真正的门禁。

计算顺序：owner/ADMINISTRATOR 短路 → @everyone 基线 → 成员角色叠加 →
频道级覆盖（deny 优先）。所有写接口先过这里，读接口用 visible_channel_ids()
批量求可见频道，避免逐频道算权限。
"""

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.errors import ApiError, ErrorCode
from app.models import Channel, ChannelOverride, MemberRole, PermissionGrant, Role, Server, ServerMember

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


async def everyone_role_id(db: AsyncSession, server_id: str) -> str | None:
    return await db.scalar(select(Role.id).where(Role.server_id == server_id, Role.is_everyone.is_(True)))


async def role_ids_for(db: AsyncSession, server_id: str, user_id: str) -> list[str]:
    """成员显式角色 + @everyone；按 position 升序，保证叠加结果稳定。"""
    ids = list(
        (
            await db.scalars(
                select(MemberRole.role_id)
                .join(Role, Role.id == MemberRole.role_id)
                .where(MemberRole.server_id == server_id, MemberRole.user_id == user_id)
                .order_by(Role.position)
            )
        ).all()
    )
    everyone = await everyone_role_id(db, server_id)
    if everyone and everyone not in ids:
        ids.insert(0, everyone)
    return ids


async def _server_grants(db: AsyncSession, server_id: str, role_ids: list[str]) -> tuple[set[str], set[str]]:
    allowed = set(BASE_PERMISSIONS)
    denied: set[str] = set()
    if not role_ids:
        return allowed, denied
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
    for grant in grants:
        if grant.state == "allow":
            allowed.add(grant.permission)
        elif grant.state == "deny":
            denied.add(grant.permission)
    return allowed, denied


async def _channel_overrides(
    db: AsyncSession, channel_ids: list[str], role_ids: list[str]
) -> dict[str, list[ChannelOverride]]:
    if not channel_ids or not role_ids:
        return {}
    rows = list(
        (
            await db.scalars(
                select(ChannelOverride).where(
                    ChannelOverride.channel_id.in_(channel_ids),
                    ChannelOverride.role_id.in_(role_ids),
                )
            )
        ).all()
    )
    grouped: dict[str, list[ChannelOverride]] = {}
    for row in rows:
        grouped.setdefault(row.channel_id, []).append(row)
    return grouped


def _apply(overrides: list[ChannelOverride], allowed: set[str], denied: set[str]) -> set[str]:
    for override in overrides:
        if override.state == "allow":
            allowed.add(override.permission)
        elif override.state == "deny":
            denied.add(override.permission)
    return allowed - denied


async def effective_permissions(
    db: AsyncSession, server_id: str, user_id: str, channel_id: str | None = None
) -> set[str]:
    """owner / ADMINISTRATOR 短路为全部；否则 基线 ∪ allow − deny，再叠频道覆盖。"""
    if await is_owner(db, server_id, user_id):
        return set(ALL_PERMISSIONS)
    if not await is_member(db, server_id, user_id):
        # 非成员：什么都没有，交给路由返 404，避免泄露服务器是否存在
        return set()

    role_ids = await role_ids_for(db, server_id, user_id)
    allowed, denied = await _server_grants(db, server_id, role_ids)
    result = allowed - denied
    if "ADMINISTRATOR" in result:
        return set(ALL_PERMISSIONS)
    if channel_id is None:
        return result

    overrides = await _channel_overrides(db, [channel_id], role_ids)
    return _apply(overrides.get(channel_id, []), allowed, denied)


async def visible_channel_ids(db: AsyncSession, server_id: str, user_id: str) -> set[str]:
    """一次算出服务器内所有可见频道；搜索、频道树、未读计数共用。"""
    if await is_owner(db, server_id, user_id):
        return set((await db.scalars(select(Channel.id).where(Channel.server_id == server_id))).all())
    if not await is_member(db, server_id, user_id):
        return set()

    role_ids = await role_ids_for(db, server_id, user_id)
    allowed, denied = await _server_grants(db, server_id, role_ids)
    if "ADMINISTRATOR" in (allowed - denied):
        return set((await db.scalars(select(Channel.id).where(Channel.server_id == server_id))).all())

    channel_ids = list((await db.scalars(select(Channel.id).where(Channel.server_id == server_id))).all())
    overrides = await _channel_overrides(db, channel_ids, role_ids)
    visible: set[str] = set()
    for channel_id in channel_ids:
        perms = _apply(overrides.get(channel_id, []), set(allowed), set(denied))
        if "VIEW_CHANNEL" in perms:
            visible.add(channel_id)
    return visible


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


def assert_can_act(actor_position: int, target_position: int, action: str = "操作") -> None:
    """层级校验：目标 position 不低于操作者时拒绝（owner 由 1_000_000 代表）。"""
    if target_position >= actor_position:
        raise ApiError(403, ErrorCode.FORBIDDEN, f"无法{action}角色不低于自己的成员", field="permission")
