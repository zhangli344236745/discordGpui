# commercial-deployment Specification

## Purpose

定义系统在真实部署环境中的扩展、配置、备份与恢复能力，使服务可以从单机 Demo 升级为可运营的多实例部署。

## Requirements

### Requirement: 数据库后端可切换

系统 SHALL 支持通过配置在 SQLite 与 Postgres 之间切换，且不改业务代码。生产配置 MUST 使用 Postgres；开发默认 SQLite。迁移 MUST 在两种后端都可执行。

#### Scenario: 切换到 Postgres

- **WHEN** 设置 `DATABASE_URL=postgresql+asyncpg://...`
- **THEN** 应用启动、迁移、接口行为与 SQLite 一致

#### Scenario: 开发保持默认

- **WHEN** 未设置数据库配置
- **THEN** 使用本地 SQLite，不要求外部服务

### Requirement: 多实例广播

系统 SHALL 通过 Redis Pub/Sub 或等价机制在多个 worker 间广播 Gateway 事件。单实例部署 MUST 继续可用。

#### Scenario: 双实例收消息

- **WHEN** 实例 A 的用户发消息，实例 B 的订阅者在线
- **THEN** B 在 2 秒内收到 `MESSAGE_CREATE`

#### Scenario: 实例故障

- **WHEN** 其中一个实例停止
- **THEN** 其他实例的实时事件不受影响

### Requirement: 对象存储可切换

附件 SHALL 支持本地磁盘与 S3/MinIO 兼容对象存储两种后端。生产 MUST 使用对象存储；开发默认本地。

#### Scenario: 上传到 MinIO

- **WHEN** 配置 S3 endpoint 并上传附件
- **THEN** 返回可下载 URL，文件存储在对象存储

#### Scenario: 本地开发

- **WHEN** 未配置对象存储
- **THEN** 附件落本地磁盘，接口形状不变

### Requirement: 容器化部署

系统 SHALL 提供一键启动的容器编排，包含 API、数据库、Redis、对象存储与可选 LiveKit。配置 MUST 通过环境变量注入，不含硬编码密钥。

#### Scenario: 一键启动

- **WHEN** 执行 `docker compose up`
- **THEN** 健康检查通过，示例用户可登录并收发消息

#### Scenario: 配置校验

- **WHEN** 缺少必需环境变量
- **THEN** 容器启动失败并输出可读错误

### Requirement: 备份与恢复

系统 SHALL 提供数据库与附件的备份命令与恢复演练脚本。备份 MUST 可在空环境恢复出可用服务。

#### Scenario: 备份恢复

- **WHEN** 执行备份后在新环境恢复
- **THEN** 用户、服务器、消息与附件可访问

#### Scenario: 定时备份

- **WHEN** 启用调度
- **THEN** 备份按策略生成并保留指定天数
