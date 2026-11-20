use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 用户资料卡弹层：点击消息作者或成员弹出，展示身份信息并提供私信入口。
/// 绝对定位，由 app 顶层 Render 挂到相对定位的根容器上。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let target = match app.profile_target {
        Some(name) => name,
        None => return div(),
    };
    let info = app.profile_info(target);

    div()
        .absolute()
        .left(px(120.))
        .top(px(120.))
        .w(px(300.))
        .rounded(px(8.))
        .bg(rgb(palette::BG_FOOTER))
        .border_1()
        .border_color(rgb(palette::BORDER_DEFAULT))
        .shadow_lg()
        .overflow_hidden()
        // 顶部横幅
        .child(
            div()
                .h(px(60.))
                .w_full()
                .bg(rgb(info.color)),
        )
        .child(
            v_flex()
                .p_4()
                .gap_3()
                .child(
                    h_flex().relative().mt(px(-30.)).child(avatar(
                        info.name.chars().next().unwrap_or('?'),
                        info.color,
                        px(64.),
                        info.online,
                    )),
                )
                .child(
                    v_flex().gap_1().child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(palette::TEXT_PRIMARY))
                            .child(info.name),
                    )
                    .child(
                        h_flex().gap_2().items_center().child(
                            div()
                                .px_2()
                                .py(px(2.))
                                .rounded(px(4.))
                                .bg(rgb(palette::BG_CHANNEL_SELECTED))
                                .text_size(px(11.))
                                .text_color(rgb(palette::TEXT_SECONDARY))
                                .child(info.role),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .text_size(px(11.))
                                .text_color(rgb(palette::TEXT_MUTED))
                                .child(
                                    div()
                                        .w(px(7.))
                                        .h(px(7.))
                                        .rounded_full()
                                        .bg(rgb(if info.online {
                                            palette::ONLINE
                                        } else {
                                            palette::TEXT_DIM
                                        })),
                                )
                                .child(if info.online { "在线" } else { "离线" }),
                        ),
                    ),
                )
                .child(
                    div()
                        .p_3()
                        .rounded(px(6.))
                        .bg(rgb(palette::BG_RAIL))
                        .text_size(px(12.))
                        .text_color(rgb(palette::TEXT_SECONDARY))
                        .child(info.activity),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .mt_1()
                        .child(
                            h_flex()
                                .flex_1()
                                .h(px(34.))
                                .items_center()
                                .justify_center()
                                .gap_2()
                                .rounded(px(4.))
                                .bg(rgb(palette::ACCENT))
                                .text_color(rgb(palette::WHITE))
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .hover(|el| el.bg(rgb(palette::ACCENT_HOVER)))
                                .child(ico(IconName::MessageSquare, Size::XSmall))
                                .child("发消息")
                                .id("profile-send-dm")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(name) = this.profile_target {
                                        this.open_dm(name.to_string(), window, cx);
                                    }
                                })),
                        )
                        .child(
                            div()
                                .w(px(34.))
                                .h(px(34.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.))
                                .bg(rgb(palette::BG_CHANNEL_HOVER))
                                .text_color(rgb(palette::TEXT_SECONDARY))
                                .hover(|el| el.text_color(rgb(palette::TEXT_PRIMARY)))
                                .child(ico(IconName::X, Size::XSmall))
                                .id("profile-close")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_profile();
                                    cx.notify();
                                })),
                        ),
                ),
        )
}
