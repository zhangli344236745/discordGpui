# LiveKit 客户端接入方案（P3 剩余部分）

服务端已完成：房间命名、AccessToken 签发、参与者投影、webhook 验签回写、RoomService 静音/踢人
（`LIVEKIT_ROOMSERVICE_ENABLED`）。客户端目前仍是 Demo 状态（本地语音面板状态，不建立真实媒体连接）。

本文件把剩余任务（tasks 4.1–4.4、4.6、4.7）拆成可验收的步骤，避免后续实现时重新调研。

## 已实现：真实语音（voice-livekit feature）

- 依赖：`livekit 0.12.1` + `tokio`（仅语音线程），Cargo feature `voice-livekit`，默认关闭；
- `src/voice/`：独立线程 + tokio 运行时，`PlatformAudio`（CoreAudio ADM）负责采集与远端播放，
  `LocalAudioTrack` 发布麦克风；UI 通过 `async_channel` 收 Connected/Reconnecting/Participant* 事件；
- 加入频道后自动连接；麦克风开关同时驱动本地引擎静音与 `PUT /voice/{room}/me`；
- 断线由 SDK 自动重连，UI 显示重连/恢复，READY 后信令层再校准参与者；
- 私信通话：接听/呼叫返回的 token 直接起同一引擎，双端加入 `dm:{call_id}` 房间并发布麦克风；
- 验证：
  - `scripts/voice-e2e.sh`：`room + microphone`、`webhook participant_joined`、`webhook track_published`、`participant projection mic=true`；
  - `scripts/dm-call-e2e.sh`：双端房间/麦克风、`participant_joined x2`、`track_published x2`、房间名 `dm:` 前缀。

### 运维注意

LiveKit 容器与宿主机的时钟可能有几秒偏差（实测 colima 下 JWT `iat` 落在未来），
webhook 验签已放宽：不校验 `iat`，`nbf/exp` 留 30s leeway。部署时仍建议开启 NTP。

### 构建（本机已跑通）

```bash
scripts/build-voice.sh                # 使用 ~/Library/Caches/dgpui-webrtc/mac-x64-release
AUTO_DOWNLOAD=1 scripts/build-voice.sh  # 缓存缺失时自动下载（镜像加速）
scripts/voice-e2e.sh                  # 需要 docker + livekit/livekit-server 镜像
```

两个只在「旧 Command Line Tools（macOS 13 / clang 14）」下需要的兼容处理（脚本已自动打）：

1. 预编译头 `rtc_base/buffer.h` 的 `std::span<T>::iterator` 缺 `typename`；
2. `cxx` 的 `Slice<T>::iterator` 缺 `element_type`，旧 libc++ 的 `pointer_traits` 找不到。

链接侧（`build.rs`，仅该 feature 生效）：
- `-Wl,-ObjC`：libwebrtc 的 ObjC 分类（`NSString+StdString`）需要强制加载，否则运行时报 selector 缺失；
- `-Wl,-undefined,dynamic_lookup`：预编译库引用 macOS 14+ ScreenCaptureKit 符号，旧 SDK 链接不到；
  音频路径不会触发，升级到 macOS 14+ SDK 后可移除。

## 更早的「仅信令」部分（离线/未启用 feature 时仍生效）

- 客户端点击语音频道 → `POST /channels/{id}/voice/join`：拿 `room` 与参与者投影，面板显示真实在线成员；
- 断开 → `POST /channels/{id}/voice/leave`；麦克风开关 → `PUT /voice/{room}/me`；
- 订阅 `VOICE_STATE_UPDATE`：其他端加入/静音/离开时参与者列表实时更新；
- Gateway 重连（READY）后自动校准：已在房间则拉取参与者，房间丢失则重新 join（4.6 的信令层）；
- 离线或未配置时保持原 Demo 行为（种子参与者）；
- 请求构造有单测（`src/net/api.rs`），服务端链路由 `pytest` 与 `smoke.py` 覆盖。

剩余仅「真实媒体」：采集、编码、传输、播放/渲染，即下面的 4.2–4.4 / 4.6 / 4.7。

## 构建可行性探测（2026-11-25）

- `cargo add livekit` 可解析（0.12.1），依赖链含 `webrtc-sys 0.3.55` / `webrtc-sys-build 0.3.21`；
- 首次构建需从 `https://github.com/livekit/rust-sdks/releases/download/webrtc-a545de4/webrtc-mac-x64-release.zip`
  下载预编译 libwebrtc；本机当前对 GitHub Release 资源的请求超时（HTTP 000），**构建会卡在下载阶段**；
- 因此在代码库中未引入该依赖，避免留下无法构建的 feature。

## 现状与约束

| 项 | 状态 |
| --- | --- |
| 服务端 token / 房间 / webhook / RoomService | 已完成（`server/app/livekit/`、`server/app/api/v1/voice.py`） |
| 客户端 UI | 语音面板、麦克风/耳机开关、参与者列表已有（Demo 数据） |
| 客户端网络模型 | `ureq` + `tungstenite`，无 tokio；新增 SDK 必须隔离在独立线程 |
| 平台 | macOS 优先（CoreAudio / Metal） |

## 依赖选择

- `livekit` crate（默认 `native` feature，自带 libwebrtc 绑定）。
- `tokio` 仅用于语音运行时：在专用线程里 `Runtime::new()`，不进入 GPUI 主线程。
- 音频采集/播放由 SDK 的 native 层处理；若需要设备枚举/切换，再加 `cpal`。
- 以 Cargo feature 隔离：`voice-livekit = ["dep:livekit", "dep:tokio"]`，默认关闭，
  保证无 SDK 环境下 `cargo check/test` 仍可跑（CI 默认不装）。

## 实施步骤

### 4.1 连接层与 VoiceProvider 抽象

1. 新建 `src/voice/mod.rs`：
   ```rust
   pub enum VoiceEvent { Connected, Disconnected, ParticipantJoined(..), TrackSubscribed(..), Reconnecting, Reconnected }
   pub trait VoiceProvider: Send {
       fn connect(&mut self, url: &str, token: &str);
       fn disconnect(&mut self);
       fn set_mic(&mut self, on: bool);
       fn set_camera(&mut self, on: bool);
       fn set_screen(&mut self, on: bool);
       fn events(&self) -> async_channel::Receiver<VoiceEvent>;
   }
   ```
2. `DemoVoiceProvider`：现有 Demo 行为（加入即本地标记）。
3. `LiveKitProvider`（feature = voice-livekit）：内部持有 tokio Runtime 与 `Room`，
   所有 SDK 调用 `runtime.block_on` 或 `runtime.spawn`，事件经 `async_channel` 抛回 UI。
4. `DiscordApp` 增加 `voice: Box<dyn VoiceProvider>`，启动时按 feature + 环境变量选择实现。
   **验收**：未启用 feature 时 `cargo test` 全绿，语音面板行为与现在一致。

### 4.2 加入频道与音频链路

1. 点击语音频道 → 调 `POST /channels/{id}/voice/join` 拿 `{url, token, room}`。
2. `provider.connect(url, token)`；订阅 `TrackSubscribed`（audio）→ 播放。
3. `TrackPublished` / `ParticipantConnected` → 更新语音面板参与者列表（保留现有 UI）。
4. 退出/断线 → 调 `POST /channels/{id}/voice/leave`。
   **验收**：两台机器同一频道可互相听到；面板参与者与 `/voice/participants` 一致。

### 4.3 开关与状态同步

1. 麦克风：`LocalAudioTrack::set_mute`；摄像头：`LocalVideoTrack`（先只做本地预览）。
2. 每次切换调 `PUT /voice/{room}/me`（现有接口）广播状态。
3. 屏幕共享：macOS `ScreenCaptureKit`（SDK native 支持）；先做窗口选择器最小实现。
   **验收**：一端静音，另一端在 1 秒内看到麦克风标记变化。

### 4.4 设备与权限

1. `cpal` 枚举输入/输出设备，设置页下拉持久化到 `UserSettings`。
2. 权限被拒（macOS TCC）时提示「前往系统设置 → 隐私与安全性 → 麦克风」，
   `AVCaptureDevice.authorizationStatus` 判断状态。
   **验收**：拒绝权限不崩溃，提示可操作；切换设备不中断房间。

### 4.6 弱网重连

1. SDK `Reconnecting/Reconnected` 事件 → UI 顶部提示条。
2. 重连后以 `GET /channels/{id}/voice/participants` 校准参与者，避免重复。
3. 15 秒未恢复 → 提示并提供「重新加入」。
   **验收**：断网 10 秒恢复后无重复参与者，本地麦克风状态保持。

### 4.7 私信真实通话

1. `POST /dms/{id}/call` 接受后拿 token（服务端 `dm:{call_id}` 房间已实现）。
2. 复用 4.2 的连接与渲染；挂断调 `DELETE /dms/{id}/call`。
3. Demo 呼叫面板保留为回退（未配置 LiveKit 时）。
   **验收**：A 呼叫 → B 接听 → 双方通话 → 任一方挂断，双方状态复原。

## 构建与验证命令

```bash
# 默认（不装 SDK）
cargo check && cargo test

# 启用语音
cargo build --features voice-livekit
DISCORD_API_BASE=http://127.0.0.1:8080 ./target/debug/discord-gpui
```

## 风险

| 风险 | 应对 |
| --- | --- |
| `webrtc-sys` 首次构建下载预编译 libwebrtc（数百 MB），代理环境可能失败 | 先在可联网环境产出 `target/` 缓存；CI 单独 job 且允许失败不阻断主干 |
| 视频帧渲染进 GPUI 需要纹理桥接（`gpui::RenderImage`） | 先音频后视频；视频作为独立增量 |
| 屏幕共享权限（Screen Recording） | 首次调用引导授权，失败回退音频 only |
| 与现有无 tokio 架构冲突 | Runtime 限定在 `src/voice/`，UI 只收 `async_channel` 事件 |

## 当前结论

P3 客户端部分需要真实 macOS 音频/视频设备与数百 MB SDK 依赖，不适合与其他模块混在同一次提交；
建议作为独立 change（例如 `add-livekit-client-voice`）落地，并在具备联网构建条件时按上面步骤推进。
