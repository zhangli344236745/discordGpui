//! 语音引擎抽象。
//!
//! - 未启用 `voice-livekit` feature 时返回 `None`，调用方保持「仅信令」行为
//!   （加入/离开、参与者投影由后端维护）。
//! - 启用后，在独立线程里跑 tokio 运行时，用 LiveKit SDK 建立真实房间、
//!   发布麦克风/摄像头/屏幕，并把本端与远端画面解码成 BGRA 交给 UI。

use async_channel::{Receiver, Sender};

#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
mod camera;
#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
mod screen;
#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
mod yuv;

/// 远端/本端视频帧（BGRA，供 UI 直接构造 RenderImage）。
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct VideoFrameData {
    pub identity: String,
    /// "camera" 或 "screen"
    pub kind: String,
    /// 是否本端画面（用于小窗预览与标注）。
    pub local: bool,
    pub width: u32,
    pub height: u32,
    pub bgra: std::sync::Arc<Vec<u8>>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum VoiceEvent {
    Connected,
    Disconnected(String),
    Reconnecting,
    Reconnected,
    ParticipantJoined(String),
    ParticipantLeft(String),
    MicUnavailable(String),
    CameraUnavailable(String),
    ScreenUnavailable(String),
    /// 码率档位因网络质量变化：low=true 表示已降档。
    QualityChanged { low: bool },
    Failed(String),
}

/// 可用音频设备：`(id, name)`，按系统顺序。
#[derive(Clone, Debug, Default)]
pub struct DeviceList {
    pub recording: Vec<(String, String)>,
    pub playout: Vec<(String, String)>,
}

/// 可共享的屏幕目标（显示器或窗口）。
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenTarget {
    pub id: u32,
    pub kind: String,
    pub title: String,
    pub app: String,
    pub width: u32,
    pub height: u32,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum VoiceCommand {
    SetMic(bool),
    ListDevices {
        reply: Sender<DeviceList>,
    },
    SetRecordingDevice(String),
    SetPlayoutDevice(String),
    SetCamera(bool),
    /// true = 整屏共享（旧入口/回退），false = 停止。
    SetScreen(bool),
    /// 共享指定显示器或窗口。
    SetScreenTarget(ScreenTarget),
    ListScreenTargets {
        reply: Sender<Result<Vec<ScreenTarget>, String>>,
    },
    Disconnect,
}

pub struct VoiceHandle {
    pub commands: Sender<VoiceCommand>,
    pub events: Receiver<VoiceEvent>,
    // 无 voice-livekit 时没有解码器写入，保留字段让 UI 代码保持同一契约
    #[allow(dead_code)]
    pub frames: Receiver<VideoFrameData>,
}

/// 视频码率档位（无 voice-livekit 时仅测试使用）。
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoQuality {
    High,
    Low,
}

/// 依据连续质量采样决定是否切换档位（纯函数，便于测试）。
///
/// - 连续 2 次 Poor/Lost 且当前为高档 → 降档；
/// - 连续 3 次 Excellent/Good 且当前为低档 → 升档；
/// - 距上次调整不足 10 秒 → 不变。
#[allow(dead_code)]
pub fn quality_transition(
    current: VideoQuality,
    poor_streak: u32,
    good_streak: u32,
    since_adjust: std::time::Duration,
) -> Option<VideoQuality> {
    const COOLDOWN: std::time::Duration = std::time::Duration::from_secs(10);
    if since_adjust < COOLDOWN {
        return None;
    }
    match current {
        VideoQuality::High if poor_streak >= 2 => Some(VideoQuality::Low),
        VideoQuality::Low if good_streak >= 3 => Some(VideoQuality::High),
        _ => None,
    }
}

/// 当前构建是否包含真实音视频能力。
pub const fn supported() -> bool {
    cfg!(feature = "voice-livekit")
}

/// 建立语音会话；`url`/`token` 来自后端 `/voice/join`。
#[cfg(not(feature = "voice-livekit"))]
pub fn start(_url: String, _token: String) -> Option<VoiceHandle> {
    None
}

#[cfg(feature = "voice-livekit")]
pub fn start(url: String, token: String) -> Option<VoiceHandle> {
    let (command_tx, command_rx) = async_channel::unbounded();
    let (event_tx, event_rx) = async_channel::unbounded();
    let (frame_tx, frame_rx) = async_channel::bounded(4);

    std::thread::Builder::new()
        .name("discord-voice".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = event_tx.send_blocking(VoiceEvent::Failed(error.to_string()));
                    return;
                }
            };
            runtime.block_on(run(url, token, event_tx, command_rx, frame_tx));
        })
        .ok()?;

    Some(VoiceHandle {
        commands: command_tx,
        events: event_rx,
        frames: frame_rx,
    })
}

#[cfg(feature = "voice-livekit")]
fn video_encoding(kind: &str, quality: VideoQuality) -> livekit::options::VideoEncoding {
    let (high_bitrate, high_fps, low_bitrate, low_fps) = if kind == "screen" {
        (1_500_000, 10.0, 600_000, 8.0)
    } else {
        (900_000, 24.0, 400_000, 15.0)
    };
    match quality {
        VideoQuality::High => livekit::options::VideoEncoding {
            max_bitrate: high_bitrate,
            max_framerate: high_fps,
        },
        VideoQuality::Low => livekit::options::VideoEncoding {
            max_bitrate: low_bitrate,
            max_framerate: low_fps,
        },
    }
}

#[cfg(feature = "voice-livekit")]
async fn publish_video(
    room: &livekit::prelude::Room,
    kind: &str,
    source: &livekit::webrtc::video_source::native::NativeVideoSource,
    quality: VideoQuality,
) -> Result<(livekit::prelude::LocalVideoTrack, livekit::prelude::TrackSid), String> {
    use livekit::prelude::*;
    use livekit::webrtc::prelude::RtcVideoSource;

    let track = LocalVideoTrack::create_video_track(kind, RtcVideoSource::Native(source.clone()));
    let options = TrackPublishOptions {
        source: if kind == "screen" {
            TrackSource::Screenshare
        } else {
            TrackSource::Camera
        },
        video_encoding: Some(video_encoding(kind, quality)),
        // 屏幕共享单层直出高码率，避免订阅端先拿到模糊低层；摄像头保留 simulcast
        simulcast: kind != "screen",
        ..Default::default()
    };
    room.local_participant()
        .publish_track(LocalTrack::Video(track.clone()), options)
        .await
        .map(|publication| (track, publication.sid()))
        .map_err(|error| error.to_string())
}

/// 把一个视频轨道解码成 BGRA 帧流，按 ~15fps 节流后发给 UI。
#[cfg(feature = "voice-livekit")]
fn spawn_video_pump(
    rtc_track: livekit::webrtc::prelude::RtcVideoTrack,
    identity: String,
    kind: String,
    local: bool,
    frames: Sender<VideoFrameData>,
) {
    use futures_util::StreamExt;
    use livekit::webrtc::native::yuv_helper;
    use livekit::webrtc::prelude::VideoBuffer;
    use livekit::webrtc::video_stream::native::NativeVideoStream;

    let mut stream = NativeVideoStream::new(rtc_track);
    tokio::spawn(async move {
        let mut last_sent = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .unwrap_or_else(std::time::Instant::now);
        while let Some(frame) = stream.next().await {
            if last_sent.elapsed() < std::time::Duration::from_millis(66) {
                continue;
            }
            last_sent = std::time::Instant::now();
            let buffer = frame.buffer.to_i420();
            let (width, height) = (buffer.width(), buffer.height());
            if width == 0 || height == 0 {
                continue;
            }
            let (y, u, v) = buffer.data();
            let (stride_y, stride_u, stride_v) = buffer.strides();
            let mut bgra = vec![0u8; (width * height * 4) as usize];
            yuv_helper::i420_to_bgra(
                y,
                stride_y,
                u,
                stride_u,
                v,
                stride_v,
                &mut bgra,
                width * 4,
                width as i32,
                height as i32,
            );
            let payload = VideoFrameData {
                identity: identity.clone(),
                kind: kind.clone(),
                local,
                width,
                height,
                bgra: std::sync::Arc::new(bgra),
            };
            if frames.send(payload).await.is_err() {
                break;
            }
        }
    });
}

// 采集会话只在 Drop 时停止；字段本身不需要读取。
#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
#[allow(dead_code)]
enum CaptureSession {
    Camera(camera::CameraSession),
    Screen(screen::ScreenSession),
}

#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
async fn republish_videos(
    room: &livekit::prelude::Room,
    quality: VideoQuality,
    camera: &mut Option<LocalVideo>,
    screen: &mut Option<LocalVideo>,
    frames: &Sender<VideoFrameData>,
) {
    for slot in [camera.as_mut(), screen.as_mut()] {
        let Some(video) = slot else { continue };
        if video.quality == quality {
            continue;
        }
        let _ = room.local_participant().unpublish_track(&video.sid).await;
        match publish_video(room, video.kind, &video.source, quality).await {
            Ok((track, sid)) => {
                spawn_video_pump(
                    track.rtc_track(),
                    "你".into(),
                    video.kind.into(),
                    true,
                    frames.clone(),
                );
                video.track = track;
                video.sid = sid;
                video.quality = quality;
            }
            Err(error) => {
                eprintln!("[voice] republish {} failed: {error}", video.kind);
            }
        }
    }
}

#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
async fn stop_screen(room: &livekit::prelude::Room, slot: &mut Option<LocalVideo>) {
    if let Some(video) = slot.take() {
        let _ = room.local_participant().unpublish_track(&video.sid).await;
        drop(video);
        eprintln!("[voice] screen unpublished");
    }
}

#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
async fn begin_screen(
    room: &livekit::prelude::Room,
    slot: &mut Option<LocalVideo>,
    target: Option<ScreenTarget>,
    quality: VideoQuality,
    frames: &Sender<VideoFrameData>,
) -> Result<(), String> {
    let (session, source) = match &target {
        Some(target) => screen::start_target(target)?,
        None => screen::start()?,
    };
    let (track, sid) = publish_video(room, "screen", &source, quality).await?;
    eprintln!("[voice] screen published sid={sid}");
    spawn_video_pump(
        track.rtc_track(),
        "你".into(),
        "screen".into(),
        true,
        frames.clone(),
    );
    *slot = Some(LocalVideo {
        kind: "screen",
        source,
        track,
        sid,
        quality,
        _capture: CaptureSession::Screen(session),
    });
    Ok(())
}

#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
struct LocalVideo {
    kind: &'static str,
    source: livekit::webrtc::video_source::native::NativeVideoSource,
    track: livekit::prelude::LocalVideoTrack,
    sid: livekit::prelude::TrackSid,
    quality: VideoQuality,
    _capture: CaptureSession,
}

#[cfg(feature = "voice-livekit")]
async fn run(
    url: String,
    token: String,
    events: Sender<VoiceEvent>,
    commands: Receiver<VoiceCommand>,
    frames: Sender<VideoFrameData>,
) {
    use livekit::prelude::*;

    // 平台 ADM：负责麦克风采集与远端音频播放，drop 时自动释放。
    let audio = match PlatformAudio::new() {
        Ok(audio) => audio,
        Err(error) => {
            eprintln!("[voice] platform audio unavailable: {error}");
            let _ = events
                .send(VoiceEvent::Failed(format!("音频设备不可用：{error}")))
                .await;
            return;
        }
    };

    let (room, mut room_events) = match Room::connect(&url, &token, RoomOptions::default()).await {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("[voice] room connect failed: {error}");
            let _ = events
                .send(VoiceEvent::Failed(format!("连接语音房间失败：{error}")))
                .await;
            return;
        }
    };
    eprintln!("[voice] room connected");

    let track = LocalAudioTrack::create_audio_track("mic", audio.rtc_source());
    let options = TrackPublishOptions {
        source: TrackSource::Microphone,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    let mut camera: Option<LocalVideo> = None;
    #[cfg(target_os = "macos")]
    let mut screen: Option<LocalVideo> = None;
    let mut quality = VideoQuality::High;
    let mut poor_streak = 0u32;
    let mut good_streak = 0u32;
    let mut last_adjust = std::time::Instant::now();
    let mut force_low_at = (std::env::var("DISCORD_VOICE_FORCE_LOW").ok().as_deref() == Some("1"))
        .then(|| tokio::time::Instant::now() + std::time::Duration::from_secs(6));

    let mic = match room
        .local_participant()
        .publish_track(LocalTrack::Audio(track.clone()), options)
        .await
    {
        Ok(publication) => {
            eprintln!("[voice] microphone published sid={:?}", publication.sid());
            let _ = events.send(VoiceEvent::Connected).await;
            Some(track)
        }
        Err(error) => {
            eprintln!("[voice] publish microphone failed: {error}");
            let _ = events
                .send(VoiceEvent::MicUnavailable(format!("麦克风发布失败：{error}")))
                .await;
            let _ = events.send(VoiceEvent::Connected).await;
            None
        }
    };

    loop {
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Ok(VoiceCommand::SetMic(on)) => {
                        if let Some(track) = &mic {
                            if on { track.unmute() } else { track.mute() }
                        }
                    }
                    Ok(VoiceCommand::ListDevices { reply }) => {
                        let list = DeviceList {
                            recording: audio
                                .recording_devices()
                                .map(|device| (device.id.as_str().to_string(), device.name))
                                .collect(),
                            playout: audio
                                .playout_devices()
                                .map(|device| (device.id.as_str().to_string(), device.name))
                                .collect(),
                        };
                        let _ = reply.send(list).await;
                    }
                    Ok(VoiceCommand::SetRecordingDevice(id)) => {
                        let device = RecordingDeviceId::from_unchecked_guid(&id);
                        if let Err(error) = audio.set_recording_device(&device) {
                            eprintln!("[voice] set recording device failed: {error}");
                        }
                    }
                    Ok(VoiceCommand::SetPlayoutDevice(id)) => {
                        let device = PlayoutDeviceId::from_unchecked_guid(&id);
                        if let Err(error) = audio.set_playout_device(&device) {
                            eprintln!("[voice] set playout device failed: {error}");
                        }
                    }
                    Ok(VoiceCommand::SetCamera(on)) => {
                        #[cfg(target_os = "macos")]
                        if on && camera.is_none() {
                            match camera::start() {
                                Ok((session, source)) => {
                                    match publish_video(&room, "camera", &source, quality).await {
                                        Ok((track, sid)) => {
                                            eprintln!("[voice] camera published sid={sid}");
                                            spawn_video_pump(
                                                track.rtc_track(),
                                                "你".into(),
                                                "camera".into(),
                                                true,
                                                frames.clone(),
                                            );
                                            camera = Some(LocalVideo {
                                                kind: "camera",
                                                source,
                                                track,
                                                sid,
                                                quality,
                                                _capture: CaptureSession::Camera(session),
                                            });
                                        }
                                        Err(error) => {
                                            eprintln!("[voice] publish camera failed: {error}");
                                            let _ = events
                                                .send(VoiceEvent::CameraUnavailable(format!(
                                                    "摄像头发布失败：{error}"
                                                )))
                                                .await;
                                        }
                                    }
                                }
                                Err(reason) => {
                                    eprintln!("[voice] camera start failed: {reason}");
                                    let _ = events.send(VoiceEvent::CameraUnavailable(reason)).await;
                                }
                            }
                        } else if !on && camera.is_some() {
                            let video = camera.take().unwrap();
                            let _ = room.local_participant().unpublish_track(&video.sid).await;
                            drop(video);
                            eprintln!("[voice] camera unpublished");
                        }
                    }
                    Ok(VoiceCommand::SetScreen(on)) => {
                        #[cfg(target_os = "macos")]
                        if on && screen.is_none() {
                            if let Err(reason) =
                                begin_screen(&room, &mut screen, None, quality, &frames).await
                            {
                                eprintln!("[voice] screen start failed: {reason}");
                                let _ = events.send(VoiceEvent::ScreenUnavailable(reason)).await;
                            }
                        } else if !on && screen.is_some() {
                            stop_screen(&room, &mut screen).await;
                        }
                    }
                    Ok(VoiceCommand::SetScreenTarget(target)) => {
                        #[cfg(target_os = "macos")]
                        {
                            if screen.is_some() {
                                stop_screen(&room, &mut screen).await;
                            }
                            if let Err(reason) =
                                begin_screen(&room, &mut screen, Some(target), quality, &frames)
                                    .await
                            {
                                eprintln!("[voice] screen target failed: {reason}");
                                let _ = events.send(VoiceEvent::ScreenUnavailable(reason)).await;
                            }
                        }
                    }
                    Ok(VoiceCommand::ListScreenTargets { reply }) => {
                        #[cfg(target_os = "macos")]
                        {
                            let result = tokio::task::spawn_blocking(screen::list_targets)
                                .await
                                .unwrap_or_else(|error| Err(error.to_string()));
                            let _ = reply.send(result).await;
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = reply.send(Err("当前平台不支持屏幕共享".into())).await;
                        }
                    }
                    Ok(VoiceCommand::Disconnect) | Err(_) => {
                        let _ = room.close().await;
                        let _ = events.send(VoiceEvent::Disconnected("已离开".into())).await;
                        return;
                    }
                }
            }
            _ = tokio::time::sleep_until(force_low_at.unwrap_or_else(tokio::time::Instant::now)),
                if force_low_at.is_some() => {
                force_low_at = None;
                if quality != VideoQuality::Low {
                    quality = VideoQuality::Low;
                    last_adjust = std::time::Instant::now();
                    eprintln!("[voice] video quality -> low (forced)");
                    #[cfg(target_os = "macos")]
                    republish_videos(&room, VideoQuality::Low, &mut camera, &mut screen, &frames)
                        .await;
                    let _ = events
                        .send(VoiceEvent::QualityChanged { low: true })
                        .await;
                }
            }
            event = room_events.recv() => {
                let Some(event) = event else {
                    let _ = events.send(VoiceEvent::Disconnected("连接已关闭".into())).await;
                    return;
                };
                match event {
                    RoomEvent::TrackSubscribed { track, publication, participant } => {
                        if let RemoteTrack::Video(video) = track {
                            let kind = match publication.source() {
                                TrackSource::Camera => "camera",
                                TrackSource::Screenshare => "screen",
                                _ => "video",
                            }
                            .to_string();
                            let identity = display_name(&participant);
                            eprintln!(
                                "[voice] remote video subscribed kind={kind} name={} sid={}",
                                publication.name(),
                                publication.sid()
                            );
                            spawn_video_pump(
                                video.rtc_track(),
                                identity,
                                kind,
                                false,
                                frames.clone(),
                            );
                        }
                    }
                    RoomEvent::ParticipantConnected(participant) => {
                        let name = display_name(&participant);
                        let _ = events.send(VoiceEvent::ParticipantJoined(name)).await;
                    }
                    RoomEvent::ParticipantDisconnected(participant) => {
                        let name = display_name(&participant);
                        let _ = events.send(VoiceEvent::ParticipantLeft(name)).await;
                    }
                    RoomEvent::ConnectionQualityChanged { quality: sample, participant } => {
                        if matches!(participant, Participant::Local(_)) {
                            match sample {
                                ConnectionQuality::Poor | ConnectionQuality::Lost => {
                                    poor_streak += 1;
                                    good_streak = 0;
                                }
                                ConnectionQuality::Good | ConnectionQuality::Excellent => {
                                    good_streak += 1;
                                    poor_streak = 0;
                                }
                            }
                            if let Some(next) = quality_transition(
                                quality,
                                poor_streak,
                                good_streak,
                                last_adjust.elapsed(),
                            ) {
                                quality = next;
                                last_adjust = std::time::Instant::now();
                                poor_streak = 0;
                                good_streak = 0;
                                eprintln!(
                                    "[voice] video quality -> {:?}",
                                    if next == VideoQuality::Low { "low" } else { "high" }
                                );
                                #[cfg(target_os = "macos")]
                                republish_videos(
                                    &room,
                                    next,
                                    &mut camera,
                                    &mut screen,
                                    &frames,
                                )
                                .await;
                                let _ = events
                                    .send(VoiceEvent::QualityChanged {
                                        low: next == VideoQuality::Low,
                                    })
                                    .await;
                            }
                        }
                    }
                    RoomEvent::ConnectionStateChanged(state) => match state {
                        ConnectionState::Reconnecting => {
                            let _ = events.send(VoiceEvent::Reconnecting).await;
                        }
                        ConnectionState::Connected => {
                            let _ = events.send(VoiceEvent::Reconnected).await;
                        }
                        ConnectionState::Disconnected => {
                            let _ = events
                                .send(VoiceEvent::Disconnected("连接已断开".into()))
                                .await;
                            return;
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}

#[cfg(feature = "voice-livekit")]
fn display_name(participant: &livekit::prelude::RemoteParticipant) -> String {
    let name = participant.name();
    if name.is_empty() {
        participant.identity().to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::{VideoQuality, quality_transition};
    use std::time::Duration;

    #[test]
    fn degrades_after_two_poor_samples() {
        assert_eq!(
            quality_transition(VideoQuality::High, 2, 0, Duration::from_secs(30)),
            Some(VideoQuality::Low)
        );
        assert_eq!(
            quality_transition(VideoQuality::High, 1, 0, Duration::from_secs(30)),
            None
        );
    }

    #[test]
    fn recovers_after_three_good_samples() {
        assert_eq!(
            quality_transition(VideoQuality::Low, 0, 3, Duration::from_secs(30)),
            Some(VideoQuality::High)
        );
        assert_eq!(
            quality_transition(VideoQuality::Low, 0, 2, Duration::from_secs(30)),
            None
        );
    }

    #[test]
    fn respects_cooldown() {
        assert_eq!(
            quality_transition(VideoQuality::High, 5, 0, Duration::from_secs(3)),
            None
        );
    }
}
