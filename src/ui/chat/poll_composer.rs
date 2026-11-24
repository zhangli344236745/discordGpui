//! 投票创建弹层：输入框右侧投票按钮打开。
//! 填写问题 + 至少两个选项，可选"多选"，回车或点按钮发起投票并立即发送一条带投票卡的消息。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 已添加的选项 chip：序号 + 文本 + 删除按钮。
fn option_chip(index: usize, label: &str, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    h_flex()
        .h(px(28.))
        .gap_2()
        .px_2()
        .rounded(px(6.))
        .bg(rgb(palette::bg_composer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .items_center()
        .child(
            div()
                .w(px(16.))
                .h(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .bg(rgb(palette::accent()))
                .text_color(rgb(palette::WHITE))
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .child((index + 1).to_string()),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(12.))
                .text_color(rgb(palette::text_primary()))
                .child(label.to_string()),
        )
        .child(
            div()
                .w(px(18.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(rgb(palette::text_secondary()))
                .hover(|el| {
                    el.bg(rgb(palette::bg_toolbar_hover()))
                        .text_color(rgb(palette::text_primary()))
                })
                .child(ico(IconName::X, Size::XSmall))
                .id(format!("poll-remove-option-{index}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.remove_poll_option(index, cx);
                })),
        )
}

/// 投票弹层主体；由 composer 决定是否挂载。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let can_send = app.poll_options_draft.len() >= 2;
    let hint = if can_send {
        format!("已添加 {} 个选项 · 回车发起投票", app.poll_options_draft.len())
    } else {
        "至少添加 2 个选项才能发起投票".to_string()
    };

    let mut options = v_flex().gap_1();
    for (index, label) in app.poll_options_draft.iter().enumerate() {
        options = options.child(option_chip(index, label, cx));
    }
    if app.poll_options_draft.is_empty() {
        options = options.child(
            div()
                .py_2()
                .text_align(TextAlign::Center)
                .text_size(px(11.))
                .text_color(rgb(palette::text_dim()))
                .child("还没有选项，在下方输入后回车添加"),
        );
    }

    v_flex()
        .absolute()
        .bottom(px(52.))
        .right(px(16.))
        .w(px(360.))
        .p_3()
        .gap_2()
        .rounded(px(10.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .id("poll-composer")
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("创建投票"),
                )
                .child(
                    div()
                        .w(px(20.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_toolbar_hover()))
                                .text_color(rgb(palette::text_primary()))
                        })
                        .child(ico(IconName::X, Size::XSmall))
                        .id("poll-close")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.close_poll_composer(window, cx);
                        })),
                ),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child("投票问题"),
        )
        .child(
            div()
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.poll_question_edit), Size::Small)
                        .flex_1(),
                ),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child("选项"),
        )
        .child(options)
        .child(
            div()
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.poll_option_edit), Size::Small)
                        .flex_1(),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .w(px(16.))
                        .h(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .bg(if app.poll_multiple {
                            rgb(palette::accent())
                        } else {
                            rgb(palette::bg_composer())
                        })
                        .border_1()
                        .border_color(if app.poll_multiple {
                            rgb(palette::accent())
                        } else {
                            rgb(palette::border_default())
                        })
                        .text_color(rgb(palette::WHITE))
                        .when(app.poll_multiple, |el| {
                            el.child(ico(IconName::Check, Size::XSmall))
                        })
                        .id("poll-toggle-multiple")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_poll_multiple(cx);
                        })),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(rgb(palette::text_secondary()))
                        .child("允许多选"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(rgb(palette::text_dim()))
                        .child(hint),
                ),
        )
        .child(
            div()
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .bg(if can_send {
                    rgb(palette::accent())
                } else {
                    rgb(palette::bg_composer())
                })
                .text_color(if can_send {
                    rgb(palette::WHITE)
                } else {
                    rgb(palette::text_dim())
                })
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .child("发起投票")
                .when(can_send, |el| {
                    el.hover(|el| el.bg(rgb(palette::accent_hover())))
                })
                .id("poll-send")
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.send_poll(window, cx);
                })),
        )
}
