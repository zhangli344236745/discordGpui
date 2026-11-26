# commercial-operations Specification

## Purpose

建立商用运维基线：暴露请求量、延迟、错误率与实时连接数等 Prometheus 指标，输出带 request_id
与 trace 关联的结构化日志，提供 CI 流水线与可回滚的镜像发布，配套数据库与对象存储的备份恢复、
压力测试脚本和容量基线，使服务可监控、可回滚并支持容量规划。

## Requirements

### Requirement: 指标暴露

系统 SHALL 暴露 Prometheus 指标，包括请求量、延迟、错误率、WS 连接数、消息吞吐与队列积压。

#### Scenario: 抓取指标

- **WHEN** Prometheus 抓取 metrics 端点
- **THEN** 得到上述核心指标

#### Scenario: 告警阈值

- **WHEN** 错误率超过阈值
- **THEN** 提供可配置的告警规则示例

### Requirement: 结构化日志与追踪

系统 SHALL 输出结构化日志并携带 request_id。关键链路 MUST 支持分布式追踪。

#### Scenario: 请求追踪

- **WHEN** 查看某请求日志
- **THEN** 可通过 request_id 串起网关、业务与数据库日志

#### Scenario: 敏感信息脱敏

- **WHEN** 日志输出请求
- **THEN** token 与密码不出现

### Requirement: CI/CD

项目 SHALL 提供 CI 流水线，自动执行 lint、测试、构建、镜像与部署演练。主干 MUST 保持绿色。

#### Scenario: 提交触发

- **WHEN** 推送到主干
- **THEN** CI 自动运行全部检查并发布镜像

#### Scenario: 失败阻断

- **WHEN** 任一检查失败
- **THEN** 流水线标记失败且不部署

### Requirement: 压测与容量

系统 SHALL 提供压测脚本与基线报告，包含并发连接、消息吞吐与附件上传容量。

#### Scenario: 消息压测

- **WHEN** 运行消息压测脚本
- **THEN** 输出 P95 延迟与最大稳定吞吐

#### Scenario: 扩容验证

- **WHEN** 增加实例数量
- **THEN** 吞吐按预期提升且无事件丢失
