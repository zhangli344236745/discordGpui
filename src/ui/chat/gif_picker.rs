//! GIF 搜索弹层：输入框右侧 GIF 按钮打开。
//! 顶部搜索框按标题 / 标签跨分类匹配，分类标签回到该分类；点任意 GIF 即发送一条只带 GIF 的消息。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;

use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 单个 GIF 格：色块 + emoji 字形模拟动图，底部标题；点击直接发送。
fn gif_cell(gif: crate::data::Gif, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    v_flex()
        .w(px(100.))
        .gap_1()
        .rounded(px(6.))
        .p_1()
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(
            div()
                .w_full()
                .h(px(62.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .bg(rgb(gif.color))
                .text_size(px(30.))
                .child(gif.emoji),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette::text_dim()))
                .child(gif.title),
        )
        .id(format!("gif-{}", gif.title))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.send_gif(gif, window, cx);
        }))
}

/// GIF 弹层主体；由 composer 决定是否挂载。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    // 有搜索词时跨分类匹配，所以不高亮任何分类标签。
    let searching = !app.gif_query.trim().is_empty();
    let mut tabs = h_flex().gap_1();
    for &category in &app.gif_categories {
        let active = !searching && category == app.gif_category;
        tabs = tabs.child(
            div()
                .h(px(24.))
                .px_2()
                .flex()
                .items_center()
                .rounded(px(4.))
                .text_size(px(11.5))
                .text_color(if active {
                    rgb(palette::text_primary())
                } else {
                    rgb(palette::text_secondary())
                })
                .when(active, |el| el.bg(rgb(palette::bg_channel_hover())))
                .hover(|el| {
                    el.bg(rgb(palette::bg_channel_hover()))
                        .text_color(rgb(palette::text_primary()))
                })
                .child(category)
                .id(format!("gif-category-{category}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.set_gif_category(category, window, cx);
                })),
        );
    }

    let items = app.visible_gifs();
    let mut grid = v_flex().gap_1();
    if items.is_empty() {
        grid = grid.child(
            div()
                .py_6()
                .flex()
                .justify_center()
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child("没有匹配的 GIF，换个关键词试试"),
        );
    }
    for row in items.chunks(3) {
        let mut line = h_flex().gap_1();
        for &gif in row {
            line = line.child(gif_cell(gif, cx));
        }
        grid = grid.child(line);
    }

    v_flex()
        .absolute()
        .bottom(px(52.))
        .right(px(16.))
        .w(px(348.))
        .p_2()
        .gap_2()
        .rounded(px(10.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::text_muted()))
                        .child("GIF"),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child("Enter 发送首个结果"),
                ),
        )
        .child(
            div()
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.gif_search), Size::Small)
                        .flex_1(),
                ),
        )
        .child(tabs)
        .child(
            div()
                .h(px(1.))
                .w_full()
                .bg(rgb(palette::bg_divider())),
        )
        .child(grid)
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette::text_dim()))
                .child(format!(
                    "{}{} · 共 {} 个",
                    if searching { "搜索 “" } else { "" },
                    if searching {
                        format!("{}\"", app.gif_query)
                    } else {
                        String::new()
                    },
                    items.len()
                )),
        )
        .id("gif-picker")
        // 此版本 overflow_y_scroll 需在 .id() 之后的 Stateful<Div> 上调用。
        .max_h(px(392.))
        .overflow_y_scroll()
}
