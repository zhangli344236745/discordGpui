//! 用户设置页：全屏覆盖主界面，左侧分类栏 + 右侧设置内容。

pub mod pages;
pub mod sidebar;

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

use crate::app::{DiscordApp, SettingsSection};
use crate::ui::ico;
use crate::ui::theme::palette;

/// 左侧分类的唯一数据源：枚举、文案、图标。
pub(crate) const SECTIONS: &[(SettingsSection, &str, IconName)] = &[
    (SettingsSection::Account, "账户", IconName::CircleUser),
    (SettingsSection::Privacy, "隐私与安全", IconName::Shield),
    (SettingsSection::Appearance, "外观", IconName::Palette),
    (SettingsSection::VoiceVideo, "声音与视频", IconName::Headphones),
    (SettingsSection::Notifications, "通知", IconName::Bell),
    (SettingsSection::TextImage, "文字与图片", IconName::TextCursorInput),
    (SettingsSection::Keybinds, "快捷键", IconName::Keyboard),
    (SettingsSection::Language, "语言", IconName::Globe),
];

/// 当前分类的标题文案。
pub(crate) fn section_title(section: SettingsSection) -> &'static str {
    SECTIONS
        .iter()
        .find(|(value, _, _)| *value == section)
        .map(|(_, label, _)| *label)
        .unwrap_or("用户设置")
}

/// 设置页全屏骨架。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let title = section_title(app.settings_section);

    h_flex()
        .size_full()
        .bg(rgb(palette::bg_chat()))
        .text_color(rgb(palette::text_message()))
        .text_size(px(14.))
        .child(sidebar::render(app, cx))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(
                    h_flex()
                        .h(px(56.))
                        .flex_shrink_0()
                        .px_6()
                        .gap_2()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(rgb(palette::bg_header_border()))
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child(title),
                        )
                        .child(
                            div()
                                .w(px(30.))
                                .h(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.))
                                .text_color(rgb(palette::text_secondary()))
                                .hover(|el| {
                                    el.bg(rgb(palette::bg_toolbar_hover()))
                                        .text_color(rgb(palette::text_primary()))
                                })
                                .child(ico(IconName::X, Size::Small))
                                .id("settings-close")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_settings();
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .id("settings-content-scroll")
                        .overflow_y_scroll()
                        .child(pages::render(app, cx)),
                ),
        )
}
