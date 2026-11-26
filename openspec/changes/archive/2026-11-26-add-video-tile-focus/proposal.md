# Proposal

## Why

视频平铺上线后，6 路画面每格只有 84px 高，屏幕共享里的文字难以辨认。用户需要一个
「点开某一路放大看」的轻量交互，而不必退出通话或切换布局。

## What Changes

- 语音房间面板：点击任意视频格进入聚焦视图（单路放大、显示名字与来源），
  点击「还原」或再次点击该画面返回平铺。
- 私信通话面板：点击主画面在「标准 / 放大」两档高度间切换。
- 聚焦目标参与者离开或关闭摄像头时自动退出聚焦。

**BREAKING**：无。纯客户端 UI 状态，不影响后端、事件与既有 e2e。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `commercial-voice`：新增「视频画面聚焦」要求。

## Impact

- **代码**：`src/app.rs`（聚焦状态与清理）、`src/ui/voice_room.rs`、`src/ui/dm_call_panel.rs`。
- **验证**：`cargo test` 覆盖聚焦状态解析；`client-e2e` 与语音 e2e 不回归。
