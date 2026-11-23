//! 已读回执详情浮层：点自己消息下方的「已读 N 人」状态打开，列出读者与时间。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{avatar, close_button, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 光标锚点浮层：与消息右键菜单共用「全屏透明遮罩 + 绝对定位卡片」骨架。
pub fn render_floating(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some((_, readers)) = app.read_receipts_detail() else {
        return div().into_any_element();
    };
    let Some((_, x, y)) = app.read_receipts else {
        return div().into_any_element();
    };

    // 靠近窗口右/下边缘时往回收，避免卡片被裁掉。
    let left = x.min(1230.);
    let top = y.min(700.);

    let header = h_flex()
        .h(px(36.))
        .px_2()
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(ico(IconName::CheckCheck, Size::XSmall))
        .child(
            div()
                .flex_1()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child("已读回执"),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child(format!("{} 人", readers.len())),
        )
        .child(close_button(
            "read-receipts-close",
            Box::new(|this, _, cx| {
                this.close_read_receipts();
                cx.notify();
            }),
            cx,
        ));

    let mut body = v_flex()
        .gap_1()
        .p_2()
        .max_h(px(240.))
        .id("read-receipts-scroll")
        .overflow_y_scroll();

    for (index, reader) in readers.iter().enumerate() {
        body = body.child(
            h_flex()
                .px_2()
                .py_1()
                .gap_2()
                .rounded(px(5.))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(avatar(
                    reader.name.chars().next().unwrap_or('?'),
                    reader.color,
                    px(24.),
                    true,
                ))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.5))
                        .text_color(rgb(palette::text_primary()))
                        .child(reader.name),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(rgb(palette::text_dim()))
                        .child(reader.time),
                )
                .id(format!("read-receipt-{index}")),
        );
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .id("read-receipts-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_read_receipts();
            cx.notify();
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, _, _, cx| {
                this.close_read_receipts();
                cx.notify();
            }),
        )
        .child(
            v_flex()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(260.))
                .rounded(px(8.))
                .overflow_hidden()
                .bg(rgb(palette::bg_footer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .child(header)
                .child(body),
        )
        .into_any_element()
}
