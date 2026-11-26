//! macOS 摄像头采集：AVFoundation（native/camera_capture.mm）→ I420 → LiveKit 视频源。

use std::ffi::{CStr, c_char, c_int, c_void};
use std::sync::{Mutex, OnceLock};

use livekit::webrtc::prelude::*;
use livekit::webrtc::video_source::native::NativeVideoSource;

#[repr(C)]
struct NativeCameraHandle {
    session: *mut c_void,
    delegate: *mut c_void,
}

type FrameCallback = extern "C" fn(*const u8, *const u8, c_int, c_int, c_int, c_int);

unsafe extern "C" {
    fn dgpu_camera_start(
        callback: FrameCallback,
        err: *mut c_char,
        err_len: usize,
    ) -> *mut NativeCameraHandle;
    fn dgpu_camera_stop(handle: *mut NativeCameraHandle);
}

/// 采集句柄：Drop 时停止会话。
pub struct CameraSession {
    handle: *mut NativeCameraHandle,
}

// 句柄只在语音线程创建与释放；AVFoundation 内部保证队列安全。
unsafe impl Send for CameraSession {}

impl Drop for CameraSession {
    fn drop(&mut self) {
        unsafe { dgpu_camera_stop(self.handle) };
    }
}

/// 全局帧目标：C 回调没有 userdata，这里只支持单路摄像头。
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
    let (w, h) = (width as usize, height as usize);
    let Some(source) = sink().lock().ok().and_then(|guard| guard.clone()) else {
        return;
    };

    let (w32, h32) = (w as u32, h as u32);
    let buffer = I420Buffer::new(w32, h32);
    unsafe {
        // I420Buffer 没有可变访问器；底层内存由本进程独占，且写入期间无其他引用。
        let (dst_y, dst_u, dst_v) = buffer.data();
        let y_src = std::slice::from_raw_parts(y, stride_y.max(0) as usize * h);
        let uv_src =
            std::slice::from_raw_parts(uv, stride_uv.max(0) as usize * h.div_ceil(2));

        let dst_y_ptr = dst_y.as_ptr() as *mut u8;
        let dst_u_ptr = dst_u.as_ptr() as *mut u8;
        let dst_v_ptr = dst_v.as_ptr() as *mut u8;
        let stride_dst_y = w;
        let stride_dst_uv = w.div_ceil(2);

        for row in 0..h {
            std::ptr::copy_nonoverlapping(
                y_src.as_ptr().add(row * stride_y as usize),
                dst_y_ptr.add(row * stride_dst_y),
                w,
            );
        }
        for row in 0..h.div_ceil(2) {
            for col in 0..w.div_ceil(2) {
                let uv_offset = row * stride_uv as usize + col * 2;
                *dst_u_ptr.add(row * stride_dst_uv + col) = *uv_src.get_unchecked(uv_offset);
                *dst_v_ptr.add(row * stride_dst_uv + col) =
                    *uv_src.get_unchecked(uv_offset + 1);
            }
        }
    }

    let frame = VideoFrame::new(VideoRotation::VideoRotation0, buffer);
    source.capture_frame(&frame);
}

/// 启动摄像头；返回可发布的视频源与会话句柄。
pub fn start() -> Result<(CameraSession, NativeVideoSource), String> {
    let source = NativeVideoSource::new(
        VideoResolution {
            width: 640,
            height: 360,
        },
        false,
    );
    if let Ok(mut guard) = sink().lock() {
        *guard = Some(source.clone());
    }

    let mut err = [0i8; 256];
    let handle = unsafe { dgpu_camera_start(on_frame, err.as_mut_ptr(), err.len()) };
    if handle.is_null() {
        if let Ok(mut guard) = sink().lock() {
            *guard = None;
        }
        let message = unsafe { CStr::from_ptr(err.as_ptr()) }
            .to_string_lossy()
            .to_string();
        return Err(if message.is_empty() {
            "摄像头启动失败".to_string()
        } else {
            message
        });
    }
    Ok((CameraSession { handle }, source))
}
