"""P5 compliance: reports, actions, retention rules, legal holds, account deletion

Revision ID: f2a3b4c5d6e7
Revises: e9f1a2b3c4d5
Create Date: 2026-11-25 19:20:00.000000

"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import op

# revision identifiers, used by Alembic.
revision: str = "f2a3b4c5d6e7"
down_revision: str | Sequence[str] | None = "e9f1a2b3c4d5"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column("users", sa.Column("deleted_at", sa.DateTime(timezone=True), nullable=True))

    op.create_table(
        "reports",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("server_id", sa.String(length=24), nullable=True),
        sa.Column("reporter_id", sa.String(length=24), nullable=False),
        sa.Column("target_kind", sa.String(length=16), nullable=False),
        sa.Column("target_id", sa.String(length=24), nullable=False),
        sa.Column("reason", sa.String(length=64), nullable=False),
        sa.Column("detail", sa.String(length=500), nullable=False),
        sa.Column("status", sa.String(length=16), nullable=False),
        sa.Column("count", sa.Integer(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["server_id"], ["servers.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["reporter_id"], ["users.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_reports_server_id", "reports", ["server_id"])
    op.create_index("ix_reports_reporter_id", "reports", ["reporter_id"])
    op.create_index(
        "uq_reports_reporter_target",
        "reports",
        ["reporter_id", "target_kind", "target_id"],
        unique=True,
    )

    op.create_table(
        "report_actions",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("report_id", sa.String(length=24), nullable=False),
        sa.Column("actor_id", sa.String(length=24), nullable=False),
        sa.Column("action", sa.String(length=16), nullable=False),
        sa.Column("note", sa.String(length=300), nullable=False),
        sa.Column("deleted_content", sa.Boolean(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["report_id"], ["reports.id"], ondelete="CASCADE"),
        sa.ForeignKeyConstraint(["actor_id"], ["users.id"]),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_report_actions_report_id", "report_actions", ["report_id"])

    op.create_table(
        "retention_rules",
        sa.Column("server_id", sa.String(length=24), nullable=False),
        sa.Column("message_days", sa.Integer(), nullable=False),
        sa.Column("attachment_days", sa.Integer(), nullable=False),
        sa.Column("audit_days", sa.Integer(), nullable=False),
        sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["server_id"], ["servers.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("server_id"),
    )

    op.create_table(
        "legal_holds",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("server_id", sa.String(length=24), nullable=False),
        sa.Column("target_kind", sa.String(length=16), nullable=False),
        sa.Column("target_id", sa.String(length=24), nullable=False),
        sa.Column("note", sa.String(length=200), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["server_id"], ["servers.id"], ondelete="CASCADE"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_legal_holds_server_id", "legal_holds", ["server_id"])
    op.create_index(
        "uq_legal_holds", "legal_holds", ["server_id", "target_kind", "target_id"], unique=True
    )


def downgrade() -> None:
    op.drop_index("uq_legal_holds", table_name="legal_holds")
    op.drop_index("ix_legal_holds_server_id", table_name="legal_holds")
    op.drop_table("legal_holds")
    op.drop_table("retention_rules")
    op.drop_index("ix_report_actions_report_id", table_name="report_actions")
    op.drop_table("report_actions")
    op.drop_index("uq_reports_reporter_target", table_name="reports")
    op.drop_index("ix_reports_reporter_id", table_name="reports")
    op.drop_index("ix_reports_server_id", table_name="reports")
    op.drop_table("reports")
    op.drop_column("users", "deleted_at")
