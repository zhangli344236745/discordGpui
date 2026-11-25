# Spec Delta

## Purpose

建立最小可用的机器人与集成体系：Bot 账号、Webhook、斜杠命令与应用令牌，让第三方可以安全接入服务器。

## ADDED Requirements

### Requirement: Bot 账号

系统 SHALL 支持创建 Bot 账号并授予服务器角色。Bot 的消息 MUST 标识来源，不可冒充普通用户。

#### Scenario: 创建 Bot

- **WHEN** 服务器管理员创建 Bot 并邀请入服
- **THEN** Bot 出现在成员列表并拥有指定角色

#### Scenario: Bot 发消息

- **WHEN** Bot 通过 API 发消息
- **THEN** 客户端显示 BOT 标记

### Requirement: Webhook

服务器 SHALL 支持创建 Webhook，外部系统向 Webhook URL POST 后 MUST 在指定频道产生消息。Webhook MUST 支持撤销与签名校验。

#### Scenario: 外部推送

- **WHEN** 向 Webhook 提交合法请求
- **THEN** 指定频道出现 Webhook 消息

#### Scenario: 撤销 Webhook

- **WHEN** 管理员撤销
- **THEN** 后续请求返回 404，不再产生消息

### Requirement: 斜杠命令

系统 SHALL 支持注册服务器级斜杠命令，客户端输入 `/` MUST 显示可用命令并回传执行结果。

#### Scenario: 执行命令

- **WHEN** 用户输入 `/help`
- **THEN** 客户端显示命令结果，消息来源标记为命令响应

#### Scenario: 无权限命令

- **WHEN** 用户缺少命令权限
- **THEN** 返回权限错误，不执行副作用

### Requirement: 应用令牌

Bot 与 Webhook MUST 使用独立令牌，可撤销、可限流，并支持最小权限范围。

#### Scenario: 撤销令牌

- **WHEN** 管理员撤销 Bot 令牌
- **THEN** 该 Bot 后续请求返回 401

#### Scenario: 令牌限流

- **WHEN** Bot 超过调用频率
- **THEN** 返回 429 并带重试时间
