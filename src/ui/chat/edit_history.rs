//! 编辑历史浮层：点消息旁的「（已编辑）」或消息菜单打开，按版本列出编辑前的正文。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{close_button, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 光标锚点浮层：与已读回执 / 消息菜单共用「全屏透明遮罩 + 绝对定位卡片」骨架。
pub fn render_floating(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some((_, revisions)) = app.edit_history_detail() else {
        return div().into_any_element();
    };
    let Some((_, x, y)) = app.edit_history else {
        return div().into_any_element();
    };

    // 靠近窗口右/下边缘时往回收，避免卡片被裁掉。
    let left = x.min(1120.);
    let top = y.min(560.);

    let header = h_flex()
        .h(px(36.))
        .px_2()
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(ico(IconName::Clock, Size::XSmall))
        .child(
            div()
                .flex_1()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child("编辑历史"),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child(format!("{} 个版本", revisions.len())),
        )
        .child(close_button(
            "edit-history-close",
            Box::new(|this, _, cx| {
                this.close_edit_history();
                cx.notify();
            }),
            cx,
        ));

    // 历史按旧 → 新保存，展示时倒序让最近版本在最上。
    let mut body = v_flex()
        .gap_2()
        .p_2()
        .max_h(px(320.))
        .id("edit-history-scroll")
        .overflow_y_scroll();

    for (index, revision) in revisions.iter().enumerate().rev() {
        body = body.child(
            v_flex()
                .p_2()
                .gap_1()
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .child(
                    h_flex()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_secondary()))
                                .child(format!("版本 {}", index + 1)),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_dim()))
                                .child(revision.time),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(rgb(palette::text_message()))
                        .child(revision.text.clone()),
                ),
        );
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .id("edit-history-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_edit_history();
            cx.notify();
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, _, _, cx| {
                this.close_edit_history();
                cx.notify();
            }),
        )
        .child(
            v_flex()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(340.))
                .rounded(px(8.))
                .overflow_hidden()
                .bg(rgb(palette::bg_footer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .child(header)
                .child(body)
                .child(
                    div()
                        .px_3()
                        .pb_2()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child("仅展示历史版本，不提供回滚"),
                ),
        )
        .into_any_element()
}
