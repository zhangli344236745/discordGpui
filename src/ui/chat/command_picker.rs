//! 斜杠命令候选：输入 `/` 时浮在输入框上方。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 渲染候选面板；没有匹配命令时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let suggestions = app.command_suggestions();
    if suggestions.is_empty() {
        return div().into_any_element();
    }

    let mut list = v_flex().gap_1();
    for (index, command) in suggestions.into_iter().enumerate() {
        let name = command.name.clone();
        let description = if command.description.is_empty() {
            if command.kind == "builtin" {
                "内置命令".to_string()
            } else {
                "服务器命令".to_string()
            }
        } else {
            command.description.clone()
        };
        list = list.child(
            h_flex()
                .gap_2()
                .px_2()
                .py_2()
                .rounded(px(6.))
                .bg(rgb(palette::bg_sidebar()))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(ico(IconName::Hash, Size::XSmall).text_color(rgb(palette::accent())))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child(format!("/{name}")),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(description),
                        ),
                )
                .id(format!("command-suggest-{index}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.apply_command_suggestion(&name, window, cx);
                })),
        );
    }

    v_flex()
        .absolute()
        .bottom(px(52.))
        .left(px(16.))
        .w(px(320.))
        .p_2()
        .gap_2()
        .rounded(px(10.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            h_flex()
                .px_1()
                .gap_1()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("斜杠命令"),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child("Tab/点击补全，回车执行"),
                ),
        )
        .child(list)
        .into_any_element()
}
