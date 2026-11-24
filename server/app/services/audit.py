"""审核日志写入。只 add 不 commit，事务边界留给调用方。"""

from sqlalchemy.ext.asyncio import AsyncSession

from app.ids import new_id
from app.models import AuditEntry

KIND_SERVER = "server"
KIND_CHANNEL = "channel"
KIND_ROLE = "role"
KIND_MEMBER = "member"
KIND_INVITE = "invite"
KIND_EMOJI = "emoji"
KIND_MESSAGE = "message"
KIND_BOOST = "boost"


def log(
    db: AsyncSession,
    server_id: str,
    actor_id: str,
    kind: str,
    detail: str,
    target: str = "",
) -> AuditEntry:
    entry = AuditEntry(
        id=new_id(),
        server_id=server_id,
        actor_id=actor_id,
        kind=kind,
        detail=detail[:300],
        target=target[:120],
    )
    db.add(entry)
    return entry
