#!/usr/bin/env bash
# 备份数据库与附件。
# 用法：
#   scripts/backup.sh [输出目录]
# 环境变量：
#   DATABASE_URL  默认用 server/.env 或 SQLite 开发库
#   UPLOAD_DIR    本地附件目录
#   BACKUP_KEEP_DAYS  自动清理天数（默认 7，0 = 不清理）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVER_DIR="$ROOT/server"
OUT_DIR="${1:-$ROOT/backups}"
BACKUP_KEEP_DAYS="${BACKUP_KEEP_DAYS:-7}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$OUT_DIR"
ARCHIVE="$OUT_DIR/discord-backup-$STAMP.tar.gz"

# 读取 DATABASE_URL / UPLOAD_DIR 默认值
# 显式传入的环境变量优先于 .env
_ENV_DB="${DATABASE_URL:-}"
_ENV_UP="${UPLOAD_DIR:-}"
_ENV_S3="${S3_ENDPOINT:-}"
if [ -f "$SERVER_DIR/.env" ]; then
  set -a
  # shellcheck disable=SC1091
  . "$SERVER_DIR/.env"
  set +a
fi
[ -n "$_ENV_DB" ] && DATABASE_URL="$_ENV_DB"
[ -n "$_ENV_UP" ] && UPLOAD_DIR="$_ENV_UP"
[ -n "$_ENV_S3" ] && S3_ENDPOINT="$_ENV_S3"
DATABASE_URL="${DATABASE_URL:-sqlite+aiosqlite:///$SERVER_DIR/data/app.db}"
UPLOAD_DIR="${UPLOAD_DIR:-$SERVER_DIR/data/uploads}"

echo "==> 备份数据库"
case "$DATABASE_URL" in
  sqlite*)
    DB_PATH="${DATABASE_URL#*:///}"
    if [ ! -f "$DB_PATH" ]; then
      echo "数据库文件不存在: $DB_PATH" >&2
      exit 1
    fi
    sqlite3 "$DB_PATH" ".backup '$WORK/app.db'"
    echo "sqlite: $DB_PATH"
    ;;
  postgresql*)
    # postgresql+asyncpg://user:pass@host:port/db
    PG_URL="${DATABASE_URL/postgresql+asyncpg/postgresql}"
    if command -v pg_dump >/dev/null 2>&1; then
      pg_dump --no-owner --no-privileges "$PG_URL" >"$WORK/app.sql"
    else
      docker compose -f "$ROOT/docker-compose.yml" exec -T postgres \
        pg_dump --no-owner --no-privileges -U discord discord >"$WORK/app.sql"
    fi
    echo "postgres: dump $(wc -c <"$WORK/app.sql") bytes"
    ;;
  *)
    echo "不支持的 DATABASE_URL: $DATABASE_URL" >&2
    exit 1
    ;;
esac

echo "==> 备份附件"
if [ -n "${S3_ENDPOINT:-}" ] && command -v mc >/dev/null 2>&1; then
  mc alias set backup "$S3_ENDPOINT" "${S3_ACCESS_KEY:-}" "${S3_SECRET_KEY:-}" >/dev/null
  mkdir -p "$WORK/uploads"
  mc mirror --quiet "backup/${S3_BUCKET:-attachments}" "$WORK/uploads"
else
  if [ -d "$UPLOAD_DIR" ]; then
    cp -R "$UPLOAD_DIR" "$WORK/uploads"
  else
    mkdir -p "$WORK/uploads"
  fi
fi

cat >"$WORK/manifest.json" <<EOF
{
  "created_at": "$STAMP",
  "database_url": "$DATABASE_URL",
  "upload_dir": "$UPLOAD_DIR",
  "s3_endpoint": "${S3_ENDPOINT:-}"
}
EOF

tar -czf "$ARCHIVE" -C "$WORK" .
echo "==> 完成: $ARCHIVE ($(du -h "$ARCHIVE" | cut -f1))"

if [ "$BACKUP_KEEP_DAYS" -gt 0 ]; then
  find "$OUT_DIR" -name 'discord-backup-*.tar.gz' -mtime +"$BACKUP_KEEP_DAYS" -delete
fi
