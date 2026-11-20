use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::{Notification, NotificationKind};
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 通知类型对应的图标与中文标签。
fn kind_meta(kind: NotificationKind) -> (IconName, &'static str) {
    match kind {
        NotificationKind::Mention => (IconName::AtSign, "提及"),
        NotificationKind::UnreadChannel => (IconName::Hash, "未读"),
        NotificationKind::Dm => (IconName::MessageSquare, "私信"),
        NotificationKind::FriendRequest => (IconName::UserPlus, "好友请求"),
    }
}

/// 单条通知：左侧类型图标 + 标题/正文 + 未读圆点，点击跳转。
fn notification_row(
    notification: &Notification,
    unread: bool,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let (icon, kind_label) = kind_meta(notification.kind);
    let author_letter = notification
        .title
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic())
        .unwrap_or('?');

    h_flex()
        .w_full()
        .p_3()
        .gap_3()
        .rounded(px(6.))
        .bg(if unread {
            rgb(palette::bg_rail())
        } else {
            rgb(palette::bg_sidebar())
        })
        .border_1()
        .border_color(if unread {
            rgb(palette::border_default())
        } else {
            rgb(palette::bg_divider())
        })
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .id(format!("inbox-item-{index}"))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.open_notification(index, window, cx);
            cx.notify();
        }))
        .child(avatar(author_letter, notification.color, px(36.), false))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(notification.color))
                                .child(kind_label),
                        )
                        .child(ico(icon, Size::XSmall).text_color(rgb(palette::text_dim())))
                        .child(
                            div()
                                .flex_1()
                                .text_right()
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_dim()))
                                .child(notification.time),
                        ),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(if unread {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(if unread {
                            rgb(palette::text_primary())
                        } else {
                            rgb(palette::text_secondary())
                        })
                        .child(notification.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .text_color(rgb(palette::text_dim()))
                        .child(notification.body.clone()),
                ),
        )
        .when(unread, |el| {
            el.child(
                div()
                    .w(px(8.))
                    .h(px(8.))
                    .mt_1()
                    .rounded_full()
                    .bg(rgb(palette::danger())),
            )
        })
}

/// 收件箱面板：覆盖聊天区右侧，聚合提及/未读/私信/好友请求，点击跳转。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let notifications = app.notifications();
    let unread_count = app.unread_notification_count();

    let header = h_flex()
        .h(px(48.))
        .flex_shrink_0()
        .px_4()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(
            v_flex()
                .gap_0()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("收件箱"),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child(format!("{} 未读", unread_count)),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_primary()))
                        })
                        .child("全部已读")
                        .id("inbox-mark-all")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.mark_all_notifications_read(cx);
                        })),
                )
                .child(
                    div()
                        .w(px(28.))
                        .h(px(28.))
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
                        .id("inbox-close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_inbox(cx);
                        })),
                ),
        );

    let mut body = v_flex()
        .flex_1()
        .min_h_0()
        .p_2()
        .gap_2()
        .id("inbox-scroll")
        .overflow_y_scroll();

    if notifications.is_empty() {
        body = body.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(palette::text_muted()))
                .child(ico(IconName::Inbox, Size::Large))
                .child(
                    div()
                        .text_size(px(13.))
                        .child("收件箱空空如也"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .child("有新的提及或未读消息时会显示在这里"),
                ),
        );
    }

    for (index, notification) in notifications.iter().enumerate() {
        let unread = app.is_notification_unread(index);
        body = body.child(notification_row(notification, unread, index, cx));
    }

    div()
        .absolute()
        .right(px(0.))
        .top(px(0.))
        .bottom(px(0.))
        .w(px(360.))
        .flex()
        .flex_col()
        .bg(rgb(palette::bg_sidebar()))
        .border_l_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(header)
        .child(body)
}
