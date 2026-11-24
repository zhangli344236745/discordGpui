"""进程内滑动窗口限流。单 worker 有效；多进程需换 Redis（已写进风险表）。"""

from __future__ import annotations

import time
from collections import defaultdict, deque
from threading import Lock

from app.errors import ApiError, ErrorCode

_hits: dict[str, deque[float]] = defaultdict(deque)
_lock = Lock()


def check(key: str, limit: int, window_seconds: int) -> None:
    """超过窗口内次数时抛 429；正常请求记一次。"""
    now = time.monotonic()
    with _lock:
        bucket = _hits[key]
        while bucket and bucket[0] <= now - window_seconds:
            bucket.popleft()
        if len(bucket) >= limit:
            retry_after = max(1, int(window_seconds - (now - bucket[0])) + 1)
            raise ApiError(
                429,
                ErrorCode.RATE_LIMITED,
                "请求过于频繁，请稍后再试",
                extra={"retry_after": retry_after},
                headers={"Retry-After": str(retry_after)},
            )
        bucket.append(now)


def reset() -> None:
    with _lock:
        _hits.clear()
