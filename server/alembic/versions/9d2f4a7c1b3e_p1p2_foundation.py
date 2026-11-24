"""P1/P2 foundation: gateway event log, FTS index, DM read/pin, bookmarks identity

Revision ID: 9d2f4a7c1b3e
Revises: 41e386ed7335
Create Date: 2026-11-24 16:30:00.000000

"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import op

from app.db import search_index_ddl

# revision identifiers, used by Alembic.
revision: str = "9d2f4a7c1b3e"
down_revision: str | Sequence[str] | None = "41e386ed7335"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    # 1. Gateway 事件日志（?since 补偿 + 24h 保留）
    op.create_table(
        "gateway_events",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("scope_kind", sa.String(length=8), nullable=False),
        sa.Column("scope_id", sa.String(length=24), nullable=False),
        sa.Column("event", sa.String(length=40), nullable=False),
        sa.Column("payload", sa.JSON(), nullable=True),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_gateway_events_scope", "gateway_events", ["scope_kind", "scope_id", "id"])
    op.create_index("ix_gateway_events_created", "gateway_events", ["created_at"])

    # 2. DM 双方已读与置顶（避免为两个用户建两张表）
    with op.batch_alter_table("dm_channels") as batch:
        batch.add_column(sa.Column("last_read_a", sa.String(length=24), nullable=True))
        batch.add_column(sa.Column("last_read_b", sa.String(length=24), nullable=True))
        batch.add_column(sa.Column("pinned_a", sa.Boolean(), nullable=False, server_default=sa.false()))
        batch.add_column(sa.Column("pinned_b", sa.Boolean(), nullable=False, server_default=sa.false()))

    # 3. 呼叫结束原因
    with op.batch_alter_table("call_sessions") as batch:
        batch.add_column(sa.Column("outcome", sa.String(length=16), nullable=False, server_default=""))

    # 4. 推送端点（Demo 只存不投递）
    with op.batch_alter_table("user_settings") as batch:
        batch.add_column(sa.Column("push_endpoints", sa.JSON(), nullable=True))

    # 5. 书签改为通用消息目标（频道 + 私信），主键不再绑定 messages 外键
    op.create_table(
        "bookmarks_new",
        sa.Column("user_id", sa.String(length=24), nullable=False),
        sa.Column("target_kind", sa.String(length=8), nullable=False),
        sa.Column("target_id", sa.String(length=24), nullable=False),
        sa.Column("scope", sa.String(length=24), nullable=False),
        sa.Column("note", sa.String(length=190), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["user_id"], ["users.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("user_id", "target_kind", "target_id"),
    )
    op.execute(
        "INSERT INTO bookmarks_new (user_id, target_kind, target_id, scope, note, created_at) "
        "SELECT user_id, 'channel', message_id, scope, note, created_at FROM bookmarks"
    )
    op.drop_index("ix_bookmarks_user_created", table_name="bookmarks")
    op.drop_table("bookmarks")
    op.rename_table("bookmarks_new", "bookmarks")
    op.create_index("ix_bookmarks_user_created", "bookmarks", ["user_id", "created_at"])

    # 6. FTS5 external-content 索引 + 同步触发器
    for stmt in search_index_ddl():
        op.execute(stmt)
    op.execute("INSERT INTO messages_fts(messages_fts) VALUES('rebuild')")
    op.execute("INSERT INTO dm_messages_fts(dm_messages_fts) VALUES('rebuild')")


def downgrade() -> None:
    for fts, _table, prefix in (
        ("messages_fts", "messages", "messages"),
        ("dm_messages_fts", "dm_messages", "dm_messages"),
    ):
        for suffix in ("au", "ad", "ai"):
            op.execute(f"DROP TRIGGER IF EXISTS {prefix}_fts_{suffix}")
        op.execute(f"DROP TABLE IF EXISTS {fts}")

    op.create_table(
        "bookmarks_old",
        sa.Column("user_id", sa.String(length=24), nullable=False),
        sa.Column("message_id", sa.String(length=24), nullable=False),
        sa.Column("scope", sa.String(length=16), nullable=False),
        sa.Column("note", sa.String(length=190), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["message_id"], ["messages.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["user_id"], ["users.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("user_id", "message_id"),
    )
    op.execute(
        "INSERT INTO bookmarks_old (user_id, message_id, scope, note, created_at) "
        "SELECT user_id, target_id, scope, note, created_at FROM bookmarks WHERE target_kind='channel'"
    )
    op.drop_index("ix_bookmarks_user_created", table_name="bookmarks")
    op.drop_table("bookmarks")
    op.rename_table("bookmarks_old", "bookmarks")
    op.create_index("ix_bookmarks_user_created", "bookmarks", ["user_id", "created_at"])

    with op.batch_alter_table("user_settings") as batch:
        batch.drop_column("push_endpoints")
    with op.batch_alter_table("call_sessions") as batch:
        batch.drop_column("outcome")
    with op.batch_alter_table("dm_channels") as batch:
        batch.drop_column("pinned_b")
        batch.drop_column("pinned_a")
        batch.drop_column("last_read_b")
        batch.drop_column("last_read_a")

    op.drop_index("ix_gateway_events_created", table_name="gateway_events")
    op.drop_index("ix_gateway_events_scope", table_name="gateway_events")
    op.drop_table("gateway_events")
