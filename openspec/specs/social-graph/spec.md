# social-graph Specification

## Purpose

定义用户之间的社交关系：好友请求、双向好友、好友分组与屏蔽。它决定好友页、私信入口和成员菜单能看到谁、能否发起会话。

## Requirements

### Requirement: 好友请求的发送与去重

系统 SHALL 允许已登录用户向另一个用户发送好友请求，请求以 `from_user + to_user + state=pending` 唯一。系统 MUST 拒绝向自己发送、拒绝目标不存在或已被屏蔽的请求，并对重复的 pending 请求返回 409。

#### Scenario: 发送好友请求

- **WHEN** 用户 A 向用户 B 的 `用户名#标签` 发送请求且双方无 pending 请求、无屏蔽关系
- **THEN** 系统创建 `pending` 请求，返回请求 id 与创建时间，并向 B 推送 `FRIEND_REQUEST_CREATE`

#### Scenario: 重复请求

- **WHEN** A 在 B 尚未处理时再次发送好友请求
- **THEN** 系统返回 409 `CONFLICT`，且不产生第二条 pending 记录

#### Scenario: 被屏蔽时发送

- **WHEN** B 已屏蔽 A，或 A 已屏蔽 B，A 仍尝试发送好友请求
- **THEN** 系统返回 404 `NOT_FOUND`，不泄露屏蔽关系

#### Scenario: 向自己发送

- **WHEN** A 使用自己的 `用户名#标签` 发送好友请求
- **THEN** 系统返回 400 `VALIDATION_ERROR`，`field=username`

### Requirement: 好友请求的处理

系统 SHALL 仅允许请求接收者接受或拒绝 pending 请求。接受后 MUST 在同一个事务内建立双向好友关系并把请求置为 `accepted`；拒绝后请求置为 `rejected`，不得再次处理。

#### Scenario: 接受请求

- **WHEN** B 接受 A 发来的 pending 请求
- **THEN** 系统写入 A→B、B→A 两条好友记录，请求变为 `accepted`，双方收到好友列表更新

#### Scenario: 非接收者处理

- **WHEN** 用户 C 尝试接受发给 B 的请求
- **THEN** 系统返回 404 `NOT_FOUND`

#### Scenario: 处理已结束的请求

- **WHEN** B 对已 `accepted` 或 `rejected` 的请求再次调用接受/拒绝
- **THEN** 系统返回 409 `CONFLICT`，好友关系不被重复创建

### Requirement: 双向好友与删除

好友关系 SHALL 以两行记录表示（A→B、B→A），删除好友 MUST 同时删除两行，并从所有好友分组中移除。好友列表 MUST 返回在线状态与所属分组。

#### Scenario: 删除好友

- **WHEN** A 删除好友 B
- **THEN** 两行好友记录被删除，B 的好友列表中不再出现 A，双方收到 `FRIEND_REQUEST_DELETE` 类的刷新信号

#### Scenario: 好友列表字段

- **WHEN** A 请求 `GET /friends`
- **THEN** 每一项包含用户 id、显示名、头像色、`status`（online/idle/dnd/offline）、`group_id`，且不包含已被屏蔽的用户

### Requirement: 好友分组的顺序与归属

系统 SHALL 允许用户创建、重命名、删除好友分组并整体重排顺序。删除分组 MUST 把组内好友改为未分组，而不是删除好友。分组名在同一用户下 MUST 唯一。

#### Scenario: 重命名重名分组

- **WHEN** A 把分组重命名为自己已有分组的同名
- **THEN** 系统返回 409 `CONFLICT`

#### Scenario: 整表重排

- **WHEN** A 提交包含全部 group_id 的新顺序
- **THEN** 系统按提交顺序写回 `position`，缺项或重复 id 返回 400

#### Scenario: 删除分组

- **WHEN** A 删除仍有成员的分组
- **THEN** 组内好友的 `group_id` 置空，好友关系保留

### Requirement: 屏蔽的可见性效果

系统 SHALL 支持屏蔽与取消屏蔽用户，操作 MUST 幂等。被屏蔽双方 MUST 无法互相发送好友请求、私信或看到对方在线状态；成员列表与历史消息不被删除。

#### Scenario: 屏蔽后不可私信

- **WHEN** A 屏蔽 B 后尝试创建或复用与 B 的 DM
- **THEN** 系统返回 403 `MISSING_PERMISSION`，不创建会话

#### Scenario: 重复屏蔽

- **WHEN** A 对同一用户重复调用屏蔽或取消屏蔽
- **THEN** 系统返回当前屏蔽状态，不报错且不产生重复记录

#### Scenario: 屏蔽列表

- **WHEN** A 请求 `GET /blocks`
- **THEN** 返回被屏蔽用户的基本资料与屏蔽时间，不返回对方是否也屏蔽了 A
