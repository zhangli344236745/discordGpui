use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{avatar_status, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 侧栏底部用户控制条：本人信息 + 麦克风/耳机/设置开关。
/// 供频道侧栏与私信侧栏共用，保持视觉与交互一致。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    h_flex()
        .h(px(52.))
        .flex_shrink_0()
        .px_2()
        .gap_1()
        .bg(rgb(palette::bg_footer()))
        .child(
            h_flex()
                .flex_1()
                .gap_2()
                .min_w_0()
                .h(px(40.))
                .px_1()
                .rounded(px(4.))
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(avatar_status(
                    'R',
                    palette::accent(),
                    px(32.),
                    Some(app.my_status.color()),
                ))
                .child(
                    v_flex()
                        .min_w_0()
                        .justify_center()
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child("Riven"),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_secondary()))
                                .child(app.my_status.label()),
                        ),
                )
                .id("user-status-toggle")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_status_menu();
                    cx.notify();
                })),
        )
        .child(
            div()
                .w(px(30.))
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(if app.mic_on {
                    rgb(palette::text_message())
                } else {
                    rgb(palette::danger())
                })
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(ico(IconName::Mic, Size::Small))
                .id("sidebar-toggle-mic")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_mic();
                    cx.notify();
                })),
        )
        .child(
            div()
                .w(px(30.))
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(if app.headphones_on {
                    rgb(palette::text_message())
                } else {
                    rgb(palette::danger())
                })
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(ico(IconName::Headphones, Size::Small))
                .id("sidebar-toggle-headphones")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_headphones();
                    cx.notify();
                })),
        )
        .child(
            div()
                .w(px(30.))
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(rgb(palette::text_message()))
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(ico(IconName::Settings, Size::Small))
                .id("sidebar-open-settings")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.open_settings();
                    cx.notify();
                })),
        )
}
