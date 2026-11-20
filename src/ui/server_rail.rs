use gpui_kit::*;

use crate::app::{DiscordApp, ViewMode};
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};

/// 最左侧服务器栏：主页、服务器入口与功能按钮。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut rail = v_flex()
        .w(px(72.))
        .h_full()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .py_3()
        .bg(rgb(palette::BG_RAIL));

    let home_active = matches!(app.view_mode, ViewMode::DirectMessage | ViewMode::Friends);
    rail = rail.child(
        v_flex()
            .relative()
            .w_full()
            .items_center()
            .gap_1()
            .child(
                div()
                    .w(px(4.))
                    .h(px(if home_active { 40. } else { 8. }))
                    .absolute()
                    .left_0()
                    .top(if home_active { px(4.) } else { px(20.) })
                    .rounded(px(2.))
                    .bg(rgb(palette::WHITE)),
            )
            .child(
                div()
                    .w(px(48.))
                    .h(px(48.))
                    .rounded(px(if home_active { 16. } else { 24. }))
                    .bg(rgb(palette::ACCENT))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(palette::WHITE))
                    .child(ico(IconName::Sparkles, Size::Medium))
                    .hover(|el| el.rounded(px(16.)))
                    .id("rail-home")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_friends(window, cx);
                    })),
            ),
    );

    rail = rail.child(
        div()
            .w(px(32.))
            .h(px(2.))
            .bg(rgb(palette::BG_DIVIDER))
            .rounded_full(),
    );

    for (index, server) in app.servers.iter().enumerate() {
        let active = app.view_mode == ViewMode::Server && index == app.active_server;
        let badge = server.badge.clone();
        let color = server.color;
        let id = format!("rail-server-{index}");

        rail = rail.child(
            v_flex()
                .relative()
                .w_full()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .w(px(4.))
                        .h(px(if active { 40. } else { 8. }))
                        .absolute()
                        .left_0()
                        .top(if active { px(4.) } else { px(20.) })
                        .rounded(px(2.))
                        .bg(rgb(palette::WHITE)),
                )
                .child(
                    div()
                        .w(px(48.))
                        .h(px(48.))
                        .rounded(px(if active { 16. } else { 24. }))
                        .bg(rgb(color))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(palette::WHITE))
                        .text_size(px(17.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(badge)
                        .hover(|el| el.rounded(px(16.)))
                        .id(id)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_server(index, window, cx);
                        })),
                ),
        );
    }

    let explore_active = app.view_mode == ViewMode::Discover;

    rail.child(
        v_flex()
            .w_full()
            .items_center()
            .gap_2()
            .mt_1()
            .child(
                div()
                    .w(px(48.))
                    .h(px(48.))
                    .rounded(px(24.))
                    .bg(rgb(palette::BG_SIDEBAR))
                    .text_color(rgb(palette::SUCCESS))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(ico(IconName::Plus, Size::Medium))
                    .hover(|el| {
                        el.bg(rgb(palette::ACCENT_HOVER))
                            .text_color(rgb(palette::WHITE))
                    })
                    .id("rail-create-server")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_create_server(window, cx);
                    })),
            )
            .child(
                div()
                    .w(px(48.))
                    .h(px(48.))
                    .rounded(px(if explore_active { 16. } else { 24. }))
                    .bg(if explore_active {
                        rgb(palette::ACCENT)
                    } else {
                        rgb(palette::BG_SIDEBAR)
                    })
                    .text_color(if explore_active {
                        rgb(palette::WHITE)
                    } else {
                        rgb(palette::SUCCESS)
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(ico(IconName::Compass, Size::Medium))
                    .hover(|el| {
                        el.bg(rgb(palette::ACCENT_HOVER))
                            .rounded(px(16.))
                            .text_color(rgb(palette::WHITE))
                    })
                    .id("rail-discover")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_discover(window, cx);
                    })),
            )
            .child(
                div()
                    .w(px(32.))
                    .h(px(2.))
                    .bg(rgb(palette::BG_DIVIDER))
                    .rounded_full(),
            )
            .child(
                div()
                    .w(px(48.))
                    .h(px(48.))
                    .rounded(px(24.))
                    .bg(rgb(palette::BG_SIDEBAR))
                    .text_color(rgb(palette::SUCCESS))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(ico(IconName::Download, Size::Medium))
                    .hover(|el| {
                        el.bg(rgb(palette::ACCENT_HOVER))
                            .text_color(rgb(palette::WHITE))
                    }),
            ),
    )
}
