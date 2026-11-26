# Proposal

## Why

当前项目是「功能演示级」而非「商用级」：接口与界面基本齐全，但身份、媒体、语音、扩展与运维仍按单机 Demo 设计。与 Discord 的商用基线相比，缺少真实音视频、持久化邮件流程、可横向扩展部署、对象存储、机器人/斜杠命令、举报与合规、观测与备份等。需要一份可验证的分期计划，把现有实现逐步推进到可对外运营的最低标准，而不是一次性追平全部 Discord 功能。

## What Changes

- **部署与扩展**：Postgres/Redis/MinIO 可选后端、多 worker 广播、容器化编排、配置与环境分离。
- **身份与安全**：邮箱验证与找回密码真实发信、会话管理、可选 2FA、审计不可篡改、限流持久化。
- **媒体管线**：客户端真实附件上传（本地 → 对象存储可选）、预览、配额、孤儿清理与病毒扫描接口预留。
- **真实语音视频**：接入 LiveKit 房间生命周期、客户端音频/视频/屏幕共享、设备控制与弱网重连。
- **机器人与集成**：Bot 账号、Webhook、斜杠命令、应用令牌与最小审批流。
- **举报与合规**：用户举报、内容保留策略、数据导出/删除、审计导出。
- **客户端可靠性**：安装包、离线缓存、多设备状态同步、弱网重连、性能预算。
- **运维可观测**：指标、结构化日志、追踪、备份恢复、CI/CD、压测脚本。

**BREAKING**：无。所有增强按配置启用，默认保持当前单机 SQLite/本地磁盘模式，便于开发与测试。

## Capabilities

### New Capabilities

- `commercial-deployment`: 可部署、可扩展、可备份的服务形态（Postgres/Redis/MinIO/容器/多 worker）。
- `commercial-identity`: 邮箱验证、找回密码、2FA、会话与令牌撤销等账号安全基线。
- `commercial-media`: 真实附件上传、对象存储、预览与配额。
- `commercial-voice`: LiveKit 真实音视频与屏幕共享端到端链路。
- `commercial-bots`: Bot 账号、Webhook、斜杠命令与应用令牌。
- `commercial-moderation`: 举报、内容保留、审计导出与合规删除。
- `commercial-client-resilience`: 客户端打包、离线缓存、弱网重连与多设备一致性。
- `commercial-operations`: 指标、日志、追踪、备份、CI/CD 与压测。

### Modified Capabilities

（无：现有 14 份主规格继续保持，作为功能基线；本变更只补商用缺口。）

## Impact

- **代码**：`server/`（部署配置、存储抽象、邮件服务、LiveKit 接入、Bot/Webhook、举报、指标）、`src/net|ui/`（真实上传、语音、离线缓存、打包）。
- **依赖**：新增可选依赖（Postgres/Redis/MinIO/LiveKit SDK/邮件库/指标库），均按 feature flag 启用。
- **数据**：新增表（bots、webhooks、commands、reports、retention、audit_export、media_quota、push_subscriptions 等），Alembic 迁移。
- **部署**：新增 Docker Compose 与 `.env.production` 模板，不改现有开发启动方式。
- **风险**：范围极大，必须按分期落地；每期保持 `cargo check/test`、`ruff/pytest`、客户端 e2e 全绿。
