"""P1 identity security: TOTP 2FA fields and security audit log

Revision ID: c7d1e2f3a4b5
Revises: 9d2f4a7c1b3e
Create Date: 2026-11-25 15:10:00.000000

"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import op

# revision identifiers, used by Alembic.
revision: str = "c7d1e2f3a4b5"
down_revision: str | Sequence[str] | None = "9d2f4a7c1b3e"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column("users", sa.Column("totp_secret", sa.String(length=64), nullable=True))
    op.add_column(
        "users",
        sa.Column("totp_enabled", sa.Boolean(), nullable=False, server_default=sa.false()),
    )
    op.add_column("users", sa.Column("recovery_codes", sa.JSON(), nullable=True))

    op.create_table(
        "security_audit",
        sa.Column("id", sa.String(length=24), nullable=False),
        sa.Column("user_id", sa.String(length=24), nullable=True),
        sa.Column("email", sa.String(length=254), nullable=False),
        sa.Column("event", sa.String(length=32), nullable=False),
        sa.Column("success", sa.Boolean(), nullable=False),
        sa.Column("ip", sa.String(length=64), nullable=False),
        sa.Column("device", sa.String(length=160), nullable=False),
        sa.Column("detail", sa.String(length=200), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
        sa.ForeignKeyConstraint(["user_id"], ["users.id"], ondelete="SET NULL"),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index("ix_security_audit_user_id", "security_audit", ["user_id"])
    op.create_index("ix_security_audit_user_created", "security_audit", ["user_id", "created_at"])


def downgrade() -> None:
    op.drop_index("ix_security_audit_user_created", table_name="security_audit")
    op.drop_index("ix_security_audit_user_id", table_name="security_audit")
    op.drop_table("security_audit")
    op.drop_column("users", "recovery_codes")
    op.drop_column("users", "totp_enabled")
    op.drop_column("users", "totp_secret")
