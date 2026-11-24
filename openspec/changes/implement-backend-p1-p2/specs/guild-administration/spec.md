# Spec Delta

## Purpose

定义服务器内部的人员与权限治理：成员资料、踢出与封禁、角色层级、13 项权限矩阵及频道级覆盖。它是所有管理类写接口的统一门禁来源。

## ADDED Requirements

### Requirement: 成员列表与服务器昵称

系统 SHALL 向服务器成员返回成员列表，包含用户资料、服务器昵称、角色 id 与在线状态。拥有 `MANAGE_SERVER` 或本人可修改服务器昵称；空字符串 MUST 清除昵称并回退到显示名。

#### Scenario: 成员列表

- **WHEN** 成员请求 `GET /servers/{id}/members`
- **THEN** 返回至少包含 `user_id`、`display_name`、`nickname`、`role_ids`、`status` 的列表，非成员得到 404

#### Scenario: 修改他人昵称

- **WHEN** 无 `MANAGE_SERVER` 的成员尝试修改他人服务器昵称
- **THEN** 系统返回 403 `MISSING_PERMISSION`

#### Scenario: 清除昵称

- **WHEN** 有权限者把某成员昵称设为空串
- **THEN** 该成员展示名回退为账号显示名，数据库 `nickname` 为空串

### Requirement: 踢出与封禁

系统 SHALL 允许具备 `KICK_MEMBERS` 的用户移出成员、具备 `BAN_MEMBERS` 的用户封禁成员。操作 MUST 遵守角色层级：不能操作服务器所有者，也不能操作最高角色不低于自己的成员。封禁 MUST 阻止其通过邀请重新加入。

#### Scenario: 踢出成员

- **WHEN** 管理员踢出角色层级更低的成员
- **THEN** 成员关系与角色关联被删除，该用户收到 `MEMBER_LEAVE`，其已发消息保留

#### Scenario: 层级不足

- **WHEN** 用户尝试踢出或封禁角色 position 高于自己的成员
- **THEN** 系统返回 403 `MISSING_PERMISSION`

#### Scenario: 封禁并删除消息

- **WHEN** 管理员以 `delete_days=3` 封禁成员
- **THEN** 写入 `bans` 记录、删除成员关系，并软删该成员近 3 天在服务器内的消息

#### Scenario: 被封禁者接受邀请

- **WHEN** 处于封禁状态的用户调用 `POST /invites/{code}/accept`
- **THEN** 系统返回 403 `MISSING_PERMISSION`，不恢复成员关系

#### Scenario: 撤销封禁

- **WHEN** 有 `BAN_MEMBERS` 的用户撤销封禁
- **THEN** `bans` 记录被删除，该用户可重新通过邀请加入

### Requirement: 角色 CRUD 与层级

系统 SHALL 支持角色创建、改名、换色、调整 position 与删除。`@everyone` 角色 MUST 不可删除且 position 恒为最低；删除角色 MUST 同时清理成员关联与权限授予记录。

#### Scenario: 新建角色

- **WHEN** 有 `MANAGE_ROLES` 的用户创建角色
- **THEN** 返回按 position 降序可排列的角色对象，并广播 `ROLE_CREATE`

#### Scenario: 修改高于自己的角色

- **WHEN** 用户的最高角色 position 低于目标角色
- **THEN** 系统返回 403 `MISSING_PERMISSION`

#### Scenario: 删除 @everyone

- **WHEN** 任何用户尝试删除 `is_everyone=true` 的角色
- **THEN** 系统返回 409 `CONFLICT`，角色保留

#### Scenario: 删除角色

- **WHEN** 管理员删除普通角色
- **THEN** 该角色的 `member_roles`、`permission_grants` 一并删除，持有者权限即时重算

### Requirement: 角色授予与移除

系统 SHALL 以 `assigned: bool` 幂等地授予或移除成员角色。授予 MUST 校验操作者可管理的角色范围，且不得把角色授予非本服务器成员。

#### Scenario: 授予角色

- **WHEN** 管理员把低于自己的角色授予成员
- **THEN** 写入成员角色关联，成员收到 `MEMBER_UPDATE`，其权限立即生效

#### Scenario: 重复授予

- **WHEN** 对已持有该角色的成员再次授予
- **THEN** 返回成功且不产生重复关联

#### Scenario: 授予非成员

- **WHEN** 管理员向不在服务器内的用户授予角色
- **THEN** 系统返回 404 `NOT_FOUND`

### Requirement: 权限矩阵读写

系统 SHALL 提供角色 × 13 项权限的三态矩阵（`inherit/allow/deny`）。读取 MUST 只返回非 `inherit` 的项；写入 MUST 要求 `MANAGE_ROLES`（所有者短路），并拒绝未知权限名。

#### Scenario: 读取矩阵

- **WHEN** 管理员请求 `GET /servers/{id}/permissions`
- **THEN** 返回仅含显式 allow/deny 的角色-权限对，未出现的项视为 inherit

#### Scenario: 批量写入

- **WHEN** 管理员提交多项 allow/deny
- **THEN** 系统在一个事务内覆盖写并广播一次 `PERMISSIONS_UPDATE`

#### Scenario: 未知权限

- **WHEN** 请求体包含不在 13 项清单里的权限名
- **THEN** 系统返回 400 `VALIDATION_ERROR`，整个批次不落库

### Requirement: 频道级权限覆盖

系统 SHALL 支持按频道覆盖角色权限（`inherit/allow/deny`）。有效权限 MUST 按 `@everyone 基线 → 角色叠加 → 频道覆盖 → 所有者短路` 计算；`VIEW_CHANNEL` 被拒绝时 MUST 以 404 隐藏频道。

#### Scenario: 拒绝查看频道

- **WHEN** 某角色在频道上被 deny `VIEW_CHANNEL`
- **THEN** 该角色成员读取频道、消息与搜索该频道均得到 404

#### Scenario: 覆盖优先于服务器级

- **WHEN** 服务器级 allow `SEND_MESSAGES` 但频道级 deny `SEND_MESSAGES`
- **THEN** 该成员发送消息返回 403 `MISSING_PERMISSION`

#### Scenario: 所有者短路

- **WHEN** 服务器所有者访问被 deny 的频道
- **THEN** 仍拥有全部权限，不被覆盖影响

#### Scenario: 写覆盖的权限

- **WHEN** 无 `MANAGE_CHANNELS` 的用户调用 `PUT /channels/{id}/overrides`
- **THEN** 系统返回 403 `MISSING_PERMISSION`
