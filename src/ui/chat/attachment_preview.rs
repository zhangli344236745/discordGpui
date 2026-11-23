//! 附件预览弹层：点击消息里的附件卡打开，图片与文件都给出放大的详情视图。
//! Demo 没有真实文件，预览区是带图标的占位块；「复制文件名」用剪贴板模拟下载。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 弹层主体：遮罩 + 居中卡片（头部 / 预览区 / 底部操作）。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let Some(attachment) = app.attachment_preview.clone() else {
        return div().into_any_element();
    };
    let name = attachment.name.clone();
    let size = attachment.size;
    let is_image = attachment.is_image();

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(0x000000cc))
        .id("attachment-preview-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_attachment_preview(cx);
        }))
        .child(
            v_flex()
                .id("attachment-preview-card")
                .w(px(560.))
                .overflow_hidden()
                .rounded(px(10.))
                .bg(rgb(palette::bg_chat()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .child(
                    h_flex()
                        .px_4()
                        .h(px(52.))
                        .gap_3()
                        .items_center()
                        .border_b_1()
                        .border_color(rgb(palette::border_default()))
                        .child(
                            div()
                                .w(px(30.))
                                .h(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_secondary()))
                                .child(ico(
                                    if is_image {
                                        IconName::Image
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
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(palette::text_primary()))
                                        .child(name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette::text_muted()))
                                        .child(format!(
                                            "{} · {size}",
                                            if is_image { "图片" } else { "文件" }
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .w(px(28.))
                                .h(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.))
                                .text_color(rgb(palette::text_muted()))
                                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                                .child(ico(IconName::X, Size::Small))
                                .id("attachment-preview-close")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_attachment_preview(cx);
                                })),
                        ),
                )
                .child(
                    v_flex()
                        .h(px(320.))
                        .w_full()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .bg(rgb(palette::bg_attachment_preview()))
                        .child(
                            div()
                                .w(px(132.))
                                .h(px(132.))
                                .rounded_full()
                                .bg(rgb(palette::bg_rail()))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(rgb(palette::text_dim()))
                                .child(ico(
                                    if is_image {
                                        IconName::Image
                                    } else {
                                        IconName::FileText
                                    },
                                    Size::Size(px(72.)),
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_secondary()))
                                .child(if is_image {
                                    "图片预览（演示数据：这里是占位图）"
                                } else {
                                    "文件预览（演示数据：不读取磁盘内容）"
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(format!("{name} · {size}")),
                        ),
                )
                .child(
                    h_flex()
                        .px_4()
                        .h(px(56.))
                        .gap_3()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child("点击预览外的区域或 X 关闭"),
                        )
                        .child(
                            h_flex()
                                .h(px(34.))
                                .px_4()
                                .gap_2()
                                .items_center()
                                .rounded(px(4.))
                                .bg(rgb(palette::accent()))
                                .text_color(rgb(palette::WHITE))
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .hover(|el| el.bg(rgb(palette::accent_hover())))
                                .child(ico(IconName::Download, Size::XSmall))
                                .child("复制文件名（演示下载）")
                                .id("attachment-preview-download")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.copy_attachment_name(cx);
                                })),
                        ),
                ),
        )
        .into_any_element()
}
