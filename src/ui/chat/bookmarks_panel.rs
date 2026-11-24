//! 消息书签面板：聊天区右侧浮层（与置顶 / 收件箱 / 日程 / 消息请求互斥）。
//!
//! 跨频道与私信汇总「我保存的消息」：顶部搜索、卡片展示来源 + 作者 + 正文预览，
//! 支持一键跳回原消息（滚动并高亮）与移除。全部本地演示，随快照持久化。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::data::Bookmark;
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

/// 单条书签卡片。
fn bookmark_card(marker: &Bookmark, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let jump_target = marker.clone();
    let remove_target = marker.clone();
    let location = marker.location_label();
    let source = marker.source_label();
    let saved = marker.saved_label();
    let preview = marker.preview(120);
    let author = marker.author.clone();
    let time = marker.time.clone();
    // 卡片身份用「来源 + 正文」拼，避免同名卡片 id 冲突。
    let key = format!(
        "{}-{}-{}-{}",
        marker.is_dm,
        marker.channel,
        marker.author,
        marker.text.len()
    );

    v_flex()
        .m_2()
        .p_3()
        .gap_2()
        .rounded(px(8.))
        .bg(rgb(palette::bg_rail()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded(px(4.))
                        .bg(rgb(palette::bg_composer()))
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(location),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child(source),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child(saved),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(avatar(
                    author.chars().next().unwrap_or('?'),
                    marker.color,
                    px(24.),
                    false,
                ))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(marker.color))
                        .child(author),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child(time),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(17.))
                .text_color(rgb(palette::text_message()))
                .child(preview),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .mt_1()
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(rgb(palette::accent()))
                        .text_color(rgb(palette::WHITE))
                        .text_size(px(11.))
                        .child(ico(IconName::ArrowUpRight, Size::XSmall))
                        .child("跳转")
                        .id(format!("bookmark-jump-{key}"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.jump_to_bookmark(&jump_target, window, cx);
                        })),
                )
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
                        .child(ico(IconName::BookmarkMinus, Size::XSmall))
                        .id(format!("bookmark-remove-{key}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_bookmark(&remove_target, cx);
                        })),
                ),
        )
}

/// 书签面板主体；由 render_main 决定是否挂载。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let visible = app.visible_bookmarks();
    let total = app.bookmarks.len();

    let header = h_flex()
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
                .child(format!("已保存 — {total}")),
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
                .id("bookmarks-close")
                .on_click(cx.listener(|this, _, _, cx| this.toggle_bookmarks(cx))),
        );

    let search_row = h_flex()
        .mx_2()
        .mt_2()
        .mb_1()
        .px_2()
        .h(px(28.))
        .gap_1()
        .items_center()
        .rounded(px(4.))
        .bg(rgb(palette::bg_rail()))
        .child(ico(IconName::Search, Size::XSmall).text_color(rgb(palette::text_dim())))
        .child(
            <Input as Sizable>::with_size(Input::new(&app.bookmark_search), Size::XSmall)
                .flex_1()
                .min_w_0(),
        );

    let mut body = v_flex()
        .flex_1()
        .min_h_0()
        .id("bookmarks-scroll")
        .overflow_y_scroll();

    if let Some(notice) = &app.bookmark_notice {
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
                        .id("bookmarks-dismiss-notice")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dismiss_bookmark_notice(cx);
                        })),
                ),
        );
    }

    if visible.is_empty() {
        let (icon, text) = if total == 0 {
            (IconName::Bookmark, "还没有保存的消息")
        } else {
            (IconName::Search, "没有匹配的书签")
        };
        body = body.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(palette::text_muted()))
                .child(ico(icon, Size::Large))
                .child(div().text_size(px(13.)).child(text))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child("在消息右键菜单里选择「保存消息」即可收藏"),
                ),
        );
    }

    for marker in &visible {
        body = body.child(bookmark_card(marker, cx));
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
        .child(header)
        .child(search_row)
        .child(body)
}
