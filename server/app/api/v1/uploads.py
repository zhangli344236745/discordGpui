from pathlib import Path

from fastapi import APIRouter, Depends, Request, status
from sqlalchemy.ext.asyncio import AsyncSession

from app import errors
from app.config import settings
from app.db import get_db
from app.deps import current_user
from app.errors import forbidden, not_found
from app.ids import new_id
from app.models import Attachment, User
from app.schemas.message import UploadIn
from app.services import storage

router = APIRouter(tags=["uploads"])


@router.post("/uploads", status_code=status.HTTP_201_CREATED)
async def create_upload(body: UploadIn, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    if body.size > settings.upload_max_bytes:
        raise errors.bad_request(f"文件超过 {settings.upload_max_bytes // 1024 // 1024}MB 上限", "size")
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
    # Demo 用同域 PUT 直传；生产换成对象存储预签名 URL
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
    size = storage.save(att.storage_key, data)
    att.size = size
    att.uploaded = True
    await db.commit()


@router.get("/attachments/{attachment_id}")
async def get_attachment(
    attachment_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> dict:
    att = await db.get(Attachment, attachment_id)
    if att is None or not att.uploaded:
        raise not_found("附件")
    return {
        "id": att.id,
        "filename": att.filename,
        "mime": att.mime,
        "size": att.size,
        "url": storage.url_for(att.storage_key),
    }


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
    storage.delete(att.storage_key)
    await db.delete(att)
    await db.commit()
