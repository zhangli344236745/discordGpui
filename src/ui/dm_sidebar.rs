use gpui_kit::*;

use crate::app::{DiscordApp, ViewMode};
use crate::ui::theme::palette;
use crate::ui::{avatar, ico, unread_badge, user_footer};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 会话行：好友头像 + 名字 + 消息预览 + 未读角标，点击进入私信。
fn conversation_row(
    cx: &mut Context<DiscordApp>,
    friend: &'static str,
    color: u32,
    online: bool,
    unread: usize,
    preview: &'static str,
    selected: bool,
) -> impl IntoElement {
    let click_friend = friend.to_string();
    let id = format!("dm-conversation-{friend}");
    h_flex()
        .h(px(52.))
        .mx_2()
        .px_2()
        .gap_2()
        .rounded(px(4.))
        .bg(if selected {
            rgb(palette::bg_channel_selected())
        } else {
            rgb(palette::bg_sidebar())
        })
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(avatar(friend.chars().next().unwrap_or('?'), color, px(32.), online))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    div()
                        .truncate()
                        .text_size(px(14.))
                        .font_weight(if unread > 0 {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(if unread > 0 {
                            rgb(palette::text_primary())
                        } else {
                            rgb(palette::text_secondary())
                        })
                        .child(friend),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(preview),
                ),
        )
        .when(unread > 0, |el| el.child(unread_badge(unread)))
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.open_dm(click_friend.clone(), window, cx);
        }))
}

/// 分组标题。
fn group_label(text: impl Into<SharedString>) -> Div {
    div()
        .px_4()
        .pt_3()
        .pb_1()
        .text_size(px(11.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(palette::text_muted()))
        .child(text.into())
}

/// 私信侧栏：好友入口、会话列表与用户控制条。
/// 顶部搜索可过滤会话；无会话匹配时列出可发起对话的好友。
/// 在「私信」和「好友」两种视图模式下显示。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let searching = !app.dm_query.trim().is_empty();
    let query = app.dm_query.trim().to_lowercase();

    // 顶部搜索框：真实输入 + 有内容时的清除按钮。
    let mut search_row = h_flex()
        .flex_1()
        .gap_2()
        .px_2()
        .h(px(26.))
        .rounded(px(4.))
        .bg(rgb(palette::bg_rail()))
        .child(ico(IconName::Search, Size::XSmall))
        .child(
            <Input as Sizable>::with_size(Input::new(&app.dm_search), Size::XSmall).flex_1(),
        );
    if searching {
        search_row = search_row.child(
            div()
                .w(px(18.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(3.))
                .text_color(rgb(palette::text_dim()))
                .hover(|el| {
                    el.bg(rgb(palette::bg_channel_hover()))
                        .text_color(rgb(palette::text_primary()))
                })
                .child(ico(IconName::X, Size::XSmall))
                .id("dm-search-clear")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.clear_dm_search(window, cx);
                    cx.notify();
                })),
        );
    }
    let header = h_flex()
        .h(px(48.))
        .w_full()
        .flex_shrink_0()
        .px_4()
        .gap_2()
        .border_b_1()
        .border_color(rgb(palette::bg_sidebar_border()))
        .child(search_row);

    let mut list = v_flex()
        .flex_1()
        .min_h_0()
        .id("dm-conversation-scroll")
        .overflow_y_scroll()
        .pt_3();

    if searching {
        // 搜索模式：匹配会话 + 可发起对话的好友；隐藏好友入口与常规标题。
        let match_conv = |friend: &str, preview: &str| {
            friend.to_lowercase().contains(&query) || preview.to_lowercase().contains(&query)
        };
        let mut matched_conv = 0usize;
        for conversation in &app.dm_conversations {
            if !match_conv(conversation.friend, conversation.preview) {
                continue;
            }
            matched_conv += 1;
            let selected = app.view_mode == ViewMode::DirectMessage
                && app.active_dm.as_deref() == Some(conversation.friend);
            list = list.child(conversation_row(
                cx,
                conversation.friend,
                conversation.color,
                conversation.online,
                conversation.unread,
                conversation.preview,
                selected,
            ));
        }
        if matched_conv > 0 {
            list = list.child(group_label("私信"));
        }

        // 没有会话、但名字匹配的好友：提供「发起对话」入口。
        let mut new_friends: Vec<(usize, &crate::data::Friend)> = app
            .friends
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                f.name.to_lowercase().contains(&query)
                    && !app.dm_conversations.iter().any(|c| c.friend == f.name)
            })
            .collect();
        if !new_friends.is_empty() {
            list = list.child(group_label("发起对话"));
            // 把入口插到「私信」标题之前：先临时收集行，再统一插入。
            let mut new_rows = Vec::new();
            for (_, friend) in new_friends.drain(..) {
                let name = friend.name;
                let color = friend.color;
                let online = friend.online;
                let row = h_flex()
                    .h(px(52.))
                    .mx_2()
                    .px_2()
                    .gap_2()
                    .rounded(px(4.))
                    .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                    .child(avatar(
                        name.chars().next().unwrap_or('?'),
                        color,
                        px(32.),
                        online,
                    ))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(palette::text_primary()))
                                    .child(name),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette::text_dim()))
                                    .child("点击发起新对话"),
                            ),
                    )
                    .child(ico(IconName::MessageSquare, Size::Small))
                    .id(format!("dm-start-{name}"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.start_dm(name, window, cx);
                        cx.notify();
                    }));
                new_rows.push(row);
            }
            for row in new_rows {
                list = list.child(row);
            }
        }

        // 完全无匹配：空状态提示。
        if matched_conv == 0 && list_child_count_hint(app, &query) == 0 {
            list = list.child(
                v_flex()
                    .flex_1()
                    .w_full()
                    .px_4()
                    .pt_10()
                    .gap_2()
                    .items_center()
                    .justify_center()
                    .child(ico(IconName::Search, Size::Large).text_color(rgb(palette::text_dim())))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(palette::text_secondary()))
                            .child("没有找到相关会话"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette::text_dim()))
                            .child("试试其他名字，或到好友页添加好友"),
                    ),
            );
        }
    } else {
        // 常规模式：好友入口 + 会话列表。
        let friends_active = app.view_mode == ViewMode::Friends;
        let friends_entry = h_flex()
            .h(px(40.))
            .mx_2()
            .px_2()
            .gap_3()
            .rounded(px(4.))
            .bg(if friends_active {
                rgb(palette::bg_channel_selected())
            } else {
                rgb(palette::bg_sidebar())
            })
            .text_color(if friends_active {
                rgb(palette::text_primary())
            } else {
                rgb(palette::text_secondary())
            })
            .hover(|el| {
                el.bg(rgb(palette::bg_channel_hover()))
                    .text_color(rgb(palette::text_message()))
            })
            .child(ico(IconName::Users, Size::Small))
            .child(
                div()
                    .flex_1()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("好友"),
            )
            .child(unread_badge(app.friends.iter().filter(|f| f.online).count()))
            .id("dm-friends-entry")
            .on_click(cx.listener(|this, _, window, cx| {
                this.show_friends(window, cx);
            }));

        list = list.child(friends_entry).child(group_label("私信"));

        for conversation in &app.dm_conversations {
            let selected = app.view_mode == ViewMode::DirectMessage
                && app.active_dm.as_deref() == Some(conversation.friend);
            list = list.child(conversation_row(
                cx,
                conversation.friend,
                conversation.color,
                conversation.online,
                conversation.unread,
                conversation.preview,
                selected,
            ));
        }
    }

    v_flex()
        .w(px(240.))
        .h_full()
        .flex_shrink_0()
        .bg(rgb(palette::bg_sidebar()))
        .child(header)
        .child(list)
        .child(user_footer::render(app, cx))
}

/// 搜索模式下名字匹配、且尚无会话的好友数量（用于判断是否显示空状态）。
fn list_child_count_hint(app: &DiscordApp, query: &str) -> usize {
    app.friends
        .iter()
        .filter(|f| {
            f.name.to_lowercase().contains(query)
                && !app.dm_conversations.iter().any(|c| c.friend == f.name)
        })
        .count()
}
