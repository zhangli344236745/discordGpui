# discord-gpui-server

`docs/backend-api.md` 里 167 个接口的**可运行骨架**：uv + FastAPI + SQLite（WAL）+ LiveKit。

## 跑起来

```bash
cd server
uv sync
uv run alembic upgrade head          # 建 51 张表（含 gateway_events）
uv run uvicorn app.main:app --reload --port 8080
```

- 接口文档：<http://127.0.0.1:8080/docs>
- 健康检查：`GET /api/v1/health`
- 端到端冒烟：先起服务，再 `uv run python scripts/smoke.py http://127.0.0.1:8080`

## 已经能用的部分（全部批次）

| 域 | 端点 | 说明 |
| --- | --- | --- |
| 认证 | `/auth/register` `/login` `/refresh` `/logout` `/password` `/sessions` | argon2 密码、JWT access 15min + refresh 30d 轮换、会话可踢 |
| 用户 | `/users/@me` `/@me/settings` `/@me/status` `/{id}` `/mutual` | 改资料、偏好、在线状态（推给好友） |
| 服务器 | `/servers` CRUD、`/leave`、`/templates`、`/templates/apply`、`/order` | 建服自动铺 `@everyone` 角色 + 分类频道 + 欢迎屏幕；4 个模板与前端同源 |
| 探索 | `/discover/servers`、`/discover/servers/{id}/join` | 返回 welcome 配置供客户端弹窗 |
| 频道 | `/servers/{id}/channels`、分类与频道 CRUD、`/notify`、`/read` | 树形返回；通知级别与已读位置按用户存 |
| 消息 | `/channels/{id}/messages`、单条 CRUD、`/edits`、`/pin`、`/pins` | 游标分页、`client_nonce` 幂等、编辑留历史、软删 |
| 回应 | `/messages/{id}/reactions` | 增删均广播 |
| 转发 | `/messages/{id}/forward` | 频道间转发（私信转发 501） |
| 附件 | `/uploads` → `PUT /uploads/{id}/content` → `/attachments/{id}` | 本地磁盘存储，接口形状与 S3 预签名一致 |
| 好友 | `/friends` `/friends/online` `/friends/requests` `/friends/{id}` `/blocks` | 请求去重、双向好友、分组排序、屏蔽后不可私信 |
| 私信 | `/dms` `/dms/{id}/messages` `/dms/{id}/read` `/message-requests` | 规范化会话复用、`client_nonce` 幂等、未读、陌生人消息请求门控 |
| 通话 | `/dms/{id}/call` `/call/accept` `/call/decline` `/call/hangup` | 状态机 ringing→active→ended、60s 超时、只给参与者签 token |
| 成员/角色 | `/servers/{id}/members` `/roles` `/permissions` `/channels/{id}/overrides` | 昵称、踢出、角色层级、13 项三态矩阵、频道覆盖 |
| 邀请 | `/channels/{id}/invites` `/invites/{code}` `/invites/{code}/accept` | 过期/用尽/撤销、封禁拦截、原子占名额 |
| 审核 | `/servers/{id}/automod/rules` `/automod/test` `/servers/{id}/audit` | 规则 CRUD、发送/编辑时 block→422 warn→warnings、日志过滤与 owner 清空 |
| 限流 | 全局 | 消息 5/5s、登录 10/60s，429 带 `Retry-After` |
| 内容 | `/channels/{id}/polls` `/threads` `/bookmarks` | 单选替换/多选切换、话题回复计数、频道与私信书签（原消息删除显示墓碑） |
| 论坛 | `/channels/{id}/posts` `/posts/{id}` `/channels/{id}/tags` | 置顶恒前、最新/最热/未回应、锁定后回帖 403、标签 ≤5 |
| 运营 | `/servers/{id}/events` `/welcome` `/boosts` | 活动状态推导与报名、欢迎配置与同意、加成等级 2/7/14 |
| 搜索 | `/search` `/channels/{id}/search` `/search/suggest` | FTS5 trigram、scope/`from:`/`has:`/`after:`/`before:`、可见性过滤、补全 |
| 视图状态 | `/me/channel-state` `/me/drafts` | 折叠/通知/已读只前进、跨设备草稿 |
| 通知 | `/notifications` `/notifications/read` `/notifications/unread-counts` `/push/subscribe` | 提及/私信/好友请求聚合、批量已读、角标、推送端点（Demo 只存） |
| 实时增强 | `WS /ws/gateway?since=` | 60s 心跳、token 过期关闭、订阅校验、24h 事件补发 |
| 语音 | `/channels/{id}/voice/join|leave|participants`、`/voice/{room}/me`、`/voice/{room}/participants/{id}` | 自签 LiveKit JWT（room `srv:{server}:{channel}`） |
| 回调 | `/livekit/webhook` | 校验 JWT + body sha256，同步 `voice_participants` 并广播 |
| 实时 | `WS /ws/gateway?token=` | 订阅制广播：只推已订阅的 server/dm |
| 系统 | `/health` `/config` `/gifs` `/gifs/search` `/help/articles` | GIF 与帮助文档仍是静态数据 |

**服务端真正执行的规则**：13 项权限矩阵（`services/permissions.py`）、自动审核关键词（block→422 不落库，warn→落库带 warnings）、`@everyone` 基线、至少保留一个文字频道、服务器所有者不能直接离开。

## 占位端点（已清零）

101 个占位已全部替换（`stubs.py` 已删除）。唯一保留的 501 是 `/servers/{id}/boost/subscribe`（Demo 不接计费）。

新增端点的方式：在 `app/api/v1/<域>.py` 实现、必要时抽到 `services/`，在 `app/api/v1/__init__.py` 注册，并补 pytest 与 smoke 链路。

## 结构

```text
server/
├── app/
│   ├── main.py          # app 工厂、CORS、异常处理、静态 /media
│   ├── config.py        # pydantic-settings（读 .env）
│   ├── db.py            # async engine + SQLite PRAGMA（WAL / busy_timeout）
│   ├── deps.py          # current_user / server_context / channel_context
│   ├── errors.py        # ApiError + 统一错误体
│   ├── ids.py           # 雪花 ID / 邀请码 / 4 位 tag
│   ├── security.py      # argon2 + JWT
│   ├── models/          # 51 张表，按域拆 12 个文件
│   ├── schemas/         # Pydantic DTO
│   ├── services/        # permissions / automod / boost / audit / storage / templates
│   ├── realtime/        # hub / events / gateway
│   ├── api/v1/          # 路由（已实现 9 个模块 + stubs）
│   └── livekit/         # AccessToken 签发（手写 HS256，与官方 SDK 等价）
├── alembic/             # 初版迁移（50 表）
├── scripts/smoke.py     # 端到端冒烟
└── tests/               # 21 个用例
```

## 验证

```bash
uv run ruff check .      # 0 error
uv run pytest -q         # 72 passed
```

测试覆盖：注册登录/重复邮箱/无 token、偏好往返、建服（默认与模板）、建频道重名、最后一个文字频道保护、发消息与分页、nonce 幂等、自动审核拦截、外部用户 404、无权建频道 403、编辑留历史、上传并挂到消息、501 占位、LiveKit token claims、语音进出与麦克风状态；P1/P2 地基覆盖 FTS 同步、outbox 提交/回滚、频道权限覆盖、层级校验、定时清理、alembic 升降级往返；B1 覆盖好友请求/分组/屏蔽、私信幂等与未读、陌生人消息请求、呼叫状态机与超时；B2 覆盖成员/角色层级、权限矩阵与频道覆盖、封禁拦截、邀请三态、自动审核 422/warn、审核日志与限流；B3 覆盖投票计票、话题计数、书签墓碑、论坛排序/锁定、活动状态、欢迎同意、加成等级；B4 覆盖全文搜索/操作符/权限过滤、视图状态、草稿、通知与未读、推送注册、并发同 nonce/邀请名额。

## SQLite 注意事项

- 生产用 `uvicorn --workers 1`；多进程会 `database is locked`（WAL 只解决读写并行，不解决多写者）。
- 备份：`sqlite3 data/app.db "VACUUM INTO 'backup.db'"`。
- 全文搜索（`/search`）还没实现，接入时建议 FTS5 + `tokenize='trigram'`。

## 客户端

Rust/GPUI 客户端主体已接入本后端（登录/实时/治理/内容域），详见根 README「客户端接后端」。
