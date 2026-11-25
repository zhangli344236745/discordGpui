"""运维观测：Prometheus 指标、request_id 中间件与 JSON 日志。

- 指标：HTTP 请求量/延迟/错误、WS 连接数、消息吞吐、REST 在途请求。
- request_id：入站取 `X-Request-ID`，否则生成；写回响应头并注入 contextvar，
  日志与业务代码都能读到。
- 日志：JSON 一行一条，便于采集；只记录路径与元数据，不落请求体与凭证。
"""

from __future__ import annotations

import contextvars
import json
import logging
import time
import uuid
from collections.abc import Callable

from prometheus_client import (
    CONTENT_TYPE_LATEST,
    Counter,
    Gauge,
    Histogram,
    generate_latest,
)
from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.responses import Response

request_id_var: contextvars.ContextVar[str] = contextvars.ContextVar("request_id", default="")

HTTP_REQUESTS = Counter(
    "dgpui_http_requests_total",
    "HTTP 请求总数",
    ["method", "path", "status"],
)
HTTP_LATENCY = Histogram(
    "dgpui_http_request_seconds",
    "HTTP 请求耗时（秒）",
    ["method", "path"],
    buckets=(0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5),
)
HTTP_INFLIGHT = Gauge("dgpui_http_inflight", "处理中的 HTTP 请求数")
WS_CONNECTIONS = Gauge("dgpui_gateway_connections", "当前 Gateway WebSocket 连接数")
MESSAGES_TOTAL = Counter("dgpui_messages", "落库消息总数", ["kind"])
EVENTS_PUBLISHED = Counter("dgpui_events_published_total", "广播事件总数", ["event"])


def current_request_id() -> str:
    return request_id_var.get()


class RequestIdMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next: Callable) -> Response:  # noqa: ANN001
        rid = request.headers.get("x-request-id") or uuid.uuid4().hex[:16]
        token = request_id_var.set(rid)
        HTTP_INFLIGHT.inc()
        started = time.perf_counter()
        try:
            response = await call_next(request)
        finally:
            HTTP_INFLIGHT.dec()
            request_id_var.reset(token)
        elapsed = time.perf_counter() - started
        # 归一化路径：把 id 段折叠成 {id}，避免标签基数爆炸
        path = _normalize(request.url.path)
        status = str(response.status_code)
        HTTP_REQUESTS.labels(request.method, path, status).inc()
        HTTP_LATENCY.labels(request.method, path).observe(elapsed)
        response.headers["X-Request-ID"] = rid
        return response


def _normalize(path: str) -> str:
    parts = []
    for segment in path.split("/"):
        if segment and not segment.isalpha() and any(ch.isdigit() for ch in segment):
            parts.append("{id}")
        else:
            parts.append(segment)
    return "/".join(parts)[:120]


async def metrics_endpoint() -> Response:
    payload = generate_latest()
    return Response(content=payload, media_type=CONTENT_TYPE_LATEST)


class JsonFormatter(logging.Formatter):
    """结构化日志：level/name/msg/request_id 一行 JSON。"""

    def format(self, record: logging.LogRecord) -> str:
        out = {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(record.created)),
            "level": record.levelname,
            "logger": record.name,
            "msg": record.getMessage(),
        }
        rid = current_request_id()
        if rid:
            out["request_id"] = rid
        if record.exc_info:
            out["exc"] = self.formatException(record.exc_info)
        return json.dumps(out, ensure_ascii=False)


def install_json_logs(level: int = logging.INFO) -> None:
    handler = logging.StreamHandler()
    handler.setFormatter(JsonFormatter())
    root = logging.getLogger()
    root.handlers = [handler]
    root.setLevel(level)
    for noisy in ("uvicorn.access",):
        logging.getLogger(noisy).handlers = []
        logging.getLogger(noisy).propagate = True


__all__ = [
    "EVENTS_PUBLISHED",
    "HTTP_INFLIGHT",
    "HTTP_LATENCY",
    "HTTP_REQUESTS",
    "MESSAGES_TOTAL",
    "RequestIdMiddleware",
    "WS_CONNECTIONS",
    "current_request_id",
    "install_json_logs",
    "metrics_endpoint",
]
