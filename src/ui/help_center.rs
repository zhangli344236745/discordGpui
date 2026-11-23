//! 帮助中心：标题栏 ? 打开，左侧分类导航 + 关键词搜索 + 文档列表 / 详情。
//!
//! 文档内容来自 `data::seed_help_articles`，全部是本地静态文案，不联网。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::{close_button, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 左侧分类项：全部（category = None）与五个分类。
fn nav_item(
    label: &'static str,
    category: Option<&'static str>,
    active: bool,
    count: usize,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    h_flex()
        .h(px(30.))
        .px_2()
        .gap_1()
        .rounded(px(4.))
        .items_center()
        .justify_between()
        .text_size(px(12.5))
        .bg(rgb(if active {
            palette::bg_channel_selected()
        } else {
            palette::bg_chat()
        }))
        .text_color(rgb(if active {
            palette::text_primary()
        } else {
            palette::text_secondary()
        }))
        .when(!active, |el| {
            el.hover(|el| {
                el.bg(rgb(palette::bg_channel_hover()))
                    .text_color(rgb(palette::text_primary()))
            })
        })
        .child(div().child(label))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child(count.to_string()),
        )
        .id(format!("help-cat-{label}"))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_help_category(category, cx);
        }))
}

/// 列表里的一行文档：分类标签 + 标题 + 摘要。
fn article_row(
    article: crate::data::HelpArticle,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    v_flex()
        .gap_1()
        .p_3()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(palette::bg_header_border()))
        .hover(|el| {
            el.bg(rgb(palette::bg_channel_hover()))
                .border_color(rgb(palette::border_default()))
        })
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(3.))
                        .bg(rgb(palette::bg_sidebar()))
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_muted()))
                        .child(article.category),
                )
                .child(
                    div()
                        .text_size(px(13.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(article.title),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::text_muted()))
                .child(article.summary),
        )
        .id(format!("help-article-{}", article.id))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.open_help_article(article.id, cx);
        }))
}

/// 文档详情：返回、标题、摘要、正文段落与要点。
fn article_detail(
    article: crate::data::HelpArticle,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let mut body = v_flex().gap_3();
    for paragraph in article.body {
        body = body.child(
            div()
                .text_size(px(13.))
                .text_color(rgb(palette::text_message()))
                .child(*paragraph),
        );
    }

    let mut tips = v_flex().gap_1();
    for tip in article.tips {
        tips = tips.child(
            h_flex()
                .gap_2()
                .items_start()
                .child(
                    div()
                        .mt(px(6.))
                        .w(px(4.))
                        .h(px(4.))
                        .rounded_full()
                        .flex_shrink_0()
                        .bg(rgb(palette::accent())),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(*tip),
                ),
        );
    }

    v_flex()
        .gap_3()
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .text_size(px(11.5))
                .text_color(rgb(palette::text_link()))
                .child(ico(IconName::ChevronLeft, Size::XSmall))
                .child("返回列表")
                .id("help-back")
                .on_click(cx.listener(|this, _, _, cx| this.close_help_article(cx))),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(3.))
                        .bg(rgb(palette::bg_sidebar()))
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_muted()))
                        .child(article.category),
                )
                .child(
                    div()
                        .text_size(px(18.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(article.title),
                ),
        )
        .child(
            div()
                .text_size(px(12.5))
                .text_color(rgb(palette::text_muted()))
                .child(article.summary),
        )
        .child(div().h(px(1.)).w_full().bg(rgb(palette::bg_divider())))
        .child(body)
        .when(!article.tips.is_empty(), |el| {
            el.child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(palette::text_primary()))
                    .child("要点"),
            )
            .child(tips)
        })
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_dim()))
                .child("内容是本地演示文档，与当前已实现的功能保持一致。"),
        )
}

/// 帮助中心浮层：大卡片 + 左侧分类 + 右侧搜索与文档。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let matches = app.help_matches();
    let match_count = matches.len();
    let query = app.help_query.trim().to_string();

    // 左侧分类导航。
    let mut nav = v_flex()
        .w(px(172.))
        .h_full()
        .flex_shrink_0()
        .p_2()
        .gap_1()
        .border_r_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(
            div()
                .px_2()
                .pb_1()
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_muted()))
                .child("分类"),
        )
        .child(nav_item(
            "全部",
            None,
            app.help_category.is_none(),
            app.help_category_count(None),
            cx,
        ));
    for category in app.help_categories.clone() {
        nav = nav.child(nav_item(
            category,
            Some(category),
            app.help_category == Some(category),
            app.help_category_count(Some(category)),
            cx,
        ));
    }

    // 右侧内容：有选中文档就看详情，否则看列表。
    let detail = app
        .help_article
        .and_then(|id| app.help_article_ref(id));

    let content: AnyElement = if let Some(article) = detail {
        article_detail(article, cx).into_any_element()
    } else if matches.is_empty() {
        v_flex()
            .items_center()
            .justify_center()
            .gap_2()
            .py_10()
            .child(ico(IconName::FileQuestionMark, Size::Large).text_color(rgb(palette::text_dim())))
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(palette::text_secondary()))
                    .child("没有匹配的帮助主题"),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(rgb(palette::text_muted()))
                    .child("换个关键词，或点左侧「全部」看完整列表"),
            )
            .into_any_element()
    } else {
        let mut list = v_flex().gap_2();
        for article in &matches {
            list = list.child(article_row(*article, cx));
        }
        list.into_any_element()
    };

    let content_col = v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .child(
            v_flex()
                .px_3()
                .pt_3()
                .pb_2()
                .gap_1()
                .child(
                    div()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_composer()))
                        .child(
                            <Input as Sizable>::with_size(
                                Input::new(&app.help_input),
                                Size::Large,
                            )
                            .w_full(),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_muted()))
                        .child(if query.is_empty() {
                            format!("共 {match_count} 篇文档")
                        } else {
                            format!("「{query}」命中 {match_count} 篇")
                        })
                        .when(app.help_category.is_some(), |el| {
                            el.child("· 切换关键词会自动回到全部分类")
                        }),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .px_3()
                .pb_3()
                .id("help-body")
                .overflow_y_scroll()
                .child(content),
        );

    // 遮罩与卡片做成兄弟节点：点卡片内部不会冒泡到遮罩，避免误关。
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(rgba(0x00000099))
                .id("help-center-backdrop")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.close_help_center(cx);
                })),
        )
        .child(
            v_flex()
                .w(px(760.))
                .h(px(520.))
                .rounded(px(10.))
                .overflow_hidden()
                .bg(rgb(palette::bg_chat()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .id("help-center-card")
                .child(
                    h_flex()
                        .h(px(52.))
                        .flex_shrink_0()
                        .px_4()
                        .gap_2()
                        .items_center()
                        .border_b_1()
                        .border_color(rgb(palette::bg_header_border()))
                        .child(
                            div()
                                .w(px(30.))
                                .h(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(8.))
                                .bg(rgb(palette::bg_channel_hover()))
                                .child(ico(IconName::CircleQuestionMark, Size::Small)),
                        )
                        .child(
                            v_flex()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .text_size(px(15.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(palette::text_primary()))
                                        .child("帮助中心"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette::text_muted()))
                                        .child("按功能查用法，或直接搜索关键词"),
                                ),
                        )
                        .child(div().flex_1())
                        .child(close_button(
                            "help-center-close",
                            Box::new(|this, _, cx| this.close_help_center(cx)),
                            cx,
                        )),
                )
                .child(h_flex().flex_1().min_h_0().child(nav).child(content_col)),
        )
}
