# Proposal

## Why

`server/` 的 166 个端点已全部可用，但 Rust/GPUI 客户端仍只读写本地 JSON 快照：没有登录、没有网络层，所有消息、好友、角色都是种子数据。结果是「两个客户端无法互相看到消息」「书签靠正文反查」「权限只在界面演示」。需要把客户端接到真实后端，同时保留一份可离线演示的种子回退，避免后端未启动时界面不可用。

## What Changes

- 新增 `src/net/`：后台网络 worker（REST + WebSocket）、DTO、错误映射、token 刷新、连接状态机。
- 新增登录/注册界面：邮箱 + 密码（注册含用户名），启动时先登录；支持退出登录。
- 启动同步：`READY` + `/users/@me`、`/servers`、频道树、成员、角色、`/dms`、`/friends`、未读计数与水合（hydrate）到现有 UI 状态。
- 写操作接线：消息发送/编辑/删除/回应/置顶/已读/输入中、好友与分组、私信与消息请求、服务器设置/成员/角色/权限/邀请、论坛/投票/话题/日程/欢迎/加成/搜索/通知。
- 实时：`/ws/gateway` 订阅、心跳、断线重连 + `?since` 补偿、事件 reducer 应用到 UI。
- 模型稳定身份：给 Server/Channel/Message/Friend/DM 等补后端字符串 id；书签改存 `target_kind + target_id`，跳转不再按正文反查。
- 端点与 Demo 开关：`DISCORD_API_BASE` 可配置；后端不可达或未配置时回退到现有快照种子模式，并显示「离线演示」。
- 语音/视频：接 `voice/join`、`dms/{id}/call*` 与 LiveKit token，UI 状态真实化；**不引入 LiveKit WebRTC SDK**，音频收发仍为非目标。
- 文档：`agent.md` 增加客户端网络层规范；README 更新运行方式（先起 server 再起客户端）。

**BREAKING**：本地快照不再作为主数据源。升级后首次启动要求登录；旧快照只作为离线演示与只读缓存，不参与写操作。

## Capabilities

### New Capabilities

- `client-connectivity`：后端地址配置、REST worker、登录态与 token 刷新、错误映射、超时与离线回退。
- `client-session-boot`：登录/注册/退出界面，首屏同步用户、服务器、频道、成员、角色、好友、私信与未读。
- `client-realtime`：Gateway 连接、订阅、心跳、`?since` 重连、事件 reducer、消息写路径与幂等。
- `client-social`：好友/分组/屏蔽、私信与消息请求、呼叫面板的真实端点接线。
- `client-server-content`：服务器治理（成员/角色/权限/邀请/审核）与内容（论坛/投票/话题/日程/欢迎/加成/搜索/通知）的写路径。
- `client-bookmarks`：消息稳定 id、书签跨频道/私信保存与跳转、原消息删除后的墓碑行为。

### Modified Capabilities

（`openspec/specs/` 里没有客户端能力；后端规格不变。）

## Impact

- **代码**：新增 `src/net/{mod,client,dto,worker,events,mapping}.rs`、`src/ui/login.rs`；`src/app.rs` 增加会话状态与水合/事件应用；`src/data/models.rs` 补 id；`src/data/persist.rs` 降级为离线缓存。
- **依赖**：新增 `ureq`（REST）与 `tungstenite`（WS），均使用 rustls；不引入 tokio。`Cargo.lock` 会变化。
- **接口**：消费既有 166 个端点与 30 类事件，不改后端契约；`/books` 相关改用消息 id。
- **安全**：密码不落盘；refresh token 默认仅内存，「记住我」时写 `0600` 文件；退出登录清 token 与用户缓存。
- **测试**：Rust 单测覆盖 DTO 解析、映射、事件 reducer、token 刷新与错误映射；联调脚本起 server 后跑登录→发消息→WS 回显→书签跳转。
