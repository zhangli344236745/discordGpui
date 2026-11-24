# moderation Specification

## Purpose

定义服务器内的自动审核、消息频率限制与审核日志。它保证在服务端而不是客户端执行拦截，并为管理员提供可追溯的操作记录。

## Requirements

### Requirement: 自动审核规则管理

系统 SHALL 支持自动审核规则的创建、启停、改动作、改关键词与删除。每条规则 MUST 属于一个服务器，动作限定为 `block` 或 `warn`，关键词 MUST 至少一个且去重。变更后 MUST 广播对应 `AUTOMOD_RULE_*` 事件。

#### Scenario: 新建规则

- **WHEN** 有 `MANAGE_SERVER` 的管理员创建 `action=block` 且含 3 个关键词的规则
- **THEN** 返回规则与关键词列表，规则对该服务器后续发送立即生效

#### Scenario: 空关键词

- **WHEN** 创建或编辑规则时关键词数组为空
- **THEN** 系统返回 400 `VALIDATION_ERROR`，规则不落库

#### Scenario: 停用规则

- **WHEN** 管理员把规则 `enabled` 改为 false
- **THEN** 该规则不再参与扫描，历史命中记录不变

#### Scenario: 无权限管理

- **WHEN** 无 `MANAGE_SERVER` 的成员调用规则写接口
- **THEN** 系统返回 403 `MISSING_PERMISSION`

### Requirement: 发送时的自动审核

系统 SHALL 在频道消息创建与编辑时执行自动审核扫描。命中 `block` MUST 返回 422 `AUTOMOD_BLOCKED` 且消息不落库；命中 `warn` MUST 正常落库并在响应中返回 `warnings[]`。私信消息不参与服务器规则扫描。

#### Scenario: 拦截命中

- **WHEN** 消息正文包含被 block 规则命中的关键词
- **THEN** 返回 422，附带 `rule_id` 与 `matched_keyword`，消息表与广播中都不出现该消息

#### Scenario: 警告命中

- **WHEN** 消息只命中 warn 规则
- **THEN** 消息正常创建，响应额外包含 `warnings`，其他成员正常收到 `MESSAGE_CREATE`

#### Scenario: 编辑命中

- **WHEN** 用户把已发布消息编辑为含 block 关键词的正文
- **THEN** 返回 422，原正文与 `version` 保持不变

#### Scenario: 私信不受服务器规则影响

- **WHEN** 私信正文包含某服务器的 block 关键词
- **THEN** 私信正常发送

### Requirement: 自动审核试跑

系统 SHALL 提供只读试跑接口，返回一段文本命中的规则与关键词，MUST NOT 写库、MUST NOT 广播。

#### Scenario: 试跑命中

- **WHEN** 管理员提交包含两个规则关键词的文本
- **THEN** 返回全部命中项（含动作与关键词），消息与审核日志均无新增

### Requirement: 审核日志查询与清空

系统 SHALL 按服务器保存审核日志，支持按 `kind` 过滤与游标分页，返回按时间倒序且包含操作者与描述。仅服务器所有者可以清空日志；清空 MUST 是硬删除。

#### Scenario: 过滤查询

- **WHEN** 管理员携带 `kind=role&limit=50&before=<id>`
- **THEN** 只返回角色类日志，`has_more` 正确反映是否还有更早记录

#### Scenario: 非所有者清空

- **WHEN** 非 owner 调用 `DELETE /servers/{id}/audit`
- **THEN** 系统返回 403 `MISSING_PERMISSION`，日志保留

#### Scenario: 所有者清空

- **WHEN** 所有者清空日志
- **THEN** 该服务器日志全部删除，其他服务器不受影响

#### Scenario: 日志不可变

- **WHEN** 客户端尝试通过任何端点修改单条日志
- **THEN** 系统不提供该能力，日志内容仅在创建时写入

### Requirement: 管理操作的审计留痕

系统 SHALL 在服务器、频道、角色、成员、邀请、消息删除、加成等管理写操作成功时写入一条审核日志，并广播 `AUDIT_ENTRY_CREATE`。审核写入 MUST 与业务数据处于同一事务。

#### Scenario: 角色变更留痕

- **WHEN** 管理员删除一个角色
- **THEN** 审计日志新增一条 `kind=role` 记录，包含操作者与角色名，实时推送给设置页

#### Scenario: 事务回滚

- **WHEN** 管理操作在写日志后因约束失败回滚
- **THEN** 审核日志一并回滚，不出现「有日志无变更」

### Requirement: 消息发送频率限制

系统 SHALL 对消息与登录接口执行频率限制。超限 MUST 返回 429 `RATE_LIMITED` 并给出 `retry_after` 秒数；限制窗口 MUST 按用户（消息）与来源（登录）独立计数。

#### Scenario: 消息超限

- **WHEN** 同一用户在 5 秒内发送第 6 条消息
- **THEN** 返回 429，`retry_after` 为正整数，第 6 条不落库

#### Scenario: 窗口恢复

- **WHEN** 超过限制窗口后再次发送
- **THEN** 请求成功，计数重新开始

#### Scenario: 登录超限

- **WHEN** 同一来源 1 分钟内第 11 次尝试登录
- **THEN** 返回 429，且不进行密码校验
