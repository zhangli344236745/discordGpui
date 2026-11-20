//! 贴纸弹层：输入框右侧的贴纸 / 礼物按钮打开。
//! 顶部按贴纸包切换，点任意贴纸即发送一条只带贴纸的消息。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;

/// 单个贴纸格：emoji 字形 + 名称，点击直接发送。
fn sticker_cell(
    sticker: crate::data::Sticker,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    v_flex()
        .w(px(76.))
        .h(px(76.))
        .items_center()
        .justify_center()
        .gap_1()
        .rounded(px(8.))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(
            div()
                .text_size(px(30.))
                .line_height(px(34.))
                .child(sticker.emoji),
        )
        .child(
            div()
                .text_size(px(9.5))
                .text_color(rgb(palette::text_dim()))
                .child(sticker.label),
        )
        .id(format!("sticker-{}-{}", sticker.pack, sticker.label))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.send_sticker(sticker, window, cx);
        }))
}

/// 贴纸弹层主体；由 composer 决定是否挂载。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    // 包标签：数据源来自 app.sticker_packs，顺序与 seed 一致。
    let mut tabs = h_flex().gap_1();
    for &pack in &app.sticker_packs {
        let active = pack == app.sticker_pack;
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
                .child(pack)
                .id(format!("sticker-pack-{pack}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_sticker_pack(pack, cx);
                })),
        );
    }

    // 当前包里的贴纸，每行 4 个。
    let mut grid = v_flex().gap_1();
    let items: Vec<_> = app
        .stickers
        .iter()
        .filter(|sticker| sticker.pack == app.sticker_pack)
        .copied()
        .collect();
    for row in items.chunks(4) {
        let mut line = h_flex().gap_1();
        for &sticker in row {
            line = line.child(sticker_cell(sticker, cx));
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
                        .child("贴纸"),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child("点击即发送"),
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
                    "{} · 共 {} 张贴纸",
                    app.sticker_pack,
                    items.len()
                )),
        )
        .id("sticker-picker")
}
