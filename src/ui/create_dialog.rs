//! 新建服务器 / 新建频道弹层：绝对定位的居中模态框，由 app 顶层挂载。

use std::rc::Rc;

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::server_templates;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 新建服务器弹层：名称输入 + 模板选择，Enter 或点创建提交。
///
/// 三层兄弟结构（外层容器 → 遮罩 + 居中容器 → 卡片）：遮罩带关闭 handler，
/// 卡片不在遮罩里，因此点模板卡不会冒泡把弹层关掉。
pub fn render_server(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let selected = app.create_template.clone();
    let input_empty = app.server_name_input.read(cx).value().trim().is_empty();
    let submit_label = if selected.is_some() {
        "用模板创建"
    } else {
        "创建服务器"
    };

    let mut grid = h_flex().w_full().flex_wrap().gap_2();
    for template in server_templates() {
        let active = selected.as_deref() == Some(template.key);
        let name = template.name;
        let tagline = template.tagline;
        let badge = template.badge;
        let color = template.color;
        let count = template.channel_count();
        let categories = template.categories.len();
        let key = template.key;

        grid = grid.child(
            v_flex()
                .w(px(248.))
                .p_2()
                .gap_1()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(if active {
                    palette::accent()
                } else {
                    palette::border_default()
                }))
                .bg(rgb(if active {
                    palette::bg_channel_selected()
                } else {
                    palette::bg_sidebar()
                }))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .w(px(32.))
                                .h(px(32.))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(9.))
                                .bg(rgb(color))
                                .text_size(px(14.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::WHITE))
                                .child(badge),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(palette::text_primary()))
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette::text_muted()))
                                        .child(tagline),
                                ),
                        )
                        .when(active, |el| {
                            el.child(
                                ico(IconName::Check, Size::XSmall)
                                    .text_color(rgb(palette::accent())),
                            )
                        }),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(format!("{categories} 个分类 · {count} 个频道")),
                )
                .id(format!("create-template-{key}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_create_template(key, cx);
                })),
        );
    }

    let hint = match selected.as_deref() {
        Some(_) if input_empty => "名称留空时用模板名；创建后会自动走一遍欢迎屏幕。",
        Some(_) => "模板会铺好分类与频道，并带上介绍和社区规则。",
        None => "不选模板就只创建一个带「闲聊」的空白服务器。",
    };

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
                .id("create-server-backdrop")
                .on_click(cx.listener(|this, _, _, _| this.close_create_server())),
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
                .child(
                    v_flex()
                        .w(px(560.))
                        .gap_4()
                        .p_5()
                        .rounded(px(10.))
                        .bg(rgb(palette::bg_chat()))
                        .border_1()
                        .border_color(rgb(palette::border_default()))
                        .shadow_lg()
                        .id("create-server-card")
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            div()
                                                .w(px(34.))
                                                .h(px(34.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(px(8.))
                                                .bg(rgb(palette::bg_channel_hover()))
                                                .child(ico(IconName::Users, Size::Small)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(18.))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(palette::text_primary()))
                                                .child("创建服务器"),
                                        ),
                                )
                                .child(crate::ui::close_button(
                                    "create-server-close",
                                    Box::new(|this, _, _| this.close_create_server()),
                                    cx,
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_muted()))
                                .child("起个名字，或挑一个模板把分类和频道一次铺好。"),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(palette::text_secondary()))
                                        .child("服务器名称"),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .rounded(px(6.))
                                        .bg(rgb(palette::bg_composer()))
                                        .child(
                                            <Input as Sizable>::with_size(
                                                Input::new(&app.server_name_input),
                                                Size::Large,
                                            )
                                            .w_full(),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(palette::text_secondary()))
                                                .child("从模板开始（可选）"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(palette::text_dim()))
                                                .child("再点一次取消选择"),
                                        ),
                                )
                                .child(grid),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(hint),
                        )
                        .child(
                            h_flex()
                                .justify_end()
                                .gap_2()
                                .mt_1()
                                .child(
                                    div()
                                        .h(px(36.))
                                        .px_4()
                                        .flex()
                                        .items_center()
                                        .rounded(px(4.))
                                        .text_size(px(13.))
                                        .text_color(rgb(palette::text_secondary()))
                                        .hover(|el| el.text_color(rgb(palette::text_primary())))
                                        .child("取消")
                                        .id("create-server-cancel")
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.close_create_server();
                                        })),
                                )
                                .child(
                                    div()
                                        .h(px(36.))
                                        .px_4()
                                        .flex()
                                        .items_center()
                                        .rounded(px(4.))
                                        .bg(rgb(palette::accent()))
                                        .text_color(rgb(palette::WHITE))
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .hover(|el| el.bg(rgb(palette::accent_hover())))
                                        .child(submit_label)
                                        .id("create-server-submit")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.create_server(window, cx);
                                        })),
                                ),
                        ),
                ),
        )
}

/// 新建频道弹层：在当前服务器第一个分类下创建文字频道。
pub fn render_channel(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    dialog(
        "创建频道",
        format!("在 #{} 下新建一个文字频道。", app.active_server_ref().name),
        "频道名称",
        "创建频道",
        IconName::Hash,
        <Input as Sizable>::with_size(Input::new(&app.channel_name_input), Size::Large).w_full(),
        "create-channel",
        |this, window, cx| this.create_channel(window, cx),
        |this, _, _| this.close_create_channel(),
        cx,
    )
}

/// 通用模态框骨架：遮罩 + 居中卡片 + 输入框 + 取消/确认。
/// 供 create_dialog 与 topic_dialog 共用，新增弹层直接调用它即可。
#[allow(clippy::too_many_arguments)]
pub(crate) fn dialog<Submit, Cancel>(
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
    input_placeholder: &'static str,
    submit_label: &'static str,
    icon: IconName,
    input: impl IntoElement,
    id_prefix: &'static str,
    submit: Submit,
    cancel: Cancel,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement
where
    Submit: Fn(&mut DiscordApp, &mut Window, &mut Context<DiscordApp>) + 'static,
    Cancel: Fn(&mut DiscordApp, &mut Window, &mut Context<DiscordApp>) + 'static,
{
    let _ = input_placeholder;
    let title = title.into();
    let subtitle = subtitle.into();
    // 三个关闭入口（遮罩/X/取消）共享同一个闭包，用 Rc 避免重复移动。
    let cancel = Rc::new(cancel);

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(0x00000099))
        .id(format!("{id_prefix}-backdrop"))
        .on_click(cx.listener({
            let cancel = cancel.clone();
            move |this, _, window, cx| {
                (*cancel)(this, window, cx);
                cx.notify();
            }
        }))
        .child(
            v_flex()
                .w(px(440.))
                .gap_4()
                .p_5()
                .rounded(px(10.))
                .bg(rgb(palette::bg_chat()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .id(format!("{id_prefix}-card"))
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(34.))
                                        .h(px(34.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(8.))
                                        .bg(rgb(palette::bg_channel_hover()))
                                        .child(ico(icon, Size::Small)),
                                )
                                .child(
                                    div()
                                        .text_size(px(18.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(palette::text_primary()))
                                        .child(title),
                                ),
                        )
                        .child(crate::ui::close_button(
                            format!("{id_prefix}-close"),
                            {
                                let cancel = cancel.clone();
                                Box::new(move |this, window, cx| (*cancel)(this, window, cx))
                            },
                            cx,
                        )),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_muted()))
                        .child(subtitle),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::text_secondary()))
                                .child(input_placeholder),
                        )
                        .child(
                            div()
                                .w_full()
                                .rounded(px(6.))
                                .bg(rgb(palette::bg_composer()))
                                .child(input),
                        ),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .mt_1()
                        .child(
                            div()
                                .h(px(36.))
                                .px_4()
                                .flex()
                                .items_center()
                                .rounded(px(4.))
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_secondary()))
                                .hover(|el| el.text_color(rgb(palette::text_primary())))
                                .child("取消")
                                .id(format!("{id_prefix}-cancel"))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    (*cancel)(this, window, cx);
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .h(px(36.))
                                .px_4()
                                .flex()
                                .items_center()
                                .rounded(px(4.))
                                .bg(rgb(palette::accent()))
                                .text_color(rgb(palette::WHITE))
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .hover(|el| el.bg(rgb(palette::accent_hover())))
                                .child(submit_label)
                                .id(format!("{id_prefix}-submit"))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    submit(this, window, cx);
                                })),
                        ),
                ),
        )
}
