use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable, Size};

/// Discord 主题色板。深色是默认值，浅色通过 `palette::set_light(true)` 运行时切换。
///
/// 所有颜色都是**函数**（形如 `palette::bg_chat()`）而不是常量：
/// 主题存在一个原子量里，渲染时读取，因此切换主题只要改这个原子量并触发一次重绘，
/// 整棵树就会用新色板重建，不需要把主题一路透传进每个视图。
pub mod palette {
    use std::sync::atomic::{AtomicBool, Ordering};

    /// 当前是否为浅色主题。
    static LIGHT: AtomicBool = AtomicBool::new(false);

    /// 切换主题；调用方负责随后 `cx.notify()` 让界面按新色板重建。
    pub fn set_light(light: bool) {
        LIGHT.store(light, Ordering::Relaxed);
    }

    pub fn is_light() -> bool {
        LIGHT.load(Ordering::Relaxed)
    }

    /// 批量声明形如 `bg_chat() -> u32` 的主题感知 token。
    macro_rules! tokens {
        ($($name:ident = $dark:expr => $light:expr),* $(,)?) => {
            $(
                #[inline]
                pub fn $name() -> u32 {
                    if is_light() { $light } else { $dark }
                }
            )*
        };
    }

    /// 纯白：只用于「彩色底上的文字/图标」（徽标、按钮图标、开关滑块），不随主题变化。
    pub const WHITE: u32 = 0xffffff;

    tokens! {
        // 品牌色与语义色
        accent = 0x5865f2 => 0x5865f2,
        accent_hover = 0x3ba55c => 0x4752c4,
        success = 0x57f287 => 0x248046,
        danger = 0xf23f43 => 0xd83c3f,
        online = 0x23a55a => 0x23a55a,

        // 背景层次（由深到浅：服务器栏 → 侧栏 → 聊天区）
        bg_rail = 0x1e1f22 => 0xe3e5e8,
        bg_sidebar = 0x2b2d31 => 0xf2f3f5,
        bg_chat = 0x313338 => 0xffffff,
        bg_footer = 0x232428 => 0xe9eaed,
        bg_header_border = 0x26272b => 0xe3e5e8,
        bg_sidebar_border = 0x1f2023 => 0xd7d9dc,
        bg_divider = 0x35363c => 0xe3e5e8,
        bg_toolbar_hover = 0x3f4147 => 0xdfe1e5,
        bg_channel_hover = 0x35373c => 0xe3e5e8,
        bg_channel_selected = 0x404249 => 0xdbdde1,
        bg_message_hover = 0x2e3035 => 0xf6f7f8,
        bg_composer = 0x383a40 => 0xebedef,
        bg_welcome_icon = 0x41434a => 0xd3d6da,
        // 附件图片缩略图 / 贴纸卡的占位底色（Demo 不加载真实图片）
        bg_attachment_preview = 0x27282c => 0xe9eaed,
        bg_reaction_mine = 0x3c4270 => 0xdfe3ff,
        bg_mention = 0x3c4270 => 0xdfe3ff,
        bg_edit = 0x2b2d31 => 0xf2f3f5,

        // 文字
        text_link = 0x00a8fc => 0x006ce7,
        text_primary = 0xf2f3f5 => 0x060607,
        text_secondary = 0xb5bac1 => 0x4e5058,
        text_muted = 0x949ba4 => 0x5c5e66,
        text_dim = 0x80848e => 0x80848e,
        text_message = 0xdbdee1 => 0x2e3035,
        text_reaction_mine = 0xc9cdfb => 0x4752c4,

        // 描边
        border_default = 0x3f4147 => 0xd7d9dc,
    }
}

/// 统一创建指定尺寸的 Lucide 图标。
pub fn ico(name: IconName, size: Size) -> Icon {
    <Icon as Sizable>::with_size(Icon::new(name), size)
}
