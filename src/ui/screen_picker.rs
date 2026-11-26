//! 屏幕共享目标选择：列出可共享的显示器与窗口。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 渲染选择器；未打开时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    if !app.screen_picker_open {
        return div().into_any_element();
    }

    let displays: Vec<_> = app
        .screen_targets
        .iter()
        .filter(|target| target.kind == "display")
        .cloned()
        .collect();
    let windows: Vec<_> = app
        .screen_targets
        .iter()
        .filter(|target| target.kind == "window")
        .take(12)
        .cloned()
        .collect();

    let mut list = v_flex().gap_1();
    if !displays.is_empty() {
        list = list.child(section_label("整个屏幕"));
        for (index, target) in displays.into_iter().enumerate() {
            list = list.child(row(target, index, cx));
        }
    }
    if !windows.is_empty() {
        list = list.child(section_label("窗口"));
        for (index, target) in windows.into_iter().enumerate() {
            list = list.child(row(target, index + 100, cx));
        }
    }

    let mut card = v_flex()
        .w(px(420.))
        .max_h(px(460.))
        .gap_2()
        .p_4()
        .rounded(px(10.))
        .bg(rgb(palette::bg_chat()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("共享屏幕"),
                )
                .child(
                    div()
                        .id("screen-picker-close")
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                        .child("取消")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_screen_picker(cx))),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child("选择要共享的屏幕或应用窗口；共享期间对方可实时看到画面。"),
        );

    if let Some(note) = &app.screen_picker_note {
        card = card.child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::danger()))
                .child(note.clone()),
        );
    }

    card = card.child(list);

    v_flex()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(rgba(0x00000099))
                .id("screen-picker-backdrop")
                .on_click(cx.listener(|this, _, _, cx| this.cancel_screen_picker(cx))),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(card),
        )
        .into_any_element()
}

fn section_label(label: &'static str) -> impl IntoElement {
    div()
        .px_1()
        .pt_1()
        .text_size(px(11.))
        .text_color(rgb(palette::text_dim()))
        .child(label)
}

fn row(
    target: crate::voice::ScreenTarget,
    index: usize,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let title = if target.title.is_empty() {
        target.app.clone()
    } else {
        target.title.clone()
    };
    let subtitle = if target.kind == "display" {
        format!("{}×{}", target.width, target.height)
    } else if target.app.is_empty() {
        format!("{}×{}", target.width, target.height)
    } else {
        format!("{} · {}×{}", target.app, target.width, target.height)
    };
    let target_for_click = target.clone();

    h_flex()
        .gap_2()
        .px_2()
        .py_2()
        .rounded(px(6.))
        .bg(rgb(palette::bg_sidebar()))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(ico(
            if target.kind == "display" {
                IconName::ScreenShare
            } else {
                IconName::FileText
            },
            Size::Small,
        ))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    div()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_primary()))
                        .child(title),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(subtitle),
                ),
        )
        .id(format!("screen-target-{index}"))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.start_screen_share(target_for_click.clone(), cx)
        }))
}
