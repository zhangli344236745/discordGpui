from httpx import AsyncClient

from tests.conftest import add_member, create_channel


async def _message(client: AsyncClient, headers: dict, channel_id: str, content: str) -> dict:
    resp = await client.post(f"/api/v1/channels/{channel_id}/messages", headers=headers, json={"content": content})
    assert resp.status_code == 201, resp.text
    return resp.json()


async def test_poll_single_multi_and_close(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    channel_id = world["channel"]["id"]
    await add_member(world["server"]["id"], world["bob_user"]["id"])
    bob = world["bob"]

    created = await client.post(
        f"/api/v1/channels/{channel_id}/polls",
        headers=alice,
        json={"question": "今晚做什么", "options": ["写代码", "看电影", "写代码"], "multi": False},
    )
    assert created.status_code == 201, created.text
    body = created.json()
    poll_id = body["poll"]["id"]
    assert [o["label"] for o in body["poll"]["options"]] == ["写代码", "看电影"]
    first_option, second_option = [o["id"] for o in body["poll"]["options"]]

    voted = await client.post(f"/api/v1/polls/{poll_id}/votes", headers=bob, json={"option_id": first_option})
    assert voted.status_code == 200
    assert voted.json()["total_votes"] == 1
    assert next(o for o in voted.json()["options"] if o["id"] == first_option)["me"] is True

    changed = await client.post(f"/api/v1/polls/{poll_id}/votes", headers=bob, json={"option_id": second_option})
    assert changed.json()["total_votes"] == 1
    assert next(o for o in changed.json()["options"] if o["id"] == first_option)["votes"] == 0

    closed = await client.post(f"/api/v1/polls/{poll_id}/close", headers=alice)
    assert closed.status_code == 200 and closed.json()["closed"] is True
    late_vote = await client.post(f"/api/v1/polls/{poll_id}/votes", headers=bob, json={"option_id": first_option})
    assert late_vote.status_code == 409

    multi = await client.post(
        f"/api/v1/channels/{channel_id}/polls",
        headers=alice,
        json={"question": "多选", "options": ["A", "B"], "multi": True},
    )
    multi_poll = multi.json()["poll"]
    option_a, option_b = [o["id"] for o in multi_poll["options"]]
    await client.post(f"/api/v1/polls/{multi_poll['id']}/votes", headers=alice, json={"option_id": option_a})
    both = await client.post(
        f"/api/v1/polls/{multi_poll['id']}/votes", headers=alice, json={"option_id": option_b}
    )
    assert both.json()["total_votes"] == 2
    toggled = await client.post(
        f"/api/v1/polls/{multi_poll['id']}/votes", headers=alice, json={"option_id": option_a}
    )
    assert toggled.json()["total_votes"] == 1

    cleared = await client.delete(f"/api/v1/polls/{multi_poll['id']}/votes", headers=alice)
    assert cleared.json()["total_votes"] == 0


async def test_thread_create_reply_and_duplicate(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    channel_id = world["channel"]["id"]
    await add_member(world["server"]["id"], world["bob_user"]["id"])
    message = await _message(client, alice, channel_id, "这条消息要开话题")

    created = await client.post(f"/api/v1/messages/{message['id']}/threads", headers=alice, json={"title": "讨论"})
    assert created.status_code == 201, created.text
    thread_id = created.json()["id"]

    duplicate = await client.post(f"/api/v1/messages/{message['id']}/threads", headers=alice, json={})
    assert duplicate.status_code == 409
    assert duplicate.json()["error"]["thread_id"] == thread_id

    reply = await client.post(f"/api/v1/threads/{thread_id}/replies", headers=bob, json={"content": "我来回复"})
    assert reply.status_code == 201
    assert reply.json()["author"]["id"] == world["bob_user"]["id"]

    detail = (await client.get(f"/api/v1/threads/{thread_id}", headers=alice)).json()
    assert detail["thread"]["reply_count"] == 1
    assert detail["original"]["id"] == message["id"]
    assert [r["content"] for r in detail["replies"]] == ["我来回复"]

    outsider_tokens = await client.post(
        "/api/v1/auth/register",
        json={"email": "outsider@example.com", "password": "password123", "username": "outsider_u"},
    )
    outsider = {"Authorization": f"Bearer {outsider_tokens.json()['access_token']}"}
    assert (await client.get(f"/api/v1/threads/{thread_id}", headers=outsider)).status_code == 404


async def test_bookmarks_channel_dm_and_tombstone(client: AsyncClient, world: dict) -> None:
    alice = world["alice"]
    channel_id = world["channel"]["id"]
    message = await _message(client, alice, channel_id, "值得收藏的频道消息")

    saved = await client.put(f"/api/v1/bookmarks/{message['id']}", headers=alice, json={"saved": True})
    assert saved.status_code == 200 and saved.json()["kind"] == "channel"

    dm_message = (
        await client.post(
            f"/api/v1/dms/{world['dm_id']}/messages",
            headers=alice,
            json={"content": "值得收藏的私信"},
        )
    ).json()
    saved_dm = await client.put(f"/api/v1/bookmarks/{dm_message['id']}", headers=alice, json={"saved": True})
    assert saved_dm.status_code == 200 and saved_dm.json()["kind"] == "dm"

    items = (await client.get("/api/v1/bookmarks", headers=alice)).json()
    assert {item["kind"] for item in items} == {"channel", "dm"}
    assert all(item["deleted"] is False for item in items)

    filtered = (await client.get("/api/v1/bookmarks?q=私信", headers=alice)).json()
    assert len(filtered) == 1 and filtered[0]["kind"] == "dm"

    # 重复保存幂等
    await client.put(f"/api/v1/bookmarks/{message['id']}", headers=alice, json={"saved": True})
    assert len((await client.get("/api/v1/bookmarks", headers=alice)).json()) == 2

    await client.delete(f"/api/v1/messages/{message['id']}", headers=alice)
    tombstone = next(
        item for item in (await client.get("/api/v1/bookmarks", headers=alice)).json() if item["kind"] == "channel"
    )
    assert tombstone["deleted"] is True and tombstone["content"] == ""

    removed = await client.delete(f"/api/v1/bookmarks/{message['id']}", headers=alice)
    assert removed.status_code == 200
    assert all(
        item["target_id"] != message["id"] for item in (await client.get("/api/v1/bookmarks", headers=alice)).json()
    )


async def test_forum_posts_sort_lock_and_tags(client: AsyncClient, world: dict) -> None:
    alice, bob = world["alice"], world["bob"]
    server_id = world["server"]["id"]
    await add_member(server_id, world["bob_user"]["id"])
    forum = await create_channel(client, alice, server_id, "讨论区", "forum")

    created = await client.post(
        f"/api/v1/channels/{forum['id']}/posts",
        headers=alice,
        json={"title": "第一篇", "body": "正文", "tags": ["公告", "公告", "求助"]},
    )
    assert created.status_code == 201, created.text
    post_id = created.json()["id"]
    assert created.json()["tags"] == ["公告", "求助"]

    second = await client.post(
        f"/api/v1/channels/{forum['id']}/posts",
        headers=alice,
        json={"title": "第二篇", "body": "另一个", "tags": ["讨论"]},
    )
    assert second.status_code == 201

    too_many = await client.post(
        f"/api/v1/channels/{forum['id']}/posts",
        headers=alice,
        json={"title": "标签太多", "body": "", "tags": ["1", "2", "3", "4", "5", "6"]},
    )
    assert too_many.status_code in (400, 422)

    pinned = await client.patch(f"/api/v1/posts/{second.json()['id']}", headers=alice, json={"pinned": True})
    assert pinned.status_code == 200
    recent = (await client.get(f"/api/v1/channels/{forum['id']}/posts?sort=recent", headers=bob)).json()
    assert recent["items"][0]["id"] == second.json()["id"]
    unanswered = (await client.get(f"/api/v1/channels/{forum['id']}/posts?sort=unanswered", headers=bob)).json()
    assert all(item["reply_count"] == 0 for item in unanswered["items"])

    reply = await client.post(f"/api/v1/posts/{post_id}/replies", headers=bob, json={"body": "回帖"})
    assert reply.status_code == 201
    assert reply.json()["reply_count"] == 1

    locked = await client.patch(f"/api/v1/posts/{post_id}", headers=alice, json={"locked": True})
    assert locked.json()["locked"] is True
    locked_reply = await client.post(f"/api/v1/posts/{post_id}/replies", headers=bob, json={"body": "再回"})
    assert locked_reply.status_code == 403

    detail = (await client.get(f"/api/v1/posts/{post_id}", headers=bob)).json()
    assert detail["post"]["reply_count"] == 1
    assert len(detail["replies"]) == 1

    tags = (await client.get(f"/api/v1/channels/{forum['id']}/tags", headers=bob)).json()
    assert {"tag": "公告", "count": 1} in tags

    deleted_reply = await client.delete(f"/api/v1/replies/{detail['replies'][0]['id']}", headers=alice)
    assert deleted_reply.status_code == 200
    assert (await client.get(f"/api/v1/posts/{post_id}", headers=alice)).json()["post"]["reply_count"] == 0

    # 文字频道不是论坛
    assert (
        await client.get(f"/api/v1/channels/{world['channel']['id']}/posts", headers=alice)
    ).status_code == 404

    # 非成员读写都被隐藏
    outsider = await client.post(
        "/api/v1/auth/register",
        json={"email": "forum_out@example.com", "password": "password123", "username": "forum_out"},
    )
    outsider_headers = {"Authorization": f"Bearer {outsider.json()['access_token']}"}
    assert (await client.get(f"/api/v1/channels/{forum['id']}/posts", headers=outsider_headers)).status_code == 404
