//! 语音房间面板：连接语音后浮在侧栏底部状态条上方的参与者列表。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 面板标题栏 + 参与者列表 + 本地设备控制。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let (server, channel) = match app.voice_label() {
        Some(label) => label,
        None => return div().into_any_element(),
    };

    let mut rows = v_flex().gap_1();

    // 自己：状态取自底部用户栏的麦克风开关。
    rows = rows.child(
        h_flex()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(4.))
            .child(avatar('你', palette::accent(), px(26.), false))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(rgb(palette::text_primary()))
                    .child("你"),
            )
            .child(
                div()
                    .text_color(rgb(if app.mic_on {
                        palette::success()
                    } else {
                        palette::danger()
                    }))
                    .child(ico(
                        if app.mic_on {
                            IconName::Mic
                        } else {
                            IconName::MicOff
                        },
                        Size::XSmall,
                    )),
            ),
    );

    for (index, participant) in app.voice_participants.iter().enumerate() {
        let muted = participant.muted;
        rows = rows.child(
            h_flex()
                .gap_2()
                .px_2()
                .py_1()
                .rounded(px(4.))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(avatar(
                    participant.name.chars().next().unwrap_or('?'),
                    participant.color,
                    px(26.),
                    true,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(participant.name),
                )
                .child(
                    div()
                        .w(px(20.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.))
                        .text_color(rgb(if muted {
                            palette::danger()
                        } else {
                            palette::text_dim()
                        }))
                        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                        .child(ico(
                            if muted { IconName::MicOff } else { IconName::Mic },
                            Size::XSmall,
                        ))
                        .id(format!("voice-mute-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_participant_mute(index, cx);
                        })),
                )
                .child(
                    div()
                        .w(px(20.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.))
                        .text_color(rgb(palette::text_dim()))
                        .hover(|el| {
                            el.bg(rgb(palette::danger()))
                                .text_color(rgb(palette::WHITE))
                        })
                        .child(ico(IconName::UserX, Size::XSmall))
                        .id(format!("voice-kick-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_voice_participant(index, cx);
                        })),
                )
                .when(muted, |el| {
                    el.child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette::text_dim()))
                            .child("已静音"),
                    )
                }),
        );
    }

    v_flex()
        .absolute()
        // 状态条位于 left 72 / bottom 52，面板贴在它上方。
        .left(px(72.))
        .bottom(px(108.))
        .w(px(300.))
        .p_2()
        .gap_2()
        .rounded(px(10.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(ico(IconName::Volume2, Size::Small).text_color(rgb(palette::success())))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(1.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child(channel),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_dim()))
                                .child(server),
                        ),
                )
                .child(
                    div()
                        .w(px(20.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_toolbar_hover()))
                                .text_color(rgb(palette::text_primary()))
                        })
                        .child(ico(IconName::X, Size::XSmall))
                        .id("voice-room-close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_voice_room(cx);
                        })),
                ),
        )
        .child(
            div()
                .px_2()
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(palette::text_muted()))
                .child(format!("语音房间 · {} 人", app.voice_participants.len() + 1)),
        )
        .child(rows)
        .child(div().h(px(1.)).bg(rgb(palette::bg_divider())))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    h_flex()
                        .flex_1()
                        .h(px(28.))
                        .justify_center()
                        .gap_1()
                        .rounded(px(4.))
                        .bg(if app.mic_on {
                            rgb(palette::bg_rail())
                        } else {
                            rgb(palette::danger())
                        })
                        .text_size(px(11.))
                        .text_color(if app.mic_on {
                            rgb(palette::text_secondary())
                        } else {
                            rgb(palette::WHITE)
                        })
                        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                        .child(ico(
                            if app.mic_on {
                                IconName::Mic
                            } else {
                                IconName::MicOff
                            },
                            Size::XSmall,
                        ))
                        .child(if app.mic_on { "静音" } else { "取消静音" })
                        .id("voice-room-mic")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_mic(cx);
                            cx.notify();
                        })),
                )
                .child(
                    h_flex()
                        .flex_1()
                        .h(px(28.))
                        .justify_center()
                        .gap_1()
                        .rounded(px(4.))
                        .bg(rgb(palette::bg_rail()))
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| {
                            el.bg(rgb(palette::danger()))
                                .text_color(rgb(palette::WHITE))
                        })
                        .child(ico(IconName::PhoneOff, Size::XSmall))
                        .child("断开")
                        .id("voice-room-disconnect")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.leave_voice(cx);
                            cx.notify();
                        })),
                ),
        )
        .into_any_element()
}
