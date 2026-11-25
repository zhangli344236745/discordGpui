# 后端接口清单与架构设计

> 目标：把当前 63 个界面文件里已经跑通的交互，逐条对应到**真实后端**。
> 技术栈：**uv + FastAPI + SQLite + LiveKit**。
> 本文只做设计，不含实现代码；接口以「界面已有行为」为准，不臆造未出现在界面上的功能。

---

## 0. 结论先行

| 项 | 数量 |
| --- | --- |
| REST 端点 | **197 个操作 / 155 条路径**（含 P1 身份安全、P4 机器人集成、P5 举报合规；PUT 含附件直传） |
| WebSocket 通道 | 2 条（Gateway 主通道 + 语音信令走 LiveKit） |
| 下行事件 | 30 类 |
| 数据表 | 51 张 |
| 对象存储桶 | 1 个（`attachments`） |

分期：**P0 跑通「登录 → 服务器 → 频道 → 聊天」= 55 个端点**；**P1 补「好友 → 私信 → 权限 → 语音」= 64 个**；**P2 长尾（论坛、日程、加成、模板、搜索）= 48 个**，合计 167。

---

## 0.1 实现状态（server/）

本设计已有可运行骨架，代码在 [`server/`](../server/)，完成度如下：

| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| P0 认证 / 用户 / 服务器 / 频道 / 消息 / 附件 | ✅ 已实现 | 含游标分页、`client_nonce` 幂等、编辑历史、软删、权限矩阵、自动审核 |
| P1 语音（LiveKit） | ✅ 已实现（频道语音） | 自签 AccessToken + webhook 验签回写 + 广播；私信通话仍是 501 |
| P0 实时 Gateway | ✅ 已实现 | `/ws/gateway` 订阅制广播，事件目录已补齐 |
| P1/P2 地基 | ✅ 已实现 | `gateway_events` 事件日志 + outbox、FTS5 双索引、频道权限覆盖、DM 已读/置顶列、书签通用目标 |
| P1 好友 / 好友分组 / 屏蔽 / 私信 / 消息请求 / 私信通话 | ✅ 已实现 | 双向好友、分组排序、屏蔽门禁、DM 复用、消息请求三态、呼叫状态机与 60s 超时 |
| P1 成员 / 角色 / 权限覆盖 / 封禁 / 邀请 | ✅ 已实现 | 角色层级、13 项三态矩阵、频道覆盖、封禁拦截、邀请三态与原子占名额 |
| P1 通知 | ⏳ 501 占位 | 路由已注册进 OpenAPI，summary 标注本节小节号 |
| P2 自动审核 / 审核日志 | ✅ 已实现 | 规则 CRUD、发送与编辑扫描、block 422 / warn warnings、日志过滤与 owner 清空 |
| P2 论坛 / 投票 / 话题 / 书签 / 日程 / 加成 / 欢迎写入 | ✅ 已实现 | 计票与结束、话题回复、跨端书签、论坛排序与锁定、活动状态机、欢迎同意、加成等级 |
| P2 搜索 / 用户级视图状态 / 通知 | ✅ 已实现 | FTS5 trigram + 操作符、视图状态与草稿、通知聚合/未读/推送注册；Gateway `?since` 补偿 |

101 个占位已全部替换，`server/app/api/v1/stubs.py` 已删除；唯一保留的 501 是 `/servers/{id}/boost/subscribe`（Demo 不接计费）。

---

## 1. 总体架构

```text
┌─────────────────────────────┐
│  Rust / GPUI 客户端          │
│  当前 data/models.rs 的字段   │
│  = 下面各 DTO 的镜像          │
└───────┬──────────────┬───────┘
        │ HTTPS        │ WSS
        │ REST         │ Gateway（事件推送）
┌───────▼──────────────▼───────────────────────┐
│  FastAPI（uv 管理，uvicorn）                  │
│  ├─ api/v1/*        路由层（只做校验+编排）    │
│  ├─ services/*      业务规则（权限、审核、加成）│
│  ├─ realtime/*      WebSocket Hub + 广播       │
│  ├─ livekit/*       Token 签发 + Webhook       │
│  └─ models/*        SQLAlchemy 2.0 ORM        │
└───────┬──────────────┬───────────────┬───────┘
        │              │               │
   ┌────▼────┐   ┌─────▼─────┐   ┌─────▼──────┐
   │ SQLite  │   │ 文件存储   │   │  LiveKit   │
   │ WAL     │   │ 本地/MinIO │   │  SFU 集群   │
   └─────────┘   └───────────┘   └────────────┘
```

**关键取舍**

- **SQLite 单写者**：开 `journal_mode=WAL`、`busy_timeout=5000`、`synchronous=NORMAL`。跑 `uvicorn --workers 1`（协程并发足够撑 Demo 到数百连接）。要多进程必须换 Postgres，否则会 `database is locked`。
- **实时不走轮询**：客户端登录后连 `/ws/gateway`，服务端把「某个 server / channel / dm 的事件」广播给订阅者。语音的房间内信令全部交给 LiveKit，不自建音频转发。
- **权限在服务端算**：当前界面的 13 项权限只是本地演示，接入后端后由 `services/permissions.py` 统一算 `effective_permissions(member)`，写接口一律先校验。
- **身份稳定**：引入 **snowflake ID（字符串形式）**，替掉现在「用正文反查消息」的书签做法——这是接入后端后能顺手修掉的已知限制。

---

## 2. 通用约定

### 2.1 基础

| 项 | 约定 |
| --- | --- |
| Base URL | `/api/v1` |
| 鉴权 | `Authorization: Bearer <access_token>`；access 15 min，refresh 30 d |
| 内容类型 | `application/json; charset=utf-8`；上传走 `multipart/form-data` 或预签名直传 |
| 时间 | 一律 ISO 8601 UTC 带 `Z`；客户端本地化 |
| ID | 字符串化 snowflake（`"178392017392017920"`），避免 JS 精度问题 |
| 版本 | 路径版本化；破坏性变更升 `v2` |

### 2.2 分页

```http
GET /channels/{id}/messages?before=<msg_id>&limit=50
```
返回 `{ "items": [...], "has_more": true }`。消息用**游标**（`before`/`after`），列表用 `?cursor=&limit=`。不使用 offset（深翻页会漂移）。

### 2.3 错误体

```json
{ "error": { "code": "MISSING_PERMISSION", "message": "缺少 MANAGE_CHANNELS", "field": null, "request_id": "req_8f2a" } }
```

| HTTP | code | 场景 |
| --- | --- | --- |
| 400 | `VALIDATION_ERROR` | 字段不合法（含 `field`） |
| 401 | `UNAUTHORIZED` / `TOKEN_EXPIRED` | 未登录 / access 过期（客户端应静默刷新） |
| 403 | `MISSING_PERMISSION` | 权限矩阵拒绝 |
| 404 | `NOT_FOUND` | 资源不存在或无权可见（不泄露存在性） |
| 409 | `CONFLICT` | 重名（服务器 / 频道 / 角色 / 好友标签） |
| 422 | `AUTOMOD_BLOCKED` | 自动审核「拦截」，附 `rule_id` 与 `matched_keyword` |
| 429 | `RATE_LIMITED` | 附 `retry_after`（消息 5 条/5 s，登录 10 次/分） |

### 2.4 幂等与并发

- 发消息带 `client_nonce`（UUID），服务端在 `messages(client_nonce)` 上建唯一索引，重试不重复落库。
- 编辑 / 删除 / 投票等修改类带 `If-Match: <version>`（乐观锁），冲突返 409。
- 所有写操作在响应里回 `X-Request-Id`，客户端可用于对账 Gateway 事件。

### 2.5 权限位（与界面 13 项一致）

`VIEW_CHANNEL` `SEND_MESSAGES` `MANAGE_MESSAGES` `MANAGE_CHANNELS` `MANAGE_ROLES` `MANAGE_SERVER` `KICK_MEMBERS` `BAN_MEMBERS` `CREATE_INVITE` `MENTION_EVERYONE` `CONNECT_VOICE` `SPEAK` `ADMINISTRATOR`

计算顺序：`@everyone 基线 → 角色按 position 升序叠加 → 频道级覆盖(allow/deny) → 服务器所有者短路为全部允许`。

---

## 3. 接口清单

> 「界面」列指向当前代码里对应的视图/方法，方便前端逐个接线。

### 3.1 认证与账户（17）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| POST | `/auth/register` | 邮箱 + 密码 + 用户名，返回 tokens；同时发验证邮件 | 无（新增登录页） |
| POST | `/auth/login` | 邮箱/用户名 + 密码；开启 2FA 时需 `totp_code`，缺码返回 `MFA_REQUIRED` | 无（新增登录页） |
| POST | `/auth/refresh` | refresh 换 access | 全局拦截器 |
| POST | `/auth/logout` | 撤销当前 refresh | 设置 → 账户 |
| POST | `/auth/logout-all` | 撤销全部会话并踢掉 Gateway 连接 | 设置 → 隐私与安全 |
| POST | `/auth/password` | 改密码（旧密码校验，撤销全部会话） | 设置 → 账户 |
| POST | `/auth/password/forgot` | 发重置邮件（不泄露邮箱是否存在） | 登录页 |
| POST | `/auth/password/reset` | 重置 token + 新密码；撤销全部会话 | 登录页 |
| GET | `/auth/sessions` | 已登录设备列表 | 设置 → 隐私与安全 |
| DELETE | `/auth/sessions/{id}` | 踢掉某设备（同步断开 Gateway） | 同上 |
| POST | `/auth/verify-email/send` | 重发邮箱验证码（已验证时幂等返回） | 注册流程 |
| POST | `/auth/verify-email` | 验证码确认；重复确认幂等 | 注册流程 |
| POST | `/auth/2fa/setup` | 生成 TOTP secret 与 otpauth URL | 设置 → 隐私与安全 |
| POST | `/auth/2fa/enable` | 校验首码并启用，返回一次性恢复码 | 同上 |
| POST | `/auth/2fa/disable` | 校验码或恢复码后关闭 2FA | 同上 |
| GET | `/auth/security-audit` | 本人安全审计（`format=csv` 可导出） | 同上 |


### 3.2 用户与在线状态（8）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/users/@me` | 当前用户完整资料 | `seed_me` / 用户栏 |
| PATCH | `/users/@me` | 改显示名 / 用户名 / 邮箱 / 关于我 / 头像色 | 设置 → 账户（保存） |
| PATCH | `/users/@me/status` | 在线 / 空闲 / 勿扰 / 隐身 | `status_menu.rs` |
| GET | `/users/{id}` | 他人资料卡 | `profile_card.rs` |
| GET | `/users/{id}/mutual` | 共同服务器 + 共同好友 | 资料卡（预留） |
| GET | `/users/@me/settings` | 全部偏好（外观/通知/文字/语音/语言） | `settings/pages.rs` |
| PATCH | `/users/@me/settings` | 局部更新偏好 | 同上各开关 |
| GET | `/users/@me/shortcuts` | 自定义快捷键映射 | 设置 → 快捷键 |

### 3.3 好友（10）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/friends` | 全部好友（含在线状态、分组） | `friends.rs` 全部 |
| GET | `/friends/online` | 在线子集 | 在线标签 |
| POST | `/friends/requests` | `用户名#标签` 发送请求 | 添加好友 |
| GET | `/friends/requests` | 收到的请求（带 count） | 待处理 |
| POST | `/friends/requests/{id}/accept` | 接受 | 待处理行 |
| POST | `/friends/requests/{id}/reject` | 拒绝 | 待处理行 |
| DELETE | `/friends/{id}` | 删除好友 | 好友右键（预留） |
| POST | `/blocks` | 屏蔽（好友或陌生人） | 屏蔽 / 成员右键 |
| DELETE | `/blocks/{user_id}` | 取消屏蔽 | 已屏蔽标签 |
| GET | `/blocks` | 屏蔽列表 | 已屏蔽标签 |

### 3.4 好友分组（6）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/friend-groups` | 分组 + 顺序 + 人数 | `friend_group_menu.rs` |
| POST | `/friend-groups` | 新建 | 新建分组 |
| PATCH | `/friend-groups/{id}` | 重命名 | 右键重命名 |
| PATCH | `/friend-groups/order` | 整表重排（上移/下移） | 上移 / 下移 |
| DELETE | `/friend-groups/{id}` | 删除，成员回「未分组」 | 删除 |
| PATCH | `/friends/{id}` | 移动到分组 | 好友行文件夹按钮 |

### 3.5 服务器（11）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/servers` | 我加入的服务器（含未读汇总、加成等级） | `server_rail.rs` |
| POST | `/servers` | 创建（`name`、`icon_color`、可选 `template_key`） | `create_dialog.rs` |
| GET | `/servers/{id}` | 详情（简介、徽标色、成员数、加成） | `server_settings` 概览 |
| PATCH | `/servers/{id}` | 改名 / 简介 / 徽标色（改名走审计 + 客户端迁移） | 概览保存 |
| DELETE | `/servers/{id}` | 解散（仅 owner） | 概览（预留） |
| POST | `/servers/{id}/leave` | 离开 | 服务器菜单「离开服务器」 |
| GET | `/servers/{id}/templates` | 可用模板（4 个：gaming/study/design/oss） | 新建弹层 |
| POST | `/servers/{id}/templates/apply` | 对已有服务器套模板 | 预留 |
| GET | `/discover/servers` | 公开服务器（`q`、`category`、`cursor`） | `discover.rs` |
| POST | `/discover/servers/{id}/join` | 加入，返回 `welcome` 配置 | 探索页「加入」 |
| PUT | `/servers/{id}/order` | 侧栏服务器排序 | 预留（拖拽） |

### 3.6 分类与频道（10）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/servers/{id}/channels` | 分类 + 频道树（含折叠状态、通知覆盖） | `channel_sidebar.rs` |
| POST | `/servers/{id}/categories` | 新建分类 | 分类标题 `+` |
| PATCH | `/categories/{id}` | 重命名 / 折叠（折叠是用户级，见 3.19） | `category_menu.rs` |
| DELETE | `/categories/{id}` | 删除（频道转未分类） | 分类右键 |
| POST | `/servers/{id}/channels` | 新建频道（`kind`: text/voice/forum） | 新建频道 |
| PATCH | `/channels/{id}` | 改名 / 改主题 / 移分类 / 排序 | `channel_menu.rs` 编辑主题 |
| DELETE | `/channels/{id}` | 删除（至少保留一个文字频道） | 删除频道 |
| PATCH | `/channels/{id}/notify` | 通知级别（all/mentions/none/inherit） | 通知设置页 |
| POST | `/channels/{id}/read` | 标记已读（到某 message_id） | 标记为已读 / 跳到最新 |
| PATCH | `/servers/{id}/notify` | 服务器默认通知级别 | 服务器通知设置 |

### 3.7 消息（13）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/channels/{id}/messages` | 游标分页历史（含回复、投票、附件、回应） | `chat/messages.rs` |
| POST | `/channels/{id}/messages` | 发送（`content`、`reply_to`、`attachments[]`、`client_nonce`） | `composer.rs` |
| GET | `/messages/{id}` | 单条（跳转定位 / 深链） | 回复跳转 |
| PATCH | `/messages/{id}` | 编辑（写 `message_edits`） | 行内编辑 |
| DELETE | `/messages/{id}` | 删除（软删，留 tombstone） | 消息菜单 |
| GET | `/messages/{id}/edits` | 编辑历史 | `edit_history.rs` |
| PUT | `/messages/{id}/pin` | 置顶 / 取消（`pinned: bool`） | 菜单、`pinned_panel.rs` |
| GET | `/channels/{id}/pins` | 置顶列表 | 图钉面板 |
| POST | `/messages/{id}/reactions` | 加回应（emoji） | 表情回应 |
| DELETE | `/messages/{id}/reactions/{emoji}` | 撤回应 | 再点一次 |
| POST | `/messages/{id}/forward` | 转发到 `target_channel`/`target_dm` | `forward_dialog.rs` |
| GET | `/channels/{id}/read-states` | 各成员已读位置（回执） | `read_receipts.rs` |
| POST | `/channels/{id}/typing` | 「正在输入」（也走 Gateway，HTTP 兜底） | 预留 |

### 3.8 附件与上传（4）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| POST | `/uploads` | 声明上传（`filename`、`size`、`mime`）→ `{upload_url, file_id}`；校验单文件 25MB 上限（超限 413）与用户配额 | `attachment_picker.rs` |
| PUT | `/uploads/{file_id}/content` | 二进制直传；MIME 魔数嗅探拒绝伪装，图片自动生成 512px 缩略图 | 客户端上传 worker |
| GET | `/attachments/{id}` | 元数据 + 短期下载 URL（图片附 `thumbnail_url`） | 附件卡 |
| DELETE | `/attachments/{id}` | 删除未引用的孤儿附件（连同缩略图） | 定时任务 |

### 3.9 投票 / 话题 / 书签（10）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| POST | `/channels/{id}/polls` | 发起投票（问题、选项、`multi`） | `poll_composer.rs` |
| POST | `/polls/{id}/votes` | 投票（单选互斥 / 多选叠加） | 投票卡 |
| DELETE | `/polls/{id}/votes` | 取消我的投票 | 再点选项 |
| POST | `/polls/{id}/close` | 结束（仅发起人） | 结束投票 |
| POST | `/messages/{id}/threads` | 开话题，返回 thread | 工具栏「开话题」 |
| GET | `/threads/{id}` | 话题 + 原消息 + 回复 | `thread_panel.rs` |
| POST | `/threads/{id}/replies` | 话题内回复 | 话题输入条 |
| GET | `/bookmarks` | 我的书签（跨频道 + 私信，`q` 搜索） | `bookmarks_panel.rs` |
| PUT | `/bookmarks/{message_id}` | 保存 / 取消（`saved: bool`） | 菜单「保存消息」 |
| DELETE | `/bookmarks/{id}` | 移除单条 | 面板 × |

### 3.10 私信与消息请求（10）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/dms` | 会话列表（最后一条 + 未读 + 草稿标记） | `dm_sidebar.rs` |
| POST | `/dms` | 与某用户开/复用会话 | 点好友发起 |
| GET | `/dms/{id}/messages` | 游标历史 | 私信聊天区 |
| POST | `/dms/{id}/messages` | 发送 | 私信输入框 |
| PATCH | `/dms/{id}` | 置顶 / 关闭会话 | 预留 |
| GET | `/message-requests` | 请求列表（`state` 筛选 + 计数） | `message_requests_panel.rs` |
| POST | `/message-requests/{id}/accept` | 接受 → 建 DM | 「接受」 |
| POST | `/message-requests/{id}/ignore` | 忽略 | 「忽略」 |
| DELETE | `/message-requests/{id}` | 删除 | 「删除」 |
| PATCH | `/dms/{id}/read` | 标记私信已读 | 打开会话 |

### 3.11 论坛频道（8）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/channels/{id}/posts` | 帖子列表（`q`、`sort=recent/popular/unanswered`，置顶恒前） | `forum.rs` 列表 |
| POST | `/channels/{id}/posts` | 发帖（标题、正文、标签 ≤5） | 新建帖子弹层 |
| GET | `/posts/{id}` | 详情（正文 + 标签 + 回帖） | 帖子详情 |
| PATCH | `/posts/{id}` | 编辑 / 置顶 / 锁定（`pinned`、`locked`） | 顶栏按钮 |
| DELETE | `/posts/{id}` | 删除 | 预留 |
| POST | `/posts/{id}/replies` | 回帖（锁定后 403） | 回帖输入条 |
| DELETE | `/replies/{id}` | 删回帖 | 预留 |
| GET | `/channels/{id}/tags` | 该频道热门标签（补全用） | 标签输入 |

### 3.12 成员 / 角色 / 权限（15）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/servers/{id}/members` | 成员（含服务器昵称、角色、在线状态） | `member_list.rs` |
| PATCH | `/servers/{id}/members/{uid}` | 改服务器昵称（空串清除） | `member_nick_dialog.rs` |
| DELETE | `/servers/{id}/members/{uid}` | 移出（踢） | 成员菜单 |
| GET | `/servers/{id}/roles` | 角色表（按 position 降序） | 设置 → 角色 |
| POST | `/servers/{id}/roles` | 新建角色 | 同上 |
| PATCH | `/roles/{id}` | 改名 / 换色 / 改 position | 角色详情 |
| DELETE | `/roles/{id}` | 删除（成员与权限记录一并清） | 删除角色 |
| PUT | `/servers/{id}/members/{uid}/roles/{rid}` | 授予 / 移除（`assigned: bool`） | 点成员行 |
| GET | `/servers/{id}/permissions` | 角色 × 13 项矩阵（只返非默认） | 设置 → 权限 |
| PUT | `/servers/{id}/permissions` | 批量写三态（inherit/allow/deny） | 三态循环 |
| GET | `/channels/{id}/overrides` | 频道级权限覆盖 | 预留 |
| PUT | `/channels/{id}/overrides` | 写频道覆盖 | 预留 |
| GET | `/servers/{id}/bans` | 封禁列表 | 设置 → 成员 |
| POST | `/servers/{id}/bans` | 封禁（`reason`、`delete_days`） | 封禁 |
| DELETE | `/servers/{id}/bans/{uid}` | 撤销封禁 | 撤销 |

### 3.13 邀请（5）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| POST | `/channels/{id}/invites` | 创建（`expiry`、`max_uses`）→ code | `invite_dialog.rs` |
| GET | `/servers/{id}/invites` | 已创建列表（含失效态：过期/用完） | 邀请管理 |
| DELETE | `/invites/{code}` | 撤销 | 撤销按钮 |
| GET | `/invites/{code}` | 解析预览（服务器名/成员数/在线数） | 加入前预览 |
| POST | `/invites/{code}/accept` | 加入服务器 | 接受邀请 |

### 3.14 审核日志与自动审核（7）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/servers/{id}/audit` | 日志（`kind` 筛选 + 条数 + 游标） | 设置 → 审核日志 |
| DELETE | `/servers/{id}/audit` | 清空（仅 owner） | 一键清空 |
| GET | `/servers/{id}/automod/rules` | 规则列表 | 设置 → 自动审核 |
| POST | `/servers/{id}/automod/rules` | 新建（`action`: block/warn，`keywords[]`） | 新建规则 |
| PATCH | `/automod/rules/{id}` | 启用/停用、改动作、改关键词 | 详情 |
| DELETE | `/automod/rules/{id}` | 删除 | 删除 |
| POST | `/servers/{id}/automod/test` | 试跑一句话，返回命中规则（调试用） | 预留 |

> 自动审核必须在**服务端**执行：`POST /channels/{id}/messages` 先过 `automod.scan()`，`block` 返 422 且不落库，`warn` 落库并在响应里带 `warnings[]`。

### 3.15 服务器日程（6）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/servers/{id}/events` | 活动（`status=upcoming/live/ended`） | `events_panel.rs` |
| POST | `/servers/{id}/events` | 新建（标题/描述/分类/落地频道/开始/时长） | 新建活动 |
| PATCH | `/events/{id}` | 编辑 | 预留 |
| DELETE | `/events/{id}` | 删除 | 删除 |
| PUT | `/events/{id}/rsvp` | 报名 / 取消（`going: bool`） | 报名按钮 |
| GET | `/events/{id}/attendees` | 报名名单 | 活动详情 |

### 3.16 欢迎屏幕（5）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/servers/{id}/welcome` | 配置（标题/介绍/规则/推荐频道/是否强制） | `welcome.rs` |
| PATCH | `/servers/{id}/welcome` | 管理员编辑（仅 MANAGE_SERVER） | 设置（预留） |
| POST | `/servers/{id}/welcome/accept` | 同意规则 + 选频道，记录 `accepted_at` | 「开始聊天」 |
| GET | `/servers/{id}/welcome/state` | 我是否已同意（决定是否自动弹） | 加入后判断 |
| POST | `/channels/{id}/welcome-read` | 标记「已读该频道欢迎」 | 推荐频道跳转 |

### 3.17 服务器加成（4）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/servers/{id}/boosts` | 次数、等级、进度、加成者列表 | 加成页 / 侧栏条 |
| PUT | `/servers/{id}/boosts/me` | 加成 / 撤销（`active: bool`） | 加成按钮 |
| GET | `/servers/{id}/boost/tiers` | 三档特权文案（2/7/14） | 特权卡 |
| POST | `/servers/{id}/boost/subscribe` | 预留：订阅计费（Demo 可 501） | 预留 |

### 3.18 搜索（3）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/search` | 全局搜索：`q`、`scope=server/all/dm/everywhere`、`from:`、`has:`、`after:`、`before:` | `search_center.rs` |
| GET | `/channels/{id}/search` | 会话内搜索 | 标题栏就地过滤 |
| GET | `/search/suggest` | 补全（频道名/用户名/命令） | 快速切换器 `⌘K` |

> SQLite 用 FTS5 虚表 `messages_fts(content, tokenize='unicode61')` 做全文检索；中文需 `tokenize='trigram'` 或外挂分词。

### 3.19 用户级视图状态（4）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/me/channel-state` | 各频道折叠/通知覆盖/已读位置 | 侧栏渲染 |
| PUT | `/me/channel-state/{channel_id}` | 更新单项 | 折叠、通知 |
| GET | `/me/drafts` | 会话草稿（跨设备保留） | 草稿标记 |
| PUT | `/me/drafts/{scope_id}` | 保存/清空草稿 | 输入框失焦 |

### 3.20 通知与收件箱（4）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/notifications` | 收件箱（mention/unread/dm/friend_request 聚合） | `inbox_panel.rs` |
| POST | `/notifications/read` | 批量标记已读（`ids[]` 或 `all`） | 点击跳转 |
| GET | `/notifications/unread-counts` | 各服务器/频道角标 | 侧栏角标 |
| POST | `/push/subscribe` | 注册推送端点（WebPush / APNs） | 设置 → 通知 |

### 3.21 语音与 LiveKit（10）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| POST | `/channels/{id}/voice/join` | 返回 `{token, url, room}`，服务端校验 `CONNECT_VOICE` | 点语音频道 |
| POST | `/channels/{id}/voice/leave` | 离开（也由 LiveKit webhook 兜底） | 再点断开 |
| GET | `/channels/{id}/voice/participants` | 当前参与者（进频道前先看有谁） | 语音频道行 |
| PATCH | `/voice/{room}/participants/{identity}` | 服务器静音 / 断开某人（`MANAGE_CHANNELS`） | `voice_room.rs` 移出 |
| PUT | `/voice/{room}/me` | 本机麦克风 / 摄像头 / 屏幕分享状态同步 | 状态条 |
| POST | `/dms/{id}/call` | 发起私信语音/视频，返回 LiveKit token | `dm_call_panel.rs` |
| POST | `/dms/{id}/call/accept` | 接听（ringing → active，签发订阅 token） | 模拟接通 → 真实 |
| POST | `/dms/{id}/call/decline` | 拒接 | 预留 |
| POST | `/dms/{id}/call/hangup` | 挂断（任一方） | 挂断按钮 |
| POST | `/livekit/webhook` | LiveKit 回调（无 Bearer，用签名校验） | 内部 |

**房间命名**：服务器 `srv:{server_id}:{channel_id}`，私信 `dm:{call_id}`。
**Token grants**：`roomJoin=true`、`canPublish`（麦克风/摄像头）、`canSubscribe=true`、`canPublishData=true`、`canUpdateOwnMetadata=true`；`identity = user_id`、`name = 显示名`、`metadata = {color, avatar}`。
**静音某人**：服务端调 LiveKit `RoomService.MutePublishedTrack`（不是自己维护状态），成功后由 webhook 回推事件。

### 3.22 系统（5）

| 方法 | 路径 | 说明 | 界面 |
| --- | --- | --- | --- |
| GET | `/health` | 存活 + DB 连接 + LiveKit 可达 | 运维 |
| GET | `/config` | 客户端启动配置（上传上限、GIF 源、语音区域、功能开关） | 启动 |
| GET | `/gifs` | GIF 分类（日常 / 猫咪 / 礼物 / 全部） | `gif_picker.rs` |
| GET | `/gifs/search` | GIF 搜索代理（KLIPY/Tenor），隐藏第三方 key | `gif_picker.rs` |
| GET | `/help/articles` | 帮助中心文档（当前 12 篇静态，可搬服务端热更） | `help_center.rs` |

---

## 4. 实时通道（WebSocket Gateway）

**连接**：`wss://<host>/ws/gateway?token=<access>`；心跳 30 s（`op:3` ping / `op:4` pong），2 次无响应断开重连。
**订阅**：连上后发 `{"op":2,"d":{"servers":[...],"dms":[...]}}`，服务端只推订阅内事件（省流量，也避免越权泄露）。

### 4.1 下行事件（30）

| 事件 | 载荷要点 | 界面响应 |
| --- | --- | --- |
| `READY` | 用户、服务器、DM、未读快照 | 首屏渲染 |
| `MESSAGE_CREATE` | message + channel_id | 追加消息、角标 +1 |
| `MESSAGE_UPDATE` | 编辑后正文 + version | 就地替换、显示「已编辑」 |
| `MESSAGE_DELETE` | id + channel_id | 移除或 tombstone |
| `MESSAGE_REACTION_ADD/REMOVE` | message_id, emoji, user | 回应计数 |
| `TYPING_START` | channel_id, user | 「X 正在输入」 |
| `CHANNEL_CREATE/UPDATE/DELETE` | channel / category | 侧栏增量 |
| `CATEGORY_*` | 同上 | 侧栏 |
| `SERVER_UPDATE` | 名称/徽标/简介 | 头部、审计 |
| `MEMBER_JOIN/LEAVE/UPDATE` | member | 成员栏 |
| `ROLE_CREATE/UPDATE/DELETE` | role | 角色页、分组顺序 |
| `PERMISSIONS_UPDATE` | server_id | 重算可见性 |
| `INVITE_CREATE/DELETE` | invite | 邀请列表 |
| `AUTOMOD_RULE_*` | rule | 规则页 |
| `AUDIT_ENTRY_CREATE` | entry | 日志页（实时追加） |
| `PRESENCE_UPDATE` | user_id, status | 好友/成员圆点 |
| `FRIEND_REQUEST_*` | request | 待处理角标 |
| `FRIEND_GROUP_*` | group | 分组列表 |
| `DM_CREATE` | dm | 侧栏新增会话 |
| `MESSAGE_REQUEST_*` | request | 面板 + 角标 |
| `THREAD_CREATE/UPDATE` | thread | 「N 条回复」徽标 |
| `POLL_UPDATE` | poll 计数 | 投票卡刷新 |
| `FORUM_POST_*` / `FORUM_REPLY_*` | post/reply | 帖子列表 |
| `EVENT_*` | server event | 日程面板 |
| `BOOST_UPDATE` | count/level | 加成条 |
| `WELCOME_UPDATE` | welcome | 欢迎屏幕 |
| `NOTIFICATION_CREATE` | notification | 收件箱角标 |
| `VOICE_STATE_UPDATE` | channel_id, participants[] | 语音状态条 |
| `CALL_RING/ACCEPT/DECLINE/END` | call | 呼叫面板 |
| `BOOKMARK_UPDATE` | 多端同步书签 | 书签面板 |

### 4.2 上行（客户端 → 服务端）

仅 4 种：`SUBSCRIBE`（改订阅）、`TYPING`、`VOICE_STATE`（进入/离开语音的轻量同步）、`PING`。
其余全部走 REST（便于校验与幂等）。

---

## 5. SQLite 数据模型

### 5.1 表清单（51）

```text
users(id, username, tag, email, password_hash, display_name, avatar_color,
      about, status, created_at, updated_at)
user_settings(user_id PK, theme, locale, volume, show_timestamps, show_emoji,
      compact_messages, notify_level, push_endpoints_json, ...)
sessions(id, user_id, refresh_hash, device, ip, expires_at, created_at)
auth_tokens(id, user_id, kind, code_hash, expires_at)      -- 邮箱验证 / 重置

friendships(user_id, friend_id, created_at, PRIMARY KEY(user_id, friend_id))
friend_requests(id, from_user, to_user, state, created_at)
friend_groups(id, owner_id, name, position)
blocks(owner_id, target_id, created_at)

servers(id, name, icon_color, description, owner_id, is_public, category,
        template_key, created_at)
server_members(server_id, user_id, nickname, joined_at, PRIMARY KEY(server_id,user_id))
roles(id, server_id, name, color, position, is_everyone, created_at)
member_roles(server_id, user_id, role_id, PRIMARY KEY(server_id,user_id,role_id))
permission_grants(server_id, role_id, permission, state)   -- inherit/allow/deny
channel_overrides(channel_id, role_id, permission, state)
bans(server_id, user_id, reason, banned_by, created_at)

categories(id, server_id, name, position)
channels(id, server_id, category_id, name, kind, topic, position, created_at)
channel_user_state(user_id, channel_id, collapsed, notify_level,
                   last_read_message_id, PRIMARY KEY(user_id,channel_id))

messages(id, channel_id, author_id, content, reply_to, type,
         pinned, edited_at, deleted_at, version, client_nonce, created_at)
dm_channels(id, user_a, user_b, last_message_id, closed,
            last_read_a, last_read_b, pinned_a, pinned_b, created_at)  -- 规范化 a<b
dm_messages(id, dm_id, author_id, content, ...同 messages)
message_edits(message_id, content, edited_at)
attachments(id, owner_id, message_id, filename, mime, size, storage_key, created_at)
reactions(message_id, user_id, emoji, PRIMARY KEY(message_id,user_id,emoji))
read_states(channel_id, user_id, last_read_message_id, updated_at)

polls(id, message_id, question, multi, closed, created_by)
poll_options(id, poll_id, label, position)
poll_votes(poll_id, option_id, user_id, PRIMARY KEY(poll_id,option_id,user_id))

threads(id, message_id, channel_id, created_at)
thread_replies(id, thread_id, author_id, content, created_at)
bookmarks(user_id, target_kind, target_id, scope, note, created_at,
          PRIMARY KEY(user_id,target_kind,target_id))          -- channel/dm 消息通用

forum_posts(id, channel_id, author_id, title, body, pinned, locked,
            reply_count, created_at, last_activity_at)
forum_replies(id, post_id, author_id, body, created_at)
forum_tags(post_id, tag)

invites(code PK, server_id, channel_id, created_by, max_uses, uses,
        expires_at, revoked, created_at)
audit_entries(id, server_id, actor_id, kind, detail, created_at)
automod_rules(id, server_id, name, enabled, action, created_at)
automod_keywords(rule_id, keyword)

server_events(id, server_id, title, description, category,
              channel_id, starts_at, duration_min, created_by)
event_rsvps(event_id, user_id, created_at)
welcome_screens(server_id PK, headline, description, require_rules)
welcome_rules(server_id, position, title, body)
welcome_recommended(server_id, channel_id)
welcome_accepts(server_id, user_id, accepted_at, PRIMARY KEY(server_id,user_id))
boosts(server_id, user_id, created_at, PRIMARY KEY(server_id,user_id))
voice_participants(room, user_id, mic, camera, screen, joined_at,
                   PRIMARY KEY(room,user_id))                  -- LiveKit webhook 回写

notifications(id, user_id, kind, target_kind, target_id, read, created_at)
message_requests(id, from_user, to_user, state, first_message, created_at)
call_sessions(id, dm_id, kind, state, outcome, caller_id, callee_id, created_at, ended_at)
drafts(user_id, scope, content, attachments_json, updated_at)

gateway_events(id, scope_kind, scope_id, event, payload_json, created_at)
  -- 实时事件日志：提交后广播，?since 24h 内补偿
```

### 5.2 关键索引

```sql
CREATE UNIQUE INDEX ux_messages_nonce ON messages(client_nonce) WHERE client_nonce IS NOT NULL;
CREATE INDEX ix_messages_channel ON messages(channel_id, id DESC);
CREATE INDEX ix_dm_messages_dm ON dm_messages(dm_id, id DESC);
CREATE INDEX ix_audit_server ON audit_entries(server_id, id DESC);
CREATE INDEX ix_notifications_user ON notifications(user_id, read, id DESC);
CREATE VIRTUAL TABLE messages_fts USING fts5(content, content='messages', content_rowid='rowid', tokenize='trigram');
CREATE VIRTUAL TABLE dm_messages_fts USING fts5(content, content='dm_messages', content_rowid='rowid', tokenize='trigram');
CREATE INDEX ix_gateway_events_scope ON gateway_events(scope_kind, scope_id, id);
```

### 5.3 SQLite 参数

```python
# app/db.py
engine = create_async_engine("sqlite+aiosqlite:///./data/app.db")
# connect 事件里执行：
#   PRAGMA journal_mode=WAL;
#   PRAGMA busy_timeout=5000;
#   PRAGMA synchronous=NORMAL;
#   PRAGMA foreign_keys=ON;
```

> 建议：`messages.content` 只存纯文本 + 结构化 `embed_json`（embeds/附件/投票引用）。当前 Rust 侧 `Message` 有多种 `MessageKind`，DTO 里用一个 `type` 字段 + `payload` 承载，避免表爆炸。

---

## 6. FastAPI 工程结构（uv）

```text
server/
├── pyproject.toml
├── uv.lock
├── alembic.ini
├── app/
│   ├── main.py                 # FastAPI()、CORS、异常处理器、路由注册
│   ├── config.py               # pydantic-settings（.env）
│   ├── db.py                   # engine / session / PRAGMA
│   ├── deps.py                 # current_user / require_perm / pagination
│   ├── ids.py                  # snowflake 生成器
│   ├── errors.py               # 统一错误体 + ErrorCode 枚举
│   ├── models/                 # SQLAlchemy 2.0（42 张表，按域拆 8 个文件）
│   ├── schemas/                # Pydantic v2 DTO（与 Rust models.rs 一一对应）
│   ├── api/v1/
│   │   ├── auth.py  users.py  friends.py  servers.py  channels.py
│   │   ├── messages.py  uploads.py  polls.py  threads.py  bookmarks.py
│   │   ├── dms.py  message_requests.py  forums.py  members.py  roles.py
│   │   ├── invites.py  audit.py  automod.py  events.py  welcome.py
│   │   ├── boosts.py  search.py  notifications.py  voice.py  system.py
│   ├── services/
│   │   ├── permissions.py      # 权限矩阵求解
│   │   ├── automod.py          # 关键词扫描
│   │   ├── boost.py            # 等级 = f(count, [2,7,14])
│   │   ├── snowflake.py
│   │   └── storage.py          # 本地磁盘 / MinIO
│   ├── realtime/
│   │   ├── hub.py              # 连接池 + 订阅索引
│   │   ├── events.py           # 事件常量与序列化
│   │   └── gateway.py          # /ws/gateway
│   ├── livekit/
│   │   ├── token.py            # AccessToken 签发
│   │   ├── room.py             # RoomService 代理（静音/移除）
│   │   └── webhook.py          # 签名校验 + 事件落库
│   └── tasks/                  # 过期邀请、孤儿附件、活动状态机
├── tests/                      # pytest + httpx.AsyncClient
└── data/app.db
```

**初始化命令**

```bash
uv init server && cd server
uv add fastapi "uvicorn[standard]" sqlalchemy aiosqlite alembic \
       pydantic-settings "python-jose[cryptography]" "passlib[argon2]" \
       python-multipart livekit-api httpx
uv add --dev pytest pytest-asyncio ruff
uv run alembic init migrations
uv run uvicorn app.main:app --reload --port 8080
```

**分层纪律**（和 Rust 侧 `data/app/ui` 同构）

| 层 | 只做 | 禁止 |
| --- | --- | --- |
| `api/` | 解析、鉴权依赖、调 service、返回 DTO | 写业务判断、直接拼 SQL |
| `services/` | 业务规则、事务边界、发事件 | 碰 `Request`/`Response` |
| `models/` | 表结构 | 业务方法 |
| `schemas/` | 入参出参校验 | 访问 DB |

---

## 7. LiveKit 接入

### 7.1 配置

```env
LIVEKIT_URL=wss://lk.example.com
LIVEKIT_API_KEY=APIxxxxxxxx
LIVEKIT_API_SECRET=xxxxxxxx
LIVEKIT_WEBHOOK_SECRET=whsec_xxx
```

### 7.2 签发流程（服务器语音频道）

```text
客户端点语音频道
  → POST /channels/{id}/voice/join
      ├─ 校验 VIEW_CHANNEL + CONNECT_VOICE
      ├─ 房间名 srv:{server_id}:{channel_id}（不存在则 CreateRoom，空房自动回收 5 min）
      ├─ AccessToken(api_key, secret).with_identity(user_id)
      │     .with_name(display_name).with_metadata(json)
      │     .with_grants(VideoGrants(room_join=True, can_publish=True,
      │                              can_subscribe=True, can_publish_data=True))
      │     .with_ttl(timedelta(hours=2))
      └─ 返回 { token, url, room }
  → 客户端 Room.connect(url, token)
  → 本地 publish 麦克风；开摄像头/屏幕分享时 publish 对应 track
```

### 7.3 事件回流

LiveKit webhook → `POST /livekit/webhook`（`Authorization` 用 `WebhookReceiver` 验签）：

| LiveKit 事件 | 后端动作 |
| --- | --- |
| `participant_joined` | 写 `voice_participants` 缓存 + 广播 `VOICE_STATE_UPDATE` |
| `participant_left` | 同上；空房则删记录 |
| `track_published/unpublished` | 更新 mic/camera/screen 标记（对应 `voice_room.rs` 的图标） |
| `room_finished` | 清理频道语音状态 |

### 7.4 与界面状态的对应

| 界面元素 | 数据来源 |
| --- | --- |
| 底部语音状态条（连接/断开） | 本地 Room 状态 + `VOICE_STATE_UPDATE` |
| 语音房间面板参与者 | `GET /channels/{id}/voice/participants` + webhook 增量 |
| 逐个静音 | `PATCH /voice/{room}/participants/{identity}` → RoomService |
| 移出 | 同上 `action=remove` |
| 本机麦克风开关 | 本地 `localParticipant.setMicrophoneEnabled()` |
| 私信呼叫面板 | `CALL_RING/ACCEPT/DECLINE/END` + 独立房间 `dm:{call_id}` |

---

## 8. 从本地快照迁移

`data/persist.rs::Snapshot` 的每个字段都能直接对上表：

| 快照字段 | 迁移目标 | 备注 |
| --- | --- | --- |
| `servers` | `servers` + `categories` + `channels` | 内存树要拆成三张表 |
| `messages: HashMap<String, Vec<Message>>` | `messages` | key 是 `server/channel` 拼接串，需换 `channel_id` |
| `dm_messages` | `dm_messages` | key 是好友名，需换 `dm_id` |
| `threads` | `threads` + `thread_replies` | 同上换 id |
| `members` / `roles` / `permissions` | 三张对应表 | `MemberGroup` 由角色 position 推导，不落库 |
| `invites` / `audit` / `bans` / `automod_rules` | 同名表 | 时间从「相对分钟」转绝对时间 |
| `events` / `message_requests` / `forum_posts` | 同名表 | 同上 |
| `bookmarks` | `bookmarks` | **改用 `message_id`**，不再靠正文反查 |
| `welcome_screens` / `welcome_joined` | `welcome_screens` + `welcome_accepts` | 拆分 |
| `boosts` | `boosts` | 等级实时算，不存 |
| `friend_groups` | `friend_groups` | `Friend.group` 变成 `friendships.group_id` |

**一次性导入脚本**：`scripts/import_snapshot.py` 读 `state.json` → 建一个本地用户 → 逐服务器导入 → 把相对时间统一映射成 `now - minutes`。跑完前端只要把 `app.rs` 里读快照的地方换成 HTTP 客户端即可。

---

## 9. 实施顺序

| 阶段 | 内容 | 端点 | 验收 |
| --- | --- | --- | --- |
| **P0** | 认证、用户、服务器、频道、消息、上传、Gateway 基础事件 | 55 | 两端能互发消息、刷新不丢、他人消息实时到 |
| **P1** | 好友/私信/成员/角色/权限/邀请/通知/**LiveKit 语音** | +64 | 双人语音可通、权限拒绝可见 |
| **P2** | 论坛/投票/话题/书签/日程/自动审核/审核日志/加成/欢迎/模板/搜索 | +48 | 逐页对齐 README 功能表 |

**每阶段的硬门槛**

1. `uv run pytest` 全绿，覆盖「越权访问 403」与「自动审核 422」两类负例。
2. `uv run ruff check` 零告警。
3. SQLite 单文件可备份（`VACUUM INTO`），启动自动跑 `alembic upgrade head`。
4. Gateway 断线重连后能靠 `?since=<event_id>` 补齐（事件表保留 24 h）。

---

## 10. 已知风险

| 风险 | 影响 | 处理 |
| --- | --- | --- |
| SQLite 写并发 | 高并发发消息会 `database is locked` | WAL + 单 worker；写热点加内存队列批写；超 500 并发换 Postgres |
| 全文搜索中文 | FTS5 默认分词对中文无效 | `tokenize='trigram'`（体积大）或接 jieba 预处理 |
| Gateway 单进程 | uvicorn 多 worker 后广播不共享 | Redis pub/sub，或坚持单 worker + 房间分片 |
| 附件体积 | 本地磁盘会涨 | 预签名直传对象存储，后端只存元数据；孤儿清理任务 |
| LiveKit 与自建权限 | 用户可能拿旧 token 继续在房间里 | token TTL 2 h + 踢人时调 RoomService 强制移除 |
| 消息 DTO 膨胀 | 当前 `Message` 类型多样（贴纸/GIF/投票/转发） | `type` + `payload` 单表承载，不建 N 张表 |
