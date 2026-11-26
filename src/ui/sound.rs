//! 系统提示音：消息提示与来电铃声（macOS 用 afplay 播放系统音效）。
//!
//! 不引入音频依赖：`afplay` 是 macOS 自带命令，播放系统 aiff 即可满足提示音需求。
//! 非 macOS 平台退化为空操作；测试环境（无音频设备）也不会阻塞调用方。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 消息提示音（短促）。
pub fn play_message(volume: f32) {
    throttle_then(|| play_file("/System/Library/Sounds/Tink.aiff", volume));
}

/// 通话来电铃声：循环播放直到 `CallRing` 被丢弃。
pub struct CallRing {
    stop: Arc<AtomicBool>,
}

impl CallRing {
    pub fn start(volume: f32) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(target_os = "macos")]
        {
            let flag = stop.clone();
            std::thread::Builder::new()
                .name("dgpui-call-ring".into())
                .spawn(move || {
                    while !flag.load(Ordering::Relaxed) {
                        play_file("/System/Library/Sounds/Glass.aiff", volume);
                        // 播放完稍作停顿，避免连续叠音
                        for _ in 0..15 {
                            if flag.load(Ordering::Relaxed) {
                                return;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    }
                })
                .ok();
        }
        Self { stop }
    }
}

impl Drop for CallRing {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn throttle_then(action: impl FnOnce()) {
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();
    let previous = LAST.load(Ordering::Relaxed);
    if now.saturating_sub(previous) < 800 {
        return;
    }
    LAST.store(now, Ordering::Relaxed);
    action();
}

#[cfg(target_os = "macos")]
fn play_file(path: &'static str, volume: f32) {
    let volume = volume.clamp(0.0, 1.0);
    let volume_arg = format!("{volume:.2}");
    std::thread::Builder::new()
        .name("dgpui-sound".into())
        .spawn(move || {
            let _ = std::process::Command::new("/usr/bin/afplay")
                .arg(path)
                .arg("-v")
                .arg(volume_arg)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        })
        .ok();
}

#[cfg(not(target_os = "macos"))]
fn play_file(_path: &str, _volume: f32) {}
