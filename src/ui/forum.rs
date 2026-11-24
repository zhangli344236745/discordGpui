//! 论坛频道页面：替代聊天区主区域，展示帖子列表 / 帖子详情 / 发帖弹层。
//! 支持搜索、三种排序（最新 / 最热 / 未回应）、置顶、锁定、回帖。
//! 所有状态与业务方法都在 app 层，这里只负责渲染与事件转发。

use std::rc::Rc;

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::{ForumPost, ForumSort};
use crate::ui::theme::palette;
use crate::ui::{avatar, ico, filter_chip};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 论坛页面入口：由 render_main 在当前频道为 Forum 时接管聊天主区域。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .bg(rgb(palette::bg_chat()))
        .child(header(app, cx))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .relative()
                .when(app.forum_open.is_some(), |el| {
                    el.child(detail(app, cx))
                })
                .when(app.forum_open.is_none(), |el| {
                    el.child(list(app, cx))
                })
                .when(app.show_forum_composer, |el| {
                    el.child(composer(app, cx))
                }),
        )
}

/// 顶栏：频道图标 + 名称 + 主题 + 新建帖子按钮。
fn header(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let channel = app.active_channel.clone();
    let topic = app
        .servers
        .get(app.active_server)
        .and_then(|server| {
            server
                .categories
                .iter()
                .flat_map(|category| category.channels.iter())
                .find(|item| item.name == channel)
        })
        .map(|item| item.topic.clone())
        .unwrap_or_default();

    h_flex()
        .h(px(48.))
        .flex_shrink_0()
        .px_4()
        .gap_2()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(ico(IconName::MessageSquare, Size::Medium).text_color(rgb(palette::text_dim())))
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child(channel),
        )
        .when(!topic.is_empty(), |el| {
            el.child(div().w(px(1.)).h(px(24.)).mx_2().bg(rgb(palette::bg_toolbar_hover())))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_muted()))
                        .child(topic),
                )
        })
        .child(div().flex_1())
        .child(
            h_flex()
                .h(px(30.))
                .px_3()
                .gap_2()
                .items_center()
                .rounded(px(4.))
                .bg(rgb(palette::accent()))
                .text_color(rgb(palette::WHITE))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .hover(|el| el.bg(rgb(palette::accent_hover())))
                .child(ico(IconName::MessageSquarePlus, Size::XSmall))
                .child("新建帖子")
                .id("forum-new-post")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.toggle_forum_composer(window, cx);
                })),
        )
}

/// 帖子列表：搜索框 + 排序 chips + 提示条 + 卡片列表。
fn list(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let posts = app.forum_visible_posts();
    let sorts = [
        ForumSort::Recent,
        ForumSort::Popular,
        ForumSort::Unanswered,
    ];

    let mut content = v_flex()
        .flex_1()
        .min_h_0()
        .id("forum-list-scroll")
        .overflow_y_scroll()
        .px_6()
        .pb_6();

    if posts.is_empty() {
        content = content.child(empty_state(app.forum_query.is_empty()));
    } else {
        for post in posts {
            content = content.child(post_card(&post, cx));
        }
    }

    v_flex()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(
            v_flex()
                .h(px(96.))
                .flex_shrink_0()
                .px_6()
                .pt_4()
                .gap_3()
                .border_b_1()
                .border_color(rgb(palette::bg_header_border()))
                .child(
                    h_flex()
                        .h(px(34.))
                        .w(px(300.))
                        .flex_shrink_0()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_composer()))
                        .border_1()
                        .border_color(rgb(palette::border_default()))
                        .child(ico(IconName::Search, Size::XSmall).text_color(rgb(palette::text_dim())))
                        .child(
                            <Input as Sizable>::with_size(Input::new(&app.forum_search), Size::Small)
                                .flex_1(),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette::text_muted()))
                                .child("排序"),
                        )
                        .children(sorts.iter().map(|sort| {
                            let value = *sort;
                            filter_chip(
                                value.label(),
                                app.forum_sort == value,
                                format!("forum-sort-{value:?}"),
                                Box::new(move |this, _, cx| this.set_forum_sort(value, cx)),
                                cx,
                            )
                        })),
                ),
        )
        .child(
            v_flex()
                .absolute()
                .top(px(96.))
                .left_0()
                .right_0()
                .bottom_0()
                .child(notice_bar(app, cx))
                .child(content),
        )
}

/// 空状态：区分「没有帖子」与「搜索无结果」。
fn empty_state(is_empty_query: bool) -> Div {
    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_3()
        .pt_16()
        .child(
            div()
                .w(px(64.))
                .h(px(64.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(rgb(palette::bg_welcome_icon()))
                .child(ico(IconName::MessageSquare, Size::Large).text_color(rgb(palette::text_dim()))),
        )
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child(if is_empty_query { "还没有帖子" } else { "没有匹配的帖子" }),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(palette::text_muted()))
                .child(if is_empty_query {
                    "点击右上角「新建帖子」发起第一个讨论"
                } else {
                    "试试其他关键词，或切换排序方式"
                }),
        )
}

/// 单个帖子卡：状态徽标 + 标题 + 标签 + 作者行 + 回应数。
fn post_card(post: &ForumPost, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let id = post.id.clone();
    let click_id = format!("forum-post-{id}");

    let mut badges = h_flex().gap_2().items_center();
    if post.pinned {
        badges = badges.child(
            h_flex()
                .h(px(18.))
                .px_1()
                .gap_1()
                .items_center()
                .rounded(px(3.))
                .bg(rgb(palette::bg_composer()))
                .text_color(rgb(palette::text_muted()))
                .text_size(px(10.))
                .child(ico(IconName::Pin, Size::XSmall))
                .child("置顶"),
        );
    }
    if post.locked {
        badges = badges.child(
            h_flex()
                .h(px(18.))
                .px_1()
                .gap_1()
                .items_center()
                .rounded(px(3.))
                .bg(rgb(palette::bg_composer()))
                .text_color(rgb(palette::text_muted()))
                .text_size(px(10.))
                .child(ico(IconName::Shield, Size::XSmall))
                .child("已锁定"),
        );
    }

    let mut tags = h_flex().gap_1().flex_wrap();
    for tag in &post.tags {
        tags = tags.child(
            div()
                .h(px(20.))
                .px_2()
                .flex()
                .items_center()
                .rounded_full()
                .bg(rgb(palette::bg_mention()))
                .text_color(rgb(palette::text_link()))
                .text_size(px(11.))
                .child(tag.clone()),
        );
    }

    div()
        .w_full()
        .mt_2()
        .p_4()
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .hover(|el| el.border_color(rgb(palette::accent())))
        .child(
            h_flex().justify_between().items_start().child(
                h_flex()
                    .min_w_0()
                    .gap_2()
                    .items_center()
                    .child(badges)
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(palette::text_primary()))
                            .child(post.title.clone()),
                    ),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .items_center()
                    .text_color(rgb(palette::text_dim()))
                    .text_size(px(12.))
                    .child(ico(IconName::MessageSquare, Size::XSmall))
                    .child(post.reply_count().to_string()),
            ),
        )
        .when(!post.tags.is_empty(), |el| el.mt_2().child(tags))
        .mt_2()
        .child(
            h_flex()
                .mt_2()
                .gap_2()
                .items_center()
                .child(avatar(
                    post.author.chars().next().unwrap_or('?'),
                    post.color,
                    px(22.),
                    false,
                ))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(palette::text_secondary()))
                        .child(post.author.clone()),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(post.time_label()),
                ),
        )
        .id(click_id)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.open_forum_post(id.clone(), window, cx);
        }))
}

/// 帖子详情：返回按钮 + 标题 + 作者 + 正文 + 回帖列表 + 回帖输入。
fn detail(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let Some(post) = app.forum_open_post() else {
        return div().into_any_element();
    };
    let id = post.id.clone();
    let pin_id = id.clone();
    let lock_id = id.clone();

    let mut tags = h_flex().gap_1().flex_wrap();
    for tag in &post.tags {
        tags = tags.child(
            div()
                .h(px(22.))
                .px_2()
                .flex()
                .items_center()
                .rounded_full()
                .bg(rgb(palette::bg_mention()))
                .text_color(rgb(palette::text_link()))
                .text_size(px(11.))
                .child(tag.clone()),
        );
    }

    let mut replies = v_flex().gap_2().mt_3();
    if post.replies.is_empty() {
        replies = replies.child(
            div()
                .py_4()
                .text_align(TextAlign::Center)
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child("还没有回帖，来抢个沙发"),
        );
    } else {
        for reply in &post.replies {
            replies = replies.child(
                h_flex()
                    .w_full()
                    .gap_3()
                    .p_3()
                    .rounded(px(6.))
                    .bg(rgb(palette::bg_message_hover()))
                    .child(avatar(
                        reply.author.chars().next().unwrap_or('?'),
                        reply.color,
                        px(26.),
                        false,
                    ))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(palette::text_primary()))
                                            .child(reply.author.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette::text_dim()))
                                            .child(relative_label(reply.minutes_ago)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(palette::text_message()))
                                    .child(reply.body.clone()),
                            ),
                    ),
            );
        }
    }

    v_flex()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(palette::bg_chat()))
        .child(
            h_flex()
                .h(px(44.))
                .flex_shrink_0()
                .px_5()
                .gap_2()
                .items_center()
                .border_b_1()
                .border_color(rgb(palette::bg_header_border()))
                .child(
                    h_flex()
                        .h(px(26.))
                        .px_2()
                        .gap_1()
                        .items_center()
                        .rounded(px(4.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                        .child(ico(IconName::ChevronLeft, Size::XSmall))
                        .child("返回帖子列表")
                        .id("forum-back")
                        .on_click(cx.listener(|this, _, _, cx| this.close_forum_post(cx))),
                )
                .child(div().flex_1())
                .child(
                    h_flex()
                        .h(px(26.))
                        .px_2()
                        .gap_1()
                        .items_center()
                        .rounded(px(4.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                        .child(ico(IconName::Pin, Size::XSmall))
                        .child(if post.pinned { "取消置顶" } else { "置顶" })
                        .id("forum-pin")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_forum_pin(pin_id.clone(), cx);
                        })),
                )
                .child(
                    h_flex()
                        .h(px(26.))
                        .px_2()
                        .gap_1()
                        .items_center()
                        .rounded(px(4.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                        .child(ico(IconName::Shield, Size::XSmall))
                        .child(if post.locked { "解锁" } else { "锁定" })
                        .id("forum-lock")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_forum_lock(lock_id.clone(), cx);
                        })),
                ),
        )
        .child(notice_bar(app, cx))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .id("forum-detail-scroll")
                .overflow_y_scroll()
                .px_6()
                .py_5()
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_size(px(22.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child(post.title.clone()),
                        )
                        .when(!post.tags.is_empty(), |el| el.child(tags))
                        .child(
                            h_flex().gap_2().items_center().child(avatar(
                                post.author.chars().next().unwrap_or('?'),
                                post.color,
                                px(28.),
                                false,
                            ))
                            .child(
                                v_flex()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(palette::text_primary()))
                                            .child(post.author.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette::text_dim()))
                                            .child(format!("发布于 {}", post.time_label())),
                                    ),
                            ),
                        )
                        .child(
                            div()
                                .mt_2()
                                .text_size(px(14.))
                                .line_height(px(22.))
                                .text_color(rgb(palette::text_message()))
                                .child(if post.body.is_empty() {
                                    "（无正文）".to_string()
                                } else {
                                    post.body.clone()
                                }),
                        )
                        .child(
                            div()
                                .mt_2()
                                .pt_3()
                                .border_t_1()
                                .border_color(rgb(palette::bg_divider()))
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_secondary()))
                                .child(format!("{} 条回帖", post.reply_count())),
                        )
                        .child(replies),
                ),
        )
        .child(reply_box(app, cx, post.locked))
        .into_any_element()
}

/// 回帖输入条：锁定时只读提示 + 禁用输入。
fn reply_box(app: &DiscordApp, cx: &mut Context<DiscordApp>, locked: bool) -> impl IntoElement {
    if locked {
        return h_flex()
            .h(px(64.))
            .flex_shrink_0()
            .px_6()
            .items_center()
            .gap_2()
            .border_t_1()
            .border_color(rgb(palette::bg_header_border()))
            .text_color(rgb(palette::text_dim()))
            .text_size(px(13.))
            .child(ico(IconName::Shield, Size::Small))
            .child("该帖子已锁定，无法继续回复")
            .into_any_element();
    }

    h_flex()
        .h(px(64.))
        .flex_shrink_0()
        .px_6()
        .gap_3()
        .items_center()
        .border_t_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(
            div()
                .flex_1()
                .h(px(38.))
                .rounded(px(8.))
                .bg(rgb(palette::bg_composer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.forum_reply_edit), Size::Large)
                        .flex_1(),
                ),
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
                .font_weight(FontWeight::MEDIUM)
                .hover(|el| el.bg(rgb(palette::accent_hover())))
                .child("回复")
                .id("forum-reply-send")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.send_forum_reply(window, cx);
                })),
        )
        .into_any_element()
}

/// 操作提示条（可点击关闭）。
fn notice_bar(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    if let Some(notice) = app.forum_notice.as_ref() {
        let mut bar = h_flex()
            .flex_shrink_0()
            .h(px(34.))
            .px_6()
            .gap_2()
            .items_center()
            .bg(rgb(palette::bg_mention()))
            .text_color(rgb(palette::text_link()))
            .text_size(px(12.))
            .child(ico(IconName::Info, Size::XSmall))
            .child(notice.clone())
            .child(div().flex_1());
        bar = bar.child(
            div()
                .w(px(18.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(ico(IconName::X, Size::XSmall))
                .id("forum-notice-close")
                .on_click(cx.listener(|this, _, _, cx| this.dismiss_forum_notice(cx))),
        );
        bar
    } else {
        div()
    }
}

/// 发帖弹层：居中模态（遮罩与卡片兄弟节点，避免点击冒泡误关）。
fn composer(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let cancel = Rc::new(|this: &mut DiscordApp, window: &mut Window, cx: &mut Context<DiscordApp>| {
        this.close_forum_composer(window, cx);
    });

    let mut tags = h_flex().gap_1().flex_wrap();
    for (index, tag) in app.forum_tags_draft.iter().enumerate() {
        let tag_label = tag.clone();
        tags = tags.child(
            h_flex()
                .h(px(22.))
                .px_2()
                .gap_1()
                .items_center()
                .rounded_full()
                .bg(rgb(palette::bg_mention()))
                .text_color(rgb(palette::text_link()))
                .text_size(px(11.))
                .child(tag_label)
                .child(
                    div()
                        .w(px(14.))
                        .h(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|el| el.text_color(rgb(palette::text_primary())))
                        .child(ico(IconName::X, Size::XSmall))
                        .id(format!("forum-remove-tag-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_forum_tag(index, cx);
                        })),
                ),
        );
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(0x00000099))
        .id("forum-composer-backdrop")
        .on_click(cx.listener({
            let cancel = cancel.clone();
            move |this, _, window, cx| {
                (*cancel)(this, window, cx);
                cx.notify();
            }
        }))
        .child(
            v_flex()
                .w(px(520.))
                .max_h(px(640.))
                .gap_4()
                .p_5()
                .rounded(px(10.))
                .bg(rgb(palette::bg_chat()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .id("forum-composer-card")
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(18.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child("新建帖子"),
                        )
                        .child(crate::ui::close_button(
                            "forum-composer-close",
                            {
                                let cancel = cancel.clone();
                                Box::new(move |this, window, cx| (*cancel)(this, window, cx))
                            },
                            cx,
                        )),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::text_secondary()))
                                .child("标题"),
                        )
                        .child(
                            div()
                                .w_full()
                                .rounded(px(6.))
                                .bg(rgb(palette::bg_composer()))
                                .border_1()
                                .border_color(rgb(palette::border_default()))
                                .child(
                                    <Input as Sizable>::with_size(
                                        Input::new(&app.forum_title_edit),
                                        Size::Small,
                                    )
                                    .w_full(),
                                ),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::text_secondary()))
                                .child("正文"),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(160.))
                                .rounded(px(6.))
                                .bg(rgb(palette::bg_composer()))
                                .border_1()
                                .border_color(rgb(palette::border_default()))
                                .child(
                                    <Input as Sizable>::with_size(
                                        Input::new(&app.forum_body_edit),
                                        Size::Large,
                                    )
                                    .w_full()
                                    .h_full(),
                                ),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::text_secondary()))
                                .child("标签（回车添加，最多 5 个）"),
                        )
                        .when(!app.forum_tags_draft.is_empty(), |el| el.child(tags))
                        .child(
                            div()
                                .w_full()
                                .rounded(px(6.))
                                .bg(rgb(palette::bg_composer()))
                                .border_1()
                                .border_color(rgb(palette::border_default()))
                                .child(
                                    <Input as Sizable>::with_size(
                                        Input::new(&app.forum_tag_edit),
                                        Size::Small,
                                    )
                                    .w_full(),
                                ),
                        ),
                )
                .when(app.forum_notice.is_some(), |el| {
                    el.child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette::danger()))
                            .child(app.forum_notice.clone().unwrap_or_default()),
                    )
                })
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
                                .id("forum-composer-cancel")
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
                                .child("发布帖子")
                                .id("forum-composer-submit")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.create_forum_post(window, cx);
                                })),
                        ),
                ),
        )
}

/// 分钟数转相对时间标签，与模型层的 time_label 保持一致。
fn relative_label(minutes: u32) -> String {
    match minutes {
        0..=1 => "刚刚".into(),
        2..=59 => format!("{} 分钟前", minutes),
        60..=1439 => format!("{} 小时前", minutes / 60),
        _ => format!("{} 天前", minutes / 1440),
    }
}
