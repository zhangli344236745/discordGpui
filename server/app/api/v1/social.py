"""好友、好友分组与屏蔽。"""

from datetime import datetime

from fastapi import APIRouter, Depends, status
from sqlalchemy import delete, func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import current_user
from app.errors import bad_request, conflict, not_found
from app.models import Block, FriendGroup, FriendRequest, Friendship, User
from app.realtime import outbox
from app.realtime.events import Event
from app.schemas.social import BlockIn, FriendGroupIn, FriendGroupOrderIn, FriendMoveIn, FriendRequestIn
from app.services import notifications, social

router = APIRouter(tags=["social"])


def _iso(value: datetime) -> str:
    return value.isoformat().replace("+00:00", "Z")


def _request_out(row: FriendRequest, from_user: User, to_user: User, direction: str) -> dict:
    return {
        "id": row.id,
        "from_user": social.public_user(from_user),
        "to_user": social.public_user(to_user),
        "state": row.state,
        "message": row.message,
        "direction": direction,
        "created_at": _iso(row.created_at),
    }


async def _friends(db: AsyncSession, user_id: str, online_only: bool = False) -> list[dict]:
    rows = (
        await db.execute(
            select(Friendship, User)
            .join(User, User.id == Friendship.friend_id)
            .where(Friendship.user_id == user_id)
            .order_by(User.display_name)
        )
    ).all()
    hidden = await social.blocked_ids(db, user_id)
    items = [social.friend_out(user, friendship.group_id, friendship.created_at) for friendship, user in rows]
    items = [item for item in items if item["id"] not in hidden]
    if online_only:
        items = [item for item in items if item["online"]]
    return items


@router.get("/friends")
async def list_friends(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    return await _friends(db, user.id)


@router.get("/friends/online")
async def list_online_friends(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    return await _friends(db, user.id, online_only=True)


@router.get("/friends/requests")
async def list_friend_requests(
    db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> dict:
    incoming_rows = (
        await db.execute(
            select(FriendRequest, User)
            .join(User, User.id == FriendRequest.from_user)
            .where(FriendRequest.to_user == user.id, FriendRequest.state == "pending")
            .order_by(FriendRequest.created_at.desc())
        )
    ).all()
    outgoing_rows = (
        await db.execute(
            select(FriendRequest, User)
            .join(User, User.id == FriendRequest.to_user)
            .where(FriendRequest.from_user == user.id, FriendRequest.state == "pending")
            .order_by(FriendRequest.created_at.desc())
        )
    ).all()
    items = [_request_out(row, other, user, "incoming") for row, other in incoming_rows]
    outgoing = [_request_out(row, user, other, "outgoing") for row, other in outgoing_rows]
    return {"items": items, "count": len(items), "outgoing": outgoing}


@router.post("/friends/requests", status_code=status.HTTP_201_CREATED)
async def send_friend_request(
    body: FriendRequestIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    target = await social.find_user(db, body.username, body.tag)
    if target is None or target.id == user.id or await social.blocked(db, user.id, target.id if target else ""):
        # 不区分「不存在」与「被屏蔽」，避免枚举
        raise not_found("用户")
    if await social.are_friends(db, user.id, target.id):
        raise conflict("已经是好友", "username")
    if await db.scalar(
        select(FriendRequest.id).where(
            FriendRequest.from_user == user.id,
            FriendRequest.to_user == target.id,
            FriendRequest.state == "pending",
        )
    ):
        raise conflict("好友请求已发送，等待对方处理", "username")
    if await db.scalar(
        select(FriendRequest.id).where(
            FriendRequest.from_user == target.id,
            FriendRequest.to_user == user.id,
            FriendRequest.state == "pending",
        )
    ):
        raise conflict("对方已向你发送好友请求，请到待处理列表接受", "username")

    row = FriendRequest(
        from_user=user.id,
        to_user=target.id,
        state="pending",
        message=body.message.strip(),
    )
    db.add(row)
    await db.flush()
    data = _request_out(row, user, target, "incoming")
    notifications.create(
        db,
        target.id,
        "friend_request",
        "friend_request",
        row.id,
        title=user.display_name,
        body=body.message.strip()[:190],
        actor_id=user.id,
    )
    outbox.queue(db, "user", target.id, Event.FRIEND_REQUEST_CREATE, data)
    await db.commit()
    return data


async def _load_request_for_recipient(db: AsyncSession, request_id: str, user_id: str) -> FriendRequest:
    row = await db.get(FriendRequest, request_id)
    if row is None or row.to_user != user_id:
        raise not_found("好友请求")
    if row.state != "pending":
        raise conflict("该请求已处理", "state")
    return row


async def _request_payload(db: AsyncSession, row: FriendRequest) -> dict:
    from_user = await db.get(User, row.from_user)
    to_user = await db.get(User, row.to_user)
    assert from_user is not None and to_user is not None
    return _request_out(row, from_user, to_user, "incoming")


@router.post("/friends/requests/{request_id}/accept")
async def accept_friend_request(
    request_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    row = await _load_request_for_recipient(db, request_id, user.id)
    row.state = "accepted"
    for a, b in ((row.from_user, row.to_user), (row.to_user, row.from_user)):
        if await db.scalar(select(Friendship).where(Friendship.user_id == a, Friendship.friend_id == b)) is None:
            db.add(Friendship(user_id=a, friend_id=b, group_id=None))
    payload = await _request_payload(db, row)
    outbox.queue(db, "user", row.from_user, Event.FRIEND_REQUEST_ACCEPT, payload)
    outbox.queue(db, "user", row.to_user, Event.FRIEND_REQUEST_ACCEPT, payload)
    await db.commit()
    return payload


@router.post("/friends/requests/{request_id}/reject")
async def reject_friend_request(
    request_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    row = await _load_request_for_recipient(db, request_id, user.id)
    row.state = "rejected"
    payload = await _request_payload(db, row)
    outbox.queue(db, "user", row.from_user, Event.FRIEND_REQUEST_REJECT, payload)
    await db.commit()
    return payload


@router.delete("/friends/{user_id}")
async def delete_friend(
    user_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    if not await social.are_friends(db, user.id, user_id):
        raise not_found("好友")
    await db.execute(
        delete(Friendship).where(
            ((Friendship.user_id == user.id) & (Friendship.friend_id == user_id))
            | ((Friendship.user_id == user_id) & (Friendship.friend_id == user.id))
        )
    )
    payload = {"user_id": user.id, "friend_id": user_id}
    outbox.queue(db, "user", user.id, Event.FRIEND_REQUEST_DELETE, payload)
    outbox.queue(db, "user", user_id, Event.FRIEND_REQUEST_DELETE, payload)
    await db.commit()
    return {"removed": True}


# ---------------------------------------------------------------------------
# 好友分组
# ---------------------------------------------------------------------------


@router.get("/friend-groups")
async def list_friend_groups(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    groups = list(
        (
            await db.scalars(
                select(FriendGroup).where(FriendGroup.owner_id == user.id).order_by(FriendGroup.position)
            )
        ).all()
    )
    counts = dict(
        (
            await db.execute(
                select(Friendship.group_id, func.count())
                .where(Friendship.user_id == user.id, Friendship.group_id.is_not(None))
                .group_by(Friendship.group_id)
            )
        ).all()
    )
    return [
        {"id": g.id, "name": g.name, "position": g.position, "count": int(counts.get(g.id, 0))} for g in groups
    ]


@router.post("/friend-groups", status_code=status.HTTP_201_CREATED)
async def create_friend_group(
    body: FriendGroupIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    name = body.name.strip()
    if await db.scalar(
        select(FriendGroup.id).where(FriendGroup.owner_id == user.id, func.lower(FriendGroup.name) == name.lower())
    ):
        raise conflict("已存在同名分组", "name")
    count = await db.scalar(select(func.count()).select_from(FriendGroup).where(FriendGroup.owner_id == user.id))
    group = FriendGroup(owner_id=user.id, name=name, position=int(count or 0))
    db.add(group)
    await db.flush()
    payload = {"id": group.id, "name": group.name, "position": group.position, "count": 0}
    outbox.queue(db, "user", user.id, Event.FRIEND_GROUP_CREATE, payload)
    await db.commit()
    return payload


@router.patch("/friend-groups/order")
async def reorder_friend_groups(
    body: FriendGroupOrderIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> list[dict]:
    groups = list(
        (await db.scalars(select(FriendGroup).where(FriendGroup.owner_id == user.id))).all()
    )
    if len(body.ids) != len(groups) or set(body.ids) != {g.id for g in groups}:
        raise bad_request("排序必须包含全部且不重复的分组", "ids")
    by_id = {g.id: g for g in groups}
    for position, group_id in enumerate(body.ids):
        by_id[group_id].position = position
    outbox.queue(db, "user", user.id, Event.FRIEND_GROUP_UPDATE, {"order": body.ids})
    await db.commit()
    return await list_friend_groups(db, user)


@router.patch("/friend-groups/{group_id}")
async def rename_friend_group(
    group_id: str,
    body: FriendGroupIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    group = await db.get(FriendGroup, group_id)
    if group is None or group.owner_id != user.id:
        raise not_found("好友分组")
    name = body.name.strip()
    if await db.scalar(
        select(FriendGroup.id).where(
            FriendGroup.owner_id == user.id,
            func.lower(FriendGroup.name) == name.lower(),
            FriendGroup.id != group.id,
        )
    ):
        raise conflict("已存在同名分组", "name")
    group.name = name
    payload = {"id": group.id, "name": group.name, "position": group.position}
    outbox.queue(db, "user", user.id, Event.FRIEND_GROUP_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/friend-groups/{group_id}")
async def delete_friend_group(
    group_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    group = await db.get(FriendGroup, group_id)
    if group is None or group.owner_id != user.id:
        raise not_found("好友分组")
    await db.execute(
        Friendship.__table__.update()
        .where(Friendship.user_id == user.id, Friendship.group_id == group_id)
        .values(group_id=None)
    )
    await db.delete(group)
    outbox.queue(db, "user", user.id, Event.FRIEND_GROUP_DELETE, {"id": group_id})
    await db.commit()
    return {"deleted": True}


@router.patch("/friends/{user_id}")
async def move_friend(
    user_id: str,
    body: FriendMoveIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    friendship = await db.get(Friendship, (user.id, user_id))
    if friendship is None:
        raise not_found("好友")
    if body.group_id is not None:
        group = await db.get(FriendGroup, body.group_id)
        if group is None or group.owner_id != user.id:
            raise not_found("好友分组")
    friendship.group_id = body.group_id
    payload = {"user_id": user_id, "group_id": body.group_id}
    outbox.queue(db, "user", user.id, Event.FRIEND_GROUP_UPDATE, payload)
    await db.commit()
    return payload


# ---------------------------------------------------------------------------
# 屏蔽
# ---------------------------------------------------------------------------


@router.get("/blocks")
async def list_blocks(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[dict]:
    rows = (
        await db.execute(
            select(Block, User).join(User, User.id == Block.target_id).where(Block.owner_id == user.id)
        )
    ).all()
    return [
        {
            "user_id": block.target_id,
            "username": target.username,
            "tag": target.tag,
            "display_name": target.display_name,
            "avatar_color": target.avatar_color,
            "reason": block.reason,
            "created_at": _iso(block.created_at),
        }
        for block, target in rows
    ]


@router.post("/blocks", status_code=status.HTTP_201_CREATED)
async def create_block(
    body: BlockIn,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    if body.user_id == user.id:
        raise bad_request("不能屏蔽自己", "user_id")
    target = await social.get_user(db, body.user_id)
    existing = await social.block_row(db, user.id, target.id)
    if existing is not None:
        return {
            "user_id": target.id,
            "blocked": True,
            "created_at": _iso(existing.created_at),
        }
    block = Block(owner_id=user.id, target_id=target.id, reason=body.reason.strip())
    db.add(block)
    await db.flush()
    payload = {"user_id": target.id, "blocked": True, "created_at": _iso(block.created_at)}
    outbox.queue(db, "user", user.id, Event.PRESENCE_UPDATE, payload)
    await db.commit()
    return payload


@router.delete("/blocks/{user_id}")
async def remove_block(
    user_id: str,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> dict:
    block = await social.block_row(db, user.id, user_id)
    if block is not None:
        await db.delete(block)
        outbox.queue(db, "user", user.id, Event.PRESENCE_UPDATE, {"user_id": user_id, "blocked": False})
        await db.commit()
    return {"user_id": user_id, "blocked": False}

