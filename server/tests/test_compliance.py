"""P5 合规：举报状态机、保留策略、法务持有、数据导出与账号删除。"""

from datetime import timedelta

from httpx import AsyncClient
from sqlalchemy import select

from app.db import SessionLocal, utcnow
from app.models import Message
from app.services import retention
from tests.conftest import auth_headers, create_server, first_channel, me, register


async def _send_message(client: AsyncClient, headers: dict, channel: dict, text: str = "垃圾消息") -> dict:
    resp = await client.post(
        f"/api/v1/channels/{channel['id']}/messages",
        headers=headers,
        json={"content": text, "client_nonce": f"rep-{text}-{channel['id']}"},
    )
    assert resp.status_code == 201, resp.text
    return resp.json()


async def test_report_dedupe_and_actions(client: AsyncClient) -> None:
    from tests.conftest import add_member, make_dm  # noqa: F401
    from tests.conftest import me as fetch_me

    owner = await auth_headers(client)
    server = await create_server(client, owner)
    channel = await first_channel(client, owner, server["id"])
    message = await _send_message(client, owner, channel, "违规内容")

    # 第二位成员举报：确保处理者与举报者通知都能落库
    reporter_tokens = await register(client, "reporter@example.com")
    reporter = {"Authorization": f"Bearer {reporter_tokens['access_token']}"}
    reporter_user = await fetch_me(client, reporter)
    await add_member(server["id"], reporter_user["id"])

    first = await client.post(
        "/api/v1/reports",
        headers=reporter,
        json={
            "target_kind": "message",
            "target_id": message["id"],
            "reason": "垃圾信息",
            "detail": "广告刷屏",
        },
    )
    assert first.status_code == 201, first.text
    assert first.json()["merged"] is False

    # 同一用户重复举报合并计数
    second = await client.post(
        "/api/v1/reports",
        headers=reporter,
        json={
            "target_kind": "message",
            "target_id": message["id"],
            "reason": "垃圾信息",
        },
    )
    assert second.json()["merged"] is True
    assert second.json()["count"] == 2

    listed = await client.get(f"/api/v1/servers/{server['id']}/reports", headers=owner)
    assert listed.status_code == 200
    assert listed.json()[0]["count"] == 2
    assert listed.json()[0]["status"] == "pending"

    # 处理者（owner）在举报后收到通知
    owner_inbox = await client.get("/api/v1/notifications", headers=owner)
    assert any("举报" in (item.get("title", "") + item.get("body", "")) for item in owner_inbox.json()["items"])

    # 处理：删除违规内容 → 举报关闭
    action = await client.post(
        f"/api/v1/reports/{first.json()['id']}/actions",
        headers=owner,
        json={"action": "resolve", "note": "已删除", "delete_content": True},
    )
    assert action.status_code == 201, action.text
    assert action.json()["status"] == "resolved"
    assert action.json()["deleted_content"] is True

    history = await client.get("/api/v1/me/reports", headers=reporter)
    assert history.json()[0]["status"] == "resolved"

    # 举报者收到处理结果
    reporter_inbox = await client.get("/api/v1/notifications", headers=reporter)
    assert any(
        "处理" in (item.get("title", "") + item.get("body", ""))
        for item in reporter_inbox.json()["items"]
    )


async def test_report_reject_keeps_content(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"])
    message = await _send_message(client, headers, channel, "边缘内容")

    report = await client.post(
        "/api/v1/reports",
        headers=headers,
        json={"target_kind": "message", "target_id": message["id"], "reason": "疑似违规"},
    )
    assert report.status_code == 201
    action = await client.post(
        f"/api/v1/reports/{report.json()['id']}/actions",
        headers=headers,
        json={"action": "reject", "note": "不构成违规"},
    )
    assert action.json()["status"] == "rejected"
    assert action.json()["deleted_content"] is False

    still = await client.get(f"/api/v1/messages/{message['id']}", headers=headers)
    assert still.status_code == 200


async def test_retention_rules_and_legal_hold(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"])
    kept = await _send_message(client, headers, channel, "被法务保留的消息")

    # 保留 30 天
    saved = await client.put(
        f"/api/v1/servers/{server['id']}/retention",
        headers=headers,
        json={"message_days": 30},
    )
    assert saved.status_code == 200 and saved.json()["message_days"] == 30

    got = await client.get(f"/api/v1/servers/{server['id']}/retention", headers=headers)
    assert got.json()["message_days"] == 30

    hold = await client.post(
        f"/api/v1/servers/{server['id']}/legal-holds",
        headers=headers,
        json={"target_kind": "message", "target_id": kept["id"], "note": "诉讼证据"},
    )
    assert hold.status_code == 201

    # 把两条消息时间拨回 40 天前
    async with SessionLocal() as db:
        rows = (await db.scalars(select(Message).where(Message.channel_id == channel["id"]))).all()
        old = utcnow() - timedelta(days=40)
        for row in rows:
            row.created_at = old
        await db.commit()

    counts = await retention.purge_once(utcnow())
    assert counts["retention_messages"] == len(rows) - 1  # 法务持有的那条跳过

    async with SessionLocal() as db:
        survived = await db.get(Message, kept["id"])
        assert survived is not None and survived.deleted_at is None
        gone = [row for row in rows if row.id != kept["id"]]
        deleted = [await db.get(Message, row.id) for row in gone]
        assert all(item is not None and item.deleted_at is not None for item in deleted)

    # 解除法务持有后到期清理生效
    removed = await client.delete(
        f"/api/v1/servers/{server['id']}/legal-holds/{hold.json()['id']}", headers=headers
    )
    assert removed.status_code == 204
    counts = await retention.purge_once(utcnow())
    assert counts["retention_messages"] == 1


async def test_data_export_and_account_delete(client: AsyncClient) -> None:
    headers = await auth_headers(client, "gone@example.com")
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"])
    await _send_message(client, headers, channel, "我的最后留言")

    export = await client.get("/api/v1/me/data-export", headers=headers)
    assert export.status_code == 200, export.text
    body = export.json()
    assert body["profile"]["email"] == "gone@example.com"
    assert any("我的最后留言" == m["content"] for m in body["messages"])
    assert any(s["id"] == server["id"] for s in body["servers"])

    deleted = await client.post(
        "/api/v1/me/account/delete", headers=headers, json={"password": "password123"}
    )
    assert deleted.status_code == 204

    # 旧 token 仍可用（撤销的是 refresh），但资料已匿名化
    profile = await me(client, headers)
    assert profile["display_name"] == "已注销用户"

    # 密码失效，无法重新登录
    login = await client.post(
        "/api/v1/auth/login", json={"email": "gone@example.com", "password": "password123"}
    )
    assert login.status_code == 401

    # 消息软删，不再出现在导出里
    export_after = await client.get("/api/v1/me/data-export", headers=headers)
    assert export_after.json()["messages"] == []
