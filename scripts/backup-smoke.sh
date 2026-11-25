#!/usr/bin/env bash
# 备份/恢复演练：建库 -> 备份 -> 清空 -> 恢复 -> 校验数据。
# 用法：scripts/backup-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVER_DIR="$ROOT/server"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

export DATABASE_URL="sqlite+aiosqlite:///$WORK/app.db"
export UPLOAD_DIR="$WORK/uploads"
mkdir -p "$UPLOAD_DIR"

cd "$SERVER_DIR"
uv run alembic upgrade head >/dev/null
uv run python - "$DATABASE_URL" <<'PY'
import asyncio
import sys

from sqlalchemy.ext.asyncio import async_sessionmaker, create_async_engine

from app.models import User


async def main() -> None:
    engine = create_async_engine(sys.argv[1])
    session = async_sessionmaker(engine, expire_on_commit=False)
    async with session() as db:
        db.add(
            User(
                id="backup-user",
                email="backup@example.com",
                username="backup",
                tag="0001",
                display_name="Backup",
                password_hash="x",
            )
        )
        await db.commit()
    await engine.dispose()


asyncio.run(main())
PY

echo "hello attachment" >"$UPLOAD_DIR/note.txt"

cd "$ROOT"
./scripts/backup.sh "$WORK/backups" >/dev/null
ARCHIVE="$(ls "$WORK"/backups/discord-backup-*.tar.gz)"
echo "backup: $ARCHIVE"

rm -f "$WORK/app.db"
rm -rf "$UPLOAD_DIR"

./scripts/restore.sh "$ARCHIVE" --yes >/dev/null

cd "$SERVER_DIR"
uv run python - "$DATABASE_URL" "$UPLOAD_DIR" <<'PY'
import asyncio
import sys
from pathlib import Path

from sqlalchemy import text
from sqlalchemy.ext.asyncio import create_async_engine


async def main() -> None:
    engine = create_async_engine(sys.argv[1])
    async with engine.connect() as conn:
        rows = (await conn.execute(text("SELECT email FROM users WHERE id='backup-user'"))).all()
    await engine.dispose()
    assert rows and rows[0][0] == "backup@example.com", rows
    assert (Path(sys.argv[2]) / "note.txt").read_text().strip() == "hello attachment"


asyncio.run(main())
PY

echo "PASS: backup and restore roundtrip"
