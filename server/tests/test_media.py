import io

from httpx import AsyncClient
from PIL import Image

from app.services import media
from tests.conftest import auth_headers, create_server, first_channel


def _png(color=(255, 0, 0)) -> bytes:
    image = Image.new("RGB", (1024, 1024), color)
    buffer = io.BytesIO()
    image.save(buffer, format="PNG")
    return buffer.getvalue()


async def test_oversize_upload_returns_413(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    init = await client.post(
        "/api/v1/uploads",
        headers=headers,
        json={"filename": "big.bin", "size": 26 * 1024 * 1024, "mime": "application/octet-stream"},
    )
    assert init.status_code == 413
    assert init.json()["error"]["code"] == "PAYLOAD_TOO_LARGE"


async def test_spoofed_image_rejected(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    init = await client.post(
        "/api/v1/uploads",
        headers=headers,
        json={"filename": "fake.png", "size": 5, "mime": "image/png"},
    )
    file_id = init.json()["file_id"]

    put = await client.put(
        f"/api/v1/uploads/{file_id}/content",
        headers=headers,
        content=b"hello text pretending to be an image",
    )
    assert put.status_code == 400
    assert put.json()["error"]["message"]

    detail = await client.get(f"/api/v1/attachments/{file_id}", headers=headers)
    assert detail.status_code == 404


async def test_image_upload_generates_thumbnail(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"])
    payload = _png()

    init = await client.post(
        "/api/v1/uploads",
        headers=headers,
        json={"filename": "photo.png", "size": len(payload), "mime": "image/png"},
    )
    file_id = init.json()["file_id"]
    put = await client.put(f"/api/v1/uploads/{file_id}/content", headers=headers, content=payload)
    assert put.status_code == 204

    detail = await client.get(f"/api/v1/attachments/{file_id}", headers=headers)
    body = detail.json()
    assert body["size"] == len(payload)
    assert body["thumbnail_url"]

    sent = await client.post(
        f"/api/v1/channels/{channel['id']}/messages",
        headers=headers,
        json={"content": "", "attachment_ids": [file_id]},
    )
    assert sent.status_code == 201
    assert sent.json()["attachments"][0]["thumbnail_url"]


async def test_quota_rejects_excess(client: AsyncClient, monkeypatch) -> None:
    from app.config import settings

    headers = await auth_headers(client)
    monkeypatch.setattr(settings, "upload_user_quota_bytes", 1024)

    init = await client.post(
        "/api/v1/uploads",
        headers=headers,
        json={"filename": "a.bin", "size": 600, "mime": "application/octet-stream"},
    )
    assert init.status_code == 201
    second = await client.post(
        "/api/v1/uploads",
        headers=headers,
        json={"filename": "b.bin", "size": 600, "mime": "application/octet-stream"},
    )
    assert second.status_code == 400
    assert "配额" in second.json()["error"]["message"]


def test_sniff_and_thumbnail_helpers() -> None:
    assert media.sniff(b"\x89PNG\r\n\x1a\n" + b"0" * 16) == "image/png"
    assert media.sniff(b"RIFF1234WEBPVP8 ") == "image/webp"
    assert media.sniff(b"plain text") is None

    assert media.verify_claimed("image/png", b"\x89PNG\r\n\x1a\n" + b"0" * 8) is None
    assert media.verify_claimed("image/png", b"plain text") is not None
    # 非图片声明 + 可嗅探内容：宽松处理（例如把 PNG 存成 octet-stream）
    assert media.verify_claimed("text/plain", b"anything") is None

    thumb = media.thumbnail(_png())
    assert thumb is not None
    image = Image.open(io.BytesIO(thumb))
    assert max(image.size) <= media.THUMBNAIL_MAX_EDGE

    assert media.thumbnail(b"not an image") is None
