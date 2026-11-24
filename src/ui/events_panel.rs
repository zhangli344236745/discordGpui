//! 服务器日程面板：聊天区右侧浮层（与置顶/收件箱互斥）。
//! 顶部状态筛选 + 创建活动表单 + 活动卡片（报名/取消、删除）。
//! 数据全部本地演示，状态由相对分钟数推算，随消息快照一起持久化。

use gpui_kit::*;

use crate::app::{DiscordApp, EventFilter};
use crate::data::{EventCategory, ServerEvent, ServerEventStatus};
use crate::ui::theme::palette;
use crate::ui::{filter_chip, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

/// 状态对应的徽标配色。
fn status_color(status: ServerEventStatus) -> u32 {
    match status {
        ServerEventStatus::Upcoming => palette::accent(),
        ServerEventStatus::Live => palette::success(),
        ServerEventStatus::Ended => palette::text_dim(),
    }
}

/// 步进器：减号 / 值 / 加号，用于调整开始偏移与时长。
fn stepper(
    label: &str,
    value: String,
    minus_id: &'static str,
    plus_id: &'static str,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .w(px(64.))
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child(label.to_string()),
        )
        .child(
            div()
                .w(px(22.))
                .h(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .bg(rgb(palette::bg_composer()))
                .text_color(rgb(palette::text_secondary()))
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(ico(IconName::Minus, Size::XSmall))
                .id(minus_id)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if minus_id == "event-start-minus" {
                        this.adjust_event_start(-15, cx);
                    } else {
                        this.adjust_event_duration(-15, cx);
                    }
                })),
        )
        .child(
            div()
                .w(px(64.))
                .text_align(TextAlign::Center)
                .text_size(px(11.5))
                .text_color(rgb(palette::text_primary()))
                .child(value),
        )
        .child(
            div()
                .w(px(22.))
                .h(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .bg(rgb(palette::bg_composer()))
                .text_color(rgb(palette::text_secondary()))
                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                .child(ico(IconName::Plus, Size::XSmall))
                .id(plus_id)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if plus_id == "event-start-plus" {
                        this.adjust_event_start(15, cx);
                    } else {
                        this.adjust_event_duration(15, cx);
                    }
                })),
        )
}

/// 创建活动表单（面板内嵌展开区）。
fn create_form(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let title_empty = app.event_title_edit.read(cx).value().trim().is_empty();
    let mut categories = h_flex().gap_1().flex_wrap();
    for category in EventCategory::ALL {
        let active = app.event_category == category;
        categories = categories.child(
            div()
                .h(px(24.))
                .px_2()
                .flex()
                .items_center()
                .rounded(px(4.))
                .bg(if active {
                    rgb(category.color())
                } else {
                    rgb(palette::bg_composer())
                })
                .text_color(if active {
                    rgb(palette::WHITE)
                } else {
                    rgb(palette::text_secondary())
                })
                .text_size(px(11.))
                .child(category.label())
                .id(format!("event-category-{}", category.label()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_event_category(category, cx);
                })),
        );
    }

    let start_label = if app.event_starts_in <= 0 {
        format!("{} 分钟后", -app.event_starts_in)
    } else {
        format!("{} 分钟后", app.event_starts_in)
    };

    v_flex()
        .m_2()
        .p_3()
        .gap_2()
        .rounded(px(8.))
        .bg(rgb(palette::bg_rail()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .child(
            div()
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.event_title_edit), Size::Small)
                        .flex_1(),
                ),
        )
        .child(
            div()
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .child(
                    <Input as Sizable>::with_size(Input::new(&app.event_desc_edit), Size::Small)
                        .flex_1(),
                ),
        )
        .child(categories)
        .child(stepper(
            "开始时间",
            start_label,
            "event-start-minus",
            "event-start-plus",
            cx,
        ))
        .child(stepper(
            "持续时间",
            format!("{} 分钟", app.event_duration),
            "event-duration-minus",
            "event-duration-plus",
            cx,
        ))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_muted()))
                        .child("落地频道"),
                )
                .child(
                    div()
                        .px_2()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .gap_1()
                        .rounded(px(4.))
                        .bg(rgb(palette::bg_composer()))
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_primary()))
                        .child(app.event_channel.clone())
                        .child(ico(IconName::ChevronDown, Size::XSmall))
                        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                        .id("event-cycle-channel")
                        .on_click(cx.listener(|this, _, _, cx| {
                            let next = match this.event_channel.as_str() {
                                "一般" => "闲聊",
                                "闲聊" => "技术闲聊",
                                _ => "一般",
                            };
                            this.set_event_channel(next.to_string(), cx);
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .text_right()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_dim()))
                        .child("Enter 创建"),
                ),
        )
        .child(
            div()
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .bg(if title_empty {
                    rgb(palette::bg_composer())
                } else {
                    rgb(palette::accent())
                })
                .text_color(if title_empty {
                    rgb(palette::text_dim())
                } else {
                    rgb(palette::WHITE)
                })
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .child("创建活动")
                .id("event-create-submit")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.create_event(window, cx);
                })),
        )
}

/// 单张活动卡片。
fn event_card(
    event: &ServerEvent,
    selected: bool,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let id = event.id.clone();
    let id_for_join = id.clone();
    let id_for_delete = id.clone();
    let status = event.status();
    let attending = event.is_attending("你");

    v_flex()
        .m_2()
        .p_3()
        .gap_2()
        .rounded(px(8.))
        .bg(if selected {
            rgb(palette::bg_rail())
        } else {
            rgb(palette::bg_sidebar())
        })
        .border_1()
        .border_color(if selected {
            rgb(palette::accent())
        } else {
            rgb(palette::border_default())
        })
        .id(format!("event-card-{}", event.id))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_event(id.clone(), cx);
        }))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                // 分类色点。
                .child(
                    div()
                        .w(px(10.))
                        .h(px(10.))
                        .rounded_full()
                        .bg(rgb(event.category.color())),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(event.title.clone()),
                )
                .child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded_full()
                        .bg(rgb(status_color(status)))
                        .text_color(rgb(palette::WHITE))
                        .text_size(px(10.))
                        .child(status.label()),
                ),
        )
        .child(
            h_flex()
                .gap_3()
                .text_size(px(10.5))
                .text_color(rgb(palette::text_dim()))
                .child(
                    h_flex()
                        .gap_1()
                        .child(ico(IconName::Clock, Size::XSmall))
                        .child(event.time_label()),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .child(ico(IconName::Hash, Size::XSmall))
                        .child(event.channel.clone()),
                ),
        )
        .when(!event.description.is_empty(), |el| {
            el.child(
                div()
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(rgb(palette::text_secondary()))
                    .child(event.description.clone()),
            )
        })
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    h_flex()
                        .gap_1()
                        .text_size(px(10.5))
                        .text_color(rgb(palette::text_muted()))
                        .child(ico(IconName::Users, Size::XSmall))
                        .child(format!(
                            "{} 人报名{}{}",
                            event.attendee_count(),
                            if attending { " · 你已报名" } else { "" },
                            if event.host == "你" { " · 你发起" } else { "" },
                        )),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(if attending {
                            rgb(palette::bg_composer())
                        } else {
                            rgb(palette::accent())
                        })
                        .text_color(if attending {
                            rgb(palette::text_secondary())
                        } else {
                            rgb(palette::WHITE)
                        })
                        .text_size(px(11.))
                        .child(if attending { "取消报名" } else { "报名" })
                        .id(format!("event-join-{}", event.id))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_attendance(id_for_join.clone(), cx);
                        })),
                )
                .when(status != ServerEventStatus::Ended, |el| {
                    el.child(
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
                            .id(format!("event-delete-{}", event.id))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_event(id_for_delete.clone(), cx);
                            })),
                    )
                }),
        )
}

/// 日程面板主体；由 render_main 决定是否挂载。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let all = app.server_events();
    let upcoming = all
        .iter()
        .filter(|event| event.status() == ServerEventStatus::Upcoming)
        .count();
    let live = all
        .iter()
        .filter(|event| event.status() == ServerEventStatus::Live)
        .count();
    let ended = all.len() - upcoming - live;

    let mut filters = h_flex().gap_1().flex_wrap();
    for filter in [EventFilter::All, EventFilter::Upcoming, EventFilter::Live, EventFilter::Ended] {
        let count = match filter {
            EventFilter::All => all.len(),
            EventFilter::Upcoming => upcoming,
            EventFilter::Live => live,
            EventFilter::Ended => ended,
        };
        let label = format!("{} {}", filter.label(), count);
        filters = filters.child(filter_chip(
            label,
            app.event_filter == filter,
            format!("event-filter-{}", filter.label()),
            Box::new(move |this, _, cx| {
                this.set_event_filter(filter, cx);
            }),
            cx,
        ));
    }

    let visible = app.visible_events();
    let mut body = v_flex().flex_1().min_h_0().id("events-scroll").overflow_y_scroll();

    if let Some(notice) = &app.event_notice {
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
                        .id("event-dismiss-notice")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dismiss_event_notice(cx);
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
                .child(ico(IconName::CalendarDays, Size::Large))
                .child(
                    div()
                        .text_size(px(13.))
                        .child("这个筛选下还没有活动"),
                ),
        );
    }
    for event in &visible {
        let selected = app.selected_event.as_deref() == Some(event.id.as_str());
        body = body.child(event_card(event, selected, cx));
    }

    div()
        .absolute()
        .right(px(0.))
        .top(px(0.))
        .bottom(px(0.))
        .w(px(380.))
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
                        .child(format!("服务器日程 — {}", all.len())),
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
                        .id("events-close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_events(cx);
                        })),
                ),
        )
        .child(
            h_flex()
                .m_2()
                .mb_1()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .h(px(26.))
                        .px_2()
                        .flex()
                        .items_center()
                        .gap_1()
                        .rounded(px(4.))
                        .bg(if app.show_event_create {
                            rgb(palette::accent())
                        } else {
                            rgb(palette::bg_composer())
                        })
                        .text_color(if app.show_event_create {
                            rgb(palette::WHITE)
                        } else {
                            rgb(palette::text_secondary())
                        })
                        .text_size(px(11.))
                        .child(ico(IconName::Plus, Size::XSmall))
                        .child("新建活动")
                        .id("event-toggle-create")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_event_create(window, cx);
                        })),
                )
                .child(filters),
        )
        .when(app.show_event_create, |el| el.child(create_form(app, cx)))
        .child(body)
}
