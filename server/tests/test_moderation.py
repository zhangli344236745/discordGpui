from httpx import AsyncClient
from sqlalchemy import select

from app.db import SessionLocal
from app.models import MemberRole, Role, ServerMember


async def _add_member(world: dict, user_id: str) -> None:
    server_id = world["server"]["id"]
    async with SessionLocal() as db:
        if await db.get(ServerMember, (server_id, user_id)) is None:
            db.add(ServerMember(server_id=server_id, user_id=user_id, nickname=""))
        everyone = await db.scalar(select(Role).where(Role.server_id == server_id, Role.is_everyone.is_(True)))
        if everyone is not None and await db.get(MemberRole, (server_id, user_id, everyone.id)) is None:
            db.add(MemberRole(server_id=server_id, user_id=user_id, role_id=everyone.id))
        await db.commit()


async def test_automod_rules_block_warn_and_test(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    server_id = world["server"]["id"]
    channel_id = world["channel"]["id"]

    block_rule = await client.post(
        f"/api/v1/servers/{server_id}/automod/rules",
        headers=alice,
        json={"name": "拦截坏词", "action": "block", "keywords": ["坏词", "违禁", "坏词"]},
    )
    assert block_rule.status_code == 201, block_rule.text
    assert block_rule.json()["keywords"] == ["坏词", "违禁"]
    block_id = block_rule.json()["id"]

    warn_rule = await client.post(
        f"/api/v1/servers/{server_id}/automod/rules",
        headers=alice,
        json={"name": "广告警告", "action": "warn", "keywords": ["广告"]},
    )
    assert warn_rule.status_code == 201
    warn_id = warn_rule.json()["id"]

    blocked = await client.post(
        f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "这里有坏词"}
    )
    assert blocked.status_code == 422
    error = blocked.json()["error"]
    assert error["code"] == "AUTOMOD_BLOCKED"
    assert error["rule_id"] == block_id and error["matched_keyword"] == "坏词"

    warned = await client.post(
        f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "这是广告"}
    )
    assert warned.status_code == 201, warned.text
    assert warned.json()["warnings"][0]["keyword"] == "广告"

    history = (await client.get(f"/api/v1/channels/{channel_id}/messages", headers=alice)).json()
    assert all("坏词" not in item["content"] for item in history["items"])

    # 编辑命中：原正文与版本不变
    benign = await client.post(
        f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "正常消息"}
    )
    message_id = benign.json()["id"]
    edited = await client.patch(
        f"/api/v1/messages/{message_id}", headers=alice, json={"content": "改成坏词"}
    )
    assert edited.status_code == 422
    current = (await client.get(f"/api/v1/messages/{message_id}", headers=alice)).json()
    assert current["content"] == "正常消息"

    before = len((await client.get(f"/api/v1/channels/{channel_id}/messages", headers=alice)).json()["items"])
    tested = await client.post(
        f"/api/v1/servers/{server_id}/automod/test", headers=alice, json={"content": "坏词和广告"}
    )
    assert tested.status_code == 200
    assert tested.json()["blocked"] is True
    assert {m["keyword"] for m in tested.json()["matches"]} == {"坏词", "广告"}
    after = len((await client.get(f"/api/v1/channels/{channel_id}/messages", headers=alice)).json()["items"])
    assert before == after

    disabled = await client.patch(f"/api/v1/automod/rules/{block_id}", headers=alice, json={"enabled": False})
    assert disabled.status_code == 200 and disabled.json()["enabled"] is False
    assert (
        await client.post(f"/api/v1/channels/{channel_id}/messages", headers=alice, json={"content": "坏词又来了"})
    ).status_code == 201

    empty = await client.patch(f"/api/v1/automod/rules/{warn_id}", headers=alice, json={"keywords": ["  "]})
    assert empty.status_code == 400

    rules = (await client.get(f"/api/v1/servers/{server_id}/automod/rules", headers=alice)).json()
    assert {r["id"] for r in rules} == {block_id, warn_id}

    deleted = await client.delete(f"/api/v1/automod/rules/{warn_id}", headers=alice)
    assert deleted.status_code == 200
    assert warn_id not in {
        r["id"] for r in (await client.get(f"/api/v1/servers/{server_id}/automod/rules", headers=alice)).json()
    }


async def test_automod_permissions_and_audit_log(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    bob_id = world["bob_user"]["id"]
    await _add_member(world, bob_id)

    denied_rule = await client.post(
        f"/api/v1/servers/{server_id}/automod/rules",
        headers=bob,
        json={"name": "越权", "action": "block", "keywords": ["x"]},
    )
    assert denied_rule.status_code == 403

    await client.post(
        f"/api/v1/servers/{server_id}/automod/rules",
        headers=alice,
        json={"name": "规则", "action": "warn", "keywords": ["提示"]},
    )

    logs = (await client.get(f"/api/v1/servers/{server_id}/audit", headers=alice)).json()
    assert logs["items"] and all("created_at" in item for item in logs["items"])

    filtered = (await client.get(f"/api/v1/servers/{server_id}/audit?kind=server", headers=alice)).json()
    assert all(item["kind"] == "server" for item in filtered["items"])

    assert (await client.get(f"/api/v1/servers/{server_id}/audit", headers=bob)).status_code == 403
    assert (await client.delete(f"/api/v1/servers/{server_id}/audit", headers=bob)).status_code == 403

    cleared = await client.delete(f"/api/v1/servers/{server_id}/audit", headers=alice)
    assert cleared.status_code == 200
    assert (await client.get(f"/api/v1/servers/{server_id}/audit", headers=alice)).json()["items"] == []
