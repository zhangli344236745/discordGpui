# discord-gpui-server

`docs/backend-api.md` 里 167 个接口的**可运行骨架**：uv + FastAPI + SQLite（WAL）+ LiveKit。

## 跑起来

```bash
cd server
uv sync
uv run alembic upgrade head          # 建 50 张表
uv run uvicorn app.main:app --reload --port 8080
```

- 接口文档：<http://127.0.0.1:8080/docs>
- 健康检查：`GET /api/v1/health`
- 端到端冒烟：先起服务，再 `uv run python scripts/smoke.py http://127.0.0.1:8080`

## 已经能用的部分（P0）

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
| 语音 | `/channels/{id}/voice/join|leave|participants`、`/voice/{room}/me`、`/voice/{room}/participants/{id}` | 自签 LiveKit JWT（room `srv:{server}:{channel}`） |
| 回调 | `/livekit/webhook` | 校验 JWT + body sha256，同步 `voice_participants` 并广播 |
| 实时 | `WS /ws/gateway?token=` | 订阅制广播：只推已订阅的 server/dm |
| 系统 | `/health` `/config` `/gifs` `/gifs/search` `/help/articles` | GIF 与帮助文档仍是静态数据 |

**服务端真正执行的规则**：13 项权限矩阵（`services/permissions.py`）、自动审核关键词（block→422 不落库，warn→落库带 warnings）、`@everyone` 基线、至少保留一个文字频道、服务器所有者不能直接离开。

## 还是占位（P1 / P2）

101 个端点已注册进 OpenAPI，但统一返回 `501 NOT_IMPLEMENTED`，并在 `summary` 里标了对应文档小节（好友、私信、论坛、角色与成员管理、邀请、审核日志、日程、欢迎屏幕写入、加成、搜索、草稿、通知、私信通话）。

替换方式：把 `app/api/v1/stubs.py` 里对应条目删掉，在 `app/api/v1/` 新建模块实现并在 `__init__.py` 注册即可。

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
│   ├── models/          # 50 张表，按域拆 11 个文件
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
uv run pytest -q         # 21 passed
```

测试覆盖：注册登录/重复邮箱/无 token、偏好往返、建服（默认与模板）、建频道重名、最后一个文字频道保护、发消息与分页、nonce 幂等、自动审核拦截、外部用户 404、无权建频道 403、编辑留历史、上传并挂到消息、501 占位、LiveKit token claims、语音进出与麦克风状态。

## SQLite 注意事项

- 生产用 `uvicorn --workers 1`；多进程会 `database is locked`（WAL 只解决读写并行，不解决多写者）。
- 备份：`sqlite3 data/app.db "VACUUM INTO 'backup.db'"`。
- 全文搜索（`/search`）还没实现，接入时建议 FTS5 + `tokenize='trigram'`。
