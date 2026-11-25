"""P7 运维观测：/metrics、request_id、JSON 日志。"""

from httpx import AsyncClient

from tests.conftest import auth_headers, create_server, first_channel


async def test_metrics_endpoint_exposes_core_series(client: AsyncClient) -> None:
    headers = await auth_headers(client)
    server = await create_server(client, headers)
    channel = await first_channel(client, headers, server["id"])
    sent = await client.post(
        f"/api/v1/channels/{channel['id']}/messages",
        headers=headers,
        json={"content": "metrics probe", "client_nonce": "metrics-1"},
    )
    assert sent.status_code == 201

    resp = await client.get("/metrics")
    assert resp.status_code == 200
    assert "dgpui_http_request_seconds" in resp.text
    assert "dgpui_messages_total" in resp.text
    assert "dgpui_gateway_connections" in resp.text
    # REST 请求已计数且状态码维度正确
    assert 'status="201"' in resp.text


async def test_request_id_echoed_and_generated(client: AsyncClient) -> None:
    echoed = await client.get("/api/v1/health", headers={"X-Request-ID": "rid-test-42"})
    assert echoed.status_code == 200
    assert echoed.headers.get("x-request-id") == "rid-test-42"

    generated = await client.get("/api/v1/health")
    assert generated.headers.get("x-request-id")
    assert generated.headers["x-request-id"] != "rid-test-42"


async def test_traceparent_is_echoed_and_inherited(client: AsyncClient) -> None:
    incoming = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
    resp = await client.get("/api/v1/health", headers={"traceparent": incoming})
    assert resp.status_code == 200
    header = resp.headers.get("traceparent", "")
    parts = header.split("-")
    assert parts[0] == "00" and parts[1] == "4bf92f3577b34da6a3ce929d0e0e4736"
    assert len(parts[2]) == 16

    generated = (await client.get("/api/v1/health")).headers.get("traceparent", "")
    assert generated.startswith("00-") and len(generated.split("-")[1]) == 32


def test_parse_traceparent_rejects_malformed() -> None:
    from app.ops import parse_traceparent

    assert parse_traceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
    assert parse_traceparent("") is None
    assert parse_traceparent("01-abc-123-01") is None
    assert parse_traceparent("00-" + "0" * 32 + "-" + "0" * 16 + "-01") is None
