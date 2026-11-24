"""端到端冒烟：真实起服务后跑「注册 → 建服 → 加入 → 实时消息 → 语音 → B1 社交/私信/呼叫 → B2 邀请/权限/审计」。

用法：
    uv run uvicorn app.main:app --port 8099 &
    uv run python scripts/smoke.py http://127.0.0.1:8099
"""

import asyncio
import json
import sys
import time
from datetime import UTC, datetime, timedelta

import httpx
import websockets

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8099"
API = f"{BASE}/api/v1"
STAMP = str(int(time.time()))


async def register(client: httpx.AsyncClient, name: str) -> tuple[str, dict]:
    resp = await client.post(
        f"{API}/auth/register",
        json={
            "email": f"{name}{STAMP}@example.com",
            "password": "password123",
            "username": f"{name}{STAMP}"[:32],
        },
    )
    resp.raise_for_status()
    tokens = resp.json()
    headers = {"Authorization": f"Bearer {tokens['access_token']}"}
    me = (await client.get(f"{API}/users/@me", headers=headers)).json()
    return tokens["access_token"], {"headers": headers, "me": me}


async def main() -> None:
    async with httpx.AsyncClient(timeout=10) as client:
        health = (await client.get(f"{API}/health")).json()
        print(f"1. health            {health}")

        _, a = await register(client, "alice")
        _, b = await register(client, "bob")
        print(f"2. register          alice={a['me']['id']} bob={b['me']['id']}")

        server = (
            await client.post(
                f"{API}/servers",
                headers=a["headers"],
                json={"name": f"冒烟服{STAMP}", "template_key": "study", "is_public": True},
            )
        ).json()
        print(f"3. create server     {server['name']} members={server['member_count']}")

        tree = (await client.get(f"{API}/servers/{server['id']}/channels", headers=a["headers"])).json()
        channels = {c["name"]: c for c in tree["channels"]}
        text_id = next(c["id"] for c in tree["channels"] if c["kind"] == "text")
        voice_id = channels["专注自习"]["id"]
        print(f"4. channel tree      {len(tree['categories'])} 分类 / {len(tree['channels'])} 频道")

        join = (await client.post(f"{API}/discover/servers/{server['id']}/join", headers=b["headers"])).json()
        print(f"5. join (discover)   welcome={join['welcome']['headline']} 规则 {len(join['welcome']['rules'])} 条")

        # 实时：B 连 Gateway，A 发消息，B 应在 2 秒内收到
        ws_url = BASE.replace("http", "ws") + f"/ws/gateway?token={b['headers']['Authorization'].split()[1]}"
        async with websockets.connect(ws_url) as ws:
            ready = json.loads(await asyncio.wait_for(ws.recv(), timeout=5))
            print(f"6. gateway READY     servers={len(ready['data']['servers'])} dms={len(ready['data']['dms'])}")

            sent = await client.post(
                f"{API}/channels/{text_id}/messages",
                headers=a["headers"],
                json={"content": "实时冒烟消息", "client_nonce": f"n-{STAMP}"},
            )
            sent.raise_for_status()
            msg_id = sent.json()["id"]
            event = json.loads(await asyncio.wait_for(ws.recv(), timeout=5))
            assert event["event"] == "MESSAGE_CREATE", event
            print(f"7. MESSAGE_CREATE    {event['data']['author']['name']}: {event['data']['content']}")

            typing = await client.post(f"{API}/channels/{text_id}/typing", headers=a["headers"])
            assert typing.status_code == 202
            print("8. typing            ok (202)")

        voice = (await client.post(f"{API}/channels/{voice_id}/voice/join", headers=b["headers"])).json()
        print(f"9. voice join        room={voice['room']} token={voice['token'][:24]}…")
        print(f"   voice participants {[p['name'] for p in voice['participants']]}")

        dup = await client.post(
            f"{API}/servers",
            headers=a["headers"],
            json={"name": f"冒烟服{STAMP}"},
        )
        print(f"10. duplicate server {dup.status_code} {dup.json()['error']['code']}")

        # --- B1：好友 → 私信 → 消息请求 → 呼叫 ---
        card = await client.post(
            f"{API}/friends/requests",
            headers=b["headers"],
            json={"username": a["me"]["username"], "tag": a["me"]["tag"]},
        )
        card.raise_for_status()
        accepted = await client.post(
            f"{API}/friends/requests/{card.json()['id']}/accept", headers=a["headers"]
        )
        accepted.raise_for_status()
        friends = (await client.get(f"{API}/friends", headers=a["headers"])).json()
        assert any(f["id"] == b["me"]["id"] for f in friends), friends
        print(f"11. friends          request accepted, alice 好友 {len(friends)} 人")

        ws_url_b = BASE.replace("http", "ws") + f"/ws/gateway?token={b['headers']['Authorization'].split()[1]}"
        async with websockets.connect(ws_url_b) as ws_b:
            await asyncio.wait_for(ws_b.recv(), timeout=5)  # READY
            dm = (await client.post(f"{API}/dms", headers=a["headers"], json={"user_id": b["me"]["id"]})).json()
            dm_id = dm["dm"]["id"]
            msg = await client.post(
                f"{API}/dms/{dm_id}/messages",
                headers=a["headers"],
                json={"content": "私信冒烟", "client_nonce": f"dm-{STAMP}"},
            )
            msg.raise_for_status()
            for _ in range(5):
                event = json.loads(await asyncio.wait_for(ws_b.recv(), timeout=5))
                if event["event"] == "MESSAGE_CREATE" and event["data"].get("dm_id") == dm_id:
                    last_event_id = event["id"]
                    break
            else:
                raise AssertionError("bob 未收到私信 MESSAGE_CREATE")
        unread = (await client.get(f"{API}/dms", headers=b["headers"])).json()
        assert next(d for d in unread if d["id"] == dm_id)["unread"] == 1
        await client.patch(f"{API}/dms/{dm_id}/read", headers=b["headers"], json={})
        print(f"12. dm message       room={dm_id} ws→bob unread→0")

        # 断线补偿：重连带 since，应补回断线期间的消息
        replay_msg = await client.post(
            f"{API}/dms/{dm_id}/messages",
            headers=a["headers"],
            json={"content": "补发消息", "client_nonce": f"dm-replay-{STAMP}"},
        )
        replay_msg.raise_for_status()
        ws_since = ws_url_b + f"&since={last_event_id}"
        async with websockets.connect(ws_since) as ws_replay:
            for _ in range(10):
                event = json.loads(await asyncio.wait_for(ws_replay.recv(), timeout=5))
                if event["event"] == "MESSAGE_CREATE" and event["data"].get("content") == "补发消息":
                    break
            else:
                raise AssertionError("since 补偿未补回断线消息")
        print(f"12b. gateway since   replayed {event['event']} id={event['id']}")

        _, c = await register(client, "carol")
        request = (
            await client.post(
                f"{API}/dms",
                headers=c["headers"],
                json={"user_id": b["me"]["id"], "first_message": "陌生人请求"},
            )
        ).json()
        assert request["kind"] == "request", request
        accepted_mr = await client.post(
            f"{API}/message-requests/{request['request']['id']}/accept", headers=b["headers"]
        )
        accepted_mr.raise_for_status()
        print(f"13. message request  accepted → dm={accepted_mr.json()['dm']['id']}")

        call = (await client.post(f"{API}/dms/{dm_id}/call", headers=a["headers"], json={"kind": "voice"})).json()
        assert call["state"] == "ringing", call
        answered = await client.post(f"{API}/dms/{dm_id}/call/accept", headers=b["headers"])
        answered.raise_for_status()
        await client.post(f"{API}/dms/{dm_id}/call/hangup", headers=a["headers"])
        print(f"14. call             ring → active → ended room={call['room']}")

        # --- B2：邀请 → 改角色 → 权限拒绝 → 审核日志 ---
        _, d = await register(client, "dave")
        invite = (
            await client.post(
                f"{API}/channels/{text_id}/invites",
                headers=a["headers"],
                json={"expiry": "1h", "max_uses": 1},
            )
        ).json()
        joined = await client.post(f"{API}/invites/{invite['code']}/accept", headers=d["headers"])
        joined.raise_for_status()
        print(f"15. invite           code={invite['code']} members={joined.json()['server']['member_count']}")

        role = (
            await client.post(
                f"{API}/servers/{server['id']}/roles",
                headers=a["headers"],
                json={"name": "冒烟角色", "color": 0x00AAFF, "position": 1},
            )
        ).json()
        assigned = await client.put(
            f"{API}/servers/{server['id']}/members/{d['me']['id']}/roles/{role['id']}",
            headers=a["headers"],
            json={"assigned": True},
        )
        assigned.raise_for_status()
        assert role["id"] in assigned.json()["role_ids"]
        print(f"16. role assign      {role['name']} → dave")

        private = (
            await client.post(
                f"{API}/servers/{server['id']}/channels",
                headers=a["headers"],
                json={"name": f"私密{STAMP}", "kind": "text"},
            )
        ).json()
        roles = (await client.get(f"{API}/servers/{server['id']}/roles", headers=a["headers"])).json()
        everyone = next(r for r in roles if r["is_everyone"])
        override = await client.put(
            f"{API}/channels/{private['id']}/overrides",
            headers=a["headers"],
            json={"items": [{"role_id": everyone["id"], "permission": "VIEW_CHANNEL", "state": "deny"}]},
        )
        override.raise_for_status()
        denied = await client.get(f"{API}/channels/{private['id']}/messages", headers=b["headers"])
        assert denied.status_code == 404, denied.text
        allowed = await client.get(f"{API}/channels/{private['id']}/messages", headers=a["headers"])
        assert allowed.status_code == 200
        print(f"17. permissions      deny VIEW#{private['name']} → bob 404 / owner 200")

        audit = (await client.get(f"{API}/servers/{server['id']}/audit", headers=a["headers"])).json()
        assert audit["items"] and all(item["kind"] for item in audit["items"])
        print(f"18. audit log        {len(audit['items'])} 条，最新：{audit['items'][0]['detail']}")

        # --- B3：欢迎 → 日程 → 投票 → 书签 → 论坛 ---
        welcome = await client.post(
            f"{API}/servers/{server['id']}/welcome/accept",
            headers=d["headers"],
            json={"rule_agreed": True, "channels": [text_id]},
        )
        welcome.raise_for_status()
        print(f"19. welcome          accepted={welcome.json()['accepted']}")

        starts = (datetime.now(UTC) + timedelta(hours=2)).isoformat()
        event = (
            await client.post(
                f"{API}/servers/{server['id']}/events",
                headers=a["headers"],
                json={"title": "冒烟活动", "starts_at": starts, "duration_minutes": 30},
            )
        ).json()
        rsvp = await client.put(f"{API}/events/{event['id']}/rsvp", headers=d["headers"], json={"going": True})
        assert rsvp.json()["going_count"] == 1
        print(f"20. event            {event['status']} rsvp={rsvp.json()['going_count']}")

        poll = (
            await client.post(
                f"{API}/channels/{text_id}/polls",
                headers=a["headers"],
                json={"question": "冒烟投票", "options": ["A", "B"], "multi": False},
            )
        ).json()
        option = poll["poll"]["options"][0]["id"]
        voted = await client.post(
            f"{API}/polls/{poll['poll']['id']}/votes", headers=d["headers"], json={"option_id": option}
        )
        assert voted.json()["total_votes"] == 1
        print(f"21. poll             votes={voted.json()['total_votes']} options={len(voted.json()['options'])}")

        saved = await client.put(
            f"{API}/bookmarks/{msg_id}", headers=a["headers"], json={"saved": True}
        )
        saved.raise_for_status()
        bookmarks = (await client.get(f"{API}/bookmarks", headers=a["headers"])).json()
        assert any(item["target_id"] == msg_id for item in bookmarks)
        await client.delete(f"{API}/bookmarks/{msg_id}", headers=a["headers"])
        print(f"22. bookmarks        saved→listed→removed ({len(bookmarks)} 条)")

        forum = (
            await client.post(
                f"{API}/servers/{server['id']}/channels",
                headers=a["headers"],
                json={"name": f"讨论{STAMP}", "kind": "forum"},
            )
        ).json()
        post = (
            await client.post(
                f"{API}/channels/{forum['id']}/posts",
                headers=a["headers"],
                json={"title": "冒烟帖子", "body": "内容", "tags": ["公告"]},
            )
        ).json()
        reply = await client.post(
            f"{API}/posts/{post['id']}/replies", headers=d["headers"], json={"body": "回帖"}
        )
        reply.raise_for_status()
        posts = (await client.get(f"{API}/channels/{forum['id']}/posts?sort=popular", headers=d["headers"])).json()
        print(f"23. forum            post reply_count={posts['items'][0]['reply_count']} tags={post['tags']}")

        # --- B4：搜索 → 视图状态 → 草稿 → 通知 → 推送 ---
        search = (await client.get(f"{API}/search?q=实时冒烟", headers=a["headers"])).json()
        assert any(item["message_id"] == msg_id for item in search["items"]), search
        suggest = (await client.get(f"{API}/search/suggest?q=私密", headers=a["headers"])).json()
        print(f"24. search           hits={len(search['items'])} suggest={len(suggest)}")

        state = await client.put(
            f"{API}/me/channel-state/{text_id}", headers=a["headers"], json={"collapsed": True}
        )
        state.raise_for_status()
        draft_scope = f"channel:{text_id}"
        await client.put(
            f"{API}/me/drafts/{draft_scope}", headers=a["headers"], json={"content": "未发送草稿"}
        )
        drafts = (await client.get(f"{API}/me/drafts", headers=a["headers"])).json()
        assert any(item["scope"] == draft_scope for item in drafts)
        await client.put(f"{API}/me/drafts/{draft_scope}", headers=a["headers"], json={"content": ""})
        print(f"25. view state       collapsed={state.json()['collapsed']} drafts→{len(drafts)}")

        notification_list = (await client.get(f"{API}/notifications", headers=b["headers"])).json()
        counts = (await client.get(f"{API}/notifications/unread-counts", headers=b["headers"])).json()
        push = await client.post(
            f"{API}/push/subscribe",
            headers=a["headers"],
            json={"endpoint": f"https://push.example.com/{STAMP}"},
        )
        push.raise_for_status()
        print(
            f"26. notifications    items={len(notification_list['items'])} "
            f"mentions={counts['mentions']} push={push.json()['delivery']}"
        )

    print("\n冒烟通过。")


if __name__ == "__main__":
    asyncio.run(main())
