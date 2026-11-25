# client-social Specification

## Purpose

定义好友、分组、屏蔽、私信、消息请求与呼叫面板的真实端点接线，让社交界面不再依赖本地模拟状态，并保证状态与后端一致。

## Requirements

### Requirement: 好友与请求的真实同步

好友页 SHALL 从 `/friends`、`/friends/online`、`/friends/requests` 加载数据；发送/接受/拒绝/删除好友 MUST 调用对应端点，并以响应或 `FRIEND_REQUEST_*` 事件更新列表与角标。

#### Scenario: 接受请求

- **WHEN** 用户在待处理标签点击接受
- **THEN** 请求从待处理消失，好友列表出现对方，双方在线状态可继续更新

#### Scenario: 重复请求

- **WHEN** 后端返回 409（已有待处理请求）
- **THEN** 界面提示「请求已发送」，不新增重复行

#### Scenario: 删除好友

- **WHEN** 用户删除好友成功
- **THEN** 好友从列表与分组中移除，私信会话保留但入口标记为非好友

### Requirement: 好友分组与排序

客户端 SHALL 从 `/friend-groups` 加载分组与人数，新建/重命名/删除/上移下移与移动好友 MUST 调用对应端点；失败时 MUST 回滚本地顺序。

#### Scenario: 移动好友

- **WHEN** 用户把好友移动到某分组
- **THEN** 好友出现在该分组下，分组人数变化，刷新后保持

#### Scenario: 删除分组

- **WHEN** 用户删除仍有成员的分组
- **THEN** 组内好友回到未分组，好友关系不丢失

### Requirement: 屏蔽与可见性

客户端 SHALL 在成员菜单/好友页提供屏蔽与取消屏蔽，调用 `/blocks`。被屏蔽用户的在线状态、好友请求与私信入口 MUST 按后端规则隐藏或拒绝。

#### Scenario: 屏蔽后私信

- **WHEN** 用户尝试给已屏蔽自己的人发私信
- **THEN** 界面显示无权限提示，不创建本地会话

#### Scenario: 取消屏蔽

- **WHEN** 用户在已屏蔽标签取消屏蔽
- **THEN** 该用户从屏蔽列表移除，好友关系（若仍在）恢复展示

### Requirement: 私信与消息请求

私信侧栏 SHALL 从 `/dms` 加载会话、最后一条消息与未读。陌生人首条消息 MUST 进入消息请求；接受后 MUST 创建/复用 DM 并显示首条消息，忽略/删除 MUST 不创建会话。

#### Scenario: 好友私信

- **WHEN** 用户点击好友发起私信
- **THEN** 复用或创建 DM，侧栏出现会话，历史消息来自后端

#### Scenario: 接受消息请求

- **WHEN** 用户接受一条 pending 消息请求
- **THEN** 该请求消失，DM 出现在侧栏，首条消息可见

#### Scenario: 私信未读

- **WHEN** 收到私信 `MESSAGE_CREATE` 且当前不在该会话
- **THEN** 会话未读数加一，侧栏与收件箱角标同步

### Requirement: 呼叫面板的端点状态

呼叫面板 SHALL 调用 `/dms/{id}/call`、`accept`、`decline`、`hangup`，并根据 `CALL_RING/ACCEPT/DECLINE/END` 更新状态。加入房间使用返回的 LiveKit token/URL；Demo 不要求实际音频。

#### Scenario: 发起呼叫

- **WHEN** 用户点击语音呼叫
- **THEN** 面板进入「呼叫中」，对端收到 `CALL_RING`；接听后双方进入「已接通」

#### Scenario: 拒接

- **WHEN** 对端拒接
- **THEN** 发起方显示「已拒绝」并回到空闲，不残留通话状态

#### Scenario: 挂断幂等

- **WHEN** 任一方挂断或重复挂断
- **THEN** 面板关闭，重复调用不报错

### Requirement: 在线状态

客户端 SHALL 把好友/成员的 `status` 与 `PRESENCE_UPDATE` 映射为在线圆点；隐身 MUST 显示为离线，但本人资料仍显示真实设置。

#### Scenario: 好友上线

- **WHEN** 收到好友的 `PRESENCE_UPDATE status=online`
- **THEN** 好友列表与成员栏圆点变为在线，无需刷新
