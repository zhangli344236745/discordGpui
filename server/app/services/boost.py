"""加成等级：门槛唯一来源，与前端 BOOST_TIERS 保持一致。"""

from dataclasses import dataclass

BOOST_TIERS: tuple[int, ...] = (2, 7, 14)


@dataclass
class BoostStats:
    count: int
    level: int
    next_threshold: int | None
    remaining: int
    progress: float

    @property
    def perks_unlocked(self) -> list[int]:
        return [i + 1 for i, need in enumerate(BOOST_TIERS) if self.count >= need]


def boost_stats(count: int) -> BoostStats:
    level = sum(1 for need in BOOST_TIERS if count >= need)
    if level >= len(BOOST_TIERS):
        return BoostStats(count=count, level=level, next_threshold=None, remaining=0, progress=1.0)

    next_threshold = BOOST_TIERS[level]
    floor = BOOST_TIERS[level - 1] if level > 0 else 0
    span = next_threshold - floor
    progress = (count - floor) / span if span > 0 else 0.0
    return BoostStats(
        count=count,
        level=level,
        next_threshold=next_threshold,
        remaining=next_threshold - count,
        progress=max(0.0, min(1.0, progress)),
    )
