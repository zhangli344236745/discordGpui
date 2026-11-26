# Proposal

## Why

目前屏幕共享只能整屏（CGDisplayStream），无法只共享某个应用窗口，也不能在多显示器之间选择。
这既不满足演示/教学场景的隐私需求，也不符合 Discord 的基本体验。

## What Changes

- 客户端屏幕共享改为先用 ScreenCaptureKit 枚举显示器与窗口，弹出选择器；
- 选择后以 `SCContentFilter` 针对该窗口/显示器采集，按 15fps、最长边 1280 编码；
- 保持整屏（CGDisplayStream）实现作为回退：选择器不可用或未取到目标时仍可整屏共享；
- 未启用 `voice-livekit` 的构建、Demo 模式与后端契约不变。

**BREAKING**：无。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `commercial-voice`：新增「屏幕共享目标选择」要求。

## Impact

- **代码**：`native/screen_capture.mm`（SCK 枚举/采集）、`src/voice/{mod,screen}.rs`（命令与解析）、
  `src/app.rs`（选择器状态）、`src/ui/screen_picker.rs`（新模态）。
- **依赖**：仅 Apple 系统框架 ScreenCaptureKit，无新增 crate。
- **验证**：`voice-e2e` 走「枚举目标→共享第一个显示器」；`dm-call-e2e` 保留整屏回退路径。
