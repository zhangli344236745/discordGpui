use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::UserStatus;
use crate::ui::theme::palette;

use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;
use crate::ui::ico;

const STATUSES: &[(UserStatus, IconName)] = &[
    (UserStatus::Online, IconName::CircleDot),
    (UserStatus::Idle, IconName::CircleDashed),
    (UserStatus::Dnd, IconName::CircleMinus),
    (UserStatus::Invisible, IconName::CircleSlash),
];

/// 在线状态选择弹层：绝对定位在底部用户栏上方，切换本人状态。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let mut menu = v_flex()
        .absolute()
        .left(px(8.))
        .bottom(px(58.))
        .w(px(200.))
        .p_1()
        .gap(px(2.))
        .rounded(px(8.))
        .bg(rgb(palette::BG_FOOTER))
        .border_1()
        .border_color(rgb(palette::BORDER_DEFAULT))
        .shadow_lg();

    for (status, icon) in STATUSES {
        let active = app.my_status == *status;
        let label = status.label();
        let color = status.color();
        let status_value = *status;
        let id = format!("status-{label}");

        menu = menu.child(
            div()
                .h(px(34.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded(px(4.))
                .bg(if active {
                    rgb(palette::BG_CHANNEL_SELECTED)
                } else {
                    rgb(palette::BG_FOOTER)
                })
                .text_size(px(13.))
                .text_color(if active {
                    rgb(palette::TEXT_PRIMARY)
                } else {
                    rgb(palette::TEXT_SECONDARY)
                })
                .hover(|el| el.bg(rgb(palette::BG_CHANNEL_HOVER)))
                .child(ico(*icon, Size::Small).text_color(rgb(color)))
                .child(label)
                .when(active, |el| {
                    el.child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_end()
                            .child(ico(IconName::Check, Size::XSmall)),
                    )
                })
                .id(id)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_my_status(status_value);
                    cx.notify();
                })),
        );
    }

    menu
}
