# Tasks

## 1. 状态与清理

- [x] 1.1 `DiscordApp` 增加 `video_focus: Option<(String, String)>` 与 `toggle_video_focus`
- [x] 1.2 `resolve_focus` 纯函数：目标不在帧 map 时返回 None；补单测
- [x] 1.3 参与者离开、断开语音、关闭摄像头时清空相关聚焦

## 2. UI

- [x] 2.1 语音房间：聚焦时单路大图 + 「还原」，平铺格可点击
- [x] 2.2 私信通话：主画面点击在标准/放大间切换
- [x] 2.3 还原按钮与画面点击互不干扰

## 3. 验证

- [x] 3.1 `cargo test` 通过
- [x] 3.2 `client-e2e.sh` 13 项不回归
- [x] 3.3 `openspec validate add-video-tile-focus --strict` 通过
