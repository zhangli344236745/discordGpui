//! 语音引擎抽象。
//!
//! - 未启用 `voice-livekit` feature 时返回 `None`，调用方保持「仅信令」行为
//!   （加入/离开、参与者投影由后端维护）。
//! - 启用后，在独立线程里跑 tokio 运行时，用 LiveKit SDK 建立真实房间、
//!   发布麦克风并自动播放远端音频；UI 通过 `async_channel` 收事件。

use async_channel::{Receiver, Sender};

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
    Failed(String),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum VoiceCommand {
    SetMic(bool),
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
