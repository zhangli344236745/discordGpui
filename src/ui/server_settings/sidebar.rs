//! 服务器设置左侧分类栏：与 mod.rs 的 SECTIONS 共用唯一数据源。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

use crate::app::DiscordApp;
use crate::ui::ico;
use crate::ui::theme::palette;

use super::SECTIONS;

/// 分类列表；点击只切换右侧内容，不离开设置页。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let mut column = v_flex()
        .w(px(220.))
        .flex_shrink_0()
        .h_full()
        .py_4()
        .px_2()
        .gap(px(2.))
        .bg(rgb(palette::BG_SIDEBAR))
        .child(
            div()
                .px_3()
                .pb_3()
                .text_size(px(11.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(palette::TEXT_MUTED))
                .child("服务器设置"),
        );

    for (index, (section, label, icon)) in SECTIONS.iter().enumerate() {
        let active = app.server_settings_section == *section;
        let value = *section;
        let id = format!("server-settings-section-{index}");

        column = column.child(
            div()
                .h(px(36.))
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .rounded(px(4.))
                .bg(if active {
                    rgb(palette::BG_CHANNEL_SELECTED)
                } else {
                    rgb(palette::BG_SIDEBAR)
                })
                .text_size(px(13.))
                .text_color(if active {
                    rgb(palette::TEXT_PRIMARY)
                } else {
                    rgb(palette::TEXT_SECONDARY)
                })
                .hover(|el| el.bg(rgb(palette::BG_CHANNEL_HOVER)))
                .child(ico(*icon, Size::Small))
                .child(*label)
                .when(active, |el| {
                    el.child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_end()
                            .child(ico(IconName::Check, Size::XSmall)),
                    )
                })
                .id(id)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_server_settings_section(value, cx);
                })),
        );
    }

    column
}
