use gpui_kit::*;

use crate::app::{DiscordApp, MenuAnchor};
use crate::data::ChannelKind;
use crate::ui::theme::palette;
use crate::ui::{draft_badge, ico, unread_badge, unread_badge_muted, user_footer};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 频道侧栏：服务器标题、分类/频道列表与底部用户控制区。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let server = app.active_server_ref();
    let server_name = server.name.clone();
    // 「跟随服务器」的频道是否静音由服务器默认级别决定，标题栏铃铛要一起反映。
    let muted = server.notify.is_muted();

    let header = h_flex()
        .h(px(48.))
        .w_full()
        .flex_shrink_0()
        .px_4()
        .gap_1()
        .border_b_1()
        .border_color(rgb(palette::bg_sidebar_border()))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(
            div()
                .flex_1()
                .truncate()
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child(server_name.clone()),
        )
        .when(muted, |el| {
            el.child(
                ico(IconName::BellOff, Size::XSmall).text_color(rgb(palette::text_dim())),
            )
        })
        .child(ico(IconName::ChevronDown, Size::Small).text_color(rgb(palette::text_primary())))
        .id("sidebar-server-menu")
        .on_click(cx.listener(|this, _, _, cx| this.toggle_server_menu(cx)));

    // 加成条：等级 + 当前区间进度，点一下直达服务器设置的「加成」页。
    let boost = app.active_boost();
    let boost_level = boost.level();
    let boost_count = boost.count();
    let boost_target = boost
        .remaining()
        .map(|left| boost_count + left)
        .map(|target| format!("{boost_count}/{target}"))
        .unwrap_or_else(|| format!("{boost_count} · 满级"));
    let boost_bar = v_flex()
        .w_full()
        .flex_shrink_0()
        .px_2()
        .pt_2()
        .child(
            v_flex()
                .w_full()
                .px_2()
                .py(px(6.))
                .gap_1()
                .rounded(px(4.))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(
                                    ico(IconName::Sparkles, Size::XSmall)
                                        .text_color(rgb(palette::accent())),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(palette::text_muted()))
                                        .child(format!("加成 · 等级 {boost_level}")),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(boost_target),
                        ),
                )
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(4.))
                        .rounded(px(2.))
                        .bg(rgb(palette::bg_divider()))
                        .child(
                            div()
                                .w(relative(boost.progress().clamp(0.0, 1.0)))
                                .h_full()
                                .rounded(px(2.))
                                .bg(rgb(palette::accent())),
                        ),
                )
                .id("sidebar-boost-bar")
                .on_click(
                    cx.listener(|this, _, window, cx| this.open_boost_page(window, cx)),
                ),
        );

    let mut channels = v_flex()
        .flex_1()
        .min_h_0()
        .id("sidebar-channel-scroll")
        .overflow_y_scroll()
        .pt_2();

    for category in &server.categories {
        let key = format!("{server_name}/{}", category.name);
        let is_collapsed = app.collapsed.contains(&key);
        let click_key = key.clone();
        let category_id = format!("sidebar-category-{key}");
        let menu_category = category.name.clone();

        channels = channels.child(
            v_flex().w_full().mt_2().child(
                h_flex()
                    .h(px(24.))
                    .px_4()
                    .gap_1()
                    .text_color(rgb(palette::text_muted()))
                    .hover(|el| el.text_color(rgb(palette::text_primary())))
                    .child(ico(
                        if is_collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        },
                        Size::XSmall,
                    ))
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(category.name.to_uppercase()),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .w(px(18.))
                            .h(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .text_color(rgb(palette::text_muted()))
                            .hover(|el| {
                                el.bg(rgb(palette::bg_channel_hover()))
                                    .text_color(rgb(palette::text_primary()))
                            })
                            .child(ico(IconName::Plus, Size::XSmall))
                            .id(format!("sidebar-create-channel-{category_id}"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_create_channel(window, cx);
                            })),
                    )
                    .id(category_id)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_category(click_key.clone());
                        cx.notify();
                    }))
                    // 右键分类标题打开分类操作菜单，菜单跟着光标位置展开。
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            let x: f32 = event.position.x.into();
                            let y: f32 = event.position.y.into();
                            this.open_category_menu(
                                menu_category.clone(),
                                MenuAnchor::Point(x, y),
                            );
                            cx.notify();
                        }),
                    ),
            ),
        );

        if !is_collapsed {
            for channel in &category.channels {
                let name = channel.name.clone();
                let is_voice = channel.kind == ChannelKind::Voice;
                let voice_joined = app
                    .voice_channel
                    .as_ref()
                    .map(|(s, c)| *s == app.active_server && c == &name)
                    .unwrap_or(false);
                let selected = if is_voice {
                    voice_joined
                } else {
                    name == app.active_channel && app.view_mode != crate::app::ViewMode::DirectMessage
                };
                let icon = match channel.kind {
                    ChannelKind::Text => IconName::Hash,
                    ChannelKind::Voice => IconName::Volume2,
                    ChannelKind::Forum => IconName::MessageSquare,
                };
                let unread = channel.unread;
                let muted = channel.notify.is_muted();
                let has_draft = !is_voice && app.channel_has_draft(&name);
                let click_name = name.clone();
                let menu_name = name.clone();
                let server_index = app.active_server;
                let channel_id = format!("sidebar-channel-{}-{}", app.active_server, name);

                let mut row = h_flex()
                    .h(px(34.))
                    .mx_2()
                    .px_2()
                    .gap_1()
                    .rounded(px(4.))
                    .bg(if selected {
                        rgb(palette::bg_channel_selected())
                    } else {
                        rgb(palette::bg_sidebar())
                    })
                    .text_color(if selected || unread > 0 {
                        rgb(palette::text_primary())
                    } else {
                        rgb(palette::text_muted())
                    })
                    .hover(|el| {
                        el.bg(rgb(palette::bg_channel_hover()))
                            .text_color(rgb(palette::text_message()))
                    })
                    .child(ico(icon, Size::Small))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_size(px(14.))
                            .child(name),
                    );

                if muted {
                    row = row.child(
                        ico(IconName::BellOff, Size::XSmall).text_color(rgb(palette::text_dim())),
                    );
                }
                if unread > 0 {
                    row = row.child(if muted {
                        unread_badge_muted(unread)
                    } else {
                        unread_badge(unread)
                    });
                }
                if has_draft {
                    row = row.child(draft_badge());
                }

                channels = channels.child(
                    row.id(channel_id)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if is_voice {
                                this.toggle_voice(server_index, click_name.clone());
                                cx.notify();
                            } else {
                                this.select_channel(click_name.clone(), window, cx);
                            }
                        }))
                        // 右键打开频道操作菜单，菜单跟着光标位置展开。
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                let x: f32 = event.position.x.into();
                                let y: f32 = event.position.y.into();
                                this.open_channel_menu(
                                    menu_name.clone(),
                                    MenuAnchor::Point(x, y),
                                );
                                cx.notify();
                            }),
                        ),
                );
            }
        }
    }

    v_flex()
        .w(px(240.))
        .h_full()
        .flex_shrink_0()
        .bg(rgb(palette::bg_sidebar()))
        .child(header)
        .child(boost_bar)
        .child(channels)
        .child(user_footer::render(app, cx))
}
