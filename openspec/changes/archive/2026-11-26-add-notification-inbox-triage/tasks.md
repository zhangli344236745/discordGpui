# Tasks

## 1. 数据与映射

- [x] 1.1 `Notification` 增加 `id`/`read`，新增 `NotificationKind::System`
- [x] 1.2 启动 hydrate 与服务端 `NOTIFICATION_CREATE` 刷新共用真实类型/目标映射
- [x] 1.3 频道 `target_id` 反查服务器/频道，私信按发送者显示名打开

## 2. 已读与筛选

- [x] 2.1 已读判定合并服务端 `read` 与本地已读集合
- [x] 2.2 点击单条与「全部已读」异步回写 `POST /notifications/read`
- [x] 2.3 收件箱筛选状态 `NotificationFilter` 与「清除已读」

## 3. UI

- [x] 3.1 筛选标签带条数；「全部」按来源分组
- [x] 3.2 空状态区分「收件箱为空」与「当前筛选无结果」
- [x] 3.3 「清除已读 N」只移除已读条目

## 4. 验证

- [x] 4.1 `cargo test` 通过（新增筛选/类型映射单测）
- [x] 4.2 `client-e2e.sh` 13 项不回归
- [x] 4.3 `openspec validate add-notification-inbox-triage --strict` 通过
