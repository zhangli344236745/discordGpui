"""媒体校验与缩略图：MIME 嗅探、伪装拒绝、图片缩略图。

- sniff：按魔数识别常见格式，识别不了返回 None（允许未知类型）。
- verify_claimed：声明为图片/PDF/ZIP 但内容不符时拒绝，防伪装上传。
- thumbnail：图片生成最长边 512px 的 JPEG 缩略图，非图片返回 None。
"""

from __future__ import annotations

import io

from PIL import Image

THUMBNAIL_MAX_EDGE = 512
THUMBNAIL_JPEG_QUALITY = 80

_MAGIC: tuple[tuple[bytes, str], ...] = (
    (b"\x89PNG\r\n\x1a\n", "image/png"),
    (b"\xff\xd8\xff", "image/jpeg"),
    (b"GIF87a", "image/gif"),
    (b"GIF89a", "image/gif"),
    (b"%PDF-", "application/pdf"),
    (b"PK\x03\x04", "application/zip"),
)


def sniff(data: bytes) -> str | None:
    for magic, mime in _MAGIC:
        if data.startswith(magic):
            return mime
    # WEBP: RIFF....WEBP
    if len(data) >= 12 and data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "image/webp"
    return None


def verify_claimed(claimed: str, data: bytes) -> str | None:
    """返回拒绝原因；None 表示通过。

    只对能嗅探出内容的类型做强校验：声称图片/PDF/ZIP 却不是，视为伪装。
    """
    sniffed = sniff(data)
    # 声明是图片但嗅探不出已知图片格式 → 视为伪装
    if claimed.startswith("image/"):
        if sniffed is None:
            return "文件内容无法识别为图片，拒绝上传"
        if not sniffed.startswith("image/"):
            return f"文件内容是 {sniffed}，与声明的 {claimed} 不符"
        return None
    if sniffed is None or sniffed == claimed:
        return None
    if claimed == "application/pdf" and sniffed != "application/pdf":
        return "文件内容不是合法 PDF"
    if claimed == "application/zip" and sniffed != "application/zip":
        return "文件内容不是合法 ZIP"
    return None


def thumbnail(data: bytes) -> bytes | None:
    """生成缩略图字节；非图片或生成失败返回 None。"""
    try:
        image = Image.open(io.BytesIO(data))
        image.load()
    except Exception:
        return None
    if image.mode not in ("RGB", "L"):
        image = image.convert("RGB")
    image.thumbnail((THUMBNAIL_MAX_EDGE, THUMBNAIL_MAX_EDGE))
    buffer = io.BytesIO()
    try:
        image.save(buffer, format="JPEG", quality=THUMBNAIL_JPEG_QUALITY)
    except Exception:
        return None
    return buffer.getvalue()


__all__ = ["THUMBNAIL_MAX_EDGE", "sniff", "thumbnail", "verify_claimed"]
