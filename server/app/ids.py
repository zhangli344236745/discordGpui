"""雪花 ID。

客户端契约里所有 ID 都是字符串（避免 JS 精度丢失），服务端内部也统一按 str 存。
41 bit 时间戳（自定义 epoch）+ 10 bit 机器位 + 12 bit 序列，单进程内严格递增。
"""

import secrets
import string
import threading
import time

EPOCH_MS = 1767225600000  # 2026-01-01T00:00:00Z
WORKER_BITS = 10
SEQUENCE_BITS = 12
MAX_SEQUENCE = (1 << SEQUENCE_BITS) - 1

_lock = threading.Lock()
_last_ms = 0
_sequence = 0
_worker_id = secrets.randbelow(1 << WORKER_BITS)


def new_id() -> str:
    global _last_ms, _sequence
    with _lock:
        now = int(time.time() * 1000)
        if now < _last_ms:
            now = _last_ms
        if now == _last_ms:
            _sequence = (_sequence + 1) & MAX_SEQUENCE
            if _sequence == 0:
                while now <= _last_ms:
                    time.sleep(0.001)
                    now = int(time.time() * 1000)
        else:
            _sequence = 0
        _last_ms = now
        value = ((now - EPOCH_MS) << (WORKER_BITS + SEQUENCE_BITS)) | (_worker_id << SEQUENCE_BITS) | _sequence
        return str(value)


_CODE_ALPHABET = string.ascii_lowercase + string.digits


def new_code(length: int = 8) -> str:
    """邀请码 / 短标识：小写字母 + 数字，避免 0/O、1/l 混淆。"""
    alphabet = _CODE_ALPHABET.replace("0", "").replace("o", "").replace("1", "").replace("l", "")
    return "".join(secrets.choice(alphabet) for _ in range(length))


def new_tag() -> str:
    """Discord 风格的 4 位数字标签。"""
    return f"{secrets.randbelow(10000):04d}"
