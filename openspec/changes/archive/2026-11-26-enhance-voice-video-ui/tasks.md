# Tasks

## 1. 引擎：本端预览与编码档位

- [x] 1.1 发布摄像头/屏幕后，为本端轨道创建 `NativeVideoStream`，以 `identity="你"` 送入帧通道
- [x] 1.2 帧数据增加 `local` 标记（或 `kind` 使用 `camera-local`/`screen-local`），远端不变
- [x] 1.3 抽取 `publish_video(kind, quality)`，摄像头/屏幕分别使用高档编码参数
- [x] 1.4 验证：dm-call-e2e 中主叫端日志出现本端摄像头/屏幕帧

## 2. 引擎：码率自适应

- [x] 2.1 监听 `ConnectionQualityChanged`，只处理本端 participant
- [x] 2.2 连续 2 次 Poor/Lost → 以低档重新发布；连续 3 次 Excellent/Good → 恢复高档
- [x] 2.3 调整加 10s 冷却，并通过事件通知 UI 提示降档/恢复
- [x] 2.4 验证：单元测试覆盖档位判定与冷却（纯函数）

## 3. 应用层：多路帧状态

- [x] 3.1 `remote_camera_frame`/`remote_screen_frame` 改为 `HashMap<(identity, kind), Arc<RenderImage>>`
- [x] 3.2 提供「按可信度排序的 tile 列表」（屏幕优先、最近更新在前、上限 6）
- [x] 3.3 断开语音/通话时清空所有帧状态
- [x] 3.4 单元测试：排序、上限、参与者离开后的清理

## 4. UI：小窗与平铺

- [x] 4.1 DM 通话面板：远端画面 + 本地 PiP 小窗（标注「你」）
- [x] 4.2 语音房间面板：两列平铺网格，格内名字标签，超过 6 路显示「+N」
- [x] 4.3 质量提示文案：降档与恢复各一条
- [x] 4.4 验证：client-e2e 不回归、无 panic

## 5. 验证与文档

- [x] 5.1 dm-call-e2e 断言本端预览帧与远端多路帧
- [x] 5.2 voice-e2e 断言平铺所需帧（摄像头+屏幕）与质量提示不崩溃
- [x] 5.3 更新 `docs/voice-livekit-plan.md` 与 README
- [x] 5.4 `openspec validate enhance-voice-video-ui --strict` 通过
