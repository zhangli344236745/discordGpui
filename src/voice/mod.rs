//! 语音引擎抽象。
//!
//! - 未启用 `voice-livekit` feature 时返回 `None`，调用方保持「仅信令」行为
//!   （加入/离开、参与者投影由后端维护）。
//! - 启用后，在独立线程里跑 tokio 运行时，用 LiveKit SDK 建立真实房间、
//!   发布麦克风并自动播放远端音频；UI 通过 `async_channel` 收事件。

use async_channel::{Receiver, Sender};

#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
mod camera;
#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
mod screen;
#[cfg(all(feature = "voice-livekit", target_os = "macos"))]
mod yuv;

// 未启用 voice-livekit 时这些变体不会构造，但 app.rs 仍按同一契约处理。
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
    Failed(String),
}

/// 可用音频设备：`(id, name)`，按系统顺序。
#[derive(Clone, Debug, Default)]
pub struct DeviceList {
    pub recording: Vec<(String, String)>,
    pub playout: Vec<(String, String)>,
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
    SetScreen(bool),
    Disconnect,
}

pub struct VoiceHandle {
    pub commands: Sender<VoiceCommand>,
    pub events: Receiver<VoiceEvent>,
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
            runtime.block_on(run(url, token, event_tx, command_rx));
        })
        .ok()?;

    Some(VoiceHandle {
        commands: command_tx,
        events: event_rx,
    })
}

#[cfg(feature = "voice-livekit")]
async fn run(
    url: String,
    token: String,
    events: Sender<VoiceEvent>,
    commands: Receiver<VoiceCommand>,
) {
    use livekit::prelude::*;
    #[cfg(all(feature = "voice-livekit", target_os = "macos"))]
    use livekit::webrtc::prelude::RtcVideoSource;

    // 平台 ADM：负责麦克风采集与远端音频播放，drop 时自动释放。
    let audio = match PlatformAudio::new() {
        Ok(audio) => audio,
        Err(error) => {
            eprintln!("[voice] platform audio unavailable: {error}");
            let _ = events.send(VoiceEvent::Failed(format!("音频设备不可用：{error}"))).await;
            return;
        }
    };

    let (room, mut room_events) = match Room::connect(&url, &token, RoomOptions::default()).await {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("[voice] room connect failed: {error}");
            let _ = events.send(VoiceEvent::Failed(format!("连接语音房间失败：{error}"))).await;
            return;
        }
    };
    eprintln!("[voice] room connected");

    let track = LocalAudioTrack::create_audio_track("mic", audio.rtc_source());
    let options = TrackPublishOptions {
        source: TrackSource::Microphone,
        ..Default::default()
    };
    // 摄像头按需启动：SetCamera(true) 才创建采集与轨道
    #[cfg(all(feature = "voice-livekit", target_os = "macos"))]
    let mut camera: Option<(camera::CameraSession, TrackSid)> = None;
    #[cfg(all(feature = "voice-livekit", target_os = "macos"))]
    let mut screen: Option<(screen::ScreenSession, TrackSid)> = None;

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
                    Ok(VoiceCommand::SetScreen(on)) => {
                        #[cfg(all(feature = "voice-livekit", target_os = "macos"))]
                        if on && screen.is_none() {
                            match screen::start() {
                                Ok((session, source)) => {
                                    let track = LocalVideoTrack::create_video_track(
                                        "screen",
                                        RtcVideoSource::Native(source),
                                    );
                                    let options = TrackPublishOptions {
                                        source: TrackSource::Screenshare,
                                        ..Default::default()
                                    };
                                    match room
                                        .local_participant()
                                        .publish_track(LocalTrack::Video(track), options)
                                        .await
                                    {
                                        Ok(publication) => {
                                            eprintln!("[voice] screen published");
                                            screen = Some((session, publication.sid()));
                                        }
                                        Err(error) => {
                                            eprintln!("[voice] publish screen failed: {error}");
                                            let _ = events
                                                .send(VoiceEvent::ScreenUnavailable(format!(
                                                    "屏幕共享发布失败：{error}"
                                                )))
                                                .await;
                                        }
                                    }
                                }
                                Err(reason) => {
                                    eprintln!("[voice] screen start failed: {reason}");
                                    let _ = events
                                        .send(VoiceEvent::ScreenUnavailable(reason))
                                        .await;
                                }
                            }
                        } else if !on && screen.is_some() {
                            let (session, sid) = screen.take().unwrap();
                            let _ = room.local_participant().unpublish_track(&sid).await;
                            drop(session);
                            eprintln!("[voice] screen unpublished");
                        }
                    }
                    Ok(VoiceCommand::SetCamera(on)) => {
                        #[cfg(all(feature = "voice-livekit", target_os = "macos"))]
                        if on && camera.is_none() {
                            match camera::start() {
                                Ok((session, source)) => {
                                    let track = LocalVideoTrack::create_video_track(
                                        "camera",
                                        RtcVideoSource::Native(source),
                                    );
                                    let options = TrackPublishOptions {
                                        source: TrackSource::Camera,
                                        ..Default::default()
                                    };
                                    match room
                                        .local_participant()
                                        .publish_track(LocalTrack::Video(track), options)
                                        .await
                                    {
                                        Ok(publication) => {
                                            eprintln!("[voice] camera published");
                                            camera = Some((session, publication.sid()));
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
                                    let _ = events
                                        .send(VoiceEvent::CameraUnavailable(reason))
                                        .await;
                                }
                            }
                        } else if !on && camera.is_some() {
                            let (session, sid) = camera.take().unwrap();
                            let _ = room.local_participant().unpublish_track(&sid).await;
                            drop(session);
                            eprintln!("[voice] camera unpublished");
                        }
                    }
                    Ok(VoiceCommand::Disconnect) | Err(_) => {
                        let _ = room.close().await;
                        let _ = events.send(VoiceEvent::Disconnected("已离开".into())).await;
                        return;
                    }
                }
            }
            event = room_events.recv() => {
                let Some(event) = event else {
                    let _ = events.send(VoiceEvent::Disconnected("连接已关闭".into())).await;
                    return;
                };
                match event {
                    RoomEvent::ParticipantConnected(participant) => {
                        let name = display_name(&participant);
                        let _ = events.send(VoiceEvent::ParticipantJoined(name)).await;
                    }
                    RoomEvent::ParticipantDisconnected(participant) => {
                        let name = display_name(&participant);
                        let _ = events.send(VoiceEvent::ParticipantLeft(name)).await;
                    }
                    RoomEvent::ConnectionStateChanged(state) => match state {
                        ConnectionState::Reconnecting => {
                            let _ = events.send(VoiceEvent::Reconnecting).await;
                        }
                        ConnectionState::Connected => {
                            let _ = events.send(VoiceEvent::Reconnected).await;
                        }
                        ConnectionState::Disconnected => {
                            let _ = events.send(VoiceEvent::Disconnected("连接已断开".into())).await;
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
