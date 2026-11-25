"""P2 media hardening: thumbnail column

Revision ID: d8e9f0a1b2c3
Revises: c7d1e2f3a4b5
Create Date: 2026-11-25 16:40:00.000000

"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import op

# revision identifiers, used by Alembic.
revision: str = "d8e9f0a1b2c3"
down_revision: str | Sequence[str] | None = "c7d1e2f3a4b5"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column("attachments", sa.Column("thumb_storage_key", sa.String(length=267), nullable=True))


def downgrade() -> None:
    op.drop_column("attachments", "thumb_storage_key")
