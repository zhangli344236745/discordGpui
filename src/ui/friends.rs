use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::FriendTab;
use crate::ui::theme::palette;
use crate::ui::{avatar, ico, section_label};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

/// 好友页标签项：枚举 + 文案 + 图标。
const TABS: &[(FriendTab, &str, IconName)] = &[
    (FriendTab::Online, "在线", IconName::Users),
    (FriendTab::Pending, "待处理", IconName::UserPlus),
    (FriendTab::All, "全部", IconName::List),
    (FriendTab::Blocked, "已屏蔽", IconName::Shield),
    (FriendTab::AddFriend, "添加好友", IconName::UserCheck),
];

/// 好友页：顶部标签栏 + 按标签切换的内容（在线/待处理/全部/已屏蔽/添加好友）。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let online: Vec<_> = app.friends.iter().filter(|f| f.online).collect();
    let incoming = app.incoming_request_count();

    let header = h_flex()
        .h(px(48.))
        .flex_shrink_0()
        .px_6()
        .gap_2()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(ico(IconName::Users, Size::Medium).text_color(rgb(palette::text_dim())))
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child("好友"),
        )
        .child(div().w(px(1.)).h(px(24.)).mx_2().bg(rgb(palette::bg_toolbar_hover())))
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(palette::text_secondary()))
                .child(format!("{} 位好友 · {} 位在线", app.friends.len(), online.len())),
        );

    // 标签栏。
    let mut tabs = h_flex()
        .h(px(44.))
        .flex_shrink_0()
        .px_6()
        .gap_1()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()));
    for (index, (tab, label, icon)) in TABS.iter().enumerate() {
        let active = app.friend_tab == *tab;
        let value = *tab;
        let badge = if *tab == FriendTab::Pending {
            Some(incoming)
        } else {
            None
        };

        let mut item = h_flex()
            .h(px(32.))
            .px_3()
            .gap_2()
            .rounded(px(4.))
            .items_center()
            .bg(if active {
                rgb(palette::bg_channel_hover())
            } else {
                rgb(palette::bg_chat())
            })
            .text_size(px(13.))
            .text_color(if active {
                rgb(palette::text_primary())
            } else {
                rgb(palette::text_muted())
            })
            .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
            .child(ico(*icon, Size::XSmall))
            .child(*label);
        if let Some(count) = badge {
            if count > 0 {
                item = item.child(
                    div()
                        .min_w(px(18.))
                        .h(px(18.))
                        .px_1()
                        .rounded_full()
                        .bg(rgb(palette::danger()))
                        .text_color(rgb(palette::WHITE))
                        .text_size(px(10.))
                        .font_weight(FontWeight::BOLD)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(count.to_string()),
                );
            }
        }
        tabs = tabs.child(
            item.id(format!("friend-tab-{index}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_friend_tab(value);
                    cx.notify();
                })),
        );
    }

    let mut content = v_flex()
        .flex_1()
        .min_h_0()
        .id("friends-scroll")
        .overflow_y_scroll()
        .px_8()
        .py_6()
        .gap_2();

    match app.friend_tab {
        FriendTab::Online => {
            let offline: Vec<_> = app.friends.iter().filter(|f| !f.online).collect();
            content = content.child(section_label(format!("在线 — {}", online.len())));
            for friend in &online {
                content = content.child(friend_row(
                    friend.name,
                    friend.color,
                    friend.activity,
                    true,
                    cx,
                ));
            }
            content = content
                .child(div().h(px(12.)))
                .child(section_label(format!("离线 — {}", offline.len())));
            for friend in &offline {
                content = content.child(friend_row(
                    friend.name,
                    friend.color,
                    friend.activity,
                    false,
                    cx,
                ));
            }
        }
        FriendTab::Pending => {
            content = content.child(request_section(app, cx));
        }
        FriendTab::All => {
            content = content.child(section_label(format!("全部好友 — {}", app.friends.len())));
            for friend in &app.friends {
                content = content.child(friend_row(
                    friend.name,
                    friend.color,
                    friend.activity,
                    friend.online,
                    cx,
                ));
            }
        }
        FriendTab::Blocked => {
            content = content.child(blocked_section(app, cx));
        }
        FriendTab::AddFriend => {
            content = content.child(add_friend_section(app, cx));
        }
    }

    v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .bg(rgb(palette::bg_chat()))
        .child(header)
        .child(tabs)
        .child(content)
}

/// 待处理请求区：收到的可 接受/拒绝，发出的显示等待中。
fn request_section(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let incoming: Vec<_> = app
        .friend_requests
        .iter()
        .enumerate()
        .filter(|(_, request)| request.incoming)
        .collect();
    let outgoing: Vec<_> = app
        .friend_requests
        .iter()
        .enumerate()
        .filter(|(_, request)| !request.incoming)
        .collect();

    let mut section = v_flex().w_full().gap_2();
    section = section.child(section_label(format!("收到的请求 — {}", incoming.len())));
    if incoming.is_empty() {
        section = section.child(empty_hint("暂时没有收到的好友请求"));
    }
    for (index, request) in incoming {
        section = section.child(request_row(index, request.name.clone(), request.color, true, cx));
    }

    section = section
        .child(div().h(px(12.)))
        .child(section_label(format!("发出的请求 — {}", outgoing.len())));
    if outgoing.is_empty() {
        section = section.child(empty_hint("还没有发出好友请求"));
    }
    for (index, request) in outgoing {
        section = section.child(request_row(index, request.name.clone(), request.color, false, cx));
    }
    section
}

/// 单条好友请求行。
fn request_row(
    index: usize,
    name: String,
    color: u32,
    incoming: bool,
    cx: &mut Context<DiscordApp>,
) -> Div {
    let short = name.chars().next().unwrap_or('?');
    let mut row = h_flex()
        .h(px(60.))
        .px_4()
        .gap_3()
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .child(avatar(short, color, px(40.), false))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    div()
                        .truncate()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(name.clone()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_dim()))
                        .child(if incoming {
                            "想添加你为好友"
                        } else {
                            "等待对方回应"
                        }),
                ),
        );

    if incoming {
        row = row
            .child(
                crate::ui::danger_icon_button(
                    IconName::X, Size::Small, px(32.), format!("request-reject-{index}"),
                    Box::new(move |this, _, cx| this.reject_request(index, cx)), cx,
                ),
            )
            .child(
                div()
                    .h(px(32.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_1()
                    .rounded(px(4.))
                    .bg(rgb(palette::accent()))
                    .text_color(rgb(palette::WHITE))
                    .text_size(px(12.))
                    .hover(|el| el.bg(rgb(palette::accent_hover())))
                    .child(ico(IconName::Check, Size::XSmall))
                    .child("接受")
                    .id(format!("request-accept-{index}"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.accept_request(index, cx);
                    })),
            );
    } else {
        row = row.child(
            h_flex()
                .gap_1()
                .text_size(px(12.))
                .text_color(rgb(palette::text_muted()))
                .child(ico(IconName::Clock, Size::XSmall))
                .child("等待中"),
        );
    }
    row
}

/// 已屏蔽列表：每行可取消屏蔽。
fn blocked_section(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut section = v_flex()
        .w_full()
        .gap_2()
        .child(section_label(format!("已屏蔽 — {}", app.blocked_users.len())));
    if app.blocked_users.is_empty() {
        return section.child(empty_hint("没有屏蔽任何用户"));
    }
    for (index, name) in app.blocked_users.iter().enumerate() {
        let short = name.chars().next().unwrap_or('?');
        let value = name.clone();
        section = section.child(
            h_flex()
                .h(px(56.))
                .px_4()
                .gap_3()
                .rounded(px(8.))
                .bg(rgb(palette::bg_sidebar()))
                .child(avatar(short, palette::text_dim(), px(40.), false))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(15.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(name.clone()),
                )
                .child(
                    div()
                        .h(px(30.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded(px(4.))
                        .bg(rgb(palette::bg_channel_hover()))
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| el.text_color(rgb(palette::text_primary())))
                        .child("取消屏蔽")
                        .id(format!("blocked-unblock-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.unblock_friend(value.clone());
                            cx.notify();
                        })),
                ),
        );
    }
    section
}

/// 添加好友区：输入用户名#标签，Enter 或点按钮发送。
fn add_friend_section(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    v_flex()
        .w_full()
        .gap_3()
        .child(
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_size(px(20.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("添加好友"),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_muted()))
                        .child("输入对方的用户名和标签来发送好友请求。"),
                ),
        )
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::text_secondary()))
                        .child("用户名#标签"),
                )
                .child(
                    div()
                        .w_full()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_composer()))
                        .child(
                            <Input as Sizable>::with_size(Input::new(&app.add_friend_input), Size::Large)
                                .w_full(),
                        ),
                )
                .child(
                    div()
                        .h(px(38.))
                        .px_5()
                        .flex()
                        .items_center()
                        .gap_2()
                        .self_start()
                        .rounded(px(4.))
                        .bg(rgb(palette::accent()))
                        .text_color(rgb(palette::WHITE))
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .hover(|el| el.bg(rgb(palette::accent_hover())))
                        .child(ico(IconName::Send, Size::XSmall))
                        .child("发送好友请求")
                        .id("add-friend-submit")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.send_friend_request(window, cx);
                        })),
                ),
        )
}

/// 空状态提示。
fn empty_hint(text: &'static str) -> Div {
    div()
        .w_full()
        .py_8()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.))
        .text_color(rgb(palette::text_dim()))
        .child(text)
}

/// 单个好友行：头像、名称、活动状态，点击发起私信。
fn friend_row(
    name: &'static str,
    color: u32,
    activity: &'static str,
    online: bool,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let click_name = name.to_string();
    let id = format!("friend-row-{name}");
    let row = h_flex()
        .h(px(56.))
        .px_4()
        .gap_3()
        .rounded(px(8.))
        .hover(|el| el.bg(rgb(palette::bg_message_hover())))
        .child(avatar(
            name.chars().next().unwrap_or('?'),
            color,
            px(40.),
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
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if online {
                            rgb(palette::text_primary())
                        } else {
                            rgb(palette::text_secondary())
                        })
                        .child(name),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_dim()))
                        .child(activity),
                ),
        );

    // 在线好友行右侧提供屏蔽入口。
    let row = row.child(
        crate::ui::danger_icon_button(
            IconName::Ban, Size::XSmall, px(28.), format!("friend-block-{name}"),
            Box::new(move |this, _, _| this.block_friend(name)), cx,
        ),
    );

    row.id(id).on_click(cx.listener(move |this, _, window, cx| {
        this.open_dm(click_name.clone(), window, cx);
    }))
}
