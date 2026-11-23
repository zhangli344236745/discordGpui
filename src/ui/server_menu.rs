//! 服务器下拉菜单：点频道侧栏头部展开。
//! 两页共用一张卡片：操作页（服务器设置 / 邀请 / 通知 / 静音 / 标记已读 / 创建 / 离开）
//! 与通知设置页（服务器默认级别的三级单选）。遮罩与卡片是兄弟节点，点卡片不会误关。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::NotifyLevel;
use crate::ui::theme::palette;
use crate::ui::{ico, menu_divider, menu_item, radio_item};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 服务器级可选的三个级别（服务器没有「继承」对象）。
const SERVER_LEVELS: [NotifyLevel; 3] = [
    NotifyLevel::All,
    NotifyLevel::Mentions,
    NotifyLevel::Nothing,
];

/// 渲染服务器菜单；未打开时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    if !app.show_server_menu {
        return div().into_any_element();
    }

    let server = app.active_server_ref();
    let name = server.name.clone();
    let level = app.server_notify_default();
    let muted = level.is_muted();
    let members = app.members.len();

    // 频道侧栏宽 240、头部高 48，菜单贴在头部下方。通知页文案更宽，卡片加宽。
    let width = if app.server_notify_page { 280. } else { 232. };
    let card = v_flex()
        .absolute()
        .left(px(8.))
        .top(px(44.))
        .w(px(width))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            v_flex()
                .px_2()
                .py_1()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .truncate()
                        .child(name.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(format!(
                            "{members} 名成员 · {}",
                            if muted {
                                String::from("已静音")
                            } else {
                                format!("通知：{}", level.short())
                            }
                        )),
                ),
        )
        .child(menu_divider());

    let card = if app.server_notify_page {
        notify_page(card, name, level, app.inherit_channel_count(), cx)
    } else {
        actions_page(card, muted, level, cx)
    };

    // 透明遮罩与卡片是兄弟节点：点遮罩关闭，点卡片（含单选行）不触发关闭。
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .id("server-menu-backdrop")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.close_server_menu();
                    cx.notify();
                })),
        )
        .child(card)
        .into_any_element()
}

/// 操作页：服务器设置 / 邀请 / 通知入口 / 静音 / 标记已读 / 创建 / 离开。
fn actions_page(card: Div, muted: bool, level: NotifyLevel, cx: &mut Context<DiscordApp>) -> Div {
    let mute_label = if muted {
        "取消静音服务器"
    } else {
        "静音服务器"
    };
    let mute_icon = if muted {
        IconName::Bell
    } else {
        IconName::BellOff
    };

    card.child(menu_item(
        IconName::Settings,
        "服务器设置",
        None,
        "server-menu-settings",
        Box::new(|this, window, cx| this.open_server_settings(window, cx)),
        cx,
    ))
    .child(menu_item(
        IconName::UserPlus,
        "邀请好友",
        None,
        "server-menu-invite",
        Box::new(|this, window, cx| this.open_invite(window, cx)),
        cx,
    ))
    .child(menu_item(
        if muted {
            IconName::BellOff
        } else {
            IconName::Bell
        },
        "通知设置",
        Some(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_dim()))
                .child(level.short())
                .into_any_element(),
        ),
        "server-menu-notify",
        Box::new(|this, _, cx| this.open_server_notify_page(cx)),
        cx,
    ))
    .child(menu_item(
        mute_icon,
        mute_label,
        None,
        "server-menu-mute",
        Box::new(|this, _, cx| this.toggle_server_mute(cx)),
        cx,
    ))
    .child(menu_item(
        IconName::CheckCheck,
        "标记服务器已读",
        None,
        "server-menu-mark-read",
        Box::new(|this, _, cx| this.mark_server_read(cx)),
        cx,
    ))
    .child(menu_item(
        IconName::Plus,
        "创建频道",
        None,
        "server-menu-create-channel",
        Box::new(|this, window, cx| this.open_create_channel(window, cx)),
        cx,
    ))
    .child(menu_item(
        IconName::FolderPlus,
        "新建分类",
        None,
        "server-menu-create-category",
        Box::new(|this, window, cx| this.open_create_category(window, cx)),
        cx,
    ))
    .child(menu_divider())
    .child(
        menu_item(
            IconName::LogOut,
            "离开服务器",
            None,
            "server-menu-leave",
            Box::new(|this, window, cx| this.leave_server(window, cx)),
            cx,
        )
        .text_color(rgb(palette::danger())),
    )
}

/// 通知页：返回行 + 三级单选 + 继承说明。
fn notify_page(
    card: Div,
    name: String,
    level: NotifyLevel,
    inheriting: usize,
    cx: &mut Context<DiscordApp>,
) -> Div {
    let header = h_flex()
        .h(px(34.))
        .px_1()
        .gap_1()
        .items_center()
        .child(
            div()
                .w(px(22.))
                .h(px(22.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(rgb(palette::text_secondary()))
                .hover(|el| {
                    el.bg(rgb(palette::bg_channel_hover()))
                        .text_color(rgb(palette::text_primary()))
                })
                .child(ico(IconName::ChevronLeft, Size::XSmall))
                .id("server-menu-notify-back")
                .on_click(cx.listener(|this, _, _, cx| this.close_server_notify_page(cx))),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child("服务器通知"),
        )
        .child(div().flex_1())
        .child(
            div()
                .px_1()
                .text_size(px(11.))
                .text_color(rgb(palette::text_dim()))
                .truncate()
                .child(name),
        );

    let mut card = card.child(header).child(menu_divider());

    for option in SERVER_LEVELS {
        let selected = option == level;
        card = card.child(radio_item(
            option.label(),
            option.hint(),
            selected,
            format!("server-notify-{}", option.index()),
            Box::new(move |this, _, cx| this.set_server_notify(option, cx)),
            cx,
        ));
    }

    card.child(menu_divider()).child(
        div()
            .px_2()
            .py_1()
            .text_size(px(11.))
            .text_color(rgb(palette::text_dim()))
            .child(format!(
                "{inheriting} 个频道选择「跟随服务器」，会跟着这里一起变；单独设过级别的频道不受影响。"
            )),
    )
}
