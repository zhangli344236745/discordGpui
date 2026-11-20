use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{div, px, rgb, Div, FontWeight, ParentElement as _, Pixels, Styled as _};

use super::theme::palette;

/// 圆形头像，可选在线状态小圆点（默认绿色）。
pub fn avatar(letter: char, color: u32, size: Pixels, online: bool) -> Div {
    avatar_status(letter, color, size, if online { Some(palette::online()) } else { None })
}

/// 圆形头像，状态点颜色可自定义（用于在线/闲置/勿扰/隐身）。
pub fn avatar_status(letter: char, color: u32, size: Pixels, status_color: Option<u32>) -> Div {
    let diameter = size.as_f32();
    div()
        .relative()
        .w(size)
        .h(size)
        .flex_shrink_0()
        .rounded(px(diameter / 2.))
        .bg(rgb(color))
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(palette::WHITE))
        .text_size(px((diameter * 0.45).max(10.)))
        .font_weight(FontWeight::SEMIBOLD)
        .child(letter.to_string())
        .when(status_color.is_some(), |el| {
            el.child(
                div()
                    .absolute()
                    .right(px(-4.))
                    .bottom(px(-4.))
                    .w(px(12.))
                    .h(px(12.))
                    .rounded_full()
                    .bg(rgb(status_color.unwrap_or(palette::online()))),
            )
        })
}

/// 频道未读消息徽标。
pub fn unread_badge(count: usize) -> Div {
    div()
        .min_w(px(16.))
        .h(px(16.))
        .px_1()
        .rounded_full()
        .bg(rgb(palette::danger()))
        .text_color(rgb(palette::WHITE))
        .text_size(px(10.))
        .font_weight(FontWeight::BOLD)
        .flex()
        .items_center()
        .justify_center()
        .child(count.to_string())
}
