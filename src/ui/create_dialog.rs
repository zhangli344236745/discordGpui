//! 新建服务器 / 新建频道弹层：绝对定位的居中模态框，由 app 顶层挂载。

use std::rc::Rc;

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

/// 新建服务器弹层：输入名称，Enter 或点创建提交。
pub fn render_server(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    dialog(
        "创建服务器",
        "给你的新社区起个名字，稍后可以随时修改。",
        "服务器名称",
        "创建服务器",
        IconName::Users,
        <Input as Sizable>::with_size(Input::new(&app.server_name_input), Size::Large).w_full(),
        "create-server",
        |this, window, cx| this.create_server(window, cx),
        |this, _, _| this.close_create_server(),
        cx,
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
