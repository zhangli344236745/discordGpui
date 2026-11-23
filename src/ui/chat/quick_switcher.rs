//! 快速切换器：Ctrl/⌘+K 打开的顶部居中模态，搜索频道/私信并回车（或点击）跳转。
//! 与转发弹层共用 app.forward_targets() 的候选集合，但使用独立的 quick_query / quick_selected。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 弹层主体：遮罩 + 顶部卡片（搜索框 / 候选列表 / 快捷键说明）。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let targets = app.quick_targets();
    let selected = app.quick_selected;

    let mut list = v_flex()
        .id("quick-switcher-list")
        .max_h(px(360.))
        .overflow_y_scroll()
        .p_1();

    if targets.is_empty() {
        list = list.child(
            v_flex()
                .h(px(96.))
                .items_center()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(format!("没有匹配的频道或私信：{}", app.quick_query)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child("换个关键词试试"),
                ),
        );
    } else {
        for (index, target) in targets.into_iter().enumerate() {
            let active = index == selected;
            let color = target.color;
            let is_dm = target.is_dm;
            let label = target.label();
            let scope = target.scope();
            let row_id = format!("quick-target-{index}");

            list = list.child(
                h_flex()
                    .w_full()
                    .h(px(44.))
                    .px_2()
                    .gap_3()
                    .items_center()
                    .rounded(px(6.))
                    .bg(if active {
                        rgb(palette::bg_channel_selected())
                    } else {
                        rgba(0x00000000)
                    })
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
                                    .child(label),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette::text_muted()))
                                    .child(scope),
                            ),
                    )
                    .when(active, |el| {
                        el.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_link()))
                                .child("↵ 跳转"),
                        )
                    })
                    .id(row_id)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_quick_at(index, window, cx);
                    })),
            );
        }
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .justify_center()
        .bg(rgba(0x00000099))
        .id("quick-switcher-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_quick_switcher();
            cx.notify();
        }))
        .child(
            v_flex()
                .absolute()
                .top(px(128.))
                .w(px(520.))
                .gap_2()
                .p_3()
                .rounded(px(10.))
                .bg(rgb(palette::bg_chat()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .id("quick-switcher-card")
                .child(
                    h_flex()
                        .h(px(44.))
                        .px_3()
                        .gap_2()
                        .items_center()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_composer()))
                        .child(ico(IconName::Search, Size::Small))
                        .child(
                            <Input as Sizable>::with_size(
                                Input::new(&app.quick_search),
                                Size::Large,
                            )
                            .flex_1(),
                        ),
                )
                .child(list)
                .child(
                    h_flex()
                        .px_2()
                        .pb_1()
                        .gap_3()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child("↑ ↓ 选择")
                        .child("Enter 跳转")
                        .child("Esc 关闭")
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .justify_end()
                                .child(format!("{} 个会话", app.quick_targets().len())),
                        ),
                ),
        )
        .into_any_element()
}
