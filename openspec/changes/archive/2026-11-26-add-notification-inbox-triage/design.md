# Design

## Context

`GET /notifications` 返回 `id/kind/target_kind/target_id/title/body/read/created_at`，
`POST /notifications/read` 支持 `ids[]` 或 `all=true`。客户端已经有收件箱面板与本地聚合回退
（无后端时从未读频道/私信/好友请求生成通知），但服务端字段此前被丢掉。

## Goals / Non-Goals

**Goals：** 真实来源类型；可跳转目标；按类型筛选与分组；已读跨设备一致；清除已读。

**Non-Goals：** 通知分页加载更多、推送投递、服务端删除通知接口（当前不存在）。

## Decisions

- `Notification` 增加 `id: String` 与 `read: bool`。本地聚合通知 `id` 为空、`read=false`，
  继续由 `read_notifications` 集合判定；服务端通知优先用 `id` 作为唯一 key。
- `NotificationKind::System` 承接 `report`/`system`，沿用 `Bell` 图标。
- `target_kind=channel` 时用 `remote.channels` 的 `id → "服务器/频道"` 反查本地索引；
  `target_kind=dm` 时直接用通知 `title`（发送者显示名）打开会话；其余回退好友页。
- 已读写入分两条路径：本地聚合只记 key；服务端通知同时把 `backend_notifications` 中对应项
  置为 `read=true`，并异步 `POST /notifications/read`。
- 「清除已读」把已读 key 记入 `cleared_notifications`，`notifications()` 统一过滤；
  仅本次会话生效，避免与服务端记录不一致。
- UI 用 `filter_chip` 展示 7 个筛选及条数；「全部」按 Mention→Dm→Channel→FriendRequest→System
  稳定排序并插入分组标题。

## Risks / Trade-offs

| 风险 | 应对 |
| --- | --- |
| 频道反查失败导致跳错 | 回退好友页并在标题中保留原始信息，不崩溃 |
| 已读接口失败 | 本地先标记已读，下次启动以服务端 `read` 为准 |
| 分组重排影响点击索引 | 列表项保留原始下标，点击仍按原数组定位 |
