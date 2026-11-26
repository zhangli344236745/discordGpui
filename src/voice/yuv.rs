//! NV12（摄像头/屏幕采集）→ I420（LiveKit 视频源）。

use livekit::webrtc::prelude::I420Buffer;

/// 把 NV12 平面拷进 I420 缓冲。
///
/// I420Buffer 没有可变访问器；底层内存由本进程独占，写入期间没有其他引用，
/// 因此在取到裸指针后按行拷贝（调用方保证帧在同一线程构建并立即送出）。
pub fn nv12_to_i420(
    y: &[u8],
    uv: &[u8],
    width: usize,
    height: usize,
    stride_y: usize,
    stride_uv: usize,
) -> I420Buffer {
    let w32 = width as u32;
    let h32 = height as u32;
    let buffer = I420Buffer::new(w32, h32);
    let chroma_width = width.div_ceil(2);
    let chroma_height = height.div_ceil(2);

    unsafe {
        let (dst_y, dst_u, dst_v) = buffer.data();
        let dst_y_ptr = dst_y.as_ptr() as *mut u8;
        let dst_u_ptr = dst_u.as_ptr() as *mut u8;
        let dst_v_ptr = dst_v.as_ptr() as *mut u8;

        for row in 0..height {
            let src = row * stride_y;
            if src + width > y.len() {
                break;
            }
            std::ptr::copy_nonoverlapping(
                y.as_ptr().add(src),
                dst_y_ptr.add(row * width),
                width,
            );
        }
        for row in 0..chroma_height {
            let src = row * stride_uv;
            if src + chroma_width * 2 > uv.len() {
                break;
            }
            for col in 0..chroma_width {
                let uv_offset = src + col * 2;
                *dst_u_ptr.add(row * chroma_width + col) = *uv.get_unchecked(uv_offset);
                *dst_v_ptr.add(row * chroma_width + col) = *uv.get_unchecked(uv_offset + 1);
            }
        }
    }
    buffer
}
