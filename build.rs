//! 构建脚本：语音 feature 下的链接参数与 macOS 原生采集代码。
//!
//! 1. `voice-livekit` 启用时放宽链接器符号解析：
//!    - `-ObjC`：libwebrtc 的 ObjC 分类（`NSString+StdString` 等）需要强制加载；
//!    - `dynamic_lookup`：预编译库引用 macOS 14+ ScreenCaptureKit 符号，旧 SDK 链接不到。
//! 2. 同 feature 下编译 `native/camera_capture.mm`（AVFoundation 摄像头采集）。
//!
//! 升级到 macOS 14+ SDK、且预编译库与本地工具链一致后，第 1 项可移除。

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=native/camera_capture.mm");
    println!("cargo:rerun-if-changed=native/camera_capture.h");
    println!("cargo:rerun-if-changed=native/screen_capture.mm");
    println!("cargo:rerun-if-changed=native/screen_capture.h");

    let voice = std::env::var_os("CARGO_FEATURE_VOICE_LIVEKIT").is_some();
    if !voice {
        return;
    }

    println!("cargo:rustc-link-arg=-Wl,-ObjC");
    println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "macos" {
        return;
    }

    cc::Build::new()
        .file("native/camera_capture.mm")
        .file("native/screen_capture.mm")
        .include("native")
        .flag("-fobjc-arc")
        .flag_if_supported("-Wno-deprecated-declarations")
        .compile("dgpu_capture");

    for framework in [
        "AVFoundation",
        "CoreMedia",
        "CoreVideo",
        "Foundation",
        "CoreGraphics",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
