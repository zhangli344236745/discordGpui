from pathlib import Path

import pytest

from app.config import settings
from app.errors import ApiError
from app.services import storage as storage_module
from app.services.storage import LocalStorage, check_size


def test_local_storage_roundtrip(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(settings, "upload_dir", tmp_path / "uploads")
    backend = LocalStorage()

    key = "user-1/file.txt"
    assert backend.save(key, b"hello") == 5
    assert backend.read(key) == b"hello"
    assert backend.url_for(key).endswith(key)

    backend.delete(key)
    assert backend.read(key) is None


def test_check_size_rejects_oversized(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(settings, "upload_max_bytes", 4)
    check_size(4)
    with pytest.raises(ApiError):
        check_size(5)


def test_init_storage_defaults_to_local(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setattr(settings, "upload_dir", tmp_path)
    monkeypatch.setattr(settings, "s3_endpoint", "")
    storage_module.init_storage()
    assert isinstance(storage_module.storage, LocalStorage)
