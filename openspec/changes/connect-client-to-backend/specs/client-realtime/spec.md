# Spec Delta

## Purpose

定义客户端实时通道与消息写路径：WebSocket 生命周期、断线补偿、事件应用，以及发送/编辑/删除/回应/已读/输入中的端到端一致性。

## ADDED Requirements

### Requirement: Gateway 连接与订阅

客户端 SHALL 使用 access token 连接 `/ws/gateway`，收到 `READY` 后进入在线状态，并只订阅当前账号可见的 server/dm。连接 MUST 每 30 秒发送 `PING`，连续两次无 `PONG` 或连接断开时进入重连。

#### Scenario: 建立连接

- **WHEN** 登录同步完成
- **THEN** 客户端连接 Gateway 并收到 `READY`，顶部显示在线状态

#### Scenario: 心跳丢失

- **WHEN** 连续 60 秒没有收到任何下行消息且 `PING` 无响应
- **THEN** 客户端关闭连接、显示「重连中」并按退避重试

#### Scenario: 鉴权失败

- **WHEN** Gateway 以 4001 关闭
- **THEN** 客户端刷新 token 后重连；刷新失败则退出登录

### Requirement: 断线补偿与事件顺序

重连时客户端 SHALL 携带最后收到的 event id 作为 `?since`。收到的事件 MUST 按 id 去重、按到达顺序应用；服务端返回 `READY` 时 MUST 用它重建状态而不是追加。

#### Scenario: 补发遗漏

- **WHEN** 断线期间产生了 3 条消息，重连带最后 event id
- **THEN** 3 条消息按原顺序出现，无重复、无丢失

#### Scenario: 游标过期

- **WHEN** 重连收到 `READY` 而非补发
- **THEN** 客户端清空当前会话缓存并按 `READY`/重新拉取结果重建

#### Scenario: 重复事件

- **WHEN** 同一 event id 因重连被投递两次
- **THEN** 第二次被忽略，消息与角标不重复增加

### Requirement: 消息发送与乐观更新

发送消息时客户端 SHALL 立即插入「发送中」消息并生成 `client_nonce`，成功后用服务端 id 替换临时项，失败则标记失败并允许重试。重试 MUST 复用同一 nonce。

#### Scenario: 发送成功

- **WHEN** 用户在频道发送文本
- **THEN** 消息立即出现为「发送中」，收到响应或 `MESSAGE_CREATE` 后变为已发送并带服务端 id

#### Scenario: 发送失败

- **WHEN** 请求返回 4xx/5xx 或超时
- **THEN** 消息显示失败状态与重试按钮，正文保留

#### Scenario: 重试幂等

- **WHEN** 用户点击失败消息的重试
- **THEN** 使用原 nonce 重发，服务端只保留一条

### Requirement: 消息编辑、删除与回应

客户端 SHALL 将编辑/删除/回应/置顶/已读操作发送到对应端点，并在成功响应或实时事件到达时更新 UI。编辑 MUST 展示编辑历史；删除 MUST 显示墓碑或移除，取决于当前视图。

#### Scenario: 编辑

- **WHEN** 作者编辑自己的消息成功
- **THEN** 正文更新并显示「已编辑」，编辑历史可查看旧版本

#### Scenario: 他人删除

- **WHEN** 收到 `MESSAGE_DELETE`
- **THEN** 消息从列表移除/变墓碑，引用它的回复显示原消息已删除

#### Scenario: 回应

- **WHEN** 用户点击表情回应
- **THEN** 计数与「我的回应」状态立即更新，失败时回滚

### Requirement: 已读与输入中

客户端 SHALL 在打开/聚焦会话时上报已读位置，并在输入时发送 `TYPING`（HTTP 或 Gateway）。收到的 `TYPING_START` MUST 在数秒后自动消失。

#### Scenario: 已读上报

- **WHEN** 用户打开有未读的频道
- **THEN** 客户端上报最后一条可见消息 id，角标清零

#### Scenario: 输入提示

- **WHEN** 另一位成员正在输入
- **THEN** 输入区上方显示「X 正在输入」，4 秒无新事件后消失

### Requirement: 实时事件 reducer 的覆盖

客户端 SHALL 处理文档第 4.1 节的 30 类事件：消息、回应、输入、频道/分类、服务器、成员、角色、权限、邀请、审核、在线状态、好友、分组、私信、消息请求、话题、投票、论坛、日程、加成、欢迎、通知、语音、呼叫、书签。未知事件 MUST 忽略且不断开连接。

#### Scenario: 成员加入

- **WHEN** 收到 `MEMBER_JOIN`
- **THEN** 成员栏出现该用户，服务器成员数加一

#### Scenario: 权限变更

- **WHEN** 收到 `PERMISSIONS_UPDATE`
- **THEN** 客户端重新拉取当前服务器频道树与权限，隐藏无权频道

#### Scenario: 未知事件

- **WHEN** 服务端新增客户端未识别的事件
- **THEN** 客户端忽略该事件，连接与现有状态不受影响
