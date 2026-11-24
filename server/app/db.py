from collections.abc import AsyncIterator
from datetime import UTC, datetime

from sqlalchemy import event
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy.orm import DeclarativeBase

from app.config import settings


class Base(DeclarativeBase):
    pass


def utcnow() -> datetime:
    return datetime.now(UTC)


engine = create_async_engine(settings.database_url, echo=False, future=True)
SessionLocal = async_sessionmaker(engine, expire_on_commit=False, class_=AsyncSession)


@event.listens_for(engine.sync_engine, "connect")
def _sqlite_pragmas(dbapi_conn, _record):  # pragma: no cover - 连接级副作用
    """SQLite 并发三件套：WAL 允许读写并行，busy_timeout 避免立刻报锁。"""
    cur = dbapi_conn.cursor()
    cur.execute("PRAGMA journal_mode=WAL")
    cur.execute("PRAGMA busy_timeout=5000")
    cur.execute("PRAGMA synchronous=NORMAL")
    cur.execute("PRAGMA foreign_keys=ON")
    cur.close()


async def get_db() -> AsyncIterator[AsyncSession]:
    async with SessionLocal() as session:
        yield session


# ---------------------------------------------------------------------------
# FTS5：external-content 表避免正文双份；trigram 让中文连续子串可检索。
# 迁移与 init_db 共用同一段 DDL，避免测试库和生产库结构漂移。
# ---------------------------------------------------------------------------

_SEARCH_SPECS: tuple[tuple[str, str, str], ...] = (
    ("messages_fts", "messages", "messages"),
    ("dm_messages_fts", "dm_messages", "dm_messages"),
)


def search_index_ddl() -> list[str]:
    stmts: list[str] = []
    for fts, table, prefix in _SEARCH_SPECS:
        stmts.append(
            f"CREATE VIRTUAL TABLE IF NOT EXISTS {fts} USING fts5("
            f"content, content='{table}', content_rowid='rowid', tokenize='trigram')"
        )
        stmts.append(
            f"CREATE TRIGGER IF NOT EXISTS {prefix}_fts_ai AFTER INSERT ON {table} BEGIN "
            f"INSERT INTO {fts}(rowid, content) VALUES (new.rowid, new.content); END"
        )
        stmts.append(
            f"CREATE TRIGGER IF NOT EXISTS {prefix}_fts_ad AFTER DELETE ON {table} BEGIN "
            f"INSERT INTO {fts}({fts}, rowid, content) VALUES ('delete', old.rowid, old.content); END"
        )
        stmts.append(
            f"CREATE TRIGGER IF NOT EXISTS {prefix}_fts_au AFTER UPDATE OF content ON {table} BEGIN "
            f"INSERT INTO {fts}({fts}, rowid, content) VALUES ('delete', old.rowid, old.content); "
            f"INSERT INTO {fts}(rowid, content) VALUES (new.rowid, new.content); END"
        )
    return stmts


async def apply_search_index(conn) -> None:  # noqa: ANN001 - AsyncConnection
    rows = (await conn.exec_driver_sql("SELECT name FROM sqlite_master WHERE type='table'")).all()
    existing = {r[0] for r in rows}
    for stmt in search_index_ddl():
        await conn.exec_driver_sql(stmt)
    # 首次创建时把历史消息灌进索引（external-content 的 rebuild）
    if "messages_fts" not in existing:
        await conn.exec_driver_sql("INSERT INTO messages_fts(messages_fts) VALUES('rebuild')")
    if "dm_messages_fts" not in existing:
        await conn.exec_driver_sql("INSERT INTO dm_messages_fts(dm_messages_fts) VALUES('rebuild')")


async def drop_search_index(conn) -> None:  # noqa: ANN001 - AsyncConnection
    for fts, _table, prefix in _SEARCH_SPECS:
        for suffix in ("au", "ad", "ai"):
            await conn.exec_driver_sql(f"DROP TRIGGER IF EXISTS {prefix}_fts_{suffix}")
        await conn.exec_driver_sql(f"DROP TABLE IF EXISTS {fts}")


async def init_db() -> None:
    """开发/测试用：直接 create_all。生产走 alembic upgrade head。"""
    from app import models  # noqa: F401  确保所有表注册到 metadata

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
        await apply_search_index(conn)


async def reset_db() -> None:
    """测试专用：连同 FTS 虚表与触发器一起重建，保证用例之间不串数据。"""
    from app import models  # noqa: F401

    async with engine.begin() as conn:
        await drop_search_index(conn)
        await conn.run_sync(Base.metadata.drop_all)
        await conn.run_sync(Base.metadata.create_all)
        await apply_search_index(conn)
