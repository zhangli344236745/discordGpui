import asyncio
from pathlib import Path

from fastapi import APIRouter, Depends, Request, status
from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app import errors
from app.config import settings
from app.db import get_db
from app.deps import current_user
from app.errors import forbidden, not_found
from app.ids import new_id
from app.models import Attachment, User
from app.schemas.message import UploadIn
from app.services import media
from app.services import storage as storage_service

router = APIRouter(tags=["uploads"])


async def _used_bytes(db: AsyncSession, user_id: str) -> int:
    # 已上传与已声明未完成的都计入，避免只申报不 PUT 绕过配额
    total = await db.scalar(
        select(func.coalesce(func.sum(Attachment.size), 0)).where(Attachment.owner_id == user_id)
    )
    return int(total or 0)


def _check_quota(used: int, incoming: int) -> None:
    if settings.upload_user_quota_bytes <= 0:
        return
    if used + incoming > settings.upload_user_quota_bytes:
        raise errors.bad_request(
            f"存储配额不足（已用 {used // 1024 // 1024}MB / "
            f"{settings.upload_user_quota_bytes // 1024 // 1024}MB），请删除旧附件后再试",
            "size",
        )


@router.post("/uploads", status_code=status.HTTP_201_CREATED)
async def create_upload(body: UploadIn, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    storage_service.check_size(body.size)
    _check_quota(await _used_bytes(db, user.id), body.size)
    file_id = new_id()
    suffix = Path(body.filename).suffix[:16]
    storage_key = f"{user.id}/{file_id}{suffix}"
    db.add(
        Attachment(
            id=file_id,
            owner_id=user.id,
            filename=body.filename,
            mime=body.mime,
            size=body.size,
            storage_key=storage_key,
            uploaded=False,
        )
    )
    await db.commit()
    # 本地磁盘用同域 PUT；对象存储由 url_for 生成预签名 GET，写入仍走本接口
    return {
        "file_id": file_id,
        "upload_url": f"{settings.api_prefix}/uploads/{file_id}/content",
        "expires_in": 900,
    }


@router.put("/uploads/{file_id}/content", status_code=status.HTTP_204_NO_CONTENT)
async def upload_content(
    file_id: str,
    request: Request,
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> None:
    att = await db.get(Attachment, file_id)
    if att is None or att.owner_id != user.id:
        raise not_found("上传任务")
    data = await request.body()
    storage_service.check_size(len(data))
    _check_quota(await _used_bytes(db, user.id), len(data))
    if reason := media.verify_claimed(att.mime, data):
        raise errors.bad_request(reason, "mime")

    size = await asyncio.to_thread(storage_service.storage.save, att.storage_key, data)
    att.size = size

    # 图片生成缩略图；失败不影响上传本身
    thumb = media.thumbnail(data)
    if thumb is not None:
        thumb_key = f"{att.storage_key}.thumb.jpg"
        await asyncio.to_thread(storage_service.storage.save, thumb_key, thumb)
        att.thumb_storage_key = thumb_key
    att.uploaded = True
    await db.commit()


@router.get("/attachments/{attachment_id}")
async def get_attachment(
    attachment_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> dict:
    att = await db.get(Attachment, attachment_id)
    if att is None or not att.uploaded:
        raise not_found("附件")
    out = {
        "id": att.id,
        "filename": att.filename,
        "mime": att.mime,
        "size": att.size,
        "url": storage_service.storage.url_for(att.storage_key),
    }
    if att.thumb_storage_key:
        out["thumbnail_url"] = storage_service.storage.url_for(att.thumb_storage_key)
    return out


@router.delete("/attachments/{attachment_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_attachment(
    attachment_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> None:
    att = await db.get(Attachment, attachment_id)
    if att is None:
        raise not_found("附件")
    if att.owner_id != user.id:
        raise forbidden("MANAGE_MESSAGES")
    if att.message_id is not None:
        # 已被消息引用，只解绑不删文件
        att.message_id = None
    await asyncio.to_thread(storage_service.storage.delete, att.storage_key)
    if att.thumb_storage_key:
        await asyncio.to_thread(storage_service.storage.delete, att.thumb_storage_key)
    await db.delete(att)
    await db.commit()


__all__ = ["router"]
