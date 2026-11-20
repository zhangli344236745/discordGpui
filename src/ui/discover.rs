//! 探索页：整屏浏览可加入的公开服务器，支持关键词搜索与分类筛选。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

use crate::app::DiscordApp;
use crate::ui::ico;
use crate::ui::theme::palette;

/// 分类筛选项；「全部」对应 discover_category = None。
const CATEGORIES: [&str; 7] = ["全部", "游戏", "科技", "设计", "音乐", "学习", "影视"];

/// 探索页整体布局：顶栏（标题/搜索/关闭）+ 可滚动内容区。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .bg(rgb(palette::BG_CHAT))
        .child(
            h_flex()
                .h(px(48.))
                .flex_shrink_0()
                .px_6()
                .gap_3()
                .items_center()
                .border_b_1()
                .border_color(rgb(palette::BG_HEADER_BORDER))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(ico(IconName::Compass, Size::Medium).text_color(rgb(palette::TEXT_DIM)))
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::TEXT_PRIMARY))
                                .child("探索"),
                        ),
                )
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .px_2()
                        .h(px(28.))
                        .rounded(px(4.))
                        .bg(rgb(palette::BG_RAIL))
                        .child(
                            <Input as Sizable>::with_size(Input::new(&app.discover_search), Size::XSmall)
                                .flex_1(),
                        )
                        .child(ico(IconName::Search, Size::XSmall).text_color(rgb(palette::TEXT_DIM))),
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
                        .id("discover-close")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.close_discover(window, cx);
                        })),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .id("discover-scroll")
                .overflow_y_scroll()
                .child(content(app, cx)),
        )
}

/// 内容区：标题说明 + 分类 chips + 服务器卡片网格 / 空状态。
fn content(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let query = app.discover_filter.trim().to_lowercase();
    let category = app.discover_category;

    let matches: Vec<(usize, &crate::data::DiscoverServer)> = app
        .discover_servers
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            let in_category = category.map(|value| entry.category == value).unwrap_or(true);
            let in_query = query.is_empty()
                || entry.name.to_lowercase().contains(&query)
                || entry.description.to_lowercase().contains(&query)
                || entry.category.contains(&query.as_str());
            in_category && in_query
        })
        .collect();

    let mut chips = h_flex().flex_wrap().gap_2();
    for (index, label) in CATEGORIES.iter().enumerate() {
        let selected = match app.discover_category {
            None => *label == "全部",
            Some(current) => *label == current,
        };
        let id = format!("discover-category-{index}");

        chips = chips.child(
            div()
                .h(px(28.))
                .px_3()
                .flex()
                .items_center()
                .rounded_full()
                .bg(if selected {
                    rgb(palette::ACCENT)
                } else {
                    rgb(palette::BG_SIDEBAR)
                })
                .text_size(px(12.))
                .text_color(if selected {
                    rgb(palette::WHITE)
                } else {
                    rgb(palette::TEXT_SECONDARY)
                })
                .hover(|el| el.bg(rgb(palette::BG_CHANNEL_HOVER)))
                .child(*label)
                .id(id)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let value = if *label == "全部" { None } else { Some(*label) };
                    this.set_discover_category(value, cx);
                })),
        );
    }

    let filtered = matches.len();
    let mut grid = h_flex().flex_wrap().gap_3();
    if matches.is_empty() {
        grid = grid.child(
            v_flex()
                .w_full()
                .py(px(56.))
                .gap_2()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(56.))
                        .h(px(56.))
                        .rounded_full()
                        .bg(rgb(palette::BG_WELCOME_ICON))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child(ico(IconName::Compass, Size::Large)),
                )
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child("没有找到匹配的服务器"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::TEXT_MUTED))
                        .child("换个关键词，或切回「全部」分类看看"),
                ),
        );
    } else {
        for (index, entry) in matches {
            grid = grid.child(card(index, entry, app, cx));
        }
    }

    v_flex()
        .w_full()
        .p(px(24.))
        .gap(px(16.))
        .child(
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_size(px(24.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .child("发现服务器"),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::TEXT_MUTED))
                        .child("挑一个感兴趣的社区加入，加入后会出现在左侧服务器栏。"),
                ),
        )
        .child(chips)
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::TEXT_DIM))
                .child(if filtered == app.discover_servers.len() {
                    format!("共 {} 个服务器", filtered)
                } else {
                    format!("筛选出 {filtered} / {} 个服务器", app.discover_servers.len())
                }),
        )
        .child(grid)
}

/// 单张服务器卡片：徽标、名称、分类、简介、成员数与加入按钮。
fn card(
    index: usize,
    entry: &crate::data::DiscoverServer,
    app: &DiscordApp,
    cx: &mut Context<DiscordApp>,
) -> Div {
    let joined = entry.joined || app.servers.iter().any(|server| server.name == entry.name);

    h_flex()
        .w(px(252.))
        .p(px(16.))
        .gap_3()
        .items_start()
        .rounded(px(10.))
        .bg(rgb(palette::BG_SIDEBAR))
        .border_1()
        .border_color(rgb(palette::BG_DIVIDER))
        .hover(|el| el.border_color(rgb(palette::BORDER_DEFAULT)))
        .child(
            div()
                .w(px(44.))
                .h(px(44.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(12.))
                .bg(rgb(entry.color))
                .text_size(px(17.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(palette::WHITE))
                .child(entry.badge.clone()),
        )
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
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::TEXT_PRIMARY))
                                .child(entry.name.clone()),
                        )
                        .child(
                            div()
                                .px_2()
                                .h(px(18.))
                                .flex()
                                .items_center()
                                .rounded_full()
                                .bg(rgb(palette::BG_RAIL))
                                .text_size(px(10.))
                                .text_color(rgb(palette::TEXT_MUTED))
                                .child(entry.category),
                        ),
                )
                .child(
                    div()
                        .h(px(34.))
                        .overflow_hidden()
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(rgb(palette::TEXT_SECONDARY))
                        .child(entry.description.clone()),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_1()
                                .text_size(px(11.))
                                .text_color(rgb(palette::TEXT_DIM))
                                .child(ico(IconName::Users, Size::XSmall))
                                .child(format!("{} 人", entry.members)),
                        )
                        .child(
                            h_flex()
                                .h(px(26.))
                                .px_3()
                                .gap_1()
                                .items_center()
                                .rounded(px(4.))
                                .bg(if joined {
                                    rgb(palette::BG_RAIL)
                                } else {
                                    rgb(palette::ACCENT)
                                })
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if joined {
                                    rgb(palette::TEXT_SECONDARY)
                                } else {
                                    rgb(palette::WHITE)
                                })
                                .hover(|el| el.bg(rgb(if joined {
                                    palette::BG_CHANNEL_HOVER
                                } else {
                                    palette::ACCENT_HOVER
                                })))
                                .child(ico(
                                    if joined {
                                        IconName::Check
                                    } else {
                                        IconName::Plus
                                    },
                                    Size::XSmall,
                                ))
                                .child(if joined { "已加入" } else { "加入" })
                                .id(format!("discover-join-{index}"))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.join_discover_server(index, window, cx);
                                })),
                        ),
                ),
        )
}
