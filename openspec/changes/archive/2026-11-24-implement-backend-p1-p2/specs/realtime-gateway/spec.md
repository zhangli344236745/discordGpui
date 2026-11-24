# Spec Delta

## Purpose

定义实时 Gateway 的对外契约：连接鉴权、订阅范围、30 类下行事件、心跳、上行命令白名单与断线补偿。它保证客户端不会收到越权事件，并能在短暂断网后补齐状态。

## ADDED Requirements

### Requirement: 连接鉴权与订阅范围

系统 SHALL 只允许持有效 access token 的用户建立 `/ws/gateway` 连接，并在连接时把订阅范围初始化为其加入的服务器与参与的 DM。所有下行事件 MUST 按订阅范围过滤，跨服务器或非参与 DM 的事件不得投递。

#### Scenario: 无效 token

- **WHEN** 客户端携带过期或伪造 token 连接
- **THEN** 服务端以 4001 关闭连接，不发送 `READY`

#### Scenario: 首次连接

- **WHEN** 合法用户连接成功
- **THEN** 收到 `READY`，包含用户摘要、服务器 id 列表、DM id 列表与 30 秒心跳间隔

#### Scenario: 修改订阅

- **WHEN** 客户端发送 `SUBSCRIBE` 只保留部分服务器
- **THEN** 后续事件只推送保留范围内的内容，未订阅服务器的消息不再到达

#### Scenario: 越权订阅

- **WHEN** 客户端 `SUBSCRIBE` 包含自己未加入的服务器 id
- **THEN** 服务端忽略该 id，连接仍保持可用

#### Scenario: DM 事件隔离

- **WHEN** 某个 DM 产生新消息
- **THEN** 只有该 DM 的两位参与者收到 `MESSAGE_CREATE`，其他订阅者收不到

### Requirement: 事件目录与信封格式

系统 SHALL 使用统一的 JSON 信封 `{event, id, ts, data}`，并覆盖文档第 4.1 节的 30 类事件。事件名 MUST 使用大写常量；未知事件 MUST 被客户端忽略而不影响连接。

#### Scenario: 信封字段

- **WHEN** 任意事件下行
- **THEN** `event` 为字符串、`id` 为可排序字符串、`ts` 为 ISO 8601 UTC、`data` 为对象

#### Scenario: 事件覆盖

- **WHEN** 依次触发消息、反应、频道、成员、角色、邀请、私信、通知、语音事件
- **THEN** 每个领域都产生文档中对应的事件名，无拼写偏差

#### Scenario: 未知事件

- **WHEN** 服务端推送客户端未实现的事件
- **THEN** 客户端忽略即可，连接不中断（服务端不依赖客户端确认）

### Requirement: 心跳与超时断开

系统 SHALL 支持 `PING/PONG` 心跳，期望间隔 30 秒。连续 2 个周期未收到任何客户端消息 MUST 断开连接；token 在连接期间过期 MUST 在下一个心跳周期关闭连接并返回 4001。

#### Scenario: 正常心跳

- **WHEN** 客户端每 30 秒发送 `PING`
- **THEN** 收到 `PONG`，连接保持

#### Scenario: 心跳丢失

- **WHEN** 客户端连续 60 秒无任何上行消息
- **THEN** 服务端关闭连接并清理订阅，不残留连接计数

#### Scenario: 连接期间 token 过期

- **WHEN** access token 到期后客户端继续发送 `PING`
- **THEN** 服务端以 4001 关闭连接，客户端应刷新 token 后重连

### Requirement: 断线补偿

系统 SHALL 支持 `?since=<event_id>` 重连，按事件 id 顺序补发该游标之后、且属于当前订阅范围的事件。补偿窗口 MUST 为 24 小时；游标早于窗口或不可解析时 MUST 退化为重新发送 `READY`。

#### Scenario: 补发遗漏事件

- **WHEN** 客户端携带最后收到的 event id 重连
- **THEN** 只收到该 id 之后的事件，顺序与产生顺序一致，不重复

#### Scenario: 游标过期

- **WHEN** `since` 早于 24 小时保留窗口
- **THEN** 服务端发送 `READY` 让客户端全量重建，不返回错误

#### Scenario: 越权补偿

- **WHEN** 补发范围内包含用户已退出服务器的历史事件
- **THEN** 这些事件被过滤，不投递

#### Scenario: 保留清理

- **WHEN** 事件超过 24 小时
- **THEN** 后台任务删除，补偿查询不再返回

### Requirement: 上行命令白名单

系统 SHALL 只接受 `SUBSCRIBE`、`TYPING`、`VOICE_STATE` 与 `PING` 四种上行命令。其他命令 MUST 返回 `ERROR` 事件且不断开连接；`TYPING` MUST 校验频道可见性与 `SEND_MESSAGES`。

#### Scenario: 非法命令

- **WHEN** 客户端发送 `MESSAGE_SEND`
- **THEN** 收到 `ERROR`，连接保持，消息不落库

#### Scenario: 非法 JSON

- **WHEN** 客户端发送无法解析的文本
- **THEN** 收到 `ERROR`，连接保持

#### Scenario: 无权输入状态

- **WHEN** 无 `SEND_MESSAGES` 的用户对某频道发送 `TYPING`
- **THEN** 不广播 `TYPING_START`，仅向发送者返回 `ERROR`

### Requirement: 事件与 REST 变更的一致性

系统 SHALL 保证每个成功的写操作产生与其语义匹配的实时事件，且 `client_nonce` 幂等重试只产生一次事件。事件 MUST 在数据库事务提交后发出，避免客户端看到未落库的数据。

#### Scenario: 发送消息

- **WHEN** `POST /channels/{id}/messages` 成功
- **THEN** 同一服务器订阅者各收到一次 `MESSAGE_CREATE`

#### Scenario: 幂等重试不重复广播

- **WHEN** 客户端用相同 `client_nonce` 重试发送
- **THEN** 返回首次结果，订阅者不收到第二条 `MESSAGE_CREATE`

#### Scenario: 事务失败

- **WHEN** 写操作在提交前失败
- **THEN** 不广播任何事件
