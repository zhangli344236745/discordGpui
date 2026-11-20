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
                    rgb(palette::BG_CHANNEL_SELECTED)
                } else {
                    rgb(palette::BG_SIDEBAR)
                })
                .hover(|el| el.bg(rgb(palette::BG_CHANNEL_HOVER)))
                .child(
                    div()
                        .w(px(28.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .bg(rgb(palette::BG_COMPOSER))
                        .text_color(if is_image {
                            rgb(palette::ACCENT)
                        } else {
                            rgb(palette::TEXT_SECONDARY)
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
                                .text_color(rgb(palette::TEXT_PRIMARY))
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::TEXT_DIM))
                                .child(size),
                        ),
                )
                .when(selected, |el| {
                    el.child(ico(IconName::Check, Size::Small).text_color(rgb(palette::SUCCESS)))
                })
                .id(format!("attach-pick-{index}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.pick_attachment(attachment.clone(), window, cx);
                })),
        );
    }

    v_flex()
        .absolute()
        .bottom(px(52.))
        .left(px(16.))
        .w(px(300.))
        .p_2()
        .gap_2()
        .rounded(px(10.))
        .bg(rgb(palette::BG_SIDEBAR))
        .border_1()
        .border_color(rgb(palette::BORDER_DEFAULT))
        .shadow_lg()
        .child(
            v_flex()
                .gap_0p5()
                .px_1()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child("上传文件"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::TEXT_DIM))
                        .child("Demo 环境，从下面的模拟文件中选择"),
                ),
        )
        .child(list)
        .child(div().h(px(1.)).bg(rgb(palette::BG_DIVIDER)))
        .child(
            h_flex()
                .px_1()
                .gap_1()
                .text_size(px(11.))
                .text_color(rgb(palette::TEXT_DIM))
                .child(ico(IconName::Info, Size::XSmall))
                .child("选择后可继续输入文字，或直接发送附件"),
        )
}
