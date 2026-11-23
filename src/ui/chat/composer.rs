use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::Attachment;
use crate::ui::theme::palette;
use crate::ui::{
    avatar,
    chat::{attachment_picker, emoji_picker, gif_picker, sticker_picker},
    ico,
};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

/// 待发送附件条：显示文件名与大小，可一键移除。
fn attachment_chip(attachment: &Attachment, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let name = attachment.name.clone();
    let size = attachment.size;
    let is_image = attachment.is_image();

    h_flex()
        .w_full()
        .gap_2()
        .p_2()
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .child(
            div()
                .w(px(32.))
                .h(px(32.))
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
                        .text_size(px(12.5))
                        .text_color(rgb(palette::text_primary()))
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(format!("{size} · 待发送")),
                ),
        )
        .child(
            div()
                .w(px(22.))
                .h(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(rgb(palette::text_secondary()))
                .hover(|el| {
                    el.bg(rgb(palette::bg_toolbar_hover()))
                        .text_color(rgb(palette::text_primary()))
                })
                .child(ico(IconName::X, Size::XSmall))
                .id("composer-clear-attachment")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.clear_attachment(cx);
                })),
        )
}

/// 消息输入区：附件条 + 输入框 + 表情/发送按钮 + 表情与附件弹层。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    // 输入框 placeholder 由 app.reset_session 在切换会话时统一更新。
    let mut wrapper = v_flex().mx_4().mb_6().gap_2().relative();

    if let Some(attachment) = &app.attachment_draft {
        wrapper = wrapper.child(attachment_chip(attachment, cx));
    }

    wrapper = wrapper.child(
        h_flex()
            .min_h(px(44.))
            .rounded(px(8.))
            .bg(rgb(palette::bg_composer()))
            .items_center()
            .px_2()
            .gap_2()
            .child(
                div()
                    .w(px(24.))
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(if app.show_attachment_picker {
                        rgb(palette::text_primary())
                    } else {
                        rgb(palette::text_secondary())
                    })
                    .hover(|el| el.bg(rgb(palette::text_primary())))
                    .text_color(rgb(palette::bg_composer()))
                    .child(ico(IconName::Plus, Size::XSmall))
                    .id("composer-attach")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_attachment_picker(cx);
                    })),
            )
            .child(<Input as Sizable>::with_size(Input::new(&app.composer), Size::Large).flex_1())
            // 礼物按钮直达「礼物」贴纸包，贴纸按钮打开当前包。
            .child(
                div()
                    .w(px(26.))
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(if app.show_sticker_picker && app.sticker_pack == "礼物" {
                        rgb(palette::accent())
                    } else {
                        rgb(palette::text_secondary())
                    })
                    .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                    .child(ico(IconName::Gift, Size::Small))
                    .id("composer-gift")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_sticker_picker(Some("礼物"), cx);
                    })),
            )
            // GIF 按钮：打开带搜索与分类的 GIF 弹层。
            .child(
                div()
                    .w(px(26.))
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(if app.show_gif_picker {
                        rgb(palette::accent())
                    } else {
                        rgb(palette::text_secondary())
                    })
                    .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                    .child(ico(IconName::ImagePlay, Size::Small))
                    .id("composer-gif")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_gif_picker(cx);
                    })),
            )
            .child(
                div()
                    .w(px(26.))
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(if app.show_sticker_picker {
                        rgb(palette::text_primary())
                    } else {
                        rgb(palette::text_secondary())
                    })
                    .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                    .child(ico(IconName::Sticker, Size::Small))
                    .id("composer-sticker")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_sticker_picker(None, cx);
                    })),
            )
            .child(
                div()
                    .w(px(28.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(if app.emoji_picker_open && app.emoji_target.is_none() {
                        rgb(palette::text_primary())
                    } else {
                        rgb(palette::text_secondary())
                    })
                    .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                    .child(ico(IconName::FaceSlightlySmiling, Size::Small))
                    .id("composer-emoji")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_emoji_picker(None);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .w(px(32.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .bg(rgb(palette::accent()))
                    .text_color(rgb(palette::WHITE))
                    .child(ico(IconName::Send, Size::Small))
                    .id("composer-send")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.send_message(window, cx);
                    })),
            ),
    );

    if let Some(reply) = app.reply_to.as_ref() {
        wrapper = wrapper.child(
            h_flex()
                .h(px(30.))
                .px_3()
                .gap_2()
                .rounded_t(px(8.))
                .bg(rgb(palette::bg_sidebar()))
                .text_size(px(12.))
                .text_color(rgb(palette::text_secondary()))
                .child(
                    div()
                        .child(ico(IconName::Reply, Size::XSmall))
                        .child(format!("回复 @{author}：{excerpt}", author = reply.author, excerpt = reply.excerpt)),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .justify_end()
                        .hover(|el| el.text_color(rgb(palette::text_primary())))
                        .child("取消")
                        .id("composer-cancel-reply")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cancel_reply();
                            cx.notify();
                        })),
                ),
        );
    }

    if app.emoji_picker_open {
        wrapper = wrapper.child(emoji_picker::render(app, cx));
    }

    if app.show_attachment_picker {
        wrapper = wrapper.child(attachment_picker::render(app, cx));
    }

    if app.show_sticker_picker {
        wrapper = wrapper.child(sticker_picker::render(app, cx));
    }

    if app.show_gif_picker {
        wrapper = wrapper.child(gif_picker::render(app, cx));
    }

    if !app.mention_candidates.is_empty() {
        wrapper = wrapper.child(mention_overlay(app, cx));
    }

    wrapper
}

/// @提及候选浮层：绝对定位在输入框上方，列出匹配成员，点击插入。
fn mention_overlay(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let prefix = app.mention_prefix.clone().unwrap_or_default();
    let mut list = v_flex()
        .id("mention-list")
        .max_h(px(220.))
        .overflow_y_scroll()
        .p_1()
        .gap_1();

    for (index, name) in app.mention_candidates.iter().enumerate() {
        let name = *name;
        let info = app.profile_info(name);
        let short = name.chars().next().unwrap_or('?');
        let role = info.role;
        list = list.child(
            h_flex()
                .h(px(38.))
                .px_2()
                .gap_2()
                .rounded(px(4.))
                .items_center()
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .id(format!("mention-option-{index}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.apply_mention(name, window, cx);
                }))
                .child(avatar(short, info.color, px(24.), false))
                .child(
                    v_flex()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(palette::text_primary()))
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_dim()))
                                .child(role),
                        ),
                ),
        );
    }

    v_flex()
        .absolute()
        .left(px(12.))
        .right(px(12.))
        .bottom(px(76.))
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            div()
                .px_3()
                .pt_2()
                .pb_1()
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(palette::text_muted()))
                .child(if prefix.is_empty() {
                    "选择一个成员".to_string()
                } else {
                    format!("匹配 \"@{prefix}\" 的成员")
                }),
        )
        .child(list)
}
