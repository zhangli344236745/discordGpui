"""附件存储。默认落本地磁盘，生产把 save/url_for 换成 MinIO 预签名即可。"""

from pathlib import Path

from app.config import settings
from app.errors import bad_request


def _root() -> Path:
    root = Path(settings.upload_dir)
    root.mkdir(parents=True, exist_ok=True)
    return root


def path_for(storage_key: str) -> Path:
    return _root() / storage_key


def save(storage_key: str, data: bytes) -> int:
    if len(data) > settings.upload_max_bytes:
        raise bad_request(f"文件超过 {settings.upload_max_bytes // 1024 // 1024}MB 上限", "file")
    target = path_for(storage_key)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    return len(data)


def url_for(storage_key: str) -> str:
    if not storage_key:
        return ""
    return f"{settings.media_base_url.rstrip('/')}/{storage_key}"


def delete(storage_key: str) -> None:
    target = path_for(storage_key)
    if target.exists():
        target.unlink()
