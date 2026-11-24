# Design

## Context

见 `proposal.md`。当前 `server/` 的约束：

- FastAPI + SQLAlchemy 2.0 async + aiosqlite，单进程 `uvicorn --workers 1`；50 张表已由 alembic 初版迁移建好，`get_db()` 只提供 session，提交由路由显式执行（43 处 `await db.commit()`）。
- 路由层已有成熟范式：`deps.current_user / server_context / channel_context`、`services/permissions.effective_permissions()`、`services/automod.scan()`、`services/audit.log()`、`services/boost.boost_stats()`、`realtime.hub` 与 `Event` 常量。P1/P2 端点全部集中在 `app/api/v1/stubs.py`，统一 501。
- `Hub` 是进程内内存订阅表，按 `servers` / `dms` 两个集合过滤；尚无持久化事件、无心跳超时、无 `since` 补偿。
- 测试 21 个，`tests/conftest.py` 提供独立 DB 与环境；`httpx.AsyncClient` 不支持 WebSocket。
- 客户端（Rust/GPUI）本轮不改；文档 `docs/backend-api.md` 是唯一契约。

## Goals / Non-Goals

**Goals:**

- 101 个 501 端点全部有真实实现或**显式**的 Demo 边界（仅 `/boost/subscribe` 保留 501），且每个域有负例测试。
- 路由薄、服务厚：新增业务规则集中在 `services/`，路由只做参数校验、依赖鉴权和 DTO 映射。
- 实时事件与数据库变更同事务登记，提交后广播；重启后可靠 `?since=` 在 24 h 内补齐。
- 用 4 个可独立合并、独立验收的批次交付，每个批次结束时 `ruff` / `pytest` / `smoke.py` 全绿。

**Non-Goals:**

- 不引入 Postgres、Redis、Celery、jieba 或任何新运行时依赖。
- 不做真实推送投递、订阅计费、邮件发送（验证码/重置邮件仍写库+日志）。
- 不做多进程广播、跨地域 LiveKit 调度、客户端 Rust 接线。
- 不重构 P0 已实现的 66 个端点响应形状；只允许向后兼容的字段扩展。

## Decisions

### D1. 交付形态：一个 change、四个可合并批次

批次按依赖排序，每批独立跑门禁，但共用本设计中的事件/迁移基础设施：

| 批次 | 范围 | 端点 |
| --- | --- | --- |
| B1 社交与私信 | 3.3 / 3.4 / 3.10 / 3.21 | 30 |
| B2 服务器治理 | 3.12 / 3.13 / 3.14 | 27 |
| B3 内容与运营 | 3.9 / 3.11 / 3.15 / 3.16 / 3.17 | 33 |
| B4 检索与实时 | 3.18 / 3.19 / 3.20 + Gateway 增强 | 11 + WS |

替代方案是把每个小节拆成独立 OpenSpec change。否决原因：`gateway_events`、FTS、权限覆盖、书签身份这四项是跨域地基，拆开后每个 change 都要重复迁移与事件改造，归档时互相阻塞。

### D2. 模块与服务划分

新增路由模块：`friends.py`、`friend_groups.py`（或合并 `social.py`）、`dms.py`、`message_requests.py`、`calls.py`、`members.py`、`roles.py`、`invites.py`、`moderation.py`、`content.py`（polls/threads/bookmarks）、`forums.py`、`events.py`、`welcome.py`、`boosts.py`、`search.py`、`me_state.py`、`notifications.py`。

新增服务：`services/social.py`、`services/dm.py`、`services/guild_admin.py`、`services/invites.py`、`services/content.py`、`services/search.py`、`services/notifications.py`、`services/outbox.py`。

约定：服务函数签名统一 `(db, actor, ...) -> ORM/DTO`；抛 `ApiError`；不 import FastAPI。`stubs.py` 每批删除对应行，最后整个文件删除并在 `api/v1/__init__.py` 移除注册。

替代方案：把逻辑继续写在路由里（P0 风格）。否决原因：权限矩阵、层级校验、事件登记在 15 个域重复 3–5 次，测试无法按服务单测。

### D3. 权限求解：补上频道覆盖与层级校验

`effective_permissions()` 目前只做「基线 ∪ 角色 allow − deny」，未读 `channel_overrides`。改造为：

```text
owner/ADMINISTRATOR 短路
→ 基线
→ 角色 grant（按 role.position 升序，deny 记账）
→ 频道覆盖（allow 加入、deny 记账，deny 最终优先）
→ 移除 deny
```

新增 `visible_channel_ids(db, server_id, user_id)` 批量版，供频道树、搜索、通知使用，避免逐频道算权限。新增 `top_role_position()` 已有，补 `assert_can_act(actor_top, target_top)`：目标 position ≥ 操作者 position → 403；owner 不可被踢/封/改角色。

替代方案：每次请求重算整棵频道树。否决原因：搜索与未读计数会触发 N+1。

### D4. 事件 outbox：同事务登记、提交后广播、可回放

新增 `gateway_events` 表：`(id TEXT PK, scope_kind TEXT, scope_id TEXT, event TEXT, payload JSON, created_at)`，`id` 为雪花，天然有序。新增 `realtime/outbox.py`：

```python
def queue(db, scope: tuple[str, str], event: str, data: dict) -> str:
    eid = new_id()
    db.add(GatewayEvent(id=eid, scope_kind=..., scope_id=..., event=event, payload=data))
    db.info.setdefault("pending_events", []).append((eid, scope, event, data))
    return eid
```

监听 SQLAlchemy `after_commit`：取出 `pending_events`，用当前事件循环 `create_task(hub.publish(...))`；`after_rollback` 清空。这样事件只在提交后可见，幂等重试不会重复入队（事务里先查 `client_nonce` 命中则早退）。

替代方案：路由 `await db.commit()` 后手写 `hub.*`。否决原因：101 个端点容易漏发/早发；也无法回放。

### D5. Gateway 增强

- `READY` 增加 `user`、`servers`、`dms`、`heartbeat_interval` 与 `event_cursor`（最新事件 id）。
- 连接参数 `?since=<id>`：查 `gateway_events` 中 `scope_kind/scope_id` 属于当前订阅范围、`id > since` 的事件，按 id 升序补发，上限 500 条；超出上限或游标早于 24 h → 只发 `READY`。
- 心跳：`asyncio.wait_for(ws.receive_text(), timeout=60)`；超时关闭。每条上行检查 token `exp`，过期以 4001 关闭。
- `SUBSCRIBE` 只接受当前成员关系内的 server 与参与的 DM；其余静默忽略。
- 30 类事件常量补齐；`Event` 保持字符串常量，避免枚举序列化差异。
- 保留任务：启动时与每小时删除 `created_at < now-24h` 的事件；与过期邀请、孤儿附件共用一个 `tasks/` 调度器（`asyncio.Task` 循环，lifespan 启停）。

替代方案：事件只放内存环形缓冲。否决原因：重启后无法补偿，且不能保证跨批次一致。

### D6. DM 与消息请求

- `dm_channels` 加唯一索引 `ux_dm_pair(user_a,user_b)`；写入前排序 `user_a < user_b`。复用查询用 `or_(and_(a,b), and_(b,a))` 兼容历史数据。
- `dm_channels` 增加 `a_last_read_id`、`b_last_read_id`、`a_pinned`、`b_pinned` 四列，避免新表；已读与置顶按当前用户所在侧读写。
- 消息请求门控：`can_dm_directly = 是好友 OR 有共同服务器 OR 已有 DM`。否则 `POST /dms` 返回 409 并提示改用 `POST /message-requests`；`POST /dms/{id}/messages` 不承担建请求职责。接受请求时在同一事务内建 DM + 把 `first_message` 写成一条 `client_nonce=f"req:{request_id}"` 的私信，保证幂等。
- 呼叫：`call_sessions` 状态机 `ringing → active → ended`，拒接直接 `ended` 并记录 `decline` 结果列（新增 `outcome` 列）。ring 超时 60 s 由任务置为 `ended`。token 复用 `livekit/token.py`，房间 `dm:{call_id}`，只给参与者签发。

### D7. 邀请与加入

- 邀请码沿用 `ids.new_code()`；`expires_at` 由 `expiry` 枚举（`1h/1d/7d/never`）换算。
- 接受时用原子更新 `UPDATE invites SET uses=uses+1 WHERE code=? AND revoked=0 AND (max_uses IS NULL OR uses<max_uses) AND (expires_at IS NULL OR expires_at>now)`，`rowcount=0` → 404；成功后再建成员关系。重复加入先查成员关系直接返回成功，不增加 uses。
- 封禁检查在邀请接受与探索页加入两处复用 `services/guild_admin.is_banned()`。
- 邀请预览要求登录（避免公开枚举），返回 `valid`、服务器摘要、成员/在线数；不泄露频道列表。

### D8. 内容工具

- 投票：消息 `msg_type="poll"`、`payload={"poll_id":...}`；创建消息与 `polls/poll_options` 同事务。单选投票先 `DELETE FROM poll_votes WHERE poll_id=? AND user_id=?` 再插入；多选按 `(poll_id,option_id,user_id)` 切换。关闭写 `closed=true` 并广播 `POLL_UPDATE`。
- 话题：`threads.message_id` 已有唯一约束；回复用 `UPDATE threads SET reply_count=reply_count+1` 原子自增；`GET /threads/{id}` 返回原消息 + 最近 50 条回复（`before` 可选）。
- 书签：把表改为通用目标 `bookmarks(user_id, target_kind, target_id, scope, note, created_at)`，主键 `(user_id,target_kind,target_id)`；迁移时把旧 `message_id` 写入 `target_kind='channel'`。DM 书签 `target_kind='dm'`，`scope` 记录会话 id 供跳转。
- 论坛：`reply_count` 与 `last_activity_at` 由回帖/删回帖事务内维护；`unanswered` = `reply_count=0`；`popular` = `reply_count DESC, last_activity_at DESC`；标签接口对 `forum_tags JOIN forum_posts` 按频道聚合 `COUNT(*)`，取前 20。
- 锁定帖子回帖 403 `MISSING_PERMISSION`；作者或 `MANAGE_MESSAGES` 可编辑/删除。

### D9. 审核、日志与限流

- 复用 `automod.scan()`；在消息创建与编辑两处调用。`block` → 422（附 `rule_id`、`matched_keyword`，不落库）；`warn` → 落库并在响应加 `warnings` 数组。私信不扫描。
- 规则写接口要求 `MANAGE_SERVER`；关键词去重、非空；`/automod/test` 只读。
- 审核日志：`GET` 游标 `before` + `kind` 过滤 + `has_more`；`DELETE` 仅 owner。所有管理写操作调用 `audit.log()`（已有），并把 `AUDIT_ENTRY_CREATE` 入 outbox。
- 限流：新增 `services/ratelimit.py`，内存滑动窗口 `dict[key, deque[timestamp]]`；消息键 `msg:{user_id}`（5/5s），登录键 `login:{ip}`（10/60s）。超限抛 429，响应头 `Retry-After`，错误体附 `retry_after`。仅单进程有效，已写入风险表。

### D10. 搜索

- FTS5 使用 external-content 表，避免正文双份存储：

```sql
CREATE VIRTUAL TABLE messages_fts USING fts5(
  content, content='messages', content_rowid='rowid', tokenize='trigram'
);
-- INSERT/UPDATE/DELETE 三个触发器同步；dm_messages_fts 同理
```

- trigram 要求查询串 ≥ 3 字符；不足 3 字符退化为 `LIKE '%q%'`。查询串按 FTS5 语法转义（`"` 包裹、内部 `"` 双写），操作符在服务层解析为结构化条件，不直接拼 SQL。
- 可见性：先取 `visible_channel_ids()` 与参与的 DM 集合，SQL 里 `IN (...)`；`scope=server` 再按 server 过滤。
- 排序：消息按 `created_at DESC`（trigram 无相关性评分）；`/search/suggest` 用前缀匹配 + 类型权重（频道 > 用户 > 命令）。
- `highlight` 由服务层对命中片段做大小写不敏感包裹，返回 `{before, match, after}`，避免客户端解析 HTML。

### D11. 通知与未读

- 写入时机：消息提及（`@user`、有 `MENTION_EVERYONE` 的 `@everyone`）、私信新消息、好友请求、消息请求。
- `notifications` 已有 `kind/target_kind/target_id/read`；列表按 `(user_id, read, id DESC)` 索引分页；批量已读只更新 `user_id=当前用户` 的行。
- 未读计数：频道用 `read_states.last_read_message_id` 与消息 id 比较；DM 用 `dm_channels` 的两个已读列；好友请求数查 `friend_requests(state=pending)`。全部一次聚合返回，避免客户端 N 次请求。
- 推送：`user_settings` 加 `push_endpoints JSON` 列（默认 `[]`），`POST /push/subscribe` 按 endpoint 去重追加；Demo 不投递，响应 `{subscribed:true, delivery:"demo"}`。

### D12. 测试与门禁

- 每个域新增 `tests/test_<domain>.py`；共享 fixture 造两个用户、一个服务器、一个频道、一个 DM。
- 负例矩阵固定覆盖：越权 403、不可见 404、重复 409、校验 400、自动审核 422、限流 429。
- WebSocket 用 `starlette.testclient.TestClient`（httpx 不支持 ws），断言 `READY`、`MESSAGE_CREATE`、订阅过滤、`since` 补发与心跳超时（用短超时可注入配置）。
- `scripts/smoke.py` 扩为两条链路：B1 好友→私信→消息请求→呼叫；B3 邀请→欢迎→活动→投票→书签。
- 每批门禁：`uv run ruff check .` 0 error、`uv run pytest -q` 全绿、真实 uvicorn + smoke 通过、`openspec validate --strict` 通过。

### D13. 文档与契约同步

- `docs/backend-api.md`：表清单 50 → 51（`gateway_events`），补 `dm_channels` 已读/置顶列、`user_settings.push_endpoints`、`bookmarks` 通用目标、`call_sessions.outcome`；3.9 书签说明改为消息 id；第 9 节验收维持。
- `server/README.md` 更新完成度表与验证命令；根 `README.md` 与 `agent.md` 更新「客户端还没接后端」的现状与端点计数。
- 每批完成后重跑文档中的计数自检脚本，避免数字不一致。

## Risks / Trade-offs

- **trigram 索引体积**：external-content 已避免正文双份，但 trigram 索引仍可能大于表本身 → 仅对 `content` 建索引，附件/JSON 不入索引；数据量超预期时切 `unicode61` + 分词服务（需换依赖，另行提案）。
- **内存限流/单进程 Hub**：多 worker 后失效 → 继续强制 `--workers 1`，README 与 `config.py` 注释明示；横向扩展需 Redis 与 Postgres，另立 change。
- **`after_commit` 调度任务与响应竞态**：客户端可能在 HTTP 响应前收到事件 → 客户端以事件 id 幂等合并；测试允许两种顺序，只断言最终状态。
- **书签表重建**：SQLite 不支持改主键 → 用「建新表→拷数据→删旧表→改名」迁移；Demo 数据量小，失败可回滚重跑。
- **`since` 补发放大**：单连接最多补 500 条 → 超出退回 `READY`，避免一次推送拖垮连接。
- **邀请并发**：两个请求同时接受最后一个名额 → 原子 `UPDATE ... WHERE uses<max_uses` 保证只有一个成功，另一个 404。
- **权限求解改动的回归**：频道覆盖会改变既有可见性 → 先补测试锁住 P0 行为，再启用覆盖；`PERMISSIONS_UPDATE` 广播后客户端重拉频道树。

## Migration Plan

1. 生成 alembic 修订 `p1p2_foundation`：
   - 建 `gateway_events` 与两个 FTS 虚表 + 触发器；
   - `dm_channels` 加唯一索引与已读/置顶列；
   - `user_settings` 加 `push_endpoints` JSON；
   - `call_sessions` 加 `outcome`；
   - 重建 `bookmarks` 为通用目标表并回填旧行。
2. 本地/CI：`uv run alembic upgrade head`；启动时 `create_all` 仍保留给测试，FTS 与触发器在迁移里创建（测试用 `init_db()` 时也执行同一段 DDL）。
3. 部署：停服 → `VACUUM INTO` 备份 → 升级 → 启动单 worker → 跑 smoke。
4. 回滚：`alembic downgrade -1` 删除新表/列并还原 `bookmarks` 旧结构；Demo 数据可接受丢失新表数据。

## Open Questions

- 推送渠道（WebPush 还是 APNs）在 Demo 之后确定；当前只存端点，不影响本轮端点形状。
- 搜索是否需要相关性排序（BM25）而非纯时间倒序；trigram 下暂不提供，若产品要求再引入 `bm25()` 与权重，不影响现有 DTO。
