"""自动审核：服务端关键词扫描。block 阻止落库，warn 落库但回传警告。"""

from dataclasses import dataclass, field

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.models import AutoModKeyword, AutoModRule


@dataclass
class AutoModHit:
    rule_id: str
    rule_name: str
    action: str
    keyword: str


@dataclass
class AutoModResult:
    blocked: AutoModHit | None = None
    warnings: list[AutoModHit] = field(default_factory=list)

    @property
    def is_blocked(self) -> bool:
        return self.blocked is not None


async def scan(db: AsyncSession, server_id: str, content: str) -> AutoModResult:
    result = AutoModResult()
    if not content.strip():
        return result

    rules = list(
        (
            await db.scalars(
                select(AutoModRule).where(AutoModRule.server_id == server_id, AutoModRule.enabled.is_(True))
            )
        ).all()
    )
    if not rules:
        return result

    lowered = content.lower()
    rule_ids = [r.id for r in rules]
    keywords = list((await db.scalars(select(AutoModKeyword).where(AutoModKeyword.rule_id.in_(rule_ids)))).all())

    by_rule: dict[str, list[str]] = {}
    for kw in keywords:
        by_rule.setdefault(kw.rule_id, []).append(kw.keyword)

    for rule in rules:
        for word in by_rule.get(rule.id, []):
            needle = word.lower().strip()
            if needle and needle in lowered:
                hit = AutoModHit(rule.id, rule.name, rule.action, word)
                if rule.action == "block":
                    result.blocked = result.blocked or hit
                else:
                    result.warnings.append(hit)
                break
    return result
