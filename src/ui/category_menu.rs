//! 分类操作菜单：右键频道分类标题打开。
//! 提供「在该分类中创建频道 / 标记为已读 / 重命名分类 / 删除分类」，带透明遮罩实现点击外部关闭。

use gpui_kit::*;

use crate::app::{DiscordApp, MenuAnchor};
use crate::ui::theme::palette;
use crate::ui::{ico, unread_badge};

use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 渲染分类操作菜单；没有打开菜单时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(menu) = app.category_menu.as_ref() else {
        return div().into_any_element();
    };

    let name = menu.name.clone();
    let channel_count = app.category_channel_count(&name);
    let unread = app.category_unread_count(&name);
    let deletable = app.category_deletable(&name);

    // 右键菜单跟随光标；没有坐标时贴左侧栏顶部兜底。
    let (left, top) = match menu.anchor {
        MenuAnchor::Point(x, y) => (x, y),
        MenuAnchor::Header => (56., 120.),
    };

    let mut card = v_flex()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(212.))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg();

    // 头部：分类名 + 频道数量（不可点击）。
    card = card
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
                        .child(format!("{channel_count} 个频道")),
                ),
        )
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::bg_divider())));

    // 在该分类中创建频道。
    card = card.child(
        div()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded(px(4.))
            .text_size(px(13.))
            .text_color(rgb(palette::text_secondary()))
            .hover(|el| {
                el.bg(rgb(palette::bg_channel_hover()))
                    .text_color(rgb(palette::text_primary()))
            })
            .child(ico(IconName::Plus, Size::XSmall))
            .child("在该分类中创建频道")
            .id("category-menu-create-channel")
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_category_create_channel(window, cx)
            })),
    );

    // 标记为已读：无未读时置灰，但点击无副作用。
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
                rgb(palette::text_secondary())
            } else {
                rgb(palette::text_dim())
            })
            .hover(|el| {
                el.bg(rgb(palette::bg_channel_hover()))
                    .text_color(rgb(palette::text_primary()))
            })
            .child(ico(IconName::CheckCheck, Size::XSmall))
            .child(div().flex_1().child("标记为已读"))
            .when(unread > 0, |el| el.child(unread_badge(unread)))
            .id("category-menu-mark-read")
            .on_click(cx.listener(|this, _, _, cx| this.mark_category_read(cx))),
    );

    // 重命名分类。
    card = card.child(
        div()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded(px(4.))
            .text_size(px(13.))
            .text_color(rgb(palette::text_secondary()))
            .hover(|el| {
                el.bg(rgb(palette::bg_channel_hover()))
                    .text_color(rgb(palette::text_primary()))
            })
            .child(ico(IconName::SquarePen, Size::XSmall))
            .child("重命名分类")
            .id("category-menu-rename")
            .on_click(cx.listener(|this, _, window, cx| this.open_category_rename(window, cx))),
    );

    // 删除分类：会删掉分类内所有频道，不可删时置灰且点击无效。
    card = card
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::bg_divider())))
        .child(
            div()
                .h(px(30.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded(px(4.))
                .text_size(px(13.))
                .text_color(if deletable {
                    rgb(palette::danger())
                } else {
                    rgb(palette::text_dim())
                })
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(ico(IconName::Trash, Size::XSmall))
                .child(div().flex_1().child(if deletable {
                    "删除分类"
                } else {
                    "至少保留一个文字分类"
                }))
                .id("category-menu-delete")
                .on_click(cx.listener(|this, _, window, cx| this.delete_category(window, cx))),
        );

    // 透明遮罩：点菜单外任意位置关闭。
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .id("category-menu-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_category_menu();
            cx.notify();
        }))
        .child(card)
        .into_any_element()
}
