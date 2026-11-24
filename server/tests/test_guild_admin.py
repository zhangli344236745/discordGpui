from httpx import AsyncClient
from sqlalchemy import select

from app.db import SessionLocal
from app.models import MemberRole, Role, ServerMember
from tests.conftest import create_server


async def _add_member(world: dict, user_id: str) -> None:
    server_id = world["server"]["id"]
    async with SessionLocal() as db:
        if await db.get(ServerMember, (server_id, user_id)) is None:
            db.add(ServerMember(server_id=server_id, user_id=user_id, nickname=""))
        everyone = await db.scalar(select(Role).where(Role.server_id == server_id, Role.is_everyone.is_(True)))
        if everyone is not None:
            existing = await db.get(MemberRole, (server_id, user_id, everyone.id))
            if existing is None:
                db.add(MemberRole(server_id=server_id, user_id=user_id, role_id=everyone.id))
        await db.commit()


async def test_members_nickname_and_kick(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    bob_id = world["bob_user"]["id"]
    await _add_member(world, bob_id)

    members = (await client.get(f"/api/v1/servers/{server_id}/members", headers=alice)).json()
    assert {m["user_id"] for m in members} == {world["alice_user"]["id"], bob_id}
    assert next(m for m in members if m["user_id"] == world["alice_user"]["id"])["is_owner"] is True

    # 本人可改自己的昵称，改他人需要 MANAGE_SERVER
    own = await client.patch(
        f"/api/v1/servers/{server_id}/members/{bob_id}", headers=bob, json={"nickname": "小鲍"}
    )
    assert own.status_code == 200 and own.json()["nickname"] == "小鲍"
    forbidden = await client.patch(
        f"/api/v1/servers/{server_id}/members/{world['alice_user']['id']}", headers=bob, json={"nickname": "x"}
    )
    assert forbidden.status_code == 403

    # 清空昵称回退显示名
    cleared = await client.patch(
        f"/api/v1/servers/{server_id}/members/{bob_id}", headers=alice, json={"nickname": ""}
    )
    assert cleared.json()["nickname"] == ""

    kicked = await client.delete(f"/api/v1/servers/{server_id}/members/{bob_id}", headers=alice)
    assert kicked.status_code == 200
    assert (await client.get(f"/api/v1/servers/{server_id}/members", headers=alice)).json().__len__() == 1

    # 目标已不是成员
    again = await client.delete(f"/api/v1/servers/{server_id}/members/{bob_id}", headers=alice)
    assert again.status_code == 404


async def test_roles_hierarchy_and_matrix(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    bob_id = world["bob_user"]["id"]
    await _add_member(world, bob_id)

    created = await client.post(
        f"/api/v1/servers/{server_id}/roles", headers=alice, json={"name": "版主", "color": 0x00FF00}
    )
    assert created.status_code == 201, created.text
    role_id = created.json()["id"]

    denied = await client.post(f"/api/v1/servers/{server_id}/roles", headers=bob, json={"name": "越权"})
    assert denied.status_code == 403

    assigned = await client.put(
        f"/api/v1/servers/{server_id}/members/{bob_id}/roles/{role_id}", headers=alice, json={"assigned": True}
    )
    assert assigned.status_code == 200
    assert role_id in assigned.json()["role_ids"]

    matrix = await client.put(
        f"/api/v1/servers/{server_id}/permissions",
        headers=alice,
        json={"items": [{"role_id": role_id, "permission": "MANAGE_ROLES", "state": "allow"}]},
    )
    assert matrix.status_code == 200
    read = (await client.get(f"/api/v1/servers/{server_id}/permissions", headers=alice)).json()
    assert {"role_id": role_id, "permission": "MANAGE_ROLES", "state": "allow"} in read["items"]

    bad_permission = await client.put(
        f"/api/v1/servers/{server_id}/permissions",
        headers=alice,
        json={"items": [{"role_id": role_id, "permission": "FLY", "state": "allow"}]},
    )
    assert bad_permission.status_code == 400

    # 低角色不能改高角色
    high = await client.post(
        f"/api/v1/servers/{server_id}/roles", headers=alice, json={"name": "高层", "position": 50}
    )
    high_id = high.json()["id"]
    patch_high = await client.patch(f"/api/v1/roles/{high_id}", headers=bob, json={"name": "改名"})
    assert patch_high.status_code == 403

    everyone = next(
        r for r in (await client.get(f"/api/v1/servers/{server_id}/roles", headers=alice)).json()
        if r["is_everyone"]
    )
    delete_everyone = await client.delete(f"/api/v1/roles/{everyone['id']}", headers=alice)
    assert delete_everyone.status_code == 409

    deleted = await client.delete(f"/api/v1/roles/{role_id}", headers=alice)
    assert deleted.status_code == 200
    current_roles = (await client.get(f"/api/v1/servers/{server_id}/roles", headers=alice)).json()
    assert all(r["id"] != role_id for r in current_roles)


async def test_channel_override_hides_channel(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    channel_id = world["channel"]["id"]
    await _add_member(world, world["bob_user"]["id"])

    async with SessionLocal() as db:
        everyone = await db.scalar(select(Role).where(Role.server_id == server_id, Role.is_everyone.is_(True)))
        assert everyone is not None
        everyone_id = everyone.id

    written = await client.put(
        f"/api/v1/channels/{channel_id}/overrides",
        headers=alice,
        json={"items": [{"role_id": everyone_id, "permission": "VIEW_CHANNEL", "state": "deny"}]},
    )
    assert written.status_code == 200, written.text

    assert (await client.get(f"/api/v1/channels/{channel_id}/messages", headers=bob)).status_code == 404
    assert (await client.get(f"/api/v1/channels/{channel_id}/messages", headers=alice)).status_code == 200

    overrides = (await client.get(f"/api/v1/channels/{channel_id}/overrides", headers=alice)).json()
    assert {"role_id": everyone_id, "permission": "VIEW_CHANNEL", "state": "deny"} in overrides["items"]

    # 继承覆盖后恢复
    restored = await client.put(
        f"/api/v1/channels/{channel_id}/overrides",
        headers=alice,
        json={"items": [{"role_id": everyone_id, "permission": "VIEW_CHANNEL", "state": "inherit"}]},
    )
    assert restored.status_code == 200
    assert (await client.get(f"/api/v1/channels/{channel_id}/messages", headers=bob)).status_code == 200


async def test_ban_blocks_invite_and_unban(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    bob_id = world["bob_user"]["id"]
    await _add_member(world, bob_id)

    banned = await client.post(
        f"/api/v1/servers/{server_id}/bans",
        headers=alice,
        json={"user_id": bob_id, "reason": "spam", "delete_days": 0},
    )
    assert banned.status_code == 201, banned.text
    assert banned.json()["user_id"] == bob_id

    listed = (await client.get(f"/api/v1/servers/{server_id}/bans", headers=alice)).json()
    assert [b["user_id"] for b in listed] == [bob_id]
    assert (await client.get(f"/api/v1/servers/{server_id}/members", headers=alice)).json().__len__() == 1

    # 封禁后不能通过邀请加入
    invite = await client.post(f"/api/v1/channels/{world['channel']['id']}/invites", headers=alice, json={})
    accept = await client.post(f"/api/v1/invites/{invite.json()['code']}/accept", headers=bob)
    assert accept.status_code == 403

    unbanned = await client.delete(f"/api/v1/servers/{server_id}/bans/{bob_id}", headers=alice)
    assert unbanned.status_code == 200
    rejoin = await client.post(f"/api/v1/invites/{invite.json()['code']}/accept", headers=bob)
    assert rejoin.status_code == 200

    no_perm = await client.get(f"/api/v1/servers/{server_id}/bans", headers=bob)
    assert no_perm.status_code == 403


async def test_owner_cannot_be_banned_or_kicked(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    bob = world["bob"]
    server_id = world["server"]["id"]
    bob_id = world["bob_user"]["id"]
    await _add_member(world, bob_id)

    # bob 无 KICK/BAN 权限
    kick = await client.delete(
        f"/api/v1/servers/{server_id}/members/{world['alice_user']['id']}", headers=bob
    )
    assert kick.status_code == 403

    # owner 不能封自己（先给 bob 一个高角色也没用）
    ban = await client.post(
        f"/api/v1/servers/{server_id}/bans",
        headers=alice,
        json={"user_id": world["alice_user"]["id"], "reason": "x", "delete_days": 0},
    )
    assert ban.status_code == 403


async def test_second_server_is_isolated(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    other = await create_server(client, alice, "另一个世界")
    assert other["id"] != world["server"]["id"]
    members = (await client.get(f"/api/v1/servers/{other['id']}/members", headers=alice)).json()
    assert len(members) == 1
