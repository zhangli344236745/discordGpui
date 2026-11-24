# direct-messaging Specification

## Purpose

定义一对一私信能力：DM 会话的规范化与复用、私信消息、陌生人的消息请求门控，以及私信语音/视频呼叫。它决定私信侧栏、聊天区和呼叫面板的数据来源。

## Requirements

### Requirement: DM 会话的规范化与复用

系统 SHALL 用有序用户对 `(user_a, user_b)`（`user_a < user_b`）唯一标识一个 DM 会话。任意一方重复发起会话 MUST 返回已存在的会话，而非创建新会话。已关闭的会话被重新发起时 MUST 重新打开。

#### Scenario: 双向发起复用同一会话

- **WHEN** A 向 B 发起 DM，随后 B 向 A 发起 DM
- **THEN** 两次返回同一个 `dm_id`，`dm_channels` 中只有一行

#### Scenario: 重新打开已关闭会话

- **WHEN** A 对 `closed=true` 的会话再次调用 `POST /dms`
- **THEN** 会话 `closed` 置为 false，历史消息保留

#### Scenario: 自我会话

- **WHEN** A 尝试与自己创建 DM
- **THEN** 系统返回 400 `VALIDATION_ERROR`

### Requirement: 私信消息的发送与分页

系统 SHALL 支持私信消息的游标分页、`client_nonce` 幂等、编辑留历史与软删。发送前 MUST 校验会话成员身份与屏蔽关系；非成员访问 MUST 返回 404。

#### Scenario: 幂等重试

- **WHEN** 客户端用同一个 `client_nonce` 重发私信
- **THEN** 系统返回首条消息，不新增记录，且只广播一次 `MESSAGE_CREATE`

#### Scenario: 非成员读取

- **WHEN** 用户 C 请求不属于自己的 DM 历史
- **THEN** 系统返回 404 `NOT_FOUND`

#### Scenario: 分页

- **WHEN** 客户端携带 `before=<message_id>&limit=50` 拉取历史
- **THEN** 返回按时间倒序的 `items` 与 `has_more`，不含已软删消息正文

### Requirement: 陌生人消息请求门控

当双方既非好友也无共同服务器时，首条私信 SHALL 进入消息请求而不是直接建立 DM。接收者接受后 MUST 创建或复用 DM 并投递首条消息；忽略或删除 MUST 不创建 DM。

#### Scenario: 陌生人首条消息

- **WHEN** 非好友 A 首次向 B 发送私信
- **THEN** 系统创建 `pending` 消息请求并保存 `first_message`，返回请求 id 而非消息 id

#### Scenario: 接受请求

- **WHEN** B 接受 A 的消息请求
- **THEN** 系统创建/复用 DM，把首条消息写入私信历史，二者收到 `DM_CREATE`

#### Scenario: 好友直接私信

- **WHEN** 已是好友的 A 向 B 发送私信
- **THEN** 直接写入 DM，不产生消息请求

#### Scenario: 请求列表筛选

- **WHEN** B 请求 `GET /message-requests?state=pending`
- **THEN** 只返回 pending 项并附带总数，已忽略/已删除项不出现

### Requirement: 私信已读与会话状态

系统 SHALL 支持按会话记录最后已读位置、置顶与关闭状态。标记已读 MUST 只前进不后退，并让该会话未读数归零。

#### Scenario: 标记已读

- **WHEN** B 打开会话并调用 `PATCH /dms/{id}/read`
- **THEN** 该会话未读数变为 0，其他会话不受影响

#### Scenario: 非参与者修改

- **WHEN** 非会话成员尝试置顶或关闭会话
- **THEN** 系统返回 404 `NOT_FOUND`

### Requirement: 私信呼叫状态机

系统 SHALL 为私信语音/视频呼叫维护 `ringing → active → ended` 状态，并只向通话双方广播 `CALL_RING/ACCEPT/DECLINE/END`。发起与接听 MUST 返回可加入 `dm:{call_id}` 房间的 LiveKit token。

#### Scenario: 发起呼叫

- **WHEN** A 在 DM 中发起 voice 呼叫
- **THEN** 创建 `ringing` 会话并返回 `{token, url, room}`，B 收到 `CALL_RING`

#### Scenario: 接听

- **WHEN** B 接受处于 `ringing` 的呼叫
- **THEN** 会话变为 `active`，B 获得订阅 token，A 收到 `CALL_ACCEPT`

#### Scenario: 拒接与挂断

- **WHEN** B 拒接，或任一方在 active 状态挂断
- **THEN** 会话变为 `ended` 并写入 `ended_at`，另一方收到 `CALL_DECLINE` 或 `CALL_END`

#### Scenario: 非法状态迁移

- **WHEN** 对已 `ended` 的呼叫再次接听或挂断
- **THEN** 挂断幂等返回当前状态，接听返回 409 `CONFLICT`

#### Scenario: 第三方操作

- **WHEN** 非通话参与者尝试接听、拒接或挂断
- **THEN** 系统返回 404 `NOT_FOUND`
