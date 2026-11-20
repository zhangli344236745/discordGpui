use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable, Size};

/// Discord 深色主题色板。组件统一从这里取色，避免散落的魔法数字。
pub mod palette {
    pub const WHITE: u32 = 0xffffff;

    pub const ACCENT: u32 = 0x5865f2;
    pub const ACCENT_HOVER: u32 = 0x3ba55c;
    pub const SUCCESS: u32 = 0x57f287;
    pub const DANGER: u32 = 0xf23f43;
    pub const ONLINE: u32 = 0x23a55a;

    pub const BG_RAIL: u32 = 0x1e1f22;
    pub const BG_SIDEBAR: u32 = 0x2b2d31;
    pub const BG_CHAT: u32 = 0x313338;
    pub const BG_FOOTER: u32 = 0x232428;
    pub const BG_HEADER_BORDER: u32 = 0x26272b;
    pub const BG_SIDEBAR_BORDER: u32 = 0x1f2023;
    pub const BG_DIVIDER: u32 = 0x35363c;
    pub const BG_TOOLBAR_HOVER: u32 = 0x3f4147;
    pub const BG_CHANNEL_HOVER: u32 = 0x35373c;
    pub const BG_CHANNEL_SELECTED: u32 = 0x404249;
    pub const BG_MESSAGE_HOVER: u32 = 0x2e3035;
    pub const BG_COMPOSER: u32 = 0x383a40;
    pub const BG_WELCOME_ICON: u32 = 0x41434a;
    /// 附件图片缩略图的占位底色（Demo 不加载真实图片）。
    pub const BG_ATTACHMENT_PREVIEW: u32 = 0x27282c;
    pub const BG_REACTION_MINE: u32 = 0x3c4270;
    pub const BG_MENTION: u32 = 0x3c4270;
    pub const BG_EDIT: u32 = 0x2b2d31;

    pub const TEXT_LINK: u32 = 0x00a8fc;
    pub const TEXT_PRIMARY: u32 = 0xf2f3f5;
    pub const TEXT_SECONDARY: u32 = 0xb5bac1;
    pub const TEXT_MUTED: u32 = 0x949ba4;
    pub const TEXT_DIM: u32 = 0x80848e;
    pub const TEXT_MESSAGE: u32 = 0xdbdee1;
    pub const TEXT_REACTION_MINE: u32 = 0xc9cdfb;

    pub const BORDER_DEFAULT: u32 = 0x3f4147;
}

/// 统一创建指定尺寸的 Lucide 图标。
pub fn ico(name: IconName, size: Size) -> Icon {
    <Icon as Sizable>::with_size(Icon::new(name), size)
}
