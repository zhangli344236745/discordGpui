//! 频道操作菜单：右键频道行或点标题栏铃铛打开。
//! 两页共用一张卡片：操作页（标记已读 / 通知设置 / 编辑主题 / 删除）与通知设置页（四级单选）。
//! 遮罩与卡片是兄弟节点，卡片内的点击不会冒泡到遮罩，所以通知设置页可以连点而不关闭。

use gpui_kit::*;

use crate::app::{ChannelMenuPage, DiscordApp, MenuAnchor};
use crate::data::{ChannelKind, NotifyLevel};
use crate::ui::theme::palette;
use crate::ui::{ico, menu_divider, menu_item, radio_item, unread_badge};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 通知级别的四级顺序，供单选列表遍历。
const LEVELS: [NotifyLevel; 4] = [
    NotifyLevel::Inherit,
    NotifyLevel::All,
    NotifyLevel::Mentions,
    NotifyLevel::Nothing,
];

/// 渲染频道操作菜单；没有打开菜单时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(menu) = app.channel_menu.as_ref() else {
        return div().into_any_element();
    };

    let page = menu.page;
    let name = menu.name.clone();
    let kind = app.channel_kind(&name);
    let is_voice = kind == Some(ChannelKind::Voice);
    let is_forum = kind == Some(ChannelKind::Forum);
    let unread = app.channel_unread_of(&name);
    let topic = app.channel_topic_of(&name);
    let level = app.channel_notify_of(&name);
    let resolved = app.effective_notify_of(&name);

    // 右键菜单跟随光标；标题栏入口贴右上角展开。通知设置页文案更长，卡片加宽。
    let (left, top, right) = match menu.anchor {
        MenuAnchor::Point(x, y) => (Some(x), y, 0.),
        MenuAnchor::Header => (None, 54., 90.),
    };
    let width = if page == ChannelMenuPage::Notify { 280. } else { 212. };

    let mut card = v_flex()
        .absolute()
        .top(px(top))
        .w(px(width))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg();
    card = match left {
        Some(x) => card.left(px(x)),
        None => card.right(px(right)),
    };

    card = match page {
        ChannelMenuPage::Actions => {
            actions_page(card, name.clone(), is_voice, is_forum, unread, topic, level, cx)
        }
        ChannelMenuPage::Notify => notify_page(card, name.clone(), level, resolved, cx),
    };

    // 透明遮罩与卡片是兄弟节点：点遮罩关闭，点卡片不触发关闭。
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
                .id("channel-menu-backdrop")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.close_channel_menu();
                    cx.notify();
                })),
        )
        .child(card)
        .into_any_element()
}

/// 操作页：频道信息头 + 标记已读 + 通知设置入口 + 编辑主题 + 删除。
fn actions_page(
    card: Div,
    name: String,
    is_voice: bool,
    is_forum: bool,
    unread: usize,
    topic: String,
    level: NotifyLevel,
    cx: &mut Context<DiscordApp>,
) -> Div {
    let card = card
        .child(
            v_flex()
                .px_2()
                .py_1()
                .gap(px(2.))
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .text_color(rgb(palette::text_primary()))
                        .child(ico(
                            if is_voice {
                                IconName::Volume2
                            } else if is_forum {
                                IconName::MessageSquare
                            } else {
                                IconName::Hash
                            },
                            Size::XSmall,
                        ))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child(name),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .truncate()
                        .child(if is_voice {
                            String::from("语音频道")
                        } else if is_forum {
                            String::from("论坛频道")
                        } else if topic.is_empty() {
                            String::from("暂无频道主题")
                        } else {
                            topic
                        }),
                ),
        )
        .child(menu_divider());

    // 标记为已读：仅文字频道有意义，未读为 0 时置灰但仍可点（无副作用）。
    let mut card = card.child(
        menu_item(
            IconName::CheckCheck,
            "标记为已读",
            (unread > 0).then(|| unread_badge(unread).into_any_element()),
            "channel-menu-mark-read",
            Box::new(|this, _, cx| this.mark_channel_read(cx)),
            cx,
        )
        .text_color(rgb(if unread > 0 {
            palette::text_secondary()
        } else {
            palette::text_dim()
        })),
    );

    // 通知设置：右侧实时显示当前级别，点开翻页（不关卡片）。
    card = card.child(menu_item(
        if level.is_muted() {
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
        "channel-menu-notify",
        Box::new(|this, _, cx| {
            this.open_channel_notify_page();
            cx.notify();
        }),
        cx,
    ));

    // 编辑主题：语音频道没有主题，直接不展示该项。
    if !is_voice {
        card = card.child(menu_item(
            IconName::SquarePen,
            "编辑频道主题",
            None,
            "channel-menu-edit-topic",
            Box::new(|this, window, cx| this.open_topic_editor(window, cx)),
            cx,
        ));
    }

    card.child(menu_divider()).child(
        menu_item(
            IconName::Trash,
            "删除频道",
            None,
            "channel-menu-delete",
            Box::new(|this, window, cx| this.delete_channel(window, cx)),
            cx,
        )
        .text_color(rgb(palette::danger())),
    )
}

/// 通知设置页：返回行 + 四级单选 + 说明页脚。
fn notify_page(
    card: Div,
    name: String,
    level: NotifyLevel,
    resolved: NotifyLevel,
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
                .id("channel-menu-notify-back")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.close_channel_notify_page();
                    cx.notify();
                })),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child("通知设置"),
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

    for level_option in LEVELS {
        let selected = level_option == level;
        // 「跟随服务器」这一行把继承到的实际级别写进说明，避免用户猜。
        let hint = if level_option == NotifyLevel::Inherit {
            format!("跟随时通知设置，当前生效：{}", resolved.label())
        } else {
            level_option.hint().to_string()
        };
        card = card.child(radio_item(
            level_option.label(),
            hint,
            selected,
            format!("channel-notify-{}", level_option.index()),
            Box::new(move |this, _, cx| this.set_channel_notify(level_option, cx)),
            cx,
        ));
    }

    card.child(menu_divider()).child(
        div()
            .px_2()
            .py_1()
            .text_size(px(11.))
            .text_color(rgb(palette::text_dim()))
            .child("覆盖只对当前频道生效；静音后未读角标转灰，仍可正常查看消息。"),
    )
}
