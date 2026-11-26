//! macOS 屏幕采集：CGDisplayStream（native/screen_capture.mm）→ I420 → LiveKit 视频源。

use std::ffi::{CStr, c_char, c_int, c_void};
use std::sync::{Mutex, OnceLock};

use livekit::webrtc::prelude::*;
use livekit::webrtc::video_source::native::NativeVideoSource;

use super::yuv::nv12_to_i420;

#[repr(C)]
struct NativeScreenHandle {
    stream: *mut c_void,
}

type FrameCallback = extern "C" fn(*const u8, *const u8, c_int, c_int, c_int, c_int);

unsafe extern "C" {
    fn dgpu_screen_start(
        callback: FrameCallback,
        max_width: c_int,
        max_height: c_int,
        err: *mut c_char,
        err_len: usize,
    ) -> *mut NativeScreenHandle;
    fn dgpu_screen_stop(handle: *mut NativeScreenHandle);
}

pub struct ScreenSession {
    handle: *mut NativeScreenHandle,
}

unsafe impl Send for ScreenSession {}

impl Drop for ScreenSession {
    fn drop(&mut self) {
        unsafe { dgpu_screen_stop(self.handle) };
    }
}

static SINK: OnceLock<Mutex<Option<NativeVideoSource>>> = OnceLock::new();

fn sink() -> &'static Mutex<Option<NativeVideoSource>> {
    SINK.get_or_init(|| Mutex::new(None))
}

extern "C" fn on_frame(
    y: *const u8,
    uv: *const u8,
    width: c_int,
    height: c_int,
    stride_y: c_int,
    stride_uv: c_int,
) {
    if y.is_null() || uv.is_null() || width <= 0 || height <= 0 {
        return;
    }
    let Some(source) = sink().lock().ok().and_then(|guard| guard.clone()) else {
        return;
    };
    let (w, h) = (width as usize, height as usize);
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static LOGGED: AtomicBool = AtomicBool::new(false);
        if !LOGGED.swap(true, Ordering::Relaxed) {
            eprintln!("[voice] screen capture first frame {w}x{h}");
        }
    }
    let (sy, suv) = (stride_y.max(0) as usize, stride_uv.max(0) as usize);
    let y_slice = unsafe { std::slice::from_raw_parts(y, sy * h) };
    let uv_slice = unsafe { std::slice::from_raw_parts(uv, suv * h.div_ceil(2)) };
    let buffer = nv12_to_i420(y_slice, uv_slice, w, h, sy, suv);
    let frame = VideoFrame::new(VideoRotation::VideoRotation0, buffer);
    source.capture_frame(&frame);
}

/// 启动主屏采集（最大 1280x720）。
pub fn start() -> Result<(ScreenSession, NativeVideoSource), String> {
    let source = NativeVideoSource::new(
        VideoResolution {
            width: 1280,
            height: 720,
        },
        false,
    );
    if let Ok(mut guard) = sink().lock() {
        *guard = Some(source.clone());
    }

    let mut err = [0i8; 256];
    let handle = unsafe { dgpu_screen_start(on_frame, 1280, 720, err.as_mut_ptr(), err.len()) };
    if handle.is_null() {
        if let Ok(mut guard) = sink().lock() {
            *guard = None;
        }
        let message = unsafe { CStr::from_ptr(err.as_ptr()) }
            .to_string_lossy()
            .to_string();
        return Err(if message.is_empty() {
            "屏幕采集启动失败".to_string()
        } else {
            message
        });
    }
    Ok((ScreenSession { handle }, source))
}
