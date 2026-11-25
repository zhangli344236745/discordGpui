# 生产部署与运维手册

本手册覆盖 `reach-commercial-baseline`（商用基线）交付的部署、扩展、备份与观测能力。
开发默认仍是单机 SQLite + 本地磁盘 + 内存广播，所有生产增强按环境变量启用。

## 1. 部署形态

### 单机开发（默认）

```bash
cd server
uv run alembic upgrade head
uv run uvicorn app.main:app --port 8080
```

### Docker Compose（Postgres + Redis + API）

```bash
docker compose up -d --build          # API 暴露在 127.0.0.1:8081
./scripts/compose-smoke.sh            # 注册→建服→发消息→附件→下载 全链路
```

### 对象存储（MinIO / S3 兼容）

```bash
docker compose -f docker-compose.yml \
  -f docker-compose.object-storage.yml \
  --profile object-storage up -d
```

- 容器内写 `minio:9000`，预签名下载 URL 用 `S3_PUBLIC_ENDPOINT`（客户端可达地址）。
- 未配置 `S3_*` 时自动回退本地磁盘，接口形状不变。

### 环境变量

见 [.env.production.example](../.env.production.example)。要点：

| 变量 | 作用 |
| --- | --- |
| `DATABASE_URL` | SQLite（默认）或 `postgresql+asyncpg://…` |
| `REDIS_URL` | 为空=内存广播；配置后多实例 Pub/Sub |
| `S3_ENDPOINT` / `S3_PUBLIC_ENDPOINT` | 对象存储内网 / 外网地址 |
| `MAIL_BACKEND` | `log`（默认，写日志+进程 outbox）或 `smtp` |
| `UPLOAD_USER_QUOTA_BYTES` | 单用户附件配额，0=不限 |
| `JWT_SECRET` | 生产必须替换 |

## 2. 水平扩展

- 多实例：每个实例 `REDIS_URL` 指向同一 Redis；事件经 Pub/Sub 广播，控制消息（会话撤销踢 Gateway）跨实例生效。
- 验证：`server/scripts/multi-instance-smoke.sh`（实例 A 发消息，实例 B 的订阅者 2 秒内收到）。
- Postgres 迁移与 SQLite 同源；FTS5（SQLite）与 pg_trgm（Postgres）在迁移里按方言分支。

## 3. 备份与恢复

```bash
scripts/backup.sh [输出目录]        # DB + 附件 → backups/discord-backup-<时间>.tar.gz
scripts/restore.sh <包> --yes      # 覆盖式恢复
scripts/backup-smoke.sh            # 建库→备份→清空→恢复→校验 演练
```

定时调度示例（每天 03:00 备份，保留 7 天）：

```cron
0 3 * * * cd /opt/discord && BACKUP_KEEP_DAYS=7 scripts/backup.sh >> backups/backup.log 2>&1
```

## 4. 观测

- `GET /metrics`：Prometheus 格式。核心序列：`dgpui_http_requests_total`（含状态码）、
  `dgpui_http_request_seconds`（P95 直方图）、`dgpui_gateway_connections`、
  `dgpui_messages_total`、`dgpui_events_published_total`、`dgpui_http_inflight`。
- 日志：一行 JSON（`ts/level/logger/msg/request_id`）；请求 ID 从 `X-Request-ID` 透传或自动生成，并写回响应头。
- 告警规则示例：

```yaml
- alert: HighErrorRate
  expr: sum(rate(dgpui_http_requests_total{status=~"5.."}[5m])) / sum(rate(dgpui_http_requests_total[5m])) > 0.02
  for: 10m
- alert: GatewayDropping
  expr: dgpui_gateway_connections < 1
  for: 30m
```

## 5. 容量基线（Apple Silicon / x86 单机，2026-11）

`server/scripts/loadtest.py --users 10 --messages 100`：

| 指标 | 结果 |
| --- | --- |
| 注册（argon2） | p50 ≈ 650ms（密码哈希成本，符合预期） |
| 消息 REST | p50 ≈ 78ms / p95 ≈ 356ms |
| Gateway 扇出 | p95 ≈ 0ms（广播在响应返回前已送达） |
| 吞吐 | ≈ 14.5 msg/s（受默认用户限流 5 条/5 秒约束，属设计上限而非瓶颈） |

提高吞吐：调大 `RATE_LIMIT_MESSAGES` / 多实例 + Postgres 后重跑。

## 6. CI

[.github/workflows/ci.yml](../.github/workflows/ci.yml)：
后端（ruff + pytest + 迁移回滚）、客户端（cargo check + test，macOS）、
Compose 与对象存储 override 配置校验。
