# Design

## Context

现有实现（归档的 `reach-commercial-baseline`）：

- 引擎在 `src/voice/mod.rs` 的 tokio 线程里发布麦克风/摄像头/屏幕轨道；
- 远端轨道经 `NativeVideoStream` 拉帧、`i420_to_bgra` 转 BGRA、节流 15fps 后送 UI；
- UI 侧把「摄像头/屏幕」各存一张 `RenderImage`，DM 通话面板优先屏幕、其次摄像头；
- 频道语音房间面板已有摄像头/屏幕开关，但只显示「最近一帧」；
- 编码参数使用 `TrackPublishOptions::default()`，未按来源区分。

## Goals / Non-Goals

**Goals：**

- 本端摄像头/屏幕有实时小窗预览；
- 多人通话按参与者平铺（2 列、上限 6 路、屏幕共享优先）；
- 屏幕共享与摄像头使用不同码率档位，弱网时自动降档、恢复后升档；
- 不改变后端接口、事件与既有 e2e 行为。

**Non-Goals：**

- 不做 UI 层美颜/镜像/虚拟背景；
- 不做按参与者的独立音量或焦点模式；
- 不引入 SFU 侧动态订阅层选择（继续依赖 WebRTC simulcast）。

## Decisions

### D1. 本地预览走同一条解码管线

发布轨道后，用 `NativeVideoStream::new(track.rtc_track())` 对**本端**轨道再挂一个 sink，
与远端完全相同的转换/节流逻辑，只是 `identity = "你"`、`kind` 加 `local` 标记。

理由：避免在采集回调里再做一次 I420→BGRA（重复计算与线程耦合），并保证预览与远端
颜色/方向一致；代价是每路多一次 I420→BGRA（15fps，可接受）。

### D2. 帧状态改为 `HashMap<(identity, kind), Arc<RenderImage>>`

键为 `(参与者标识, "camera"|"screen")`，值为最近一帧。UI 读取：

- DM 通话：优先对方屏幕，其次对方摄像头，最后本地小窗；
- 语音房间：收集所有键，按「屏幕优先、最近更新在前」排序，最多 6 格。

`identity` 用显示名（引擎已用 `display_name`），本地固定为 `你`。

### D3. 编码档位与自适应策略

| 来源 | 高档 | 低档 |
| --- | --- | --- |
| 摄像头 | 900 kbps / 24 fps | 400 kbps / 15 fps |
| 屏幕共享 | 1500 kbps / 10 fps | 600 kbps / 8 fps |

- 发布时按来源使用对应高档；
- 监听 `RoomEvent::ConnectionQualityChanged` 中**本端**participant 的质量：
  连续 2 次 `Poor`/`Lost` → 用低档重新发布；连续 3 次 `Excellent`/`Good` → 恢复高档；
- 重新发布保留原 `TrackSource` 与采集源（`NativeVideoSource` 可复用），只重建轨道与发布，
  并在 UI 状态条提示「网络较差，已降低画面质量」/「画面质量已恢复」。
- 降档/升档有 10 秒冷却，避免抖动。

### D4. 平铺布局

用 GPUI 的 `flex_wrap`（或两行 `h_flex`）实现 2 列网格；每格固定 16:9 高度 90px，
左上角名字标签，本地格右上角「你」。超过 6 路时显示「+N」占位。

## Risks / Trade-offs

| 风险 | 应对 |
| --- | --- |
| 重新发布会造成短暂黑屏 | 仅在质量持续较差时触发，带冷却；提示文案解释 |
| 多路 15fps 转换增加 CPU | 上限 6 路 + 节流；BGRA 缓冲复用可后续优化 |
| `flex_wrap` 在 gpui-kit 的可用性不确定 | 若不可用，退化为两行 `h_flex` 手动分行 |
| 本地预览与远端同一参与者重名 | 本地固定 `你`，远端用显示名；键仍包含来源 |

## Migration Plan

1. 引擎：抽 `publish_video(kind, quality)` 与 `restart_video(kind, quality)`；
2. 引擎：发布后为本端轨道建 `NativeVideoStream` 并送入同一帧通道（identity=你）；
3. 应用层：帧状态改 map，补排序与上限逻辑；
4. UI：DM 面板加本地小窗，语音房间改平铺网格；
5. e2e 增加断言后归档。

## Open Questions

- 是否需要「点击某格放大」？暂不做，留给后续交互增量。
- 屏享是否需要「仅共享窗口」？需要 ScreenCaptureKit，另行立项。
