# Proposal

## Why

多路视频可用后仍有三个体验缺口：

- 本地预览固定在右下角，会挡住字幕/共享内容，用户无法挪开；
- 平铺按「屏幕优先 + 最近更新」排序，正在说话的人可能被排到后面；
- 屏幕共享用固定 1500/600 kbps，与共享区域大小无关：小窗口浪费带宽，大屏又偏糊。

## What Changes

- 本地预览小窗支持拖动，位置按比例记忆并在边界内约束；
- 语音引擎上报 ActiveSpeakers，平铺排序改为「发言者 > 屏幕共享 > 最近更新」并高亮；
- 屏幕共享码率按采集分辨率、帧率与画质档位计算（带上下限），窗口变化后按新尺寸生效。

**BREAKING**：无。后端契约与事件不变，未启用 `voice-livekit` 的构建行为不变。

## Capabilities

### Modified Capabilities

- `commercial-voice`：新增预览拖动、发言者置顶、按尺寸自适应码率三项要求。

## Impact

- **代码**：`src/app.rs`（预览位置/发言者状态、排序）、`src/ui/dm_call_panel.rs`（拖动）、
  `src/ui/voice_room.rs`（高亮）、`src/voice/mod.rs`（ActiveSpeakers、码率计算）。
- **验证**：新增排序与码率纯函数单测；语音 e2e 增加发言者事件与屏享码率日志断言。
