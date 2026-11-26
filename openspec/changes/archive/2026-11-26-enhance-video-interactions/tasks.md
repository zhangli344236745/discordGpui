# Tasks

## 1. 预览拖动

- [x] 1.1 `pip_position` 状态与 clamp 逻辑（纯函数 + 单测）
- [x] 1.2 DM 通话面板预览小窗接入拖拽（ghost + on_drag_move）
- [x] 1.3 渲染按归一化坐标定位，窗口尺寸变化仍保持比例

## 2. 发言者置顶

- [x] 2.1 引擎监听 ActiveSpeakersChanged 并上报显示名
- [x] 2.2 App 保存发言者集合，离开/断开时清理
- [x] 2.3 `video_tiles` 排序改为发言者优先，平铺格高亮；补排序单测

## 3. 屏享码率自适应

- [x] 3.1 `screen_bitrate(w,h,quality)` 纯函数与单测（单调、夹紧、档位关系）
- [x] 3.2 发布/降档时按目标尺寸计算码率（窗口与整屏路径都覆盖）

## 4. 验证与文档

- [x] 4.1 voice-e2e 断言发言者事件与屏享码率日志
- [x] 4.2 `cargo test` / `client-e2e` 不回归
- [ ] 4.3 更新语音文档与 README
- [ ] 4.4 `openspec validate enhance-video-interactions --strict` 通过
