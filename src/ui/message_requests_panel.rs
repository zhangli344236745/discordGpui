//! 私信消息请求面板：聊天区右侧浮层（与置顶/收件箱/日程互斥）。
//! 顶部状态筛选（待处理/已接受/已忽略，带计数）+ 请求卡片（接受→真正建立私信会话、忽略、删除）。
//! 全部本地演示，随快照持久化；接受后切入与对方的私信会话。

use gpui_kit::*;

use crate::app::{DiscordApp, MessageRequestFilter};
use crate::data::{MessageRequest, MessageRequestState};
use crate::ui::theme::palette;
use crate::ui::{avatar, filter_chip, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 状态徽标配色。
fn state_color(state: MessageRequestState) -> u32 {
    match state {
        MessageRequestState::Pending => palette::accent(),
        MessageRequestState::Accepted => palette::success(),
        MessageRequestState::Ignored => palette::text_dim(),
    }
}

/// 单张请求卡片：头像 + 名字 + 时间 + 状态徽标 + 消息预览 + 操作按钮。
fn request_card(
    request: &MessageRequest,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let sender = request.sender.clone();
    let sender_accept = sender.clone();
    let sender_ignore = sender.clone();
    let sender_delete = sender.clone();
    let pending = request.state == MessageRequestState::Pending;

    v_flex()
        .m_2()
        .p_3()
        .gap_2()
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(avatar(
                    request.sender.chars().next().unwrap_or('?'),
                    request.color,
                    px(32.),
                    false,
                ))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(palette::text_primary()))
                                        .child(sender),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(rgb(state_color(request.state)))
                                        .text_color(rgb(palette::WHITE))
                                        .text_size(px(10.))
                                        .child(request.state.label()),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .text_color(rgb(palette::text_dim()))
                                .child(request.time_label()),
                        ),
                ),
        )
        .child(
            div()
                .text_size(px(11.5))
                .line_height(px(16.))
                .text_color(rgb(palette::text_secondary()))
                .child(request.preview.clone()),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .mt_1()
                .when(pending, |el| {
                    el.child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded(px(4.))
                            .bg(rgb(palette::accent()))
                            .text_color(rgb(palette::WHITE))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("接受")
                            .id(format!("req-accept-{sender_accept}"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.accept_message_request(sender_accept.clone(), window, cx);
                            })),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded(px(4.))
                            .bg(rgb(palette::bg_composer()))
                            .text_color(rgb(palette::text_secondary()))
                            .text_size(px(11.))
                            .child("忽略")
                            .id(format!("req-ignore-{sender_ignore}"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.ignore_message_request(sender_ignore.clone(), cx);
                            })),
                    )
                })
                .child(div().flex_1())
                .child(
                    div()
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_toolbar_hover()))
                                .text_color(rgb(palette::danger()))
                        })
                        .child(ico(IconName::Trash, Size::XSmall))
                        .id(format!("req-delete-{sender_delete}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.delete_message_request(sender_delete.clone(), cx);
                        })),
                ),
        )
}

/// 消息请求面板主体；由 render_main 决定是否挂载。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let pending = app.pending_request_count();
    let accepted = app.request_count_for(MessageRequestFilter::Accepted);
    let ignored = app.request_count_for(MessageRequestFilter::Ignored);

    let mut filters = h_flex().gap_1().flex_wrap();
    for filter in [
        MessageRequestFilter::Pending,
        MessageRequestFilter::Accepted,
        MessageRequestFilter::Ignored,
    ] {
        let count = match filter {
            MessageRequestFilter::Pending => pending,
            MessageRequestFilter::Accepted => accepted,
            MessageRequestFilter::Ignored => ignored,
        };
        let label = format!("{} {}", filter.label(), count);
        filters = filters.child(filter_chip(
            label,
            app.message_request_filter == filter,
            format!("req-filter-{}", filter.label()),
            Box::new(move |this, _, cx| {
                this.set_message_request_filter(filter, cx);
            }),
            cx,
        ));
    }

    let visible = app.visible_message_requests();
    let mut body = v_flex()
        .flex_1()
        .min_h_0()
        .id("message-requests-scroll")
        .overflow_y_scroll();

    if let Some(notice) = &app.message_request_notice {
        body = body.child(
            h_flex()
                .mx_2()
                .mt_2()
                .px_3()
                .py_2()
                .gap_2()
                .rounded(px(6.))
                .bg(rgb(palette::bg_rail()))
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(notice.clone()),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(palette::text_dim()))
                        .hover(|el| el.text_color(rgb(palette::text_primary())))
                        .child("×")
                        .id("req-dismiss-notice")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dismiss_message_request_notice(cx);
                        })),
                ),
        );
    }

    if visible.is_empty() {
        body = body.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(palette::text_muted()))
                .child(ico(IconName::Inbox, Size::Large))
                .child(
                    div()
                        .text_size(px(13.))
                        .child("这个筛选下没有请求"),
                ),
        );
    }
    for request in &visible {
        body = body.child(request_card(request, cx));
    }

    div()
        .absolute()
        .right(px(0.))
        .top(px(0.))
        .bottom(px(0.))
        .w(px(360.))
        .flex()
        .flex_col()
        .bg(rgb(palette::bg_sidebar()))
        .border_l_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            h_flex()
                .h(px(48.))
                .flex_shrink_0()
                .px_4()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(rgb(palette::bg_header_border()))
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(format!("消息请求 — {pending} 待处理")),
                )
                .child(
                    div()
                        .w(px(28.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_toolbar_hover()))
                                .text_color(rgb(palette::text_primary()))
                        })
                        .child(ico(IconName::X, Size::Small))
                        .id("message-requests-close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_message_requests(cx);
                        })),
                ),
        )
        .child(h_flex().m_2().mb_1().child(filters))
        .child(body)
}
