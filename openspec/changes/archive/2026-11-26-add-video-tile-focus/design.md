# Design

## Context

视频帧按 `(参与者, 来源)` 存在 `HashMap` 里，UI 通过 `video_tiles(limit)` 取排序后的
列表渲染。DM 面板展示第一路远端画面（屏幕优先），语音房间以两列平铺。

## Goals / Non-Goals

**Goals：** 单路放大查看；还原；参与者离开自动退出。

**Non-Goals：** 拖拽排序、全屏独立窗口、画面录制。

## Decisions

- 聚焦状态放在 `DiscordApp::video_focus: Option<(String, String)>`（identity, kind），
  与帧 map 同键；渲染时若 key 仍存在则单路放大，否则自动退回平铺。
- 语音房间：聚焦时高度 220px、铺满宽度，左上角名字标签、右上角「还原」；
  平铺格仍保留点击进入聚焦。
- DM 面板：聚焦时主画面高度 180 → 260，点击画面切换；底部提示当前状态。
- 参与者离开（`drop_video_frames`）或断开时清空 `video_focus`。
- 纯函数 `resolve_focus(focus, keys)` 便于单测：目标不存在时返回 None。

## Risks / Trade-offs

| 风险 | 应对 |
| --- | --- |
| 点击小窗误触 | 仅点击画面区域生效，按钮区域阻止冒泡（按钮自身 on_click 优先） |
| 聚焦目标消失后空白 | 渲染前用 `resolve_focus` 校验，自动回退平铺 |
