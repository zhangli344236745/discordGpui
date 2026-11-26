# Design

## Context

`native/screen_capture.mm` 已有 CGDisplayStream 整屏采集；LiveKit 侧以
`ScreenSession` + `NativeVideoSource` 发布 `TrackSource::Screenshare`。

## Decisions

- 新增 C 接口：
  - `dgpu_screen_list_json()`：同步等待 `SCShareableContent`（6s 超时），返回
    `[{id,kind,title,app,width,height}]`，显示器在前、窗口按面积过滤（≥120×80）；
  - `dgpu_screen_start_target(id, is_window, max_w, max_h, cb, err)`：
    `SCContentFilter` + `SCStreamConfiguration`（NV12、15fps、queueDepth 3），
    失败写错误串；
  - 句柄增加 `kind`（0=CGDisplayStream，1=SCStream），`stop` 按类型释放。
- Rust 侧 `screen::list_targets()` 用 serde_json 解析；`start_target()` 复用同一
  NV12→I420 管线与 `ScreenSession`。
- 引擎命令：
  - `ListScreenTargets { reply }`（`spawn_blocking`，避免阻塞房间循环）；
  - `SetScreenTarget(target)`（先停旧共享，再按目标启动）；
  - 保留 `SetScreen(bool)` 作为整屏回退与 e2e 强制入口。
- UI：`screen_picker` 顶层模态（显示器分组在前、窗口最多 12 个），点击即开始共享；
  再次点击屏幕按钮则停止共享。

## Risks

| 风险 | 应对 |
| --- | --- |
| 首次调用触发屏幕录制授权 | 预检 `CGPreflightScreenCaptureAccess`，未授权时提示并保留整屏回退 |
| 枚举超时或内容为空 | 6s 超时 + 空列表提示「没有可共享目标」 |
| 已停止的窗口 id 失效 | 启动失败回传错误并在 UI 提示，不崩溃 |
