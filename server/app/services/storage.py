"""附件存储抽象。

- 默认 `LocalStorage`：落本地磁盘，由 `/media` 静态路由提供下载。
- 配置 `S3_ENDPOINT` 后切换 `S3Storage`：写 MinIO/S3，下载走预签名 URL。
"""

from __future__ import annotations

import contextlib
import logging
from pathlib import Path
from typing import Protocol

from app.config import settings
from app.errors import payload_too_large

log = logging.getLogger("server.storage")


class Storage(Protocol):
    def save(self, storage_key: str, data: bytes) -> int: ...

    def read(self, storage_key: str) -> bytes | None: ...

    def url_for(self, storage_key: str) -> str: ...

    def delete(self, storage_key: str) -> None: ...


class LocalStorage:
    """本地磁盘实现；开发与测试默认使用。"""

    def _root(self) -> Path:
        root = Path(settings.upload_dir)
        root.mkdir(parents=True, exist_ok=True)
        return root

    def path_for(self, storage_key: str) -> Path:
        return self._root() / storage_key

    def save(self, storage_key: str, data: bytes) -> int:
        target = self.path_for(storage_key)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        return len(data)

    def read(self, storage_key: str) -> bytes | None:
        target = self.path_for(storage_key)
        if not target.exists():
            return None
        return target.read_bytes()

    def url_for(self, storage_key: str) -> str:
        if not storage_key:
            return ""
        return f"{settings.media_base_url.rstrip('/')}/{storage_key}"

    def delete(self, storage_key: str) -> None:
        target = self.path_for(storage_key)
        if target.exists():
            target.unlink()


class S3Storage:
    """MinIO / S3 兼容实现；同步 boto3 客户端，调用方放在线程里执行。"""

    def __init__(self) -> None:
        import boto3
        from botocore.config import Config

        self._bucket = settings.s3_bucket
        common = {
            "aws_access_key_id": settings.s3_access_key,
            "aws_secret_access_key": settings.s3_secret_key,
            "region_name": settings.s3_region,
            "config": Config(signature_version="s3v4", s3={"addressing_style": "path"}),
        }
        self._client = boto3.client("s3", endpoint_url=settings.s3_endpoint, **common)
        # 容器内写 minio:9000，但预签名给客户端需用宿主可达地址
        public = settings.s3_public_endpoint or settings.s3_endpoint
        self._signer = (
            self._client
            if public == settings.s3_endpoint
            else boto3.client("s3", endpoint_url=public, **common)
        )
        self._ensure_bucket()

    def _ensure_bucket(self) -> None:
        try:
            self._client.head_bucket(Bucket=self._bucket)
        except Exception:
            with contextlib.suppress(Exception):
                self._client.create_bucket(Bucket=self._bucket)

    def save(self, storage_key: str, data: bytes) -> int:
        self._client.put_object(Bucket=self._bucket, Key=storage_key, Body=data)
        return len(data)

    def read(self, storage_key: str) -> bytes | None:
        try:
            resp = self._client.get_object(Bucket=self._bucket, Key=storage_key)
        except Exception:
            return None
        return resp["Body"].read()

    def url_for(self, storage_key: str) -> str:
        if not storage_key:
            return ""
        # 生产应配置 CDN 或公网 endpoint；预签名保证私有桶可下载
        return self._signer.generate_presigned_url(
            "get_object",
            Params={"Bucket": self._bucket, "Key": storage_key},
            ExpiresIn=settings.s3_url_ttl_seconds,
        )

    def delete(self, storage_key: str) -> None:
        with contextlib.suppress(Exception):
            self._client.delete_object(Bucket=self._bucket, Key=storage_key)


storage: Storage = LocalStorage()


def init_storage() -> None:
    """配置了 S3 endpoint 时切换对象存储；否则保持本地磁盘。"""
    global storage
    if not settings.s3_endpoint:
        storage = LocalStorage()
        return
    try:
        storage = S3Storage()
        log.info("object storage enabled: %s bucket=%s", settings.s3_endpoint, settings.s3_bucket)
    except Exception:  # pragma: no cover - 对象存储不可用时应阻断启动
        log.exception("failed to initialise S3 storage")
        raise


def check_size(size: int) -> None:
    if size > settings.upload_max_bytes:
        raise payload_too_large(
            f"文件超过 {settings.upload_max_bytes // 1024 // 1024}MB 上限", "file"
        )


__all__ = ["LocalStorage", "S3Storage", "Storage", "check_size", "init_storage", "storage"]
