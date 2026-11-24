# Spec Delta

## Purpose

定义服务器增长与活跃相关能力：邀请加入、欢迎屏幕与规则同意、日程活动与报名、加成等级。它覆盖新成员从邀请到首屏引导、以及老成员参与活动和加成的完整链路。

## ADDED Requirements

### Requirement: 邀请的创建与生命周期

系统 SHALL 允许具备 `CREATE_INVITE` 的用户为频道创建邀请码。邀请 MUST 记录 `max_uses`、`expires_at`、`uses` 与 `revoked`；创建者或具备 `MANAGE_SERVER` 的用户可撤销。邀请码 MUST 全局唯一。

#### Scenario: 创建邀请

- **WHEN** 成员为文字频道创建邀请并指定 7 天有效、最多 10 次
- **THEN** 返回唯一 `code` 与完整失效条件，列表中出现该邀请

#### Scenario: 撤销邀请

- **WHEN** 创建者撤销自己的邀请
- **THEN** `revoked=true`，后续解析与接受均失败

#### Scenario: 无权限创建

- **WHEN** 无 `CREATE_INVITE` 的成员创建邀请
- **THEN** 系统返回 403 `MISSING_PERMISSION`

### Requirement: 邀请解析与接受

系统 SHALL 在解析邀请时返回服务器名、成员数、在线数与邀请状态。接受邀请 MUST 在单个事务内校验有效期/次数/封禁状态、创建成员关系并递增 `uses`；重复加入 MUST 幂等成功。

#### Scenario: 解析有效邀请

- **WHEN** 用户请求 `GET /invites/{code}`
- **THEN** 返回服务器预览与 `valid=true`，不要求已加入服务器

#### Scenario: 接受邀请

- **WHEN** 未加入且未被封禁的用户接受有效邀请
- **THEN** 创建成员关系、`uses` 加一，返回服务器详情与欢迎屏幕配置

#### Scenario: 次数耗尽

- **WHEN** 邀请 `uses` 已达 `max_uses`
- **THEN** 解析返回 `valid=false`，接受返回 404 `NOT_FOUND`

#### Scenario: 重复接受

- **WHEN** 已是成员的用户再次接受同一邀请
- **THEN** 返回成功且不重复增加 `uses`

### Requirement: 欢迎屏幕配置与同意

系统 SHALL 为每个服务器维护欢迎屏幕（标题、介绍、规则、推荐频道、是否强制同意）。推荐频道 MUST 只包含当前存在的文字频道。`require_rules=true` 时，未同意的成员 MUST 被标记为待引导。

#### Scenario: 读取欢迎配置

- **WHEN** 成员请求 `GET /servers/{id}/welcome`
- **THEN** 返回标题、介绍、按 position 排序的规则与推荐频道

#### Scenario: 同意规则

- **WHEN** 成员同意全部规则并选择推荐频道
- **THEN** 写入 `welcome_accepts` 并返回所选频道；重复同意只更新时间

#### Scenario: 未同意状态

- **WHEN** 未同意规则的成员请求 `GET /servers/{id}/welcome/state`
- **THEN** 返回 `accepted=false`，客户端据此自动弹出欢迎屏幕

#### Scenario: 编辑欢迎配置

- **WHEN** 无 `MANAGE_SERVER` 的用户调用 `PATCH /servers/{id}/welcome`
- **THEN** 系统返回 403 `MISSING_PERMISSION`

### Requirement: 日程活动的状态与报名

系统 SHALL 支持服务器活动的创建、编辑、删除与报名。活动状态 MUST 由 `starts_at` 与 `duration_minutes` 推导为 `upcoming/live/ended`；报名 MUST 幂等，取消后从名单移除。

#### Scenario: 状态推导

- **WHEN** 当前时间早于 `starts_at`
- **THEN** 活动状态为 `upcoming`；处于区间内为 `live`；超过结束时间为 `ended`

#### Scenario: 报名与取消

- **WHEN** 成员对活动报名，随后取消报名
- **THEN** 第一次写入报名记录，第二次删除记录，名单与计数同步变化

#### Scenario: 删除活动

- **WHEN** 创建者或 `MANAGE_SERVER` 删除活动
- **THEN** 活动与其全部报名记录被删除，并广播 `EVENT_DELETE`

#### Scenario: 无权限创建

- **WHEN** 普通成员创建活动
- **THEN** 系统返回 403 `MISSING_PERMISSION`

### Requirement: 加成状态与撤销

系统 SHALL 按 `BOOST_TIERS = [2, 7, 14]` 计算加成次数、等级、下一档差额与进度。加成/撤销 MUST 幂等，并在成功后写入审核日志与广播 `BOOST_UPDATE`。

#### Scenario: 加成

- **WHEN** 成员首次为本服务器加成
- **THEN** 写入 `boosts`，等级按新次数重算，返回加成者列表与进度

#### Scenario: 撤销

- **WHEN** 加成者撤销自己的加成
- **THEN** 记录被删除，等级下降，进度重算

#### Scenario: 进度区间

- **WHEN** 服务器有 8 次加成（等级 2）
- **THEN** 进度按 7→14 区间计算，`remaining=6`，而不是按 0→14 计算

#### Scenario: 订阅计费

- **WHEN** 调用 `POST /servers/{id}/boost/subscribe`
- **THEN** Demo 阶段返回 501 `NOT_IMPLEMENTED`，不产生计费记录
