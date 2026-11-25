from fastapi import APIRouter, Depends
from sqlalchemy import or_, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import current_user
from app.errors import bad_request, conflict, not_found
from app.ids import new_tag
from app.models import (
    DmChannel,
    Friendship,
    ServerMember,
    User,
    UserSettings,
)
from app.realtime.bus import bus
from app.realtime.events import Event
from app.schemas.user import SettingsOut, SettingsPatch, StatusIn, UserOut, UserPatch

router = APIRouter(prefix="/users", tags=["users"])


def _out(user: User, include_email: bool = False) -> UserOut:
    data = UserOut.model_validate(user)
    data.email = user.email if include_email else None
    return data


async def _settings(db: AsyncSession, user_id: str) -> UserSettings:
    row = await db.get(UserSettings, user_id)
    if row is None:  # 老账号兜底，避免 404
        row = UserSettings(user_id=user_id)
        db.add(row)
        await db.commit()
    return row


@router.get("/@me", response_model=UserOut)
async def me(user: User = Depends(current_user)) -> UserOut:
    return _out(user, include_email=True)


@router.patch("/@me", response_model=UserOut)
async def patch_me(body: UserPatch, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> UserOut:
    if body.username is not None and body.username != user.username:
        # 改名同时换 tag，避免与他人撞名
        user.username = body.username
        user.tag = new_tag()
    if body.display_name is not None:
        if not body.display_name.strip():
            raise bad_request("显示名称不能为空", "display_name")
        user.display_name = body.display_name.strip()
    if body.email is not None and body.email != user.email:
        exists = await db.scalar(select(User.id).where(User.email == body.email.lower()))
        if exists is not None:
            raise conflict("邮箱已被使用", "email")
        user.email = body.email.lower()
        user.email_verified = False
    if body.about is not None:
        user.about = body.about
    if body.avatar_color is not None:
        user.avatar_color = body.avatar_color

    await db.commit()
    return _out(user, include_email=True)


@router.patch("/@me/status", response_model=UserOut)
async def patch_status(
    body: StatusIn, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> UserOut:
    user.status = body.status
    await db.commit()

    # 只推给好友，别人不需要知道
    friend_ids = list((await db.scalars(select(Friendship.friend_id).where(Friendship.user_id == user.id))).all())
    for fid in friend_ids:
        await bus.to_user(
            fid,
            Event.PRESENCE_UPDATE,
            {"user_id": user.id, "status": body.status},
        )
    return _out(user, include_email=True)


@router.get("/@me/settings", response_model=SettingsOut)
async def get_settings(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> SettingsOut:
    return SettingsOut.model_validate(await _settings(db, user.id))


@router.patch("/@me/settings", response_model=SettingsOut)
async def patch_settings(
    body: SettingsPatch,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> SettingsOut:
    row = await _settings(db, user.id)
    for field, value in body.model_dump(exclude_none=True).items():
        setattr(row, field, value)
    await db.commit()
    return SettingsOut.model_validate(row)


@router.get("/@me/shortcuts")
async def shortcuts(user: User = Depends(current_user)) -> dict:
    """Demo：快捷键映射存在客户端，这里只回默认值，接后端后可覆盖。"""
    return {
        "quick_switcher": "Mod+K",
        "close_overlay": "Escape",
        "send_message": "Enter",
        "newline": "Shift+Enter",
    }


@router.get("/{user_id}", response_model=UserOut)
async def get_user(user_id: str, db: AsyncSession = Depends(get_db)) -> UserOut:
    target = await db.get(User, user_id)
    if target is None:
        raise not_found("用户")
    return _out(target)


@router.get("/{user_id}/mutual")
async def mutual(user_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    my_servers = set((await db.scalars(select(ServerMember.server_id).where(ServerMember.user_id == user.id))).all())
    their_servers = set((await db.scalars(select(ServerMember.server_id).where(ServerMember.user_id == user_id))).all())
    my_friends = set((await db.scalars(select(Friendship.friend_id).where(Friendship.user_id == user.id))).all())
    their_friends = set((await db.scalars(select(Friendship.friend_id).where(Friendship.user_id == user_id))).all())
    # 共同会话也算一种关系
    common_dms = list(
        (
            await db.scalars(
                select(DmChannel.id).where(
                    or_(
                        (DmChannel.user_a == user.id) & (DmChannel.user_b == user_id),
                        (DmChannel.user_a == user_id) & (DmChannel.user_b == user.id),
                    )
                )
            )
        ).all()
    )
    return {
        "servers": sorted(my_servers & their_servers),
        "friends": sorted(my_friends & their_friends),
        "dms": common_dms,
    }
