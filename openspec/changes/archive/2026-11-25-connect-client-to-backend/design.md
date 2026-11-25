# Design

## Context

客户端是 63 个文件、约 1.3 万行的 GPUI 应用，当前所有状态来自 `data/seed.rs`，由 `data/persist.rs` 的 JSON 快照持久化。`app.rs` 的 `DiscordApp::new()` 先 `persist::load()`，失败才用种子；消息、好友、角色都以显示名/字符串键索引，`Message` 没有 id，书签靠正文反查。工程没有 tokio、reqwest 或任何异步运行时依赖，`gpui-kit` 直接 re-export `gpui`（含 `cx.spawn`、`Timer`、`background_executor`）。

后端契约见 `docs/backend-api.md`：REST 167 个操作、30 类 Gateway 事件、access 15 分钟 / refresh 30 天。服务端已全部可用。

## Goals / Non-Goals

**Goals:**

- 在线模式下所有读写走后端，两个客户端能实时互见。
- 后端未启动时保留完全可用的离线演示，不阻塞 UI 开发。
- 网络 I/O 不阻塞 GPUI 主线程；断线、token 过期、限流都有明确 UI 状态。
- 书签以消息 id 为身份，跨频道/私信可跳转。
- 每阶段可编译、可回退；旧快照不被破坏。

**Non-Goals:**

- 不实现 LiveKit WebRTC 音频/视频收发（只接 token 与呼叫状态）。
- 不做离线写队列/冲突合并；离线模式只演示。
- 不引入 tokio、不重写 UI 渲染层、不把 166 个端点一次性全部接完。
- 不做多账号同时在线或端到端加密。

## Decisions

### D1. 网络栈：阻塞 HTTP/WS + `async-channel` + 独立线程

- REST 用 `ureq`（`json` + rustls），WebSocket 用 `tungstenite`（rustls），都不依赖 tokio。
- 一个 `net` 工作线程顺序执行 REST 命令；一个独立 WS 线程负责连接、心跳与重连。两个线程都只做 I/O 与 DTO 解析。
- UI 与线程之间用 `async_channel`: `Command` 下行、`NetEvent` 上行；`DiscordApp` 用 `cx.spawn` 起一个消费循环 `while let Ok(event) = rx.recv().await { apply; cx.notify() }`，不在 UI 线程阻塞。
- 每个请求带 `request_id` 与 `PendingKind`，响应回到 UI 后按 id 完成对应状态迁移，避免多个请求互相覆盖。

替代方案：`reqwest` + tokio。否决原因：引入第二套运行时与较大依赖树，且 GPUI 已有执行器；阻塞调用放在专用线程已足够。

### D2. 网络层分层

```text
src/net/
├── mod.rs        # NetHandle / AppMode / NetEvent 公共类型
├── worker.rs     # REST 线程、token 刷新、超时、错误映射
├── ws.rs         # Gateway 线程、心跳、退避重连、?since
├── dto.rs        # 与 backend-api.md 对应的 serde 结构
├── api.rs        # 按域分组的 typed 请求构造（不直接碰 UI）
├── hydrate.rs    # DTO -> data::models 的映射与排序
└── reducer.rs    # GatewayEvent -> DiscordApp 状态变更
```

`api.rs` 只产出 `Command::Http { method, path, body, expect }`；`expect` 指定响应如何解析（单对象/列表/无内容），避免 UI 里散落 URL。

### D3. 认证与 token

- `Session { access, refresh, expires_at }` 只存内存；worker 在请求前检查剩余有效期，<30 秒先刷新。
- 收到 401 `TOKEN_EXPIRED` 时刷新一次并重放；并发请求共用同一把刷新锁，避免多次刷新。
- 刷新失败清空会话并推 `NetEvent::SessionExpired`，UI 回登录页。
- “记住我”时把 refresh token 写到 `~/.config/discord-gpui/session.json`（Unix `0600`）；登录/注册/退出都会更新或删除。密码永不落盘。
- WS 使用 access token；重连前确保 token 有效；Gateway 4001 触发刷新或登出。

### D4. 模型稳定身份

给核心模型补 `id: String`（`#[serde(default)]`，兼容旧快照）：

- `Server.id`、`Category.id`、`Channel.id`、`Message.id`、`Friend.id`、`DmConversation.id`、`Member.user_id`、`Role.id`。
- `Bookmark` 改为 `target_kind: BookmarkKind` + `target_id: String` + `scope: String`（频道 id / dm id）；`text` 仅作缓存展示，不参与查找。
- `Message.delivery` 增加 `Sending` / `Failed` 状态，用于乐观发送。

现有以显示名/`"server/channel"` 为键的 HashMap 暂不重写；`RemoteIndex` 维护 `name ↔ id` 与 `(conversation, position) ↔ message id` 的双向映射。C1 先用 `RemoteIndex` 保存服务器/频道/成员/角色/好友/私信 id，UI 渲染代码几乎不动；`Message.id` 与 `Bookmark.target_id` 在 C3/C6 随写路径和书签改造补齐。

替代方案：把全部键改成 id。否决原因：会波及 63 个 UI 文件的索引与断言，风险大于收益；列为后续重构。

### D5. 启动、登录与回退

- 启动先读 `DISCORD_API_BASE`（默认 `http://127.0.0.1:8080`），由 worker 做 `GET /health`（3 秒超时）。
- 在线：显示登录/注册页；登录成功执行 `boot_sync()`，完成前显示进度；失败可重试。
- 离线：`AppMode::Demo`，走现有种子/快照路径，顶部显示「离线演示」且所有写操作只改本地。
- 两种模式互斥，不做数据合并；退出登录清空在线状态。

### D6. 首屏同步

顺序：`/users/@me` → `/servers` → 每个 server 的 `/channels` + `/members` + `/roles` + `/permissions` → `/friends` + `/friend-groups` → `/dms` → `/me/channel-state` → `/notifications/unread-counts`。

`hydrate.rs` 把 DTO 映射为 UI 模型：时间标签由 ISO 转相对时间；颜色沿用 `avatar_color`/`icon_color`；消息附件映射为现有 `Attachment`；投票/话题按 `payload` 与后续请求填充。同步完成后 `saved_json` 仅作为缓存，不再回写主状态。

### D7. 实时 reducer

- `ws.rs` 维护 `last_event_id`，重连带 `?since`；收到 `READY` 时清空事件缓存并发 `NetEvent::Ready`，由 UI 重新拉取必要聚合。
- `reducer.rs` 按事件名分派：
  - `MESSAGE_CREATE/UPDATE/DELETE`：按消息 id upsert/移除；`client_nonce` 或临时 id 命中则替换。
  - `MESSAGE_REACTION_*`、`POLL_UPDATE`、`THREAD_*`：更新对应消息/面板。
  - `CHANNEL_*`/`CATEGORY_*`/`SERVER_UPDATE`/`MEMBER_*`/`ROLE_*`/`PERMISSIONS_UPDATE`：增量更新；权限事件触发频道树重拉。
  - `PRESENCE_UPDATE`、`NOTIFICATION_CREATE`、`CALL_*`、`VOICE_STATE_UPDATE`、`BOOKMARK_UPDATE`：更新对应 UI 状态。
- 去重：保留最近 256 个 event id 的 `HashSet`；事件按到达顺序应用。
- 未知事件只记 debug 日志，不影响连接。

### D8. 写路径与乐观更新

- 发消息：本地插入 `Sending` + `client_nonce`；HTTP 成功或 `MESSAGE_CREATE` 命中 nonce 时替换为服务端 id；失败置 `Failed` 并保留正文/附件供重试。
- 编辑/删除/回应/置顶/已读/输入中：先发请求，成功响应或事件到达后更新；回应可先本地加减再按响应回滚。
- 管理类操作（角色、权限、邀请、审核、内容）不做乐观更新，以响应为准，失败回滚并提示。
- 所有错误走统一 `Toast`/提示条，不在 UI 线程 panic。

### D9. 书签

- 保存：`PUT /bookmarks/{message_id}`，本地记 `target_kind/target_id/scope`。
- 列表：`GET /bookmarks`，按 `saved_at` 倒序；原消息删除时保留墓碑项。
- 跳转：先切会话，再按消息 id 在已加载窗口查找；找不到就带 `before` 继续分页，最多 20 页；确认不存在则提示。
- 多端同步：`BOOKMARK_UPDATE` 只更新对应项，不整表重建。

### D10. 呼叫与语音

- 呼叫面板调用现有 `dms/{id}/call*`，保存 `call_id/room/token/url`，按 `CALL_*` 事件更新状态。
- 语音频道仍调用 `/voice/join` 拿 token，展示参与者与静音状态；不连接 LiveKit 房间，不做音频采集。
- LiveKit SDK 接入列为后续独立 change。

### D11. 配置、日志与安全

- `DISCORD_API_BASE`、`DISCORD_HTTP_TIMEOUT`（默认 10s）、`DISCORD_DEMO`（强制离线）从环境变量读取，设置页可改 base。
- 日志脱敏：不打印 `Authorization`、refresh token、密码；错误日志带 `request_id`。
- 退出登录、刷新失败、切换账号都会取消 WS、清空 pending 请求与 token。

### D12. 测试与验收

- Rust 单测：DTO 反序列化（用后端真实响应 fixture）、hydrate 映射、event reducer、token 刷新状态机、错误码映射、书签跳转分页。
- 联调：先起 `server/`，用脚本登录、发消息、开两个客户端或 WS 客户端验证双向实时；`cargo test` 不依赖网络（fixture）。
- 手工验收：登录/退出、断网重连、token 过期、429、403、书签跨频道跳转、私信请求。
- 每个阶段 `cargo check` + `cargo test` 零警告（沿用现有验收标准）。

## Risks / Trade-offs

- **模型 id 改造波及面大** → 只给核心模型加 `id` 且 `serde(default)`，先编译通过再逐域接线；`RemoteIndex` 隔离键变化。
- **阻塞线程与 GPUI 执行器混用** → 所有 I/O 只在 net 线程，UI 只 `await` channel；禁止在 `cx.spawn` 里做同步网络调用。
- **166 端点范围蔓延** → 分 6 个阶段，每阶段独立可合并；未接的界面继续用在线只读数据或禁用入口，不假装成功。
- **事件乱序/重复** → event id 去重 + `?since` + `READY` 全量重建；乐观消息以 nonce 对齐。
- **单 worker 后端** → 客户端串行请求 + 超时，避免并发打爆；服务端仍是 Demo 规模。
- **LiveKit 期望落差** → UI 明确「已接通（Demo 无音频）」，音频另立 change。
- **快照污染** → 在线模式不写快照；旧快照只在 Demo 模式读取，退出在线不清除，避免丢演示数据。

## Migration Plan

1. C0：加依赖与 `net/` 骨架、DTO、worker、错误类型；`cargo check` + DTO 单测。
2. C1：登录/注册/退出 UI + token 刷新 + 启动探测 + Demo 回退。
3. C2：首屏同步与 hydration；在线模式不再读种子。
4. C3：Gateway + 消息写路径 + reducer；实现双端实时。
5. C4：好友/私信/消息请求/呼叫接线。
6. C5：服务器治理与内容工具接线。
7. C6：书签 id 化、快照降级为缓存、文档与最终验收。

回退：`DISCORD_DEMO=1` 强制离线；代码层面每个阶段独立提交，必要时 revert 单阶段。数据层面不改旧快照文件结构（只加字段），可随时回到 Demo。

## Open Questions

- LiveKit 音频接入用官方 Rust SDK 还是自建 WebRTC，留到独立 change；本轮只消费 token。
- 是否需要在多窗口间共享一个 `NetHandle`（当前单窗口，暂不需要）。
