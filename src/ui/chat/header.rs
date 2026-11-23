use gpui_kit::*;

use crate::app::{CallMode, DiscordApp, MenuAnchor, ViewMode};
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 聊天区顶部标题栏：频道名、主题、搜索框与开关。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let is_dm = app.is_dm();
    let active_dm = app.active_dm.clone();
    let dm_call_mode = app
        .dm_call
        .as_ref()
        .filter(|call| Some(&call.friend) == app.active_dm.as_ref())
        .map(|call| call.mode);
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
        .border_color(rgb(palette::bg_header_border()))
        .child(ico(title_icon, Size::Medium).text_color(rgb(palette::text_dim())))
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child(app.session_title()),
        )
        .when(is_dm, |el| {
            el.child(
                h_flex()
                    .ml_1()
                    .gap_1()
                    .text_size(px(11.))
                    .text_color(rgb(palette::text_muted()))
                    .child(
                        div()
                            .w(px(7.))
                            .h(px(7.))
                            .rounded_full()
                            .bg(rgb(if app.active_dm_online() {
                                palette::online()
                            } else {
                                palette::text_dim()
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
                .bg(rgb(palette::bg_toolbar_hover())),
        )
        .child(
            div()
                .flex_1()
                .truncate()
                .text_size(px(13.))
                .text_color(rgb(palette::text_secondary()))
                .child(app.active_topic()),
        )
        .child(
            h_flex()
                .gap_4()
                .text_color(rgb(palette::text_secondary()))
                .child(
                    // 频道设置入口：与频道右键菜单同一个浮层。
                    crate::ui::icon_button(
                        IconName::Bell, Size::Small, px(24.), "chat-channel-menu",
                        app.channel_menu.is_some(),
                        Box::new(|this, _, _| {
                            if this.view_mode == ViewMode::Server {
                                let name = this.active_channel.clone();
                                this.open_channel_menu(name, MenuAnchor::Header);
                            }
                        }), cx,
                    ),
                )
                .child(
                    crate::ui::icon_button(
                        IconName::Pin, Size::Small, px(24.), "chat-toggle-pins",
                        app.show_pins,
                        Box::new(|this, _, _| this.toggle_pins()), cx,
                    ),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .px_2()
                        .h(px(26.))
                        .rounded(px(4.))
                        .bg(rgb(palette::bg_rail()))
                        .child(
                            <Input as Sizable>::with_size(Input::new(&app.search), Size::XSmall)
                                .w(px(130.)),
                        )
                        // 放大镜进搜索中心（跨频道 / 私信），输入框仍做当前会话就地过滤。
                        .child(
                            crate::ui::icon_button(
                                IconName::Search, Size::XSmall, px(18.), "chat-open-search-center",
                                app.show_search_center,
                                Box::new(|this, window, cx| this.open_search_center(window, cx)), cx,
                            ),
                        )
                        // 快速切换器入口：与 Ctrl/⌘+K 等价，便于不使用键盘的用户发现。
                        .child(
                            div()
                                .px_1()
                                .h(px(18.))
                                .flex()
                                .items_center()
                                .rounded(px(3.))
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_dim()))
                                .hover(|el| {
                                    el.bg(rgb(palette::bg_toolbar_hover()))
                                        .text_color(rgb(palette::text_primary()))
                                })
                                .child("⌘K")
                                .id("chat-open-quick-switcher")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_quick_switcher(window, cx);
                                })),
                        ),
                )
                // 私信通话入口：语音 / 视频；与当前会话的通话状态联动高亮。
                .when(is_dm, |el| {
                    let voice_friend = active_dm.clone();
                    let video_friend = active_dm.clone();
                    el.child(
                        h_flex().gap_1().child(
                            crate::ui::icon_button(
                                IconName::Phone,
                                Size::Small,
                                px(24.),
                                "chat-dm-voice-call",
                                dm_call_mode == Some(CallMode::Voice),
                                Box::new(move |this, _, cx| {
                                    if let Some(friend) = voice_friend.clone() {
                                        this.start_dm_call(friend, CallMode::Voice, cx);
                                    }
                                }),
                                cx,
                            ),
                        ).child(
                            crate::ui::icon_button(
                                IconName::Video,
                                Size::Small,
                                px(24.),
                                "chat-dm-video-call",
                                dm_call_mode == Some(CallMode::Video),
                                Box::new(move |this, _, cx| {
                                    if let Some(friend) = video_friend.clone() {
                                        this.start_dm_call(friend, CallMode::Video, cx);
                                    }
                                }),
                                cx,
                            ),
                        ),
                    )
                })
                .child(
                    crate::ui::icon_button(
                        IconName::Inbox, Size::Small, px(24.), "chat-open-inbox",
                        app.show_inbox,
                        Box::new(|this, _, cx| this.toggle_inbox(cx)), cx,
                    )
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
                                    .bg(rgb(palette::danger()))
                                    .text_color(rgb(palette::WHITE))
                                    .text_size(px(9.))
                                    .font_weight(FontWeight::BOLD)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(app.unread_notification_count().to_string()),
                            )
                        }),
                )
                .child(crate::ui::icon_button(
                    IconName::CircleQuestionMark,
                    Size::Small,
                    px(24.),
                    "chat-open-help",
                    app.show_help_center,
                    Box::new(|this, window, cx| this.toggle_help_center(window, cx)),
                    cx,
                )),
        )
        .when(!is_dm, |el| {
            el.child(
                crate::ui::icon_button(
                    IconName::Users, Size::Small, px(28.), "chat-toggle-members",
                    app.show_members,
                    Box::new(|this, _, _| this.toggle_members()), cx,
                ),
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
                .text_color(if !is_dm && app.channel_muted(&app.active_channel) {
                    rgb(palette::text_dim())
                } else if app.notifications_on {
                    rgb(palette::text_message())
                } else {
                    rgb(palette::danger())
                })
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(ico(
                    if (!is_dm && app.channel_muted(&app.active_channel))
                        || (is_dm && !app.notifications_on)
                    {
                        IconName::BellOff
                    } else {
                        IconName::BellDot
                    },
                    Size::Small,
                ))
                .id("chat-toggle-notifications")
                // 服务器频道：打开该频道的通知设置页；私信：仍按全局通知开关处理。
                .on_click(cx.listener(move |this, _, _, cx| {
                    if is_dm {
                        this.toggle_notifications();
                    } else {
                        this.open_active_channel_notify(cx);
                    }
                    cx.notify();
                })),
        )
}
