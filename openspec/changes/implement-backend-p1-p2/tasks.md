# Tasks

## 1. 地基：迁移、事件与权限

- [x] 1.1 新增 alembic 修订 `p1p2_foundation`：建 `gateway_events` 表与 `(scope_kind, scope_id, id)` 索引；验证 `alembic upgrade head` 与 `downgrade -1` 在临时库可往返
- [x] 1.2 在同一修订中建 `messages_fts`、`dm_messages_fts` external-content 虚表与增删改触发器；验证插入/更新/删除消息后 `MATCH` 结果同步，且 `docs/backend-api.md` 5.2 的索引存在
- [x] 1.3 扩展表结构：`dm_channels` 唯一索引与已读/置顶列、`call_sessions.outcome`、`user_settings.push_endpoints`、`bookmarks` 重建为 `(user_id,target_kind,target_id)`；验证迁移前旧书签行迁移后仍可读
- [x] 1.4 实现 `realtime/outbox.py`：`queue()` 写事件行并登记 session，`after_commit` 广播、`after_rollback` 丢弃；验证单测提交后只广播一次、回滚零广播
- [x] 1.5 改造 `services/permissions.py`：加入频道覆盖求解、`visible_channel_ids()` 批量版、`assert_can_act()` 层级校验；验证新增单测覆盖 allow/deny/inherit 与 owner 短路，P0 既有权限测试仍绿
- [x] 1.6 新增 `services/retention.py` 定时清理（gateway_events 24 h、过期邀请、孤儿附件），接入 FastAPI lifespan；验证单测可注入时钟删除到期行且不误删新行
- [x] 1.7 扩充 `tests/conftest.py`：两名用户、一个服务器、一个文字频道、一个 DM 的工厂与认证头 fixture；验证现有 21 个测试全部通过
- [x] 1.8 同步文档：`docs/backend-api.md` 表数 50→51 与新列、`server/README.md` 完成度；验证第 0/5/9 节计数脚本输出一致

## 2. 批次 B1：社交与私信（30 端点）

- [x] 2.1 实现好友请求发送/列表/接受/拒绝（`3.3` 前 6 项）：自请求 400、重复 409、屏蔽 404；验证 `tests/test_social.py` 四类负例
- [x] 2.2 实现好友列表、在线子集、删除好友（`3.3` 其余）：双向行、状态与分组字段；验证删除后双方列表同步且 `pytest` 通过
- [x] 2.3 实现好友分组 CRUD、整表排序、移动好友（`3.4`）：重名 409、删除回未分组；验证顺序与归属断言
- [x] 2.4 实现屏蔽/取消/列表及其效果（好友请求、DM、在线状态）：幂等；验证屏蔽后建 DM 返回 403、重复调用不新增行
- [x] 2.5 实现 DM 会话创建/复用/列表/置顶/关闭/已读（`3.10` 前 5 与末项）：规范化用户对与唯一索引；验证 A→B 与 B→A 命中同一 `dm_id`
- [x] 2.6 实现私信历史与发送（游标、`client_nonce`、编辑、软删）：验证幂等重试只落一条且只广播一次
- [x] 2.7 实现消息请求列表/接受/忽略/删除与陌生人门控：接受后建 DM 并投递首条；验证好友直发不产生请求、接受幂等
- [x] 2.8 实现私信呼叫发起/接听/拒接/挂断与 60 s 超时（`3.21`）：状态机与只对参与者签发 token；验证非法迁移 409、第三方 404
- [x] 2.9 为 B1 全部写操作接入 outbox 事件（`FRIEND_REQUEST_*`、`DM_CREATE`、`MESSAGE_CREATE`、`CALL_*`）：验证 WS 订阅者收到且非参与者收不到
- [x] 2.10 扩展 `scripts/smoke.py` 的 B1 链路（好友→私信→消息请求→呼叫）并更新 README 好友/私信章节；验证真实 uvicorn 下脚本通过

## 3. 批次 B2：服务器治理（27 端点）

- [x] 3.1 实现成员列表与服务器昵称修改（`3.12` 前 2 项）：空串回退显示名；验证非成员 404、无权限 403
- [x] 3.2 实现踢出、封禁、撤销封禁与 `delete_days` 软删消息：层级与 owner 保护；验证高角色目标 403、封禁后接受邀请 403
- [x] 3.3 实现角色列表/创建/编辑/删除（`3.12` 角色 4 项）：`@everyone` 保护、position 重排、删除清关联；验证 `ROLE_*` 事件各一次
- [x] 3.4 实现角色授予/移除（`assigned` 布尔）：幂等且不可越级；验证重复授予不产生重复行
- [x] 3.5 实现权限矩阵读取/批量写入（只返非 inherit、13 项校验）：验证未知权限 400 且整批不落库、写后权限即时生效
- [x] 3.6 实现频道权限覆盖读取/写入并接入有效权限：deny `VIEW_CHANNEL` 后频道读取与搜索均 404；验证覆盖优先于服务器级
- [x] 3.7 实现邀请创建/列表/撤销/解析/接受（`3.13`）：过期、用尽、撤销三态；验证并发接受最后一个名额只有一个成功
- [x] 3.8 实现自动审核规则 CRUD 与试跑（`3.14` 规则 5 项 + test）：关键词非空去重、动作枚举；验证无权限 403、试跑不写库
- [x] 3.9 实现审核日志查询/清空：kind 过滤、游标、仅 owner 清空；验证清空只影响本服务器、日志不可改
- [x] 3.10 实现消息与登录限流：消息 5/5 s、登录 10/60 s，返回 429 与 `Retry-After` / `retry_after`；验证窗口恢复后放行
- [x] 3.11 把自动审核接入消息创建与编辑（block 422 不落库、warn 带回 `warnings`）：验证编辑被拦时原正文不变
- [x] 3.12 为 B2 管理操作补齐审计留痕与事件（`ROLE_*`、`MEMBER_*`、`PERMISSIONS_UPDATE`、`INVITE_*`、`AUTOMOD_RULE_*`、`AUDIT_ENTRY_CREATE`）；验证回滚时日志与事件都不留
- [x] 3.13 扩展 smoke 的 B2 链路（邀请加入→改角色→权限拒绝→审核日志）并更新文档；验证脚本通过且 README 完成度更新

## 4. 批次 B3：内容与运营（33 端点）

- [x] 4.1 实现投票创建/投票/取消/结束（`3.9` 前 4 项）：单选替换、多选切换、关闭 409；验证票数与 `POLL_UPDATE` 一致
- [x] 4.2 实现话题创建/详情/回复（`3.9` 中间 3 项）：一消息一话题、计数原子自增；验证重复开启 409、非成员 404
- [x] 4.3 实现书签列表/保存/移除与通用目标迁移：频道与 DM 均可保存、删除消息后显示墓碑；验证幂等保存与跨端事件
- [x] 4.4 实现论坛帖子列表/详情/编辑/置顶/锁定/删除（`3.11` 前 5 项）：置顶恒前、三种排序与 `q`；验证他人删除 403
- [x] 4.5 实现论坛回帖、删回帖与频道标签（`3.11` 其余）：锁定后回帖 403、标签 ≤5、热门标签去重降序；验证计数与活跃时间更新
- [x] 4.6 实现日程 CRUD 与报名/取消/名单（`3.15`）：状态由时间推导；验证重复报名幂等、删除清报名
- [x] 4.7 实现欢迎配置读取/编辑/同意/状态/频道欢迎已读（`3.16`）：推荐频道裁剪、强制规则门槛；验证未同意状态与重复同意
- [x] 4.8 实现加成状态/等级/特权/加成撤销与订阅 501（`3.17`）：验证 8 次加成进度按 7→14 区间、撤销后等级下降
- [x] 4.9 为 B3 接入事件（`POLL_UPDATE`、`THREAD_*`、`FORUM_POST_*`、`FORUM_REPLY_*`、`EVENT_*`、`WELCOME_UPDATE`、`BOOST_UPDATE`、`BOOKMARK_UPDATE`）：验证各事件在对应 REST 成功后只发一次
- [x] 4.10 扩展 smoke 的 B3 链路（邀请→欢迎→活动→投票→书签→论坛）并更新文档；验证脚本通过、文档小节计数一致

## 5. 批次 B4：检索、状态与实时（11 端点 + Gateway）

- [x] 5.1 实现 `GET /search`：scope、`from:/has:/after:/before:` 解析、可见性过滤、游标与高亮片段；验证中文 ≥3 字命中、无权频道不泄露、不足 3 字走 LIKE
- [x] 5.2 实现频道内搜索与 `/search/suggest`：范围固定、前缀匹配与类型权重；验证空 `q` 400、非成员 404
- [x] 5.3 实现 `/me/channel-state` 读取与单项更新：折叠/通知/已读，只影响本人且已读只前进；验证越权写不改他人
- [x] 5.4 实现 `/me/drafts` 读取与保存：空正文无附件即删除、按 scope 隔离；验证发送成功后草稿清除
- [x] 5.5 实现通知写入钩子（提及、DM、好友请求、消息请求）与列表/批量已读/未读计数：验证越权 id 被忽略、角标随已读减少
- [x] 5.6 实现 `/push/subscribe`：endpoint 校验、按 endpoint 去重存储、Demo 不投递；验证非法 URL 400、重复注册不新增
- [x] 5.7 补齐 30 类 `Event` 常量并统一走 outbox 的 scope 路由（server/dm/user）：验证事件目录测试逐项断言，无拼写偏差
- [x] 5.8 增强 Gateway 连接：`SUBSCRIBE` 成员校验、60 s 心跳超时、token 过期 4001 关闭；验证越权订阅被忽略、超时后连接清理
- [x] 5.9 实现 `?since=` 补偿与 24 h 保留：按订阅范围补发 ≤500 条、过期退回 `READY`；验证补发不重复、越权事件被过滤
- [x] 5.10 删除 `stubs.py` 中已实现条目并在 `api/v1/__init__.py` 注册新模块；验证 OpenAPI 不再出现 501 占位（`/boost/subscribe` 除外），`pytest -q` 全绿

## 6. 集成验收

- [x] 6.1 全量 `uv run ruff check .` 0 error、`uv run pytest -q` 全绿（含 WebSocket 与负例矩阵），记录用例数
- [x] 6.2 真实 uvicorn 单 worker 跑扩展后的 `scripts/smoke.py`，覆盖 B1/B2/B3/B4 主链路；验证无 5xx、无残留进程与测试数据
- [x] 6.3 文档一致性检查：`docs/backend-api.md` 端点计数 167、表数 51、事件数 30 与 `server/README.md`、根 `README.md`、`agent.md` 一致；`openspec validate implement-backend-p1-p2 --strict` 通过
- [x] 6.4 并发与限流冒烟：2 个客户端同时发消息/接受邀请，验证无 `database is locked`、幂等与 429 行为符合预期

## Workflow follow-up

- 按 B1 → B2 → B3 → B4 分批提交与合并，每批独立跑门禁；全部批次完成后再归档本 change（`openspec archive implement-backend-p1-p2`）。
- 归档前确认 `openspec/changes/implement-backend-p1-p2/.openspec.yaml` 未被手工修改，`specs/` 增量已随归档落入 `openspec/specs/`。
- 客户端接入另立 change：替换 Rust 侧本地快照读取为 HTTP/WS 客户端，书签改用 `message_id` 跳转。
