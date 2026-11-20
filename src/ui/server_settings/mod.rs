//! 服务器设置页：整屏覆盖主界面，左侧分类栏 + 右侧内容。

mod pages;
mod sidebar;

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

use crate::app::{DiscordApp, ServerSettingsSection};
use crate::ui::ico;
use crate::ui::theme::palette;

/// 左侧分类的唯一数据源：枚举、文案、图标。
pub(crate) const SECTIONS: &[(ServerSettingsSection, &str, IconName)] = &[
    (ServerSettingsSection::Overview, "概览", IconName::Server),
    (ServerSettingsSection::Roles, "角色", IconName::Shield),
    (
        ServerSettingsSection::Emoji,
        "表情",
        IconName::FaceSlightlySmiling,
    ),
    (ServerSettingsSection::Invites, "邀请", IconName::Link),
];

/// 当前分类的标题文案。
pub(crate) fn section_title(section: ServerSettingsSection) -> &'static str {
    SECTIONS
        .iter()
        .find(|(value, _, _)| *value == section)
        .map(|(_, label, _)| *label)
        .unwrap_or("服务器设置")
}

/// 设置页全屏骨架：胶囊徽标 + 服务器名 + 关闭按钮。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let title = section_title(app.server_settings_section);
    let server = app.active_server_ref();
    let badge = server.badge.clone();
    let name = server.name.clone();
    let color = server.color;

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
                        .gap_3()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(rgb(palette::bg_header_border()))
                        .child(
                            h_flex()
                                .gap_3()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(28.))
                                        .h(px(28.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(8.))
                                        .bg(rgb(color))
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(palette::WHITE))
                                        .child(badge),
                                )
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(palette::text_primary()))
                                        .child(format!("{name} · {title}")),
                                ),
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
                                .id("server-settings-close")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_server_settings(cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .id("server-settings-content-scroll")
                        .overflow_y_scroll()
                        .child(pages::render(app, cx)),
                ),
        )
}
