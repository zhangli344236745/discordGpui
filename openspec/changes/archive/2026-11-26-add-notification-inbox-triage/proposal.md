# Proposal

## Why

客户端收件箱此前把所有服务端通知都映射成「未读频道」，点击一律跳好友页，也无法按来源
筛选；服务端返回的 `read` 状态被丢弃，导致多端未读角标不一致。通知一多就无法快速分诊。

## What Changes

- 服务端通知按 `kind` / `target_kind` / `target_id` 真实映射到提及、私信、频道、好友请求与系统，
  并解析出可跳转的本地目标。
- 收件箱新增筛选标签（全部 / 未读 / 提及 / 私信 / 频道 / 好友请求 / 系统，均带条数），
  「全部」按来源分组展示。
- 点击单条通知或「全部已读」时调用 `POST /notifications/read`，把已读状态写回服务端；
  服务端 `read` 字段参与未读判定与角标。
- 新增「清除已读 N」：仅从当前收件箱移除已读条目，不影响未读与服务端记录。

**BREAKING**：无。后端契约不变，仅客户端消费既有字段。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `search-and-notifications`：补充客户端通知中心的分诊、跳转与已读同步要求。

## Impact

- **代码**：`src/data/models.rs`（通知增加 `id`/`read` 与 `System` 类型）、`src/app.rs`
  （类型映射、目标解析、筛选/清除状态、已读回写）、`src/net/hydrate.rs`、
  `src/ui/chat/inbox_panel.rs`。
- **验证**：`cargo test`、`client-e2e.sh`；服务端 pytest 与契约不变。
