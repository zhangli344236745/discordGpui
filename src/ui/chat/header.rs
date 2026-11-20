use gpui_kit::*;

use crate::app::{DiscordApp, MenuAnchor, ViewMode};
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 聊天区顶部标题栏：频道名、主题、搜索框与开关。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let is_dm = app.is_dm();
    let title_icon = if is_dm {
        IconName::AtSign
    } else {
        IconName::Hash
    };

    h_flex()
        .h(px(48.))
        .flex_shrink_0()
        .px_4()
        .gap_2()
        .border_b_1()
        .border_color(rgb(palette::BG_HEADER_BORDER))
        .child(ico(title_icon, Size::Medium).text_color(rgb(palette::TEXT_DIM)))
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::TEXT_PRIMARY))
                .child(app.session_title()),
        )
        .when(is_dm, |el| {
            el.child(
                h_flex()
                    .ml_1()
                    .gap_1()
                    .text_size(px(11.))
                    .text_color(rgb(palette::TEXT_MUTED))
                    .child(
                        div()
                            .w(px(7.))
                            .h(px(7.))
                            .rounded_full()
                            .bg(rgb(if app.active_dm_online() {
                                palette::ONLINE
                            } else {
                                palette::TEXT_DIM
                            })),
                    )
                    .child(if app.active_dm_online() {
                        "在线"
                    } else {
                        "离线"
                    }),
            )
        })
        .child(
            div()
                .w(px(1.))
                .h(px(24.))
                .mx_2()
                .bg(rgb(palette::BG_TOOLBAR_HOVER)),
        )
        .child(
            div()
                .flex_1()
                .truncate()
                .text_size(px(13.))
                .text_color(rgb(palette::TEXT_SECONDARY))
                .child(app.active_topic()),
        )
        .child(
            h_flex()
                .gap_4()
                .text_color(rgb(palette::TEXT_SECONDARY))
                .child(
                    // 频道设置入口：与频道右键菜单同一个浮层。
                    div()
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_color(if app.channel_menu.is_some() {
                            rgb(palette::TEXT_PRIMARY)
                        } else {
                            rgb(palette::TEXT_SECONDARY)
                        })
                        .hover(|el| el.bg(rgb(palette::BG_TOOLBAR_HOVER)))
                        .child(ico(IconName::Bell, Size::Small))
                        .id("chat-channel-menu")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.view_mode == ViewMode::Server {
                                let name = this.active_channel.clone();
                                this.open_channel_menu(name, MenuAnchor::Header);
                            }
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_color(if app.show_pins {
                            rgb(palette::TEXT_PRIMARY)
                        } else {
                            rgb(palette::TEXT_SECONDARY)
                        })
                        .hover(|el| el.bg(rgb(palette::BG_TOOLBAR_HOVER)))
                        .child(ico(IconName::Pin, Size::Small))
                        .id("chat-toggle-pins")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_pins();
                            cx.notify();
                        })),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .px_2()
                        .h(px(26.))
                        .rounded(px(4.))
                        .bg(rgb(palette::BG_RAIL))
                        .child(
                            <Input as Sizable>::with_size(Input::new(&app.search), Size::XSmall)
                                .w(px(130.)),
                        )
                        // 放大镜进搜索中心（跨频道 / 私信），输入框仍做当前会话就地过滤。
                        .child(
                            div()
                                .w(px(18.))
                                .h(px(18.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(3.))
                                .text_color(if app.show_search_center {
                                    rgb(palette::TEXT_PRIMARY)
                                } else {
                                    rgb(palette::TEXT_SECONDARY)
                                })
                                .hover(|el| {
                                    el.bg(rgb(palette::BG_TOOLBAR_HOVER))
                                        .text_color(rgb(palette::TEXT_PRIMARY))
                                })
                                .child(ico(IconName::Search, Size::XSmall))
                                .id("chat-open-search-center")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_search_center(window, cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .relative()
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_color(if app.show_inbox {
                            rgb(palette::TEXT_PRIMARY)
                        } else {
                            rgb(palette::TEXT_SECONDARY)
                        })
                        .hover(|el| el.bg(rgb(palette::BG_TOOLBAR_HOVER)))
                        .child(ico(IconName::Inbox, Size::Small))
                        .when(app.unread_notification_count() > 0, |el| {
                            el.child(
                                div()
                                    .absolute()
                                    .right(px(-2.))
                                    .top(px(-2.))
                                    .min_w(px(14.))
                                    .h(px(14.))
                                    .px_1()
                                    .rounded_full()
                                    .bg(rgb(palette::DANGER))
                                    .text_color(rgb(palette::WHITE))
                                    .text_size(px(9.))
                                    .font_weight(FontWeight::BOLD)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(app.unread_notification_count().to_string()),
                            )
                        })
                        .id("chat-open-inbox")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_inbox(cx);
                        })),
                )
                .child(ico(IconName::CircleQuestionMark, Size::Small)),
        )
        .when(!is_dm, |el| {
            el.child(
                div()
                    .ml_2()
                    .w(px(28.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(if app.show_members {
                        rgb(palette::TEXT_PRIMARY)
                    } else {
                        rgb(palette::TEXT_DIM)
                    })
                    .hover(|el| el.bg(rgb(palette::BG_TOOLBAR_HOVER)))
                    .child(ico(IconName::Users, Size::Small))
                    .id("chat-toggle-members")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_members();
                        cx.notify();
                    })),
            )
        })
        .child(
            div()
                .w(px(28.))
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(if app.notifications_on {
                    rgb(palette::TEXT_MESSAGE)
                } else {
                    rgb(palette::DANGER)
                })
                .hover(|el| el.bg(rgb(palette::BG_TOOLBAR_HOVER)))
                .child(ico(IconName::BellDot, Size::Small))
                .id("chat-toggle-notifications")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_notifications();
                    cx.notify();
                })),
        )
}
