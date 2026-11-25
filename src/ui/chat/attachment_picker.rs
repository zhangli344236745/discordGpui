use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::seed_attachments;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 附件选择弹层：绝对定位在输入框上方，列出模拟文件供选择。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let mut list = v_flex().gap_1();

    for (index, attachment) in seed_attachments().into_iter().enumerate() {
        let name = attachment.name.clone();
        let label = attachment.name.clone();
        let size = attachment.size;
        let is_image = attachment.is_image();
        let selected = app
            .attachment_draft
            .as_ref()
            .map(|current| current.name == label)
            .unwrap_or(false);

        list = list.child(
            h_flex()
                .gap_2()
                .px_2()
                .py_2()
                .rounded(px(6.))
                .bg(if selected {
                    rgb(palette::bg_channel_selected())
                } else {
                    rgb(palette::bg_sidebar())
                })
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(
                    div()
                        .w(px(28.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_composer()))
                        .text_color(if is_image {
                            rgb(palette::accent())
                        } else {
                            rgb(palette::text_secondary())
                        })
                        .child(ico(
                            if is_image {
                                IconName::FileImage
                            } else {
                                IconName::FileText
                            },
                            Size::Small,
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
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_primary()))
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(size),
                        ),
                )
                .when(selected, |el| {
                    el.child(ico(IconName::Check, Size::Small).text_color(rgb(palette::success())))
                })
                .id(format!("attach-pick-{index}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.pick_attachment(attachment.clone(), window, cx);
                })),
        );
    }

    let mut panel = v_flex()
        .absolute()
        .bottom(px(52.))
        .left(px(16.))
        .w(px(300.))
        .p_2()
        .gap_2()
        .rounded(px(10.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            v_flex()
                .gap_0p5()
                .px_1()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("上传文件"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child("选择本地文件上传，或从下面的演示文件中选择"),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .px_2()
                .py_2()
                .rounded(px(6.))
                .bg(rgb(palette::accent()))
                .text_color(rgb(palette::WHITE))
                .hover(|el| el.opacity(0.9))
                .child(ico(IconName::FileText, Size::Small))
                .child(div().text_size(px(12.)).child("选择本地文件…"))
                .id("attach-pick-local")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.pick_local_attachment(window, cx);
                })),
        );

    if let Some(error) = &app.upload_error {
        panel = panel.child(
            div()
                .px_1()
                .text_size(px(11.))
                .text_color(rgb(palette::danger()))
                .child(error.clone()),
        );
    }

    panel
        .child(list)
        .child(div().h(px(1.)).bg(rgb(palette::bg_divider())))
        .child(
            h_flex()
                .px_1()
                .gap_1()
                .text_size(px(11.))
                .text_color(rgb(palette::text_dim()))
                .child(ico(IconName::Info, Size::XSmall))
                .child("选择后可继续输入文字，或直接发送附件"),
        )
}
