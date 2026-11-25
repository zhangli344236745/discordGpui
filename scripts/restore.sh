#!/usr/bin/env bash
# 从备份包恢复数据库与附件。
# 用法：
#   scripts/restore.sh <备份包> [--yes]
# 环境变量：
#   DATABASE_URL  目标数据库；默认 SQLite 开发库
#   UPLOAD_DIR    本地附件目录
#   S3_*          配置后附件恢复到对象存储（需 mc）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVER_DIR="$ROOT/server"
ARCHIVE="${1:-}"
CONFIRM="${2:-}"

if [ -z "$ARCHIVE" ] || [ ! -f "$ARCHIVE" ]; then
  echo "用法: scripts/restore.sh <backup.tar.gz> [--yes]" >&2
  exit 1
fi
if [ "$CONFIRM" != "--yes" ]; then
  echo "恢复会覆盖当前数据库与附件。确认请加 --yes" >&2
  exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
tar -xzf "$ARCHIVE" -C "$WORK"

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

echo "==> 恢复数据库"
case "$DATABASE_URL" in
  sqlite*)
    DB_PATH="${DATABASE_URL#*:///}"
    mkdir -p "$(dirname "$DB_PATH")"
    rm -f "$DB_PATH-wal" "$DB_PATH-shm"
    cp "$WORK/app.db" "$DB_PATH"
    echo "sqlite: $DB_PATH"
    ;;
  postgresql*)
    PG_URL="${DATABASE_URL/postgresql+asyncpg/postgresql}"
    if command -v psql >/dev/null 2>&1; then
      psql "$PG_URL" -v ON_ERROR_STOP=1 -f "$WORK/app.sql" >/dev/null
    else
      docker compose -f "$ROOT/docker-compose.yml" exec -T postgres \
        psql -U discord -d discord -v ON_ERROR_STOP=1 <"$WORK/app.sql" >/dev/null
    fi
    echo "postgres: restored"
    ;;
  *)
    echo "不支持的 DATABASE_URL: $DATABASE_URL" >&2
    exit 1
    ;;
esac

echo "==> 恢复附件"
if [ -n "${S3_ENDPOINT:-}" ] && command -v mc >/dev/null 2>&1; then
  mc alias set backup "$S3_ENDPOINT" "${S3_ACCESS_KEY:-}" "${S3_SECRET_KEY:-}" >/dev/null
  mc mirror --quiet --overwrite "$WORK/uploads" "backup/${S3_BUCKET:-attachments}"
else
  mkdir -p "$UPLOAD_DIR"
  cp -R "$WORK/uploads/." "$UPLOAD_DIR/"
fi

echo "==> 恢复完成。请重启服务并执行健康检查。"
