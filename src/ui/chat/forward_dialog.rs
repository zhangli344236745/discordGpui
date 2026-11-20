//! 转发消息弹层：居中模态，列出全部文字频道与私信，点击目标即完成转发。
//! 独立组件（不复用 create_dialog）：因为需要搜索框 + 可滚动目标列表，而不是单行输入框。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

/// 单个目标行：图标 + 名称 + 归属，点击即转发到该会话。
fn target_row(
    target: crate::data::ForwardTarget,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let label = target.label();
    let scope = target.scope();
    let color = target.color;
    let is_dm = target.is_dm;

    h_flex()
        .w_full()
        .h(px(44.))
        .px_2()
        .gap_3()
        .items_center()
        .rounded(px(6.))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(
            div()
                .w(px(28.))
                .h(px(28.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(rgb(color))
                .text_color(rgb(palette::WHITE))
                .child(ico(
                    if is_dm {
                        IconName::AtSign
                    } else {
                        IconName::Hash
                    },
                    Size::XSmall,
                )),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    div()
                        .truncate()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(palette::text_primary()))
                        .child(label.clone()),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_muted()))
                        .child(scope),
                ),
        )
        .child(
            h_flex()
                .gap_1()
                .text_size(px(11.))
                .text_color(rgb(palette::text_dim()))
                .child(format!("转发到 {label}"))
                .child(ico(IconName::Forward, Size::XSmall)),
        )
        .id(format!("forward-target-{index}"))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.forward_message(target.clone(), cx);
        }))
}

/// 弹层主体：来源预览 + 搜索 + 目标列表。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let summary = app.forward_summary();
    let origin_label = app.current_origin_label();
    let targets = app.filtered_forward_targets();
    let query = app.forward_query.trim().to_string();

    let mut list = v_flex()
        .id("forward-target-list")
        .max_h(px(268.))
        .overflow_y_scroll();

    if targets.is_empty() {
        list = list.child(
            v_flex()
                .h(px(88.))
                .items_center()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_secondary()))
                        .child("没有匹配的频道或私信"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(format!("换个关键词试试：{query}")),
                ),
        );
    } else {
        for (index, target) in targets.into_iter().enumerate() {
            list = list.child(target_row(target, index, cx));
        }
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(0x00000099))
        .id("forward-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_forward();
            cx.notify();
        }))
        .child(
            v_flex()
                .w(px(468.))
                .gap_3()
                .p_5()
                .rounded(px(10.))
                .bg(rgb(palette::bg_chat()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .id("forward-card")
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(34.))
                                        .h(px(34.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(8.))
                                        .bg(rgb(palette::bg_channel_hover()))
                                        .child(ico(IconName::Forward, Size::Small)),
                                )
                                .child(
                                    v_flex()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_size(px(18.))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(palette::text_primary()))
                                                .child("转发消息"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(palette::text_muted()))
                                                .child(format!(
                                                    "从 {origin_label} 转发到其他频道或私信"
                                                )),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .w(px(28.))
                                .h(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.))
                                .text_color(rgb(palette::text_muted()))
                                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                                .child(ico(IconName::X, Size::Small))
                                .id("forward-close")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_forward();
                                    cx.notify();
                                })),
                        ),
                )
                // 来源消息预览
                .child(
                    v_flex()
                        .gap_1()
                        .p_3()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_sidebar()))
                        .border_1()
                        .border_color(rgb(palette::border_default()))
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_muted()))
                                .child(ico(IconName::Forward, Size::XSmall))
                                .child(match summary.as_ref() {
                                    Some((author, _)) => format!("{author} 的消息"),
                                    None => "原消息已不存在".to_string(),
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(19.))
                                .text_color(rgb(palette::text_secondary()))
                                .child(
                                    summary
                                        .map(|(_, excerpt)| excerpt)
                                        .unwrap_or_else(|| "——".to_string()),
                                ),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_composer()))
                        .child(
                            <Input as Sizable>::with_size(
                                Input::new(&app.forward_search),
                                Size::Large,
                            )
                            .w_full(),
                        ),
                )
                .child(list)
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child("点击目标即可转发，原消息保持不变"),
                        )
                        .child(
                            div()
                                .h(px(32.))
                                .px_3()
                                .flex()
                                .items_center()
                                .rounded(px(4.))
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_secondary()))
                                .hover(|el| el.text_color(rgb(palette::text_primary())))
                                .child("取消")
                                .id("forward-cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_forward();
                                    cx.notify();
                                })),
                        ),
                ),
        )
}
