import os
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

SERVER_DIR = Path(__file__).resolve().parents[1]


def _alembic(db_path: Path, *args: str) -> subprocess.CompletedProcess:
    env = os.environ.copy()
    env["DATABASE_URL"] = f"sqlite+aiosqlite:///{db_path}"
    for key in ("http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY"):
        env.pop(key, None)
    return subprocess.run(
        [sys.executable, "-m", "alembic", *args],
        cwd=SERVER_DIR,
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
    )


def test_alembic_upgrade_and_downgrade_roundtrip() -> None:
    tmp = Path(tempfile.mkdtemp(prefix="dgpui-mig-"))
    db_path = tmp / "mig.db"

    up = _alembic(db_path, "upgrade", "head")
    assert up.returncode == 0, up.stderr

    con = sqlite3.connect(db_path)
    tables = {row[0] for row in con.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    assert {"gateway_events", "messages_fts", "dm_messages_fts", "bookmarks"} <= tables

    bookmark_cols = {row[1] for row in con.execute("PRAGMA table_info(bookmarks)")}
    assert bookmark_cols == {"user_id", "target_kind", "target_id", "scope", "note", "created_at"}

    dm_cols = {row[1] for row in con.execute("PRAGMA table_info(dm_channels)")}
    assert {"last_read_a", "last_read_b", "pinned_a", "pinned_b"} <= dm_cols

    # FTS 对迁移后的新写入立即生效
    con.execute(
        "INSERT INTO messages (id, channel_id, author_id, content, msg_type, pinned, version, created_at) "
        "VALUES ('m1', 'c1', 'u1', '迁移验证文本', 'text', 0, 1, '2026-01-01 00:00:00')"
    )
    con.commit()
    hit = con.execute("SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH '迁移验'").fetchone()[0]
    assert hit == 1
    con.close()

    down = _alembic(db_path, "downgrade", "41e386ed7335")
    assert down.returncode == 0, down.stderr

    con = sqlite3.connect(db_path)
    tables = {row[0] for row in con.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    assert "gateway_events" not in tables and "messages_fts" not in tables
    old_cols = {row[1] for row in con.execute("PRAGMA table_info(bookmarks)")}
    assert "message_id" in old_cols and "target_kind" not in old_cols
    con.close()

    base = _alembic(db_path, "downgrade", "base")
    assert base.returncode == 0, base.stderr


def test_alembic_preserves_legacy_bookmarks() -> None:
    tmp = Path(tempfile.mkdtemp(prefix="dgpui-mig-bm-"))
    db_path = tmp / "legacy.db"

    base = _alembic(db_path, "upgrade", "41e386ed7335")
    assert base.returncode == 0, base.stderr

    con = sqlite3.connect(db_path)
    con.execute(
        "INSERT INTO users (id, username, tag, email, password_hash, display_name, avatar_color, about, status,"
        " email_verified, created_at, updated_at) VALUES"
        " ('u1','legacy','0001','legacy@example.com','x','Legacy',0,'','offline',0,"
        " '2026-01-01 00:00:00','2026-01-01 00:00:00')"
    )
    con.execute(
        "INSERT INTO messages (id, channel_id, author_id, content, msg_type, pinned, version, created_at) VALUES"
        " ('m1','c1','u1','旧书签正文','text',0,1,'2026-01-01 00:00:00')"
    )
    con.execute(
        "INSERT INTO bookmarks (user_id, message_id, scope, note, created_at) VALUES"
        " ('u1','m1','channel','', '2026-01-01 00:00:00')"
    )
    con.commit()
    con.close()

    up = _alembic(db_path, "upgrade", "head")
    assert up.returncode == 0, up.stderr

    con = sqlite3.connect(db_path)
    rows = con.execute("SELECT target_kind, target_id FROM bookmarks").fetchall()
    assert rows == [("channel", "m1")]
    con.close()

