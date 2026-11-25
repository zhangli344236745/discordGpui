"""P4 bots & integrations: bot accounts, webhooks, slash commands

Revision ID: e9f1a2b3c4d5
Revises: d8e9f0a1b2c3
Create Date: 2026-11-25 18:00:00.000000

"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import op

# revision identifiers, used by Alembic.
revision: str = "e9f1a2b3c4d5"
down_revision: str | Sequence[str] | None = "d8e9f0a1b2c3"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column(
        "users",
        sa.Column("is_bot", sa.Boolean(), nullable=False, server_default=sa.false()),
    )

    op.create_table(
        "bot_tokens",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("bot_user_id", sa.String(length=24), nullable=False),
        sa.Column("server_id", sa.String(length=24), nullable=False),
        sa.Column("token_hash", sa.String(length=128), nullable=False),
        sa.Column("revoked", sa.Boolean(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["bot_user_id"], ["users.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["server_id"], ["servers.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_bot_tokens_bot_user_id", "bot_tokens", ["bot_user_id"])
    op.create_index("ix_bot_tokens_server_id", "bot_tokens", ["server_id"])
    op.create_index("ix_bot_tokens_token_hash", "bot_tokens", ["token_hash"], unique=True)

    op.create_table(
        "webhooks",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("server_id", sa.String(length=24), nullable=False),
        sa.Column("channel_id", sa.String(length=24), nullable=False),
        sa.Column("bot_user_id", sa.String(length=24), nullable=False),
        sa.Column("name", sa.String(length=80), nullable=False),
        sa.Column("token_hash", sa.String(length=128), nullable=False),
        sa.Column("revoked", sa.Boolean(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["server_id"], ["servers.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["channel_id"], ["channels.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["bot_user_id"], ["users.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_webhooks_server_id", "webhooks", ["server_id"])
    op.create_index("ix_webhooks_channel_id", "webhooks", ["channel_id"])
    op.create_index("ix_webhooks_token_hash", "webhooks", ["token_hash"])

    op.create_table(
        "slash_commands",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("server_id", sa.String(length=24), nullable=False),
        sa.Column("name", sa.String(length=32), nullable=False),
        sa.Column("description", sa.String(length=200), nullable=False),
        sa.Column("kind", sa.String(length=16), nullable=False),
        sa.Column("response", sa.String(length=500), nullable=False),
        sa.Column("created_by", sa.String(length=24), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["server_id"], ["servers.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["created_by"], ["users.id"]),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_slash_commands_server_id", "slash_commands", ["server_id"])
    op.create_index(
        "uq_slash_commands_server_name", "slash_commands", ["server_id", "name"], unique=True
    )


def downgrade() -> None:
    op.drop_index("uq_slash_commands_server_name", table_name="slash_commands")
    op.drop_index("ix_slash_commands_server_id", table_name="slash_commands")
    op.drop_table("slash_commands")
    op.drop_index("ix_webhooks_token_hash", table_name="webhooks")
    op.drop_index("ix_webhooks_channel_id", table_name="webhooks")
    op.drop_index("ix_webhooks_server_id", table_name="webhooks")
    op.drop_table("webhooks")
    op.drop_index("ix_bot_tokens_token_hash", table_name="bot_tokens")
    op.drop_index("ix_bot_tokens_server_id", table_name="bot_tokens")
    op.drop_index("ix_bot_tokens_bot_user_id", table_name="bot_tokens")
    op.drop_table("bot_tokens")
    op.drop_column("users", "is_bot")
