use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 置顶消息面板：覆盖在聊天区右侧，列出当前会话所有置顶消息，可取消置顶。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let pinned = app.pinned_messages();

    let header = h_flex()
        .h(px(48.))
        .flex_shrink_0()
        .px_4()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child(format!("置顶消息 — {}", pinned.len())),
        )
        .child(
            div()
                .w(px(28.))
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(rgb(palette::text_secondary()))
                .hover(|el| {
                    el.bg(rgb(palette::bg_toolbar_hover()))
                        .text_color(rgb(palette::text_primary()))
                })
                .child(ico(IconName::X, Size::Small))
                .id("pins-close")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_pins();
                    cx.notify();
                })),
        );

    let mut body = v_flex().flex_1().min_h_0().id("pins-scroll").overflow_y_scroll();

    if pinned.is_empty() {
        body = body.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(palette::text_muted()))
                .child(ico(IconName::Pin, Size::Large))
                .child(
                    div()
                        .text_size(px(13.))
                        .child("当前会话还没有置顶消息"),
                ),
        );
    }

    for (index, message) in pinned.iter().enumerate() {
        body = body.child(
            div()
                .m_2()
                .p_3()
                .rounded(px(6.))
                .bg(rgb(palette::bg_rail()))
                .child(
                    h_flex().gap_2().items_center().child(avatar(
                        message.author.chars().next().unwrap_or('?'),
                        message.color.into(),
                        px(28.),
                        false,
                    ))
                    .child(
                        div()
                            .text_size(px(13.))
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
                        .mt_2()
                        .text_size(px(13.))
                        .line_height(px(19.))
                        .text_color(rgb(palette::text_message()))
                        .child(message.text.clone()),
                )
                .child(
                    div()
                        .mt_2()
                        .flex()
                        .justify_end()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded(px(4.))
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_secondary()))
                                .hover(|el| {
                                    el.bg(rgb(palette::bg_channel_hover()))
                                        .text_color(rgb(palette::danger()))
                                })
                                .child("取消置顶")
                                .id(format!("pins-unpin-{index}"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.unpin_at(index, cx);
                                })),
                        ),
                ),
        );
    }

    div()
        .absolute()
        .right(px(0.))
        .top(px(0.))
        .bottom(px(0.))
        .w(px(340.))
        .flex()
        .flex_col()
        .bg(rgb(palette::bg_sidebar()))
        .border_l_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(header)
        .child(body)
}
