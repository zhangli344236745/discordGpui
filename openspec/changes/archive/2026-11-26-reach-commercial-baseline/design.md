# Design

## Context

现有系统已具备功能演示基线：

- 后端 166 个可用端点、51 张表、30 类实时事件、自动审核、权限矩阵、搜索、通知等。
- 客户端已接登录、首屏同步、实时消息、私信、治理、论坛、日程、书签、通知与搜索。
- 运行形态仍是单机 SQLite + 本地磁盘 + 单 worker + Demo 语音状态。

与 Discord 商用基线的主要差距集中在：部署扩展、账号安全、真实媒体、真实音视频、机器人生态、举报合规、客户端可靠性与运维可观测。

## Goals / Non-Goals

**Goals:**

- 以可验证分期把系统推进到「可对外试点运营」的最低标准。
- 每期不破坏现有开发体验：默认 SQLite/本地磁盘/Demo 语音仍可运行。
- 每期都有自动化验收：`ruff/pytest`、`cargo check/test`、客户端 e2e、部署 smoke。

**Non-Goals:**

- 不一次性追平 Discord 全部产品面（Nitro 计费细节、应用市场、手机端、直播、大型活动等另行分期）。
- 不引入微服务拆分；先保持单体 + 可插拔基础设施。
- 不做端到端加密与联邦协议。
- 不承诺真实生产 SLA，仅建立可运营基础。

## Decisions

### D1. 分期策略：P0 → P7

| 期 | 主题 | 交付 |
| --- | --- | --- |
| P0 | 部署基线 | Docker Compose、Postgres/Redis/MinIO 可选、配置模板 |
| P1 | 身份安全 | 邮件、验证码、重置、2FA、安全审计 |
| P2 | 媒体管线 | 客户端真实上传、类型校验、缩略图、配额 |
| P3 | 真实语音 | LiveKit SDK、采集渲染、设备控制、重连 |
| P4 | 机器人 | Bot、Webhook、斜杠命令、应用令牌 |
| P5 | 合规 | 举报、处理流、保留策略、数据导出/删除 |
| P6 | 客户端可靠性 | 安装包、离线缓存、弱网重连、性能预算 |
| P7 | 运维 | 指标、日志、CI/CD、备份、压测 |

每期独立可合并，禁止把后续期依赖混入当前期。

### D2. 基础设施可插拔

- 数据库：保留 SQLAlchemy 抽象，新增 `DATABASE_PROVIDER`。SQLite 为默认，Postgres 通过 URL 切换；迁移必须双后端可执行。
- 广播：抽象 `EventBus`，默认内存实现，生产 Redis Pub/Sub。
- 存储：抽象 `Storage`，默认本地磁盘，生产 S3/MinIO。
- 邮件：抽象 `Mailer`，测试模式写库/日志，生产 SMTP 或云服务。
- 语音：抽象 `VoiceProvider`，Demo 状态实现保留，生产 LiveKit。

理由：避免开发环境被迫安装全部基础设施，同时让生产部署可替换。

### D3. 部署形态

- `docker-compose.yml` 提供 `api`、`postgres`、`redis`、`minio`、可选 `livekit`。
- `.env.production.example` 列出必需变量；启动前做配置校验。
- API 仍为单体，但通过 Redis 广播支持多实例。
- 备份脚本分别覆盖 Postgres 与 MinIO；恢复脚本在空容器验证。

### D4. 安全基线

- 邮件验证与重置使用一次性 token，存哈希、设有效期。
- 2FA 使用 TOTP；恢复码仅显示一次，存哈希。
- 所有安全事件进入 `security_audit`，不记录明文敏感信息。
- 限流改为可持久化实现，默认内存，生产 Redis。

### D5. 媒体链路

- 客户端新增真实文件选择器，上传走现有 `/uploads` 形状。
- 后端增加 MIME sniff、大小校验、图片缩略图、配额表。
- 附件引用与消息发送解耦：先上传拿 file_id，再发送引用。
- 孤儿清理任务保留现有逻辑，扩展对象存储删除。

### D6. 真实语音

- 客户端引入 LiveKit Rust SDK。
- 服务端已有 token 签发与 webhook；补房间生命周期治理与参与者状态。
- UI 保留现有面板，但将 Demo 状态替换为真实 Room 状态。
- 服务器侧静音/移除走 RoomService。

### D7. 机器人体系

- 新增 `bots`、`bot_tokens`、`webhooks`、`slash_commands`。
- Bot 通过独立 Bearer token 调用 REST；Webhook 使用签名。
- 斜杠命令先支持内置命令与 Webhook 转发，不做完整应用市场。

### D8. 合规与运维

- 举报状态机：pending → reviewing → resolved/rejected/escalated。
- 保留策略按表配置，法务保留优先。
- 指标使用 `prometheus-fastapi-instrumentator` 或等价库。
- CI 用 GitHub Actions；压测用 `locust` 或 `k6`。
- 日志 JSON 化，注入 `request_id`。

## Risks / Trade-offs

- **范围大**：必须按 P0–P7 严格分期；每期完成即归档或至少独立验收。
- **基础设施依赖**：默认 Demo 不变，生产通过环境变量启用，避免开发阻塞。
- **LiveKit 客户端接入复杂**：先服务器语音频道，后私信通话；Demo provider 保留为回退。
- **Postgres 迁移差异**：所有迁移在双后端测试，禁止 SQLite-only SQL。
- **客户端打包**：跨平台安装包工作量大，先 macOS，再 Linux/Windows。
- **性能预算**：先建立测量脚本，再优化，不凭感觉改代码。

## Migration Plan

1. P0 不改业务行为，只新增部署与抽象层；现有测试必须全绿。
2. P1 引入安全表与邮件抽象；已有账号不受影响，未验证账号仅功能受限。
3. P2 扩展 uploads，不破坏旧附件读取。
4. P3 引入 LiveKit feature flag；未配置时回退 Demo 语音。
5. P4–P7 均新增表与配置，不做破坏性变更。
6. 回滚：每期单独分支与镜像版本，配置可切回旧实现。

## Open Questions

- 商用目标地区的合规要求（GDPR/CCPA 之外）需法务确认。
- 是否需要手机端与 Web 端，属于后续独立 change。
- 商用计费/Nitro 是否纳入本期，暂不纳入。
