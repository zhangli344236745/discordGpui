# Proposal

## Why

`server/` 已经落地 P0（认证、用户、服务器、频道、消息、附件、频道语音、Gateway 基础广播），但 `app/api/v1/stubs.py` 里仍有 **101 个端点统一返回 501**，覆盖好友、私信、成员与权限、邀请、审核、日程、内容工具、搜索、通知和私信通话。客户端界面已经能操作这些功能，接口却是空壳，P1/P2 无法联调。现在需要一份可执行的、按批次验收的实现计划，把 501 占位逐个替换为与 `docs/backend-api.md` 契约一致的真实实现，并补齐实时事件、负例测试与文档同步。

## What Changes

- 按 `docs/backend-api.md` 第 3 节的 15 个未实现小节，替换 `stubs.py` 中的 101 个 501 端点：
  - 3.3 好友（10）、3.4 好友分组（6）：好友请求、双向好友、分组排序、屏蔽。
  - 3.10 私信与消息请求（10）、3.21 私信通话（4）：DM 复用、私信消息、消息请求三态、呼叫状态机。
  - 3.12 成员/角色/权限（15）、3.13 邀请（5）、3.14 审核与自动审核（7）：成员管理、角色层级、权限矩阵写入、频道覆盖、封禁、邀请生命周期、规则 CRUD、审核日志读写。
  - 3.9 投票/话题/书签（10）、3.11 论坛（8）：投票互斥与结束、话题回复、书签跨端同步、帖子排序与锁定。
  - 3.15 日程（6）、3.16 欢迎屏幕（5）、3.17 加成（4）：活动状态机、欢迎配置与同意状态、加成等级与撤销。
  - 3.18 搜索（3）、3.19 用户级视图状态（4）、3.20 通知（4）：FTS5 搜索与操作符、频道折叠/草稿、通知聚合与未读角标、推送端点注册。
- 把 `app/realtime/events.py` 补齐到文档第 4 节的 **30 类下行事件**，新增统一 `emit` 出口；Gateway 增加 `?since=` 断线补偿与 24 h 事件保留。
- 新增按域拆分的 `services/`（social、dm、guild_admin、invites、content、search、notifications、gateway_log），路由层只做校验与编排。
- 新增 alembic 迁移：`gateway_events` 表、FTS5 虚表与同步触发器、`push_endpoints` JSON 列、以及 DM 规范化唯一索引。
- 测试从 21 个扩到覆盖全部新域的负例（403/404/409/422/429）与实时事件断言；`scripts/smoke.py` 增加 P1/P2 主链路。
- 同步 `docs/backend-api.md`（表数由 50 调整为 51、新增字段与状态枚举）与 `server/README.md`、根 `README.md`、`agent.md` 的实现状态。

**BREAKING**：无。全部是新增端点与新增表/列；P0 既有响应形状不变（仅 `READY` 事件补字段，属向后兼容扩展）。

## Capabilities

### New Capabilities

- `social-graph`：好友请求生命周期、双向好友关系、好友分组与排序、屏蔽与可见性。
- `direct-messaging`：DM 会话复用与规范化、私信消息、消息请求门控、私信语音/视频呼叫状态机。
- `guild-administration`：成员列表与昵称/踢出、角色 CRUD 与层级、权限矩阵与频道覆盖、封禁。
- `guild-growth`：邀请创建/解析/接受、欢迎屏幕配置与同意、服务器日程与报名、加成等级与撤销。
- `content-tools`：投票、话题、书签、论坛帖子与回帖。
- `moderation`：自动审核规则与发送时扫描、审核日志查询与清空。
- `search-and-notifications`：全文搜索与会话内搜索、用户级视图状态与草稿、通知收件箱与未读计数、推送端点。
- `realtime-gateway`：订阅范围授权、事件目录与广播、心跳与断线补偿。

### Modified Capabilities

（无。`openspec/specs/` 目前为空，全部按新能力建立基线。）

## Impact

- **代码**：`server/app/api/v1/`（新增 12+ 路由模块、`stubs.py` 清空）、`server/app/services/`（新增 8 个服务）、`server/app/schemas/`、`server/app/realtime/`、`server/alembic/versions/`、`server/tests/`、`server/scripts/smoke.py`。
- **API**：101 个端点由 501 转为可用；新增 `?since=` 查询参数与 `gateway_events` 游标；`READY` 载荷扩展。
- **数据**：新增 `gateway_events`（第 51 张表）、`messages_fts` 虚表、`user_settings.push_endpoints` 列、`dm_channels(user_a,user_b)` 唯一索引、`bookmarks` 已有主键无需改。
- **依赖**：无新增运行时依赖；`pytest-asyncio` 与 `ruff` 已就绪。FTS5 使用 SQLite 内置模块，无外部依赖。
- **运维**：仍要求 `uvicorn --workers 1`；新增 24 h 事件清理与孤儿附件/过期邀请定时任务。
- **客户端**：Rust 侧本轮不改；按端点形状接线，`bookmarks` 改用 `message_id` 后客户端可移除正文反查。
