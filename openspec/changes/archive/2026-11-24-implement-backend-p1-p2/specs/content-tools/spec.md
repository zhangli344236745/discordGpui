# Spec Delta

## Purpose

定义消息之上的内容形态：投票、话题、跨频道与私信书签、论坛帖子与回帖。它让聊天区、论坛页、话题面板和书签面板共享同一套稳定身份与权限规则。

## ADDED Requirements

### Requirement: 投票的创建与选项

系统 SHALL 在文字频道内创建投票消息，选项数量 MUST 为 2–10 个且标签非空。投票 MUST 记录单选/多选、创建者与关闭状态，并与一条消息原子创建。

#### Scenario: 创建单选投票

- **WHEN** 有 `SEND_MESSAGES` 的成员提交问题与 3 个选项
- **THEN** 返回消息 id 与投票对象，频道内广播 `MESSAGE_CREATE`，投票卡显示 0 票

#### Scenario: 选项不足

- **WHEN** 请求只包含 1 个选项或空标签
- **THEN** 系统返回 400 `VALIDATION_ERROR`，消息与投票都不落库

#### Scenario: 无发送权限

- **WHEN** 无 `SEND_MESSAGES` 的成员创建投票
- **THEN** 系统返回 403 `MISSING_PERMISSION`

### Requirement: 投票的计票与结束

单选投票 MUST 用新选项替换旧选项；多选投票 MUST 支持逐项切换。关闭后的投票 MUST 拒绝新投票并保持结果不变。关闭操作仅限发起人或具备 `MANAGE_MESSAGES` 的用户。

#### Scenario: 单选改票

- **WHEN** 用户先投选项 A 再投选项 B
- **THEN** A 的计数减一、B 加一，该用户只保留 B

#### Scenario: 多选切换

- **WHEN** 用户在多选投票中再次点击已选选项
- **THEN** 该选项的投票被取消，其余选择保留

#### Scenario: 关闭后投票

- **WHEN** 投票已关闭，用户仍尝试投票
- **THEN** 系统返回 409 `CONFLICT`

#### Scenario: 非发起人关闭

- **WHEN** 既非发起人也无 `MANAGE_MESSAGES` 的用户尝试关闭投票
- **THEN** 系统返回 403 `MISSING_PERMISSION`

### Requirement: 话题的创建与回复

系统 SHALL 允许从一条消息开启唯一话题，并在话题内追加回复。话题对象 MUST 暴露原消息上下文、回复列表与回复计数；回复 MUST 校验频道 `VIEW_CHANNEL` 与 `SEND_MESSAGES`。

#### Scenario: 开启话题

- **WHEN** 成员对一条普通消息调用 `POST /messages/{id}/threads`
- **THEN** 创建话题并返回原消息与空回复列表，消息上出现「0 条回复」标记

#### Scenario: 重复开启

- **WHEN** 同一消息已有话题，再次调用创建
- **THEN** 系统返回 409 `CONFLICT` 与既有话题 id

#### Scenario: 追加回复

- **WHEN** 成员在话题内回复
- **THEN** 写入回复、`reply_count` 加一，并广播 `THREAD_UPDATE`

#### Scenario: 无权查看

- **WHEN** 非频道成员请求 `GET /threads/{id}`
- **THEN** 系统返回 404 `NOT_FOUND`

### Requirement: 书签的稳定身份与跨端同步

书签 MUST 以消息的稳定 id 为身份，覆盖频道消息与私信消息，不得以正文反查。保存/取消 MUST 幂等，并广播 `BOOKMARK_UPDATE` 供多端同步。原消息被删除后，书签项 MUST 以墓碑形式展示而不是消失。

#### Scenario: 保存频道消息

- **WHEN** 用户对频道消息调用 `PUT /bookmarks/{message_id}` 且 `saved=true`
- **THEN** 写入书签，`GET /bookmarks` 中出现该条并带频道/服务器信息

#### Scenario: 保存私信消息

- **WHEN** 用户保存一条私信消息
- **THEN** 书签列表以私信来源展示，仍可按消息 id 跳回原会话

#### Scenario: 重复保存

- **WHEN** 用户对已保存消息再次保存
- **THEN** 返回成功且不产生重复项，`saved` 状态不变

#### Scenario: 原消息已删除

- **WHEN** 用户查看的书签指向已软删消息
- **THEN** 书签仍在列表中，正文显示为「原消息已删除」，跳转不报 500

### Requirement: 论坛帖子列表与排序

系统 SHALL 在论坛频道按 `recent/popular/unanswered` 排序返回帖子，置顶帖 MUST 恒在最前，并支持关键词与标签过滤。排序 MUST 稳定且支持游标分页。

#### Scenario: 置顶恒前

- **WHEN** 帖子同时包含置顶与非置顶项
- **THEN** 无论排序方式，置顶项都出现在非置顶项之前

#### Scenario: 未回应筛选

- **WHEN** 使用 `sort=unanswered`
- **THEN** 只返回 `reply_count=0` 的帖子

#### Scenario: 关键词搜索

- **WHEN** 携带 `q` 查询
- **THEN** 标题或正文命中的帖子被返回，大小写不敏感

### Requirement: 论坛发帖、回帖与锁定

系统 SHALL 支持发帖（标题、正文、最多 5 个标签）、回帖、置顶、锁定与删除。锁定后 MUST 拒绝新回帖并返回 403；作者或具备 `MANAGE_MESSAGES` 的用户可编辑/删除。

#### Scenario: 发帖

- **WHEN** 有 `SEND_MESSAGES` 的成员在论坛频道发帖
- **THEN** 创建帖子与标签，`reply_count=0`，列表按当前排序可见

#### Scenario: 标签超限

- **WHEN** 提交 6 个标签
- **THEN** 系统返回 400 `VALIDATION_ERROR`，帖子不落库

#### Scenario: 锁定后回帖

- **WHEN** 帖子 `locked=true` 时成员尝试回帖
- **THEN** 系统返回 403 `MISSING_PERMISSION`，不新增回复

#### Scenario: 回帖计数

- **WHEN** 成员成功回帖
- **THEN** `reply_count` 加一，`last_activity_at` 更新，列表排序随之变化

#### Scenario: 他人删帖

- **WHEN** 非作者且无 `MANAGE_MESSAGES` 的用户删除帖子
- **THEN** 系统返回 403 `MISSING_PERMISSION`

### Requirement: 频道热门标签补全

系统 SHALL 按使用频次返回指定论坛频道的高频标签，供发帖输入框补全，结果 MUST 去重并按频次降序。

#### Scenario: 标签补全

- **WHEN** 请求 `GET /channels/{id}/tags`
- **THEN** 返回最多 20 个去重标签及使用次数，不含其他频道标签
