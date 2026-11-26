# search-and-notifications Specification

## Purpose

定义跨会话的检索与提醒能力：消息全文搜索、频道内过滤、搜索补全、用户级视图状态与草稿、通知收件箱与未读角标、推送端点注册。它让客户端首屏、侧栏和搜索页可以完全由服务端驱动。

## Requirements

### Requirement: 全局消息搜索

系统 SHALL 提供跨频道与私信的全文搜索，支持 `q`、`scope=server|all|dm|everywhere`、`from:`、`has:`、`after:`、`before:` 过滤，并按游标分页返回命中片段。结果 MUST 只包含调用者有权查看的会话，无权内容不得以任何形式泄露。

#### Scenario: 服务器内搜索

- **WHEN** 用户在服务器 A 内搜索 `q=部署 scope=server`
- **THEN** 只返回 A 内可见频道的命中，包含消息 id、频道 id、作者、时间与高亮片段

#### Scenario: 无权频道过滤

- **WHEN** 某频道 deny 了调用者的 `VIEW_CHANNEL`
- **THEN** 即使正文命中，该频道消息也不出现在结果中

#### Scenario: 操作符组合

- **WHEN** 使用 `from:alice has:link after:2026-11-01`
- **THEN** 只返回 alice 发送、含链接且时间晚于指定日期的消息

#### Scenario: 游标分页

- **WHEN** 结果超过一页并携带上一页返回的游标
- **THEN** 返回下一批结果且不重复、不跳项，`has_more` 准确

#### Scenario: 中文检索

- **WHEN** 搜索中文关键词
- **THEN** 通过 trigram 索引命中包含该连续子串的消息

### Requirement: 会话内搜索

系统 SHALL 提供单频道或单会话的搜索，语义与全局搜索一致但范围固定，且 MUST 校验 `VIEW_CHANNEL`/DM 成员身份。

#### Scenario: 频道内搜索

- **WHEN** 成员在可见频道内搜索关键词
- **THEN** 只返回该频道命中，按时间倒序，支持游标

#### Scenario: 私信内搜索

- **WHEN** 会话参与者在 DM 内搜索
- **THEN** 只返回该 DM 的消息，非参与者得到 404

### Requirement: 搜索补全

系统 SHALL 为快速切换器提供补全建议，覆盖可见频道名、用户名与支持的命令，结果 MUST 按匹配度排序并限制条数。

#### Scenario: 前缀匹配

- **WHEN** 输入 `de` 触发补全
- **THEN** 返回名称以 `de` 开头的可见频道与用户，最多 20 条，不含无权频道

#### Scenario: 空查询

- **WHEN** `q` 为空或只有空白
- **THEN** 返回 400 `VALIDATION_ERROR`

### Requirement: 用户级频道视图状态

系统 SHALL 按用户保存频道折叠、通知覆盖与最后已读位置，并支持整体读取与单项更新。写入 MUST 只影响调用者自己的状态，且 `last_read_message_id` 只能前进。

#### Scenario: 折叠频道

- **WHEN** 用户把某频道 `collapsed` 设为 true
- **THEN** 仅该用户的状态更新，其他成员侧栏不受影响

#### Scenario: 已读位置后退

- **WHEN** 用户提交比当前更早的 `last_read_message_id`
- **THEN** 系统保留较新的位置，不产生未读回升

#### Scenario: 读取全部状态

- **WHEN** 客户端请求 `GET /me/channel-state`
- **THEN** 返回该用户所有频道的折叠、通知级别与已读位置

### Requirement: 跨设备草稿

系统 SHALL 按 `(user_id, scope)` 保存草稿正文与待发附件，支持覆盖写入与清空。空正文且无附件 MUST 删除草稿记录，使侧栏草稿标记消失。

#### Scenario: 保存草稿

- **WHEN** 用户在频道输入框失焦时提交非空正文
- **THEN** 草稿被写入，重新进入该会话时返回相同正文

#### Scenario: 清空草稿

- **WHEN** 用户提交空正文且无附件，或消息发送成功
- **THEN** 该 scope 的草稿记录被删除

#### Scenario: 隔离

- **WHEN** A 保存草稿后 B 读取自己的草稿
- **THEN** B 看不到 A 的内容

### Requirement: 通知收件箱与未读计数

系统 SHALL 聚合 mention、未读、私信与好友请求四类通知，支持按类型筛选、分页与批量已读。未读计数 MUST 按服务器与频道分组返回，且与已读操作即时一致。

#### Scenario: 收件箱列表

- **WHEN** 用户请求 `GET /notifications?kind=mention`
- **THEN** 只返回 mention 通知，按时间倒序，含目标频道与会话信息

#### Scenario: 批量已读

- **WHEN** 用户提交若干通知 id 或 `all=true`
- **THEN** 对应通知 `read=true`，未读计数同步减少

#### Scenario: 未读角标

- **WHEN** 用户请求 `GET /notifications/unread-counts`
- **THEN** 返回每个服务器的未读总数与每个频道的未读数，不含已读项

#### Scenario: 越权已读

- **WHEN** 用户提交不属于自己的通知 id
- **THEN** 系统忽略这些 id，不修改他人通知

### Requirement: 推送端点注册

系统 SHALL 接受 WebPush/APNs 端点注册，按用户保存端点与密钥，重复注册 MUST 幂等。Demo 阶段只持久化端点，不实际投递推送。

#### Scenario: 注册端点

- **WHEN** 已登录用户提交有效 endpoint 与 keys
- **THEN** 端点被保存，重复提交同一 endpoint 不产生重复项

#### Scenario: 非法端点

- **WHEN** endpoint 不是 http(s) URL 或缺少必要密钥
- **THEN** 系统返回 400 `VALIDATION_ERROR`

### Requirement: 客户端通知中心分诊

客户端 SHALL 按服务端通知的真实来源展示收件箱，支持按来源筛选、按来源分组、跳转到对应会话，并把已读状态同步到服务端；无法解析的目标 MUST 安全回退，不得崩溃或误导跳转。

#### Scenario: 按来源筛选

- **WHEN** 用户选择「未读」「提及」「私信」「频道」「好友请求」或「系统」
- **THEN** 只显示对应来源或未读条目，每个标签显示条数，「全部」按来源分组展示

#### Scenario: 已读同步

- **WHEN** 用户点击某条服务端通知或「全部已读」
- **THEN** 客户端调用 `POST /notifications/read`（`ids[]` 或 `all=true`）并本地标记已读，其他设备重新同步后同样为已读

#### Scenario: 清除已读

- **WHEN** 用户点击「清除已读 N」
- **THEN** 已读条目从当前收件箱移除，未读条目与服务端记录不受影响

#### Scenario: 目标解析

- **WHEN** 收到 `target_kind=channel` 或 `target_kind=dm` 的通知
- **THEN** 分别解析到本地服务器/频道或私信会话并跳转；解析失败时回退到好友页并保留通知内容
