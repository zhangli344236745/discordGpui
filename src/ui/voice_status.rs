use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};

/// 语音连接状态条：浮在侧栏底部、用户控制条上方，显示当前连接的语音频道。
/// 跨视图模式保持显示，符合 Discord 行为。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let (server, channel) = match app.voice_label() {
        Some(label) => label,
        None => return div(),
    };

    div()
        .absolute()
        .left(px(72.))
        .bottom(px(52.))
        .w(px(240.))
        .h(px(52.))
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .bg(rgb(palette::bg_footer()))
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(ico(IconName::Signal, Size::Small).text_color(rgb(palette::success())))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(1.))
                .id("voice-open-room")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_voice_room(cx);
                }))
                .child(
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::success()))
                        .child("语音已连接 · 点此查看房间"),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(format!("{channel} / {server}")),
                ),
        )
        .child(
            div()
                .w(px(26.))
                .h(px(26.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(rgb(palette::text_secondary()))
                .hover(|el| {
                    el.bg(rgb(palette::bg_toolbar_hover()))
                        .text_color(rgb(palette::danger()))
                })
                .child(ico(IconName::PhoneOff, Size::XSmall))
                .id("voice-disconnect")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.leave_voice(cx);
                    cx.notify();
                })),
        )
}
