//! 服务器欢迎屏幕：新加入服务器时的顶层模态。
//!
//! 展示服务器介绍、社区规则（需勾选同意）与推荐频道（可多选），确认后按
//! 选中的第一个频道进入。没有专属文案的服务器由 `app::welcome_screen_for`
//! 用简介 + 文字频道现场合成，因此每个服务器都有欢迎页。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 渲染欢迎屏幕；未打开时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(server) = app.welcome_target.clone() else {
        return div().into_any_element();
    };
    let Some(welcome) = app.welcome_screen_for(&server) else {
        return div().into_any_element();
    };
    let Some(info) = app.servers.iter().find(|item| item.name == server) else {
        return div().into_any_element();
    };
    let badge = info.badge.clone();
    let color = info.color;
    let agreed = app.welcome_rule_agreed;
    let can_confirm = app.welcome_can_confirm();
    let selected_count = app.welcome_selected.len();

    // 头部：服务器徽标 + 欢迎语 + 关闭。
    let mut card = v_flex()
        .w(px(600.))
        .gap_4()
        .p_6()
        .rounded(px(12.))
        .bg(rgb(palette::bg_chat()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .id("welcome-card")
        .child(
            h_flex()
                .gap_4()
                .items_start()
                .child(
                    div()
                        .w(px(56.))
                        .h(px(56.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(14.))
                        .bg(rgb(color))
                        .text_color(rgb(palette::WHITE))
                        .text_size(px(22.))
                        .font_weight(FontWeight::BOLD)
                        .child(badge),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(ico(IconName::Sparkles, Size::XSmall))
                                .child("服务器欢迎屏幕"),
                        )
                        .child(
                            div()
                                .text_size(px(20.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child(welcome.headline.clone()),
                        ),
                )
                .child(
                    div()
                        .w(px(28.))
                        .h(px(28.))
                        .flex_shrink_0()
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
                        .id("welcome-close")
                        .on_click(cx.listener(|this, _, _, cx| this.close_welcome(cx))),
                ),
        )
        .child(
            div()
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(rgb(palette::text_secondary()))
                .child(welcome.description.clone()),
        );

    // 规则列表。
    if !welcome.rules.is_empty() {
        let mut rules = v_flex()
            .gap_3()
            .p_3()
            .rounded(px(8.))
            .bg(rgb(palette::bg_rail()))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(palette::text_primary()))
                    .child(ico(IconName::ScrollText, Size::XSmall))
                    .child(format!("社区规则 · {}", welcome.rule_count())),
            );
        for (index, rule) in welcome.rules.iter().enumerate() {
            rules = rules.child(
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(
                        div()
                            .w(px(20.))
                            .h(px(20.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(rgb(palette::bg_composer()))
                            .text_size(px(10.))
                            .text_color(rgb(palette::text_secondary()))
                            .child(format!("{}", index + 1)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(palette::text_primary()))
                                    .child(rule.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .line_height(px(17.))
                                    .text_color(rgb(palette::text_secondary()))
                                    .child(rule.body.clone()),
                            ),
                    ),
            );
        }
        card = card.child(rules);
    }

    // 同意勾选：只有需要规则同意的服务器才出现。
    if welcome.require_rules {
        card = card.child(
            h_flex()
                .gap_2()
                .items_center()
                .p_3()
                .rounded(px(6.))
                .bg(rgb(if agreed {
                    palette::bg_reaction_mine()
                } else {
                    palette::bg_composer()
                }))
                .border_1()
                .border_color(rgb(if agreed {
                    palette::accent()
                } else {
                    palette::border_default()
                }))
                .id("welcome-agree")
                .on_click(cx.listener(|this, _, _, cx| this.toggle_welcome_rule(cx)))
                .child(
                    div()
                        .w(px(18.))
                        .h(px(18.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.))
                        .bg(rgb(if agreed {
                            palette::accent()
                        } else {
                            palette::bg_chat()
                        }))
                        .border_1()
                        .border_color(rgb(if agreed {
                            palette::accent()
                        } else {
                            palette::border_default()
                        }))
                        .text_color(rgb(palette::WHITE))
                        .when(agreed, |el| el.child(ico(IconName::Check, Size::XSmall))),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(rgb(palette::text_message()))
                        .child("我已阅读并同意以上社区规则"),
                ),
        );
    }

    // 推荐频道多选。
    if !welcome.recommended.is_empty() {
        let mut chips = h_flex().gap_2().flex_wrap();
        for channel in &welcome.recommended {
            let selected = app.welcome_selected.contains(channel);
            let name = channel.clone();
            chips = chips.child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .px_3()
                    .py_1()
                    .rounded_full()
                    .bg(rgb(if selected {
                        palette::accent()
                    } else {
                        palette::bg_rail()
                    }))
                    .border_1()
                    .border_color(rgb(if selected {
                        palette::accent()
                    } else {
                        palette::border_default()
                    }))
                    .text_size(px(12.))
                    .text_color(rgb(if selected {
                        palette::WHITE
                    } else {
                        palette::text_secondary()
                    }))
                    .child(ico(IconName::Hash, Size::XSmall))
                    .child(name.clone())
                    .id(format!("welcome-channel-{name}"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_welcome_channel(name.clone(), cx);
                    })),
            );
        }
        card = card.child(
            v_flex()
                .gap_2()
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child("先看看这些频道"),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child("可以多选，之后随时能在侧栏切换。"),
                        ),
                )
                .child(chips),
        );
    }

    // 底部：提示 + 稍后再说 + 开始聊天。
    let hint = if welcome.require_rules && !agreed {
        String::from("请先阅读并同意社区规则")
    } else if selected_count > 0 {
        format!("将带你进入已选的 {} 个频道中的第一个", selected_count)
    } else {
        String::from("也可以稍后自己在侧栏里挑频道")
    };

    card = card.child(
        h_flex()
            .items_center()
            .justify_between()
            .pt_1()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(11.))
                    .text_color(rgb(palette::text_dim()))
                    .child(hint),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded(px(4.))
                            .text_size(px(12.))
                            .text_color(rgb(palette::text_secondary()))
                            .hover(|el| {
                                el.bg(rgb(palette::bg_toolbar_hover()))
                                    .text_color(rgb(palette::text_primary()))
                            })
                            .child("稍后再说")
                            .id("welcome-skip")
                            .on_click(cx.listener(|this, _, _, cx| this.close_welcome(cx))),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .px_4()
                            .py_2()
                            .rounded(px(4.))
                            .bg(rgb(if can_confirm {
                                palette::accent()
                            } else {
                                palette::bg_composer()
                            }))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(if can_confirm {
                                palette::WHITE
                            } else {
                                palette::text_dim()
                            }))
                            .child("开始聊天")
                            .child(ico(IconName::ArrowRight, Size::XSmall))
                            .id("welcome-confirm")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.confirm_welcome(window, cx);
                            })),
                    ),
            ),
    );

    // 遮罩与卡片是兄弟节点：点遮罩不误触卡片内的按钮。
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(rgba(0x00000099))
                .id("welcome-backdrop"),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(card),
        )
        .into_any_element()
}
