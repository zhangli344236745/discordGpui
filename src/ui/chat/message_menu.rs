use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 消息操作菜单：绝对定位在触发消息附近，提供置顶/复制/删除。
pub fn render(index: usize, app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let pinned = app
        .current_messages()
        .get(index)
        .map(|m| m.pinned)
        .unwrap_or(false);

    let pin_id = format!("msg-menu-pin-{index}");
    let copy_id = format!("msg-menu-copy-{index}");
    let delete_id = format!("msg-menu-delete-{index}");

    let edit_id = format!("msg-menu-edit-{index}");

    v_flex()
        .absolute()
        .right(px(12.))
        .top(px(8.))
        .w(px(160.))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::BG_FOOTER))
        .border_1()
        .border_color(rgb(palette::BORDER_DEFAULT))
        .shadow_lg()
        .when(app.is_mine(index), |el| {
            el.child(
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
                    .child("编辑消息")
                    .id(edit_id)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.start_edit(index, window, cx);
                    })),
            )
        })
        .child(
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
                .child(ico(
                    if pinned {
                        IconName::PinOff
                    } else {
                        IconName::Pin
                    },
                    Size::XSmall,
                ))
                .child(if pinned { "取消置顶" } else { "置顶消息" })
                .id(pin_id)
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_pin(index, cx))),
        )
        .child(
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
                .child(ico(IconName::Copy, Size::XSmall))
                .child("复制文本")
                .id(copy_id)
                .on_click(cx.listener(move |this, _, _, cx| this.copy_message(index, cx))),
        )
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::BG_DIVIDER)))
        .child(
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
                .child("删除消息")
                .id(delete_id)
                .on_click(cx.listener(move |this, _, _, cx| this.delete_message(index, cx))),
        )
}
