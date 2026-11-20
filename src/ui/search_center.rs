use gpui_kit::*;

use crate::app::{DiscordApp, SearchScope};
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 范围筛选：唯一数据源，app 与视图都只读这一份。
const SCOPES: [(SearchScope, &str); 4] = [
    (SearchScope::CurrentServer, "当前服务器"),
    (SearchScope::AllServers, "全部服务器"),
    (SearchScope::Dm, "私信"),
    (SearchScope::Everything, "全部"),
];

/// 搜索中心：替换聊天主区域，跨频道 / 私信展示命中结果，点击即跳转。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let query = app.search_center_query.trim().to_string();

    let header = h_flex()
        .h(px(48.))
        .flex_shrink_0()
        .px_4()
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(rgb(palette::BG_HEADER_BORDER))
        .child(ico(IconName::Search, Size::Medium).text_color(rgb(palette::TEXT_DIM)))
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::TEXT_PRIMARY))
                .child("搜索"),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(11.5))
                .text_color(rgb(palette::TEXT_DIM))
                .child("跨频道与私信查找历史消息"),
        )
        .child(
            div()
                .w(px(28.))
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(rgb(palette::TEXT_SECONDARY))
                .hover(|el| {
                    el.bg(rgb(palette::BG_TOOLBAR_HOVER))
                        .text_color(rgb(palette::TEXT_PRIMARY))
                })
                .child(ico(IconName::X, Size::Small))
                .id("search-center-close")
                .on_click(cx.listener(|this, _, _, cx| this.close_search_center(cx))),
        );

    // 搜索框 + 范围 chips。
    let mut chips = h_flex().gap_2().flex_wrap();
    for (scope, label) in SCOPES {
        let active = app.search_scope == scope;
        chips = chips.child(
            div()
                .px_3()
                .h(px(26.))
                .flex()
                .items_center()
                .rounded_full()
                .text_size(px(12.))
                .bg(if active {
                    rgb(palette::ACCENT)
                } else {
                    rgb(palette::BG_SIDEBAR)
                })
                .text_color(if active {
                    rgb(palette::WHITE)
                } else {
                    rgb(palette::TEXT_SECONDARY)
                })
                .hover(|el| el.bg(rgb(palette::BG_TOOLBAR_HOVER)))
                .child(label)
                .id(format!("search-scope-{label}"))
                .on_click(cx.listener(move |this, _, _, cx| this.set_search_scope(scope, cx))),
        );
    }

    let search_bar = v_flex()
        .flex_shrink_0()
        .px_4()
        .py_3()
        .gap_3()
        .border_b_1()
        .border_color(rgb(palette::BG_HEADER_BORDER))
        .child(
            h_flex()
                .h(px(40.))
                .px_3()
                .gap_2()
                .rounded(px(8.))
                .bg(rgb(palette::BG_COMPOSER))
                .items_center()
                .child(ico(IconName::Search, Size::Small).text_color(rgb(palette::TEXT_DIM)))
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.search_center_input), Size::Large)
                        .flex_1(),
                )
                .when(!query.is_empty(), |el| {
                    el.child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette::TEXT_DIM))
                            .child(format!("{} 条结果", app.search_results.len())),
                    )
                }),
        )
        .child(chips);

    let mut body = v_flex()
        .flex_1()
        .min_h_0()
        .id("search-center-scroll")
        .overflow_y_scroll()
        .py_2();

    if query.is_empty() {
        body = body.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_3()
                .child(
                    div()
                        .w(px(56.))
                        .h(px(56.))
                        .rounded_full()
                        .bg(rgb(palette::BG_WELCOME_ICON))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(ico(IconName::Search, Size::Medium)),
                )
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child("输入关键词开始搜索"),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(rgb(palette::TEXT_MUTED))
                        .child("会同时匹配消息正文与发送者昵称"),
                ),
        );
    } else if app.search_results.is_empty() {
        body = body.child(
            v_flex()
                .h_full()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child(format!("没有找到匹配 “{}” 的消息", app.search_center_query)),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::TEXT_DIM))
                        .child("换个关键词，或把范围切到「全部」再试一次"),
                ),
        );
    } else {
        for (index, hit) in app.search_results.iter().enumerate() {
            let letter = hit.author.chars().next().unwrap_or('?');
            let scope_label = hit.scope_label();
            let is_dm = hit.is_dm;

            body = body.child(
                h_flex()
                    .mx_3()
                    .mb_1()
                    .px_3()
                    .py_2()
                    .gap_3()
                    .items_start()
                    .rounded(px(6.))
                    .hover(|el| el.bg(rgb(palette::BG_MESSAGE_HOVER)))
                    .child(avatar(letter, hit.color.into(), px(32.), false))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(3.))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(palette::TEXT_PRIMARY))
                                            .child(hit.author),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette::TEXT_DIM))
                                            .child(hit.time),
                                    )
                                    .child(
                                        div()
                                            .px_1p5()
                                            .rounded(px(3.))
                                            .bg(if is_dm {
                                                rgb(palette::BG_EDIT)
                                            } else {
                                                rgb(palette::BG_CHANNEL_HOVER)
                                            })
                                            .text_size(px(10.5))
                                            .text_color(rgb(palette::TEXT_MUTED))
                                            .child(scope_label),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(19.))
                                    .text_color(rgb(palette::TEXT_MESSAGE))
                                    .child(highlight(&hit.text, &query)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(rgb(palette::TEXT_DIM))
                            .child("跳转"),
                    )
                    .id(format!("search-hit-{index}"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_search_hit(index, window, cx);
                    })),
            );
        }
    }

    v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .bg(rgb(palette::BG_CHAT))
        .child(header)
        .child(search_bar)
        .child(body)
}

/// 把命中的片段用强调色标出：按关键词切分，命中段单独成一段。
fn highlight(text: &str, query: &str) -> Div {
    let mut line = h_flex().flex_wrap().items_center();
    let lowered = text.to_lowercase();
    let needle = query.to_lowercase();

    // 大小写转换可能改变字节长度（极少见字符），此时不做高亮，直接整段输出。
    if needle.is_empty() || lowered.len() != text.len() || !lowered.contains(&needle) {
        return line.child(text.to_string());
    }

    let mut cursor = 0usize;
    let mut from = 0usize;
    while let Some(offset) = lowered[from..].find(&needle) {
        let start = from + offset;
        let end = start + needle.len();
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            break;
        }
        if start > cursor {
            line = line.child(text[cursor..start].to_string());
        }
        line = line.child(
            div()
                .px_1()
                .rounded(px(3.))
                .bg(rgb(palette::BG_MENTION))
                .text_color(rgb(palette::TEXT_LINK))
                .child(text[start..end].to_string()),
        );
        cursor = end;
        from = end;
        if from >= lowered.len() {
            break;
        }
    }

    if cursor < text.len() {
        line = line.child(text[cursor..].to_string());
    }
    line
}
