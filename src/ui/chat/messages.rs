use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::Attachment;
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
                        .bg(rgb(palette::BG_MENTION))
                        .text_color(rgb(palette::TEXT_LINK))
                        .hover(|el| {
                            el.bg(rgb(palette::ACCENT))
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

/// 消息附件卡片：图片给缩略图占位块，其他文件给文件行。
fn attachment_card(attachment: &Attachment) -> Div {
    let name = attachment.name.clone();
    let size = attachment.size;
    let download = h_flex()
        .gap_1()
        .text_size(px(11.))
        .text_color(rgb(palette::TEXT_MUTED))
        .child(ico(IconName::Download, Size::XSmall))
        .child(size);

    if attachment.is_image() {
        v_flex()
            .mt_1()
            .w(px(300.))
            .rounded(px(8.))
            .overflow_hidden()
            .bg(rgb(palette::BG_SIDEBAR))
            .border_1()
            .border_color(rgb(palette::BORDER_DEFAULT))
            .child(
                div()
                    .w_full()
                    .h(px(150.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(palette::BG_ATTACHMENT_PREVIEW))
                    .text_color(rgb(palette::TEXT_DIM))
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
                            .text_color(rgb(palette::TEXT_PRIMARY))
                            .child(name),
                    )
                    .child(download),
            )
    } else {
        h_flex()
            .mt_1()
            .w(px(300.))
            .p_3()
            .gap_3()
            .rounded(px(8.))
            .bg(rgb(palette::BG_SIDEBAR))
            .border_1()
            .border_color(rgb(palette::BORDER_DEFAULT))
            .child(
                div()
                    .w(px(34.))
                    .h(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .bg(rgb(palette::BG_COMPOSER))
                    .text_color(rgb(palette::TEXT_SECONDARY))
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
                            .text_color(rgb(palette::TEXT_PRIMARY))
                            .child(name),
                    )
                    .child(download),
            )
    }
}

/// 消息列表：搜索过滤、频道欢迎页、消息分组与表情回应。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let messages = app.current_messages();
    let query = app.search_query.trim().to_lowercase();
    let mut list = v_flex().flex_1().min_h_0().id("chat-message-list");

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
                .text_color(rgb(palette::TEXT_SECONDARY))
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
                        .bg(rgb(palette::BG_WELCOME_ICON))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(palette::TEXT_PRIMARY))
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
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child(if app.is_dm() {
                            app.session_title()
                        } else {
                            format!("欢迎来到 #{}", app.session_title())
                        }),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::TEXT_MUTED))
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
                    .when(app.show_timestamps, |el| {
                        el.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::TEXT_DIM))
                                .child(message.time),
                        )
                    }),
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
                            .bg(rgb(palette::BG_EDIT))
                            .border_1()
                            .border_color(rgb(palette::ACCENT))
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
                                    .text_color(rgb(palette::TEXT_MUTED))
                                    .child("Enter 保存 · Esc 放弃"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .justify_end()
                                    .text_color(rgb(palette::TEXT_LINK))
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
            if !message.text.is_empty() {
                content = content.child(
                    div()
                        .text_size(px(14.5))
                        .line_height(px(21.))
                        .text_color(rgb(palette::TEXT_MESSAGE))
                        .child(render_text(&message.text, app, cx, index)),
                );
            }
            if let Some(attachment) = &message.attachment {
                content = content.child(attachment_card(attachment));
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
                            rgb(palette::BG_REACTION_MINE)
                        } else {
                            rgb(palette::BG_SIDEBAR)
                        })
                        .border_1()
                        .border_color(if mine {
                            rgb(palette::ACCENT)
                        } else {
                            rgb(palette::BORDER_DEFAULT)
                        })
                        .text_size(px(12.))
                        .text_color(if mine {
                            rgb(palette::TEXT_REACTION_MINE)
                        } else {
                            rgb(palette::TEXT_SECONDARY)
                        })
                        .hover(|el| el.border_color(rgb(palette::TEXT_DIM)))
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

        // 消息工具栏：添加表情回应、回复、更多操作。
        let author = message.author;
        let add_emoji_id = format!("msg-add-emoji-{index}");
        let reply_id = format!("msg-reply-{index}");
        let more_id = format!("msg-more-{index}");
        let menu_open = app.menu_message == Some(index);
        content = content.child(
            h_flex()
                .relative()
                .gap_1()
                .mt_1()
                .text_color(rgb(palette::TEXT_DIM))
                .child(
                    div()
                        .w(px(22.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .hover(|el| {
                            el.bg(rgb(palette::BG_CHANNEL_HOVER))
                                .text_color(rgb(palette::TEXT_MESSAGE))
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
                            el.bg(rgb(palette::BG_CHANNEL_HOVER))
                                .text_color(rgb(palette::TEXT_MESSAGE))
                        })
                        .child(ico(IconName::Reply, Size::XSmall))
                        .child(
                            div()
                                .text_size(px(11.))
                                .child("回复"),
                        )
                        .id(reply_id)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_reply(author);
                            cx.notify();
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
                            el.bg(rgb(palette::BG_CHANNEL_HOVER))
                                .text_color(rgb(palette::TEXT_MESSAGE))
                        })
                        .child(ico(IconName::Ellipsis, Size::XSmall))
                        .id(more_id)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_message_menu(index);
                            cx.notify();
                        })),
                )
                .when(menu_open, |el| el.child(message_menu::render(index, app, cx))),
        );

        scroll = scroll.child(
            h_flex()
                .w_full()
                .px_4()
                .gap_3()
                .items_start()
                .hover(|el| el.bg(rgb(palette::BG_MESSAGE_HOVER)))
                .child(avatar_view)
                .child(content),
        );
    }

    list.child(scroll).child(
        h_flex()
            .h(px(24.))
            .flex_shrink_0()
            .px_4()
            .gap_2()
            .text_size(px(12.))
            .text_color(rgb(palette::TEXT_MUTED))
            .child(
                div()
                    .w(px(6.))
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(palette::SUCCESS)),
            )
            .child(format!("{} 正在输入…", app.session_title())),
    )
}
