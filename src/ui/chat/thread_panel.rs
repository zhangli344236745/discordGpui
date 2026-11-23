//! 话题面板：覆盖在聊天区右侧，展示原消息上下文、话题回复与独立输入条。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{avatar, close_button, ico, icon_button};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

/// 话题面板：由消息工具栏或右键菜单的「话题」入口打开，Esc / X 关闭。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let thread = app.current_thread();
    let root = app.thread_root_message();
    let title = thread
        .as_ref()
        .map(|thread| thread.title.clone())
        .unwrap_or_else(|| "话题".to_string());

    let header = h_flex()
        .h(px(48.))
        .flex_shrink_0()
        .px_4()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(ico(IconName::MessageSquare, Size::Small))
                        .child(div().truncate().child("话题")),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(title),
                ),
        )
        .child(close_button(
            "thread-close",
            Box::new(|this, _, cx| {
                this.close_thread();
                cx.notify();
            }),
            cx,
        ));

    let mut body = v_flex()
        .flex_1()
        .min_h_0()
        .gap_3()
        .p_3()
        .id("thread-scroll")
        .overflow_y_scroll();

    if let Some(message) = root {
        body = body.child(
            v_flex()
                .p_3()
                .gap_2()
                .rounded(px(8.))
                .bg(rgb(palette::bg_rail()))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(avatar(
                            message.author.chars().next().unwrap_or('?'),
                            message.color.into(),
                            px(24.),
                            false,
                        ))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(message.color)
                                .child(message.author),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_right()
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_dim()))
                                .child(message.time),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(rgb(palette::text_message()))
                        .child(if message.text.is_empty() {
                            "[贴纸或附件]".to_string()
                        } else {
                            message.text
                        }),
                ),
        );
    }

    let replies = thread.map(|thread| thread.replies).unwrap_or_default();
    body = body.child(
        h_flex()
            .gap_2()
            .items_center()
            .pb_1()
            .child(div().h(px(1.)).flex_1().bg(rgb(palette::bg_divider())))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette::text_muted()))
                    .child(format!("{} 条回复", replies.len())),
            )
            .child(div().h(px(1.)).flex_1().bg(rgb(palette::bg_divider()))),
    );

    if replies.is_empty() {
        body = body.child(
            v_flex()
                .h(px(120.))
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(rgb(palette::text_dim()))
                .child(ico(IconName::MessageSquarePlus, Size::Large))
                .child(
                    div()
                        .text_size(px(12.))
                        .child("还没有回复，来开个头吧"),
                ),
        );
    }

    for (index, reply) in replies.iter().enumerate() {
        body = body.child(
            div()
                .p_2()
                .rounded(px(6.))
                .hover(|el| el.bg(rgb(palette::bg_message_hover())))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(avatar(
                            reply.author.chars().next().unwrap_or('?'),
                            reply.color.into(),
                            px(24.),
                            false,
                        ))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(reply.color)
                                .child(reply.author),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_right()
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_dim()))
                                .child(reply.time),
                        ),
                )
                .child(
                    div()
                        .mt_1()
                        .pl(px(32.))
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(rgb(palette::text_message()))
                        .child(reply.text.clone()),
                )
                .id(format!("thread-reply-{index}")),
        );
    }

    let footer = v_flex()
        .flex_shrink_0()
        .gap_2()
        .p_3()
        .border_t_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(
            h_flex()
                .min_h(px(38.))
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .items_center()
                .px_2()
                .gap_2()
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.thread_input), Size::Small)
                        .flex_1(),
                )
                .child(icon_button(
                    IconName::SendHorizontal,
                    Size::Small,
                    px(24.),
                    "thread-send",
                    true,
                    Box::new(|this, window, cx| this.send_thread_reply(window, cx)),
                    cx,
                )),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette::text_dim()))
                .child("Enter 发送回复 · Esc 关闭话题"),
        );

    div()
        .absolute()
        .right(px(0.))
        .top(px(0.))
        .bottom(px(0.))
        .w(px(360.))
        .flex()
        .flex_col()
        .bg(rgb(palette::bg_sidebar()))
        .border_l_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(header)
        .child(body)
        .child(footer)
}
