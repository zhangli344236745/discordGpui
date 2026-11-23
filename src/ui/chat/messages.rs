use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::{Attachment, DeliveryStatus, ForwardOrigin, ReplyRef, Sticker};
use crate::ui::theme::palette;
use crate::ui::{avatar, ico, chat::message_menu};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 把消息文本渲染为可高亮 @提及的行内片段。
/// 已知用户的 `@名字` 显示为可点击链接，点击弹出资料卡；其余按普通文本渲染。
fn render_text(
    text: &str,
    app: &DiscordApp,
    cx: &mut Context<DiscordApp>,
    index: usize,
) -> Div {
    let mut line = h_flex().flex_wrap().items_center();
    let mut plain = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    let flush = |line: Div, plain: &mut String| -> Div {
        if plain.is_empty() {
            line
        } else {
            let chunk = std::mem::take(plain);
            line.child(chunk)
        }
    };

    while i < chars.len() {
        if chars[i] == '@' {
            // 读取 @ 之后的连续非空白、非标点字符作为提及名。
            let mut j = i + 1;
            while j < chars.len()
                && !chars[j].is_whitespace()
                && !matches!(chars[j], '，' | ',' | '。' | '.' | '！' | '!' | '？' | '?')
            {
                j += 1;
            }
            let token: String = chars[i + 1..j].iter().collect();
            if let Some(name) = app.resolve_mention(&token) {
                line = flush(line, &mut plain);
                let mention_id = format!("mention-{index}-{i}");
                line = line.child(
                    div()
                        .px_1()
                        .rounded(px(3.))
                        .bg(rgb(palette::bg_mention()))
                        .text_color(rgb(palette::text_link()))
                        .hover(|el| {
                            el.bg(rgb(palette::accent()))
                                .text_color(rgb(palette::WHITE))
                        })
                        .child(format!("@{name}"))
                        .id(mention_id)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_profile(name);
                            cx.notify();
                        })),
                );
                i = j;
                continue;
            }
        }
        plain.push(chars[i]);
        i += 1;
    }

    flush(line, &mut plain)
}

/// 贴纸卡片：Demo 用大号 emoji 字形代替真实贴纸图，底部带名称。
fn sticker_card(sticker: &Sticker) -> Div {
    v_flex()
        .mt_1()
        .w(px(132.))
        .h(px(132.))
        .items_center()
        .justify_center()
        .gap_1()
        .rounded(px(10.))
        .bg(rgb(palette::bg_attachment_preview()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .child(
            div()
                .text_size(px(54.))
                .line_height(px(60.))
                .child(sticker.emoji),
        )
        .child(
            div()
                .text_size(px(10.5))
                .text_color(rgb(palette::text_dim()))
                .child(sticker.label),
        )
}

/// GIF 卡片：色块 + emoji 字形模拟动图，右侧标题与 GIF 角标。
fn gif_card(gif: &crate::data::Gif) -> Div {
    h_flex()
        .mt_1()
        .w(px(184.))
        .h(px(96.))
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(px(8.))
        .bg(rgb(gif.color))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .child(
            div()
                .text_size(px(40.))
                .line_height(px(44.))
                .child(gif.emoji),
        )
        .child(
            v_flex()
                .gap_0p5()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::WHITE))
                        .child(gif.title),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(rgb(palette::WHITE))
                        .child("GIF · 演示占位"),
                ),
        )
}

/// 消息附件卡片：图片给缩略图占位块，其他文件给文件行；整卡点击打开大图/文件预览。
fn attachment_card(
    attachment: &Attachment,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let name = attachment.name.clone();
    let size = attachment.size;
    let preview = attachment.clone();
    let preview_id = format!("attachment-preview-{index}");
    let download = h_flex()
        .gap_1()
        .text_size(px(11.))
        .text_color(rgb(palette::text_muted()))
        .child(ico(IconName::Download, Size::XSmall))
        .child(size);

    if attachment.is_image() {
        v_flex()
            .mt_1()
            .w(px(300.))
            .rounded(px(8.))
            .overflow_hidden()
            .bg(rgb(palette::bg_sidebar()))
            .border_1()
            .border_color(rgb(palette::border_default()))
            .child(
                div()
                    .w_full()
                    .h(px(150.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(palette::bg_attachment_preview()))
                    .text_color(rgb(palette::text_dim()))
                    .child(ico(IconName::Image, Size::Large)),
            )
            .child(
                h_flex()
                    .w_full()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(rgb(palette::text_primary()))
                            .child(name),
                    )
                    .child(download),
            )
            .hover(|el| el.border_color(rgb(palette::accent())))
            .id(preview_id.clone())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_attachment_preview(preview.clone(), cx);
            }))
    } else {
        h_flex()
            .mt_1()
            .w(px(300.))
            .p_3()
            .gap_3()
            .rounded(px(8.))
            .bg(rgb(palette::bg_sidebar()))
            .border_1()
            .border_color(rgb(palette::border_default()))
            .child(
                div()
                    .w(px(34.))
                    .h(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .bg(rgb(palette::bg_composer()))
                    .text_color(rgb(palette::text_secondary()))
                    .child(ico(IconName::FileText, Size::Small)),
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
                    .child(download),
            )
            .hover(|el| el.border_color(rgb(palette::accent())))
            .id(preview_id)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_attachment_preview(preview.clone(), cx);
            }))
    }
}

/// 未读分隔线：红色细线 + 居中「N 条新消息」胶囊，点击也能跳到最新。
fn unread_divider_row(count: usize, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let line = || div().h(px(1.)).flex_1().bg(rgb(palette::danger()));
    h_flex()
        .w_full()
        .px_4()
        .py_1()
        .gap_2()
        .items_center()
        .child(line())
        .child(
            div()
                .px_2()
                .py(px(2.))
                .rounded_full()
                .bg(rgb(palette::danger()))
                .text_color(rgb(palette::WHITE))
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("{count} 条新消息"))
                .id("unread-divider-jump")
                .hover(|el| el.bg(rgb(palette::accent())))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.jump_to_latest(cx);
                })),
        )
        .child(line())
}

/// 右下角「跳到最新」浮标：当前会话有未读时显示。
fn jump_latest_pill(count: usize, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    h_flex()
        .absolute()
        .bottom(px(40.))
        .right(px(20.))
        .h(px(32.))
        .px_3()
        .gap_2()
        .items_center()
        .rounded_full()
        .bg(rgb(palette::accent()))
        .text_color(rgb(palette::WHITE))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .shadow_lg()
        .hover(|el| el.bg(rgb(palette::accent_hover())))
        .child(ico(IconName::ArrowDown, Size::XSmall))
        .child(format!("{count} 条新消息 · 跳到最新"))
        .id("chat-jump-latest")
        .on_click(cx.listener(|this, _, _, cx| {
            this.jump_to_latest(cx);
        }))
}

/// 转发来源提示：正文上方的小字「转发自 #频道」，点击跳回原会话。
fn forward_origin_row(
    origin: &ForwardOrigin,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> Div {
    let label = origin.label.clone();
    let target = origin.clone();

    h_flex()
        .gap_1()
        .items_center()
        .text_size(px(11.5))
        .text_color(rgb(palette::text_muted()))
        .child(ico(IconName::Forward, Size::XSmall))
        .child(
            div()
                .text_color(rgb(palette::text_link()))
                .hover(|el| el.underline())
                .child(format!("转发自 {label}"))
                .id(format!("forward-origin-{index}-{label}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.jump_to_origin(target.clone(), window, cx);
                })),
        )
}

/// 回复引用条：正文上方的「回复 @作者：摘要」，左侧竖线，点击跳回原消息。
fn reply_quote_row(
    reply: &ReplyRef,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let excerpt = reply.excerpt.clone();
    let author = reply.author;

    div()
        .mb_1()
        .pl_2()
        .border_l_2()
        .border_color(rgb(palette::border_default()))
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .text_size(px(11.5))
                .child(ico(IconName::Reply, Size::XSmall))
                .child(
                    h_flex()
                        .gap_1()
                        .min_w_0()
                        .text_color(rgb(palette::text_link()))
                        .hover(|el| el.underline())
                        .child(format!("回复 @{author}"))
                        .child(
                            div()
                                .truncate()
                                .text_color(rgb(palette::text_muted()))
                                .child(excerpt.clone()),
                        )
                        .id(format!("reply-quote-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.jump_to_reply(&excerpt, cx);
                        })),
                ),
        )
}

/// 话题回复数徽标：已有话题的消息显示「N 条回复 · 查看话题」。
fn thread_badge(
    count: usize,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    h_flex()
        .mt_1()
        .gap_1()
        .items_center()
        .text_size(px(11.))
        .text_color(rgb(palette::text_link()))
        .hover(|el| el.underline())
        .child(ico(IconName::MessageSquare, Size::XSmall))
        .child(format!("{count} 条回复 · 查看话题"))
        .id(format!("msg-thread-badge-{index}"))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.toggle_thread(index, window, cx);
        }))
}

/// 自己消息底部的送达状态：已发送 / 已送达 / 已读 N 人；已读时可点开读者详情。
fn delivery_row(
    message: &crate::data::Message,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> AnyElement {
    if message.author != "你" {
        return div().into_any_element();
    }

    let readers = message.readers.len();
    let (icon, label) = match message.delivery {
        DeliveryStatus::Sent => (IconName::Check, "已发送".to_string()),
        DeliveryStatus::Delivered => (IconName::CheckCheck, "已送达".to_string()),
        DeliveryStatus::Read => (
            IconName::CheckCheck,
            if readers > 0 {
                format!("已读 {readers} 人")
            } else {
                "已读".to_string()
            },
        ),
    };
    let row = h_flex()
        .mt_1()
        .gap_1()
        .items_center()
        .text_size(px(10.5))
        .text_color(if message.delivery == DeliveryStatus::Read {
            rgb(palette::text_link())
        } else {
            rgb(palette::text_dim())
        })
        .child(ico(icon, Size::XSmall))
        .child(label);

    if readers == 0 {
        return row.into_any_element();
    }

    row.hover(|el| el.underline())
        .id(format!("msg-delivery-{index}"))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                this.open_read_receipts(index, x, y, cx);
            }),
        )
        .into_any_element()
}

/// 转发成功提示条：显示在消息列表顶部，点 X 关闭。
fn notice_bar(notice: String, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    h_flex()
        .mx_4()
        .mt_2()
        .mb_1()
        .px_3()
        .h(px(32.))
        .gap_2()
        .items_center()
        .rounded(px(6.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::success()))
        .text_size(px(12.))
        .text_color(rgb(palette::text_secondary()))
        .child(ico(IconName::CircleCheck, Size::XSmall))
        .child(div().flex_1().child(notice))
        .child(
            div()
                .w(px(18.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(3.))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(ico(IconName::X, Size::XSmall))
                .id("forward-notice-close")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dismiss_forward_notice(cx);
                })),
        )
}

/// 消息列表：搜索过滤、频道欢迎页、消息分组与表情回应。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let messages = app.current_messages();
    let divider = app.unread_divider();
    let unread = app.current_unread_count();
    let query = app.search_query.trim().to_lowercase();
    let mut list = v_flex().flex_1().min_h_0().id("chat-message-list");

    if let Some(notice) = app.forward_notice.clone() {
        list = list.child(notice_bar(notice, cx));
    }

    if !query.is_empty() {
        let count = messages
            .iter()
            .filter(|m| m.text.to_lowercase().contains(&query))
            .count();
        list = list.child(
            h_flex()
                .px_4()
                .py_2()
                .gap_2()
                .text_size(px(12.))
                .text_color(rgb(palette::text_secondary()))
                .child(format!("搜索 “{}” — {} 条结果", app.search_query, count)),
        );
    }

    let mut scroll = div()
        .flex_1()
        .min_h_0()
        .id("chat-message-scroll")
        .overflow_y_scroll()
        .track_scroll(&app.scroll_handle)
        .pb_2();

    if messages.is_empty() {
        scroll = scroll.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .w(px(68.))
                        .h(px(68.))
                        .rounded_full()
                        .bg(rgb(palette::bg_welcome_icon()))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(palette::text_primary()))
                        .child(ico(
                            if app.is_dm() {
                                IconName::AtSign
                            } else {
                                IconName::Hash
                            },
                            Size::Large,
                        )),
                )
                .child(
                    div()
                        .text_size(px(20.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(if app.is_dm() {
                            app.session_title()
                        } else {
                            format!("欢迎来到 #{}", app.session_title())
                        }),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_muted()))
                        .child(if app.is_dm() {
                            "这是你们私聊的开始。"
                        } else {
                            "这里是这个频道的开始。"
                        }),
                ),
        );
    }

    let mut last_author = "";
    for (index, message) in messages.iter().enumerate() {
        if !query.is_empty() && !message.text.to_lowercase().contains(&query) {
            last_author = "";
            continue;
        }

        if let Some((at, count)) = divider {
            if at == index {
                scroll = scroll.child(unread_divider_row(count, cx));
                last_author = "";
            }
        }

        let same_author = last_author == message.author;
        if !same_author {
            let gap = if app.compact_messages {
                px(4.)
            } else {
                px(16.)
            };
            scroll = scroll.child(div().h(gap));
        }
        last_author = message.author;

        let grouped = message.grouped && same_author && index > 0;
        let author_name = message.author;
        let avatar_view = if grouped {
            div().w(px(40.)).flex_shrink_0()
        } else {
            avatar(
                message.author.chars().next().unwrap_or('?'),
                message.color.into(),
                px(40.),
                false,
            )
        };

        let mut content = v_flex().flex_1().min_w_0().gap_1();
        if !grouped {
            content = content.child(
                h_flex()
                    .gap_2()
                    .items_end()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(message.color)
                            .hover(|el| el.underline())
                            .child(message.author)
                            .id(format!("msg-author-{index}"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_profile(author_name);
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .when(app.show_timestamps, |el| {
                                el.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette::text_dim()))
                                        .child(message.time),
                                )
                            })
                            .when(message.is_edited(), |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(rgb(palette::text_dim()))
                                        .hover(|el| {
                                            el.underline()
                                                .text_color(rgb(palette::text_link()))
                                        })
                                        .child("（已编辑）")
                                        .id(format!("msg-edited-{index}"))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                                let x: f32 = event.position.x.into();
                                                let y: f32 = event.position.y.into();
                                                this.open_edit_history(index, x, y, cx);
                                            }),
                                        ),
                                )
                            }),
                    ),
            );
        }

        if app.editing_message == Some(index) {
            content = content.child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        div()
                            .w_full()
                            .rounded(px(6.))
                            .bg(rgb(palette::bg_edit()))
                            .border_1()
                            .border_color(rgb(palette::accent()))
                            .child(
                                <Input as Sizable>::with_size(
                                    Input::new(&app.editor),
                                    Size::Small,
                                )
                                .w_full(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .text_size(px(11.))
                            .child(
                                div()
                                    .text_color(rgb(palette::text_muted()))
                                    .child("Enter 保存 · Esc 放弃"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .justify_end()
                                    .text_color(rgb(palette::text_link()))
                                    .hover(|el| el.underline())
                                    .child("取消")
                                    .id(format!("edit-cancel-{index}"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_edit();
                                        cx.notify();
                                    })),
                            ),
                    ),
            );
        } else {
            if let Some(origin) = &message.forwarded {
                content = content.child(forward_origin_row(origin, index, cx));
            }
            if let Some(reply) = &message.reply_to {
                content = content.child(reply_quote_row(reply, index, cx));
            }
            if !message.text.is_empty() {
                content = content.child(
                    div()
                        .text_size(px(14.5))
                        .line_height(px(21.))
                        .text_color(rgb(palette::text_message()))
                        .child(render_text(&message.text, app, cx, index)),
                );
            }
            if let Some(attachment) = &message.attachment {
                content = content.child(attachment_card(attachment, index, cx));
            }
            if let Some(sticker) = &message.sticker {
                content = content.child(sticker_card(sticker));
            }
            if let Some(gif) = &message.gif {
                content = content.child(gif_card(gif));
            }
        }

        if app.show_emoji && !message.reactions.is_empty() {
            let mut row = h_flex().gap_1().mt_1();
            for (reaction_index, reaction) in message.reactions.iter().enumerate() {
                let emoji = reaction.emoji;
                let count = reaction.count;
                let mine = reaction.mine;
                let id = format!("reaction-{index}-{reaction_index}");

                row = row.child(
                    h_flex()
                        .gap_1()
                        .h(px(24.))
                        .px_2()
                        .rounded(px(8.))
                        .bg(if mine {
                            rgb(palette::bg_reaction_mine())
                        } else {
                            rgb(palette::bg_sidebar())
                        })
                        .border_1()
                        .border_color(if mine {
                            rgb(palette::accent())
                        } else {
                            rgb(palette::border_default())
                        })
                        .text_size(px(12.))
                        .text_color(if mine {
                            rgb(palette::text_reaction_mine())
                        } else {
                            rgb(palette::text_secondary())
                        })
                        .hover(|el| el.border_color(rgb(palette::text_dim())))
                        .child(emoji)
                        .child(count.to_string())
                        .id(id)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_reaction(index, reaction_index, cx);
                        })),
                );
            }
            content = content.child(row);
        }

        let thread_count = app.thread_count(index);
        if thread_count > 0 {
            content = content.child(thread_badge(thread_count, index, cx));
        }
        if app.is_mine(index) {
            content = content.child(delivery_row(message, index, cx));
        }

        // 消息工具栏：添加表情回应、回复、话题、更多操作。
        let add_emoji_id = format!("msg-add-emoji-{index}");
        let reply_id = format!("msg-reply-{index}");
        let thread_id = format!("msg-thread-{index}");
        let more_id = format!("msg-more-{index}");
        let menu_open = app.menu_message == Some(index);
        content = content.child(
            h_flex()
                .relative()
                .gap_1()
                .mt_1()
                .text_color(rgb(palette::text_dim()))
                .child(
                    div()
                        .w(px(22.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_message()))
                        })
                        .child(ico(IconName::FaceSlightlySmiling, Size::XSmall))
                        .id(add_emoji_id)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_emoji_picker(Some(index));
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .h(px(20.))
                        .px_1()
                        .flex()
                        .items_center()
                        .gap_1()
                        .rounded(px(4.))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_message()))
                        })
                        .child(ico(IconName::Reply, Size::XSmall))
                        .child(
                            div()
                                .text_size(px(11.))
                                .child("回复"),
                        )
                        .id(reply_id)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_reply_at(index, cx);
                        })),
                )
                .child(
                    div()
                        .h(px(20.))
                        .px_1()
                        .flex()
                        .items_center()
                        .gap_1()
                        .rounded(px(4.))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_message()))
                        })
                        .child(ico(IconName::MessageSquare, Size::XSmall))
                        .child(
                            div()
                                .text_size(px(11.))
                                .child(if thread_count > 0 { "话题" } else { "开话题" }),
                        )
                        .id(thread_id)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.toggle_thread(index, window, cx);
                        })),
                )
                .child(
                    div()
                        .w(px(22.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_message()))
                        })
                        .child(ico(IconName::Ellipsis, Size::XSmall))
                        .id(more_id)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_message_menu(index);
                            cx.notify();
                        })),
                )
                // 右键菜单走根级浮层，行内只在省略号入口时渲染。
                .when(menu_open && app.menu_message_point.is_none(), |el| {
                    el.child(message_menu::render(index, app, cx))
                }),
        );

        scroll = scroll.child(
            h_flex()
                .w_full()
                .px_4()
                .gap_3()
                .items_start()
                .hover(|el| el.bg(rgb(palette::bg_message_hover())))
                .when(app.reply_highlight == Some(index), |el| {
                    el.bg(rgb(palette::bg_reaction_mine()))
                })
                .child(avatar_view)
                .child(content)
                // 右键整行：在该行任意位置唤起操作菜单，跟随光标展开。
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        let x: f32 = event.position.x.into();
                        let y: f32 = event.position.y.into();
                        this.open_message_menu_at(index, x, y, cx);
                    }),
                ),
        );
    }

    list.child(scroll).child(
        h_flex()
            .h(px(24.))
            .flex_shrink_0()
            .px_4()
            .gap_2()
            .text_size(px(12.))
            .text_color(rgb(palette::text_muted()))
            .child(
                div()
                    .w(px(6.))
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(palette::success())),
            )
            .child(format!("{} 正在输入…", app.session_title())),
    )
    .when(unread > 0, |el| el.child(jump_latest_pill(unread, cx)))
}
