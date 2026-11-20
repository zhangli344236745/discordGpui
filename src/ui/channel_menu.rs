//! 频道操作菜单：右键频道行或点标题栏铃铛打开。
//! 提供「标记为已读 / 编辑频道主题 / 删除频道」，带透明遮罩实现点击外部关闭。

use gpui_kit::*;

use crate::app::{DiscordApp, MenuAnchor};
use crate::data::ChannelKind;
use crate::ui::theme::palette;
use crate::ui::{ico, unread_badge};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 渲染频道操作菜单；没有打开菜单时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(menu) = app.channel_menu.as_ref() else {
        return div().into_any_element();
    };

    let name = menu.name.clone();
    let is_voice = app.channel_kind(&name) == Some(ChannelKind::Voice);
    let unread = app.channel_unread_of(&name);
    let topic = app.channel_topic_of(&name);

    // 右键菜单跟随光标；标题栏入口贴右上角展开。
    let (left, top, right) = match menu.anchor {
        MenuAnchor::Point(x, y) => (Some(x), y, 0.),
        MenuAnchor::Header => (None, 54., 90.),
    };

    let mut card = v_flex()
        .absolute()
        .top(px(top))
        .w(px(212.))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::BG_FOOTER))
        .border_1()
        .border_color(rgb(palette::BORDER_DEFAULT))
        .shadow_lg();
    card = match left {
        Some(x) => card.left(px(x)),
        None => card.right(px(right)),
    };

    // 头部：图标 + 频道名 + 类型/主题提示（不可点击）。
    card = card
        .child(
            v_flex()
                .px_2()
                .py_1()
                .gap(px(2.))
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child(ico(
                            if is_voice {
                                IconName::Volume2
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
                                .child(name.clone()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::TEXT_DIM))
                        .truncate()
                        .child(if is_voice {
                            String::from("语音频道")
                        } else if topic.is_empty() {
                            String::from("暂无频道主题")
                        } else {
                            topic.clone()
                        }),
                ),
        )
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::BG_DIVIDER)));

    // 标记为已读：仅文字频道有意义，未读为 0 时置灰但仍可点（无副作用）。
    card = card.child(
        div()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded(px(4.))
            .text_size(px(13.))
            .text_color(if unread > 0 {
                rgb(palette::TEXT_SECONDARY)
            } else {
                rgb(palette::TEXT_DIM)
            })
            .hover(|el| {
                el.bg(rgb(palette::BG_CHANNEL_HOVER))
                    .text_color(rgb(palette::TEXT_PRIMARY))
            })
            .child(ico(IconName::CheckCheck, Size::XSmall))
            .child(div().flex_1().child("标记为已读"))
            .when(unread > 0, |el| el.child(unread_badge(unread)))
            .id("channel-menu-mark-read")
            .on_click(cx.listener(|this, _, _, cx| this.mark_channel_read(cx))),
    );

    // 编辑主题：语音频道没有主题，直接不展示该项。
    if !is_voice {
        card = card.child(
            div()
                .h(px(30.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded(px(4.))
                .text_size(px(13.))
                .text_color(rgb(palette::TEXT_SECONDARY))
                .hover(|el| {
                    el.bg(rgb(palette::BG_CHANNEL_HOVER))
                        .text_color(rgb(palette::TEXT_PRIMARY))
                })
                .child(ico(IconName::SquarePen, Size::XSmall))
                .child("编辑频道主题")
                .id("channel-menu-edit-topic")
                .on_click(cx.listener(|this, _, window, cx| this.open_topic_editor(window, cx))),
        );
    }

    card = card.child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::BG_DIVIDER))).child(
        div()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded(px(4.))
            .text_size(px(13.))
            .text_color(rgb(palette::DANGER))
            .hover(|el| {
                el.bg(rgb(palette::BG_CHANNEL_HOVER))
                    .text_color(rgb(palette::DANGER))
            })
            .child(ico(IconName::Trash, Size::XSmall))
            .child("删除频道")
            .id("channel-menu-delete")
            .on_click(cx.listener(|this, _, window, cx| this.delete_channel(window, cx))),
    );

    // 透明遮罩：点菜单外任意位置关闭。
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .id("channel-menu-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_channel_menu();
            cx.notify();
        }))
        .child(card)
        .into_any_element()
}
