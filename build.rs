//! 构建脚本：仅启用 `voice-livekit` 时放宽链接器符号解析。
//!
//! 预编译的 libwebrtc 引入了 macOS 14+ 的 ScreenCaptureKit 符号
//! （`SCContentSharingPicker` 等）。本机若只有较旧的 Command Line Tools，
//! 链接阶段会报 undefined symbols。音频场景不会走到屏幕共享代码路径，
//! 因此在该 feature 下使用 dynamic_lookup，把符号解析推迟到运行时。
//! 升级到 macOS 14+ SDK 后可移除。

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_VOICE_LIVEKIT").is_some() {
        // libwebrtc 的 ObjC 分类（NSString+StdString 等）没有显式引用，
        // 静态库默认会被 dead-strip；-ObjC 强制加载含 ObjC 类/分类的目标文件。
        println!("cargo:rustc-link-arg=-Wl,-ObjC");
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    }
}
