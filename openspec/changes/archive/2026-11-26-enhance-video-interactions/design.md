# Design

## Decisions

### D1. 预览拖动

- 状态：`pip_position: (f32, f32)`，为画中画中心点在舞台内的归一化坐标，默认 (0.82, 0.76)；
- 交互：以 gpui 拖拽系统（`on_drag` + `on_drag_move`）实现，`DragMoveEvent` 提供舞台
  `bounds` 与鼠标位置，换算为归一化坐标并 clamp 到 [0.08,0.92]；
- 渲染：`left(relative(x)) / top(relative(y))` + 负 margin 居中；
- 拖拽 ghost 用一个 96×54 的半透明占位，避免每帧渲染整帧图像。

### D2. 发言者置顶

- 引擎监听 `RoomEvent::ActiveSpeakersChanged`，发送 `VoiceEvent::ActiveSpeakers(Vec<String>)`
  （显示名，与参与者身份一致）；
- App 保存 `active_speakers`，参与者离开时移除；`video_tiles` 排序键：
  `(0 if speaking else 1) → (0 if screen else 1) → seq desc`；
- 平铺格对 speaking 项加 accent 边框；新增 `tile_order_rank` 纯函数便于单测。

### D3. 屏享码率按尺寸

- `screen_bitrate(width, height, quality)`：
  - 采集最长边限制 1280，先等比缩放；
  - 目标码率 = w×h×fps×bpp，高清档 bpp≈0.12、低档≈0.07；
  - 高清档 clamp 600k–4M，低档 clamp 300k–2M；
- 发布/降档重发时传入该路的目标尺寸；摄像头仍用固定 900k/400k；
- 提供 `screen_bitrate` 单测：随分辨率单调、边界夹紧、低档小于高档。

## Risks

| 风险 | 应对 |
| --- | --- |
| 拖拽事件在 gpui-kit 版本差异 | 若 `on_drag_move` 不可用，退化为「点击四角循环」并在文档注明 |
| 发言者列表抖动导致排序频繁变化 | 排序只在集合变化时重算；列表来自 SDK 的稳定聚合 |
| 码率过高触发弱网降档 | 计算与既有自适应联动：降档后按低档重新计算 |
