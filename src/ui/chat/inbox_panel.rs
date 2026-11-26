use gpui_kit::*;

use crate::app::{DiscordApp, NotificationFilter};
use crate::data::{Notification, NotificationKind};
use crate::ui::theme::palette;
use crate::ui::{avatar, filter_chip, ico};

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
        NotificationKind::System => (IconName::Bell, "系统"),
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

/// 分组顺序：提及 → 私信 → 频道 → 好友请求 → 系统。
fn kind_rank(kind: NotificationKind) -> usize {
    match kind {
        NotificationKind::Mention => 0,
        NotificationKind::Dm => 1,
        NotificationKind::UnreadChannel => 2,
        NotificationKind::FriendRequest => 3,
        NotificationKind::System => 4,
    }
}

/// 分组标题：类型图标 + 名称 + 条数。
fn group_header(kind: NotificationKind, count: usize) -> impl IntoElement {
    let (icon, label) = kind_meta(kind);
    h_flex()
        .w_full()
        .px_2()
        .pt_2()
        .pb_1()
        .gap_2()
        .items_center()
        .child(ico(icon, Size::XSmall).text_color(rgb(palette::text_dim())))
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(palette::text_dim()))
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette::text_muted()))
                .child(format!("{count}")),
        )
        .child(
            div()
                .flex_1()
                .h(px(1.))
                .bg(rgb(palette::bg_divider())),
        )
}

/// 收件箱面板：覆盖聊天区右侧，聚合提及/未读/私信/好友请求，点击跳转。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let notifications = app.notifications();
    let unread_count = notifications
        .iter()
        .filter(|n| !app.notification_is_read(n))
        .count();
    let read_count = notifications.len().saturating_sub(unread_count);
    let active = app.inbox_filter;

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
                        .child(format!("{} 未读 · 共 {} 条", unread_count, notifications.len())),
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
                        .when(unread_count == 0, |el| {
                            el.text_color(rgb(palette::text_muted()))
                        })
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
                .when(read_count > 0, |el| {
                    el.child(
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
                            .child(format!("清除已读 {read_count}"))
                            .id("inbox-clear-read")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.clear_read_notifications(cx);
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

    // 筛选标签：全部 / 未读 / 各来源类型，带条数。
    let mut filters = h_flex()
        .flex_shrink_0()
        .px_3()
        .py_2()
        .gap_1()
        .flex_wrap()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()));
    for filter in NotificationFilter::ALL {
        let count = match filter {
            NotificationFilter::All => notifications.len(),
            NotificationFilter::Unread => unread_count,
            filter => notifications
                .iter()
                .filter(|n| filter.matches_kind(n.kind))
                .count(),
        };
        filters = filters.child(filter_chip(
            format!("{} {count}", filter.label()),
            active == filter,
            format!("inbox-filter-{}", filter.label()),
            Box::new(move |this, _, cx| {
                this.set_inbox_filter(filter, cx);
            }),
            cx,
        ));
    }

    // 应用筛选；「全部」按来源分组展示。
    let mut visible: Vec<(usize, &Notification)> = notifications
        .iter()
        .enumerate()
        .filter(|(_, n)| active.matches_kind(n.kind))
        .filter(|(_, n)| active != NotificationFilter::Unread || !app.notification_is_read(n))
        .collect();
    if active == NotificationFilter::All {
        visible.sort_by_key(|(_, n)| kind_rank(n.kind));
    }

    let mut body = v_flex()
        .flex_1()
        .min_h_0()
        .p_2()
        .gap_1()
        .id("inbox-scroll")
        .overflow_y_scroll();

    if visible.is_empty() {
        let (title, hint) = if notifications.is_empty() {
            ("收件箱空空如也", "有新的提及或未读消息时会显示在这里")
        } else {
            ("没有符合条件的通知", "换个筛选条件，或点「全部已读」后再清除")
        };
        body = body.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(palette::text_muted()))
                .child(ico(IconName::Inbox, Size::Large))
                .child(div().text_size(px(13.)).child(title))
                .child(div().text_size(px(11.)).child(hint)),
        );
    } else {
        let grouped = active == NotificationFilter::All;
        let mut last_group: Option<NotificationKind> = None;
        for (index, notification) in visible {
            if grouped && last_group != Some(notification.kind) {
                let count = notifications
                    .iter()
                    .filter(|n| n.kind == notification.kind)
                    .count();
                body = body.child(group_header(notification.kind, count));
                last_group = Some(notification.kind);
            }
            let unread = !app.notification_is_read(notification);
            body = body.child(notification_row(notification, unread, index, cx));
        }
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
        .child(filters)
        .child(body)
}
