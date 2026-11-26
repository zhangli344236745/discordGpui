# Spec Delta

## ADDED Requirements

### Requirement: 客户端通知中心分诊

客户端 SHALL 按服务端通知的真实来源展示收件箱，支持按来源筛选、按来源分组、跳转到对应会话，并把已读状态同步到服务端；无法解析的目标 MUST 安全回退，不得崩溃或误导跳转。

#### Scenario: 按来源筛选

- **WHEN** 用户选择「未读」「提及」「私信」「频道」「好友请求」或「系统」
- **THEN** 只显示对应来源或未读条目，每个标签显示条数，「全部」按来源分组展示

#### Scenario: 已读同步

- **WHEN** 用户点击某条服务端通知或「全部已读」
- **THEN** 客户端调用 `POST /notifications/read`（`ids[]` 或 `all=true`）并本地标记已读，其他设备重新同步后同样为已读

#### Scenario: 清除已读

- **WHEN** 用户点击「清除已读 N」
- **THEN** 已读条目从当前收件箱移除，未读条目与服务端记录不受影响

#### Scenario: 目标解析

- **WHEN** 收到 `target_kind=channel` 或 `target_kind=dm` 的通知
- **THEN** 分别解析到本地服务器/频道或私信会话并跳转；解析失败时回退到好友页并保留通知内容
