//! 公共 UI 组件库：各页面复用的原子控件与行组件。
//! 只负责渲染与事件转发，不持有业务状态；交互回调由调用方传入。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

use super::ico;
use super::theme::palette;
use crate::app::DiscordApp;

/// 业务点击回调：忽略鼠标事件细节，只把 view / window / cx 交给调用方。
pub type ClickHandler = dyn Fn(&mut DiscordApp, &mut Window, &mut Context<DiscordApp>) + 'static;

/// 圆形头像，可选在线状态小圆点（默认绿色）。
pub fn avatar(letter: char, color: u32, size: Pixels, online: bool) -> Div {
    avatar_status(
        letter,
        color,
        size,
        if online {
            Some(palette::online())
        } else {
            None
        },
    )
}

/// 圆形头像，状态点颜色可自定义（用于在线/闲置/勿扰/隐身）。
pub fn avatar_status(letter: char, color: u32, size: Pixels, status_color: Option<u32>) -> Div {
    let diameter = size.as_f32();
    div()
        .relative()
        .w(size)
        .h(size)
        .flex_shrink_0()
        .rounded(px(diameter / 2.))
        .bg(rgb(color))
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(palette::WHITE))
        .text_size(px((diameter * 0.45).max(10.)))
        .font_weight(FontWeight::SEMIBOLD)
        .child(letter.to_string())
        .when(status_color.is_some(), |el| {
            el.child(
                div()
                    .absolute()
                    .right(px(-4.))
                    .bottom(px(-4.))
                    .w(px(12.))
                    .h(px(12.))
                    .rounded_full()
                    .bg(rgb(status_color.unwrap_or(palette::online()))),
            )
        })
}

/// 会话草稿标记：侧栏里提示该会话有未发送内容。
pub fn draft_badge() -> Div {
    div()
        .px_1()
        .rounded(px(3.))
        .bg(rgb(palette::bg_toolbar_hover()))
        .text_color(rgb(palette::accent()))
        .text_size(px(9.))
        .font_weight(FontWeight::MEDIUM)
        .child("草稿")
}

/// 频道未读消息徽标。
pub fn unread_badge(count: usize) -> Div {
    div()
        .min_w(px(16.))
        .h(px(16.))
        .px_1()
        .rounded_full()
        .bg(rgb(palette::danger()))
        .text_color(rgb(palette::WHITE))
        .text_size(px(10.))
        .font_weight(FontWeight::BOLD)
        .flex()
        .items_center()
        .justify_center()
        .child(count.to_string())
}

/// 分组小标题：侧栏 / 设置页通用的加粗小字，padding 交给调用方外层容器。
pub fn section_label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(palette::text_muted()))
        .child(text.into())
}

/// 幽灵图标按钮：24×24 / 圆角 4 / 默认次要色、active 时主要色 / hover 工具栏底色。
/// 返回 Stateful<Div>，调用方可继续 .when(...) 追加徽标等子元素。
pub fn icon_button(
    icon: IconName,
    icon_size: Size,
    side: Pixels,
    id: impl Into<ElementId>,
    active: bool,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    div()
        .w(side)
        .h(side)
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_color(if active {
            rgb(palette::text_primary())
        } else {
            rgb(palette::text_secondary())
        })
        .hover(|el| {
            el.bg(rgb(palette::bg_toolbar_hover()))
                .text_color(rgb(palette::text_primary()))
        })
        .child(ico(icon, icon_size))
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}

/// 危险操作图标按钮：hover 变 danger 底色 + 白字（拒绝请求、拉黑、移除等）。
pub fn danger_icon_button(
    icon: IconName,
    icon_size: Size,
    side: Pixels,
    id: impl Into<ElementId>,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    div()
        .w(side)
        .h(side)
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_color(rgb(palette::text_dim()))
        .hover(|el| el.bg(rgb(palette::danger())).text_color(rgb(palette::WHITE)))
        .child(ico(icon, icon_size))
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}

/// X 关闭按钮：弹层 / 浮层右上角通用（28×28 幽灵按钮）。
pub fn close_button(
    id: impl Into<ElementId>,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    div()
        .w(px(28.))
        .h(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_color(rgb(palette::text_muted()))
        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
        .child(ico(IconName::X, Size::Small))
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}

/// 一行开关：左侧标题 + 描述、右侧拨动条，点击整行生效（设置页通用）。
pub fn toggle_row(
    label: impl Into<SharedString>,
    desc: impl Into<SharedString>,
    on: bool,
    id: impl Into<ElementId>,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    let mut track = h_flex()
        .w(px(40.))
        .h(px(22.))
        .flex_shrink_0()
        .items_center()
        .px(px(2.))
        .rounded_full()
        .bg(if on {
            rgb(palette::accent())
        } else {
            rgb(palette::bg_toolbar_hover())
        });
    if on {
        track = track.justify_end();
    }
    let knob = div()
        .w(px(18.))
        .h(px(18.))
        .rounded_full()
        .bg(rgb(palette::WHITE));

    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .px(px(16.))
        .py(px(12.))
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .child(
            v_flex()
                .gap(px(4.))
                .min_w_0()
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(palette::text_primary()))
                        .child(label.into()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_muted()))
                        .child(desc.into()),
                ),
        )
        .child(track.child(knob))
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}

/// 文字胶囊筛选按钮：搜索 / 探索页的分类切换（选中强调色，未选中次要底色）。
pub fn filter_chip(
    label: impl Into<SharedString>,
    active: bool,
    id: impl Into<ElementId>,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    div()
        .px_3()
        .h(px(26.))
        .flex()
        .items_center()
        .rounded_full()
        .text_size(px(12.))
        .bg(if active {
            rgb(palette::accent())
        } else {
            rgb(palette::bg_sidebar())
        })
        .text_color(if active {
            rgb(palette::WHITE)
        } else {
            rgb(palette::text_secondary())
        })
        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
        .child(label.into())
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}

/// 浮层菜单里的分组分隔线（1px，上下各留 2px）。
pub fn menu_divider() -> Div {
    div().h(px(1.)).my(px(2.)).bg(rgb(palette::bg_divider()))
}

/// 浮层菜单行：左侧图标 + 文案 + 可选右侧补充（角标 / 当前值），整行可点。
/// 菜单类浮层（频道、分类、服务器、消息）统一用它，避免每处复制一份行样式。
pub fn menu_item(
    icon: IconName,
    label: impl Into<SharedString>,
    trailing: Option<AnyElement>,
    id: impl Into<ElementId>,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    let mut row = h_flex()
        .h(px(30.))
        .px_2()
        .gap_2()
        .rounded(px(4.))
        .text_size(px(13.))
        .text_color(rgb(palette::text_secondary()))
        .hover(|el| {
            el.bg(rgb(palette::bg_channel_hover()))
                .text_color(rgb(palette::text_primary()))
        })
        .child(ico(icon, Size::XSmall))
        .child(div().flex_1().truncate().child(label.into()));
    if let Some(trailing) = trailing {
        row = row.child(trailing);
    }
    row.id(id).on_click(cx.listener(move |this, _, window, cx| {
        handler(this, window, cx);
        cx.notify();
    }))
}

/// 单选行：左侧圆环指示选中，标题 + 说明两行文案，用于通知级别等互斥选项。
pub fn radio_item(
    title: impl Into<SharedString>,
    hint: impl Into<SharedString>,
    selected: bool,
    id: impl Into<ElementId>,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    // 外圈 16px 圆环，选中时描边换成强调色并填充中心圆点。
    let mut ring = div()
        .w(px(16.))
        .h(px(16.))
        .flex_shrink_0()
        .rounded_full()
        .border_2()
        .border_color(rgb(if selected {
            palette::accent()
        } else {
            palette::text_dim()
        }));
    if selected {
        ring = ring.child(
            div()
                .size(px(8.))
                .m_auto()
                .rounded_full()
                .bg(rgb(palette::accent())),
        );
    }

    h_flex()
        .h(px(48.))
        .px_2()
        .gap_2()
        .rounded(px(4.))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(ring)
        .child(
            v_flex()
                .flex_1()
                .gap(px(1.))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_primary()))
                        .child(title.into()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .truncate()
                        .child(hint.into()),
                ),
        )
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}

/// 静音频道的未读角标：同尺寸但灰底，表示「有未读但不提醒」。
pub fn unread_badge_muted(count: usize) -> Div {
    div()
        .min_w(px(16.))
        .h(px(16.))
        .px_1()
        .rounded_full()
        .bg(rgb(palette::text_dim()))
        .text_color(rgb(palette::bg_sidebar()))
        .text_size(px(10.))
        .font_weight(FontWeight::BOLD)
        .flex()
        .items_center()
        .justify_center()
        .child(count.to_string())
}

/// 行内小图标按钮：以**点击位置**为锚点打开浮层（右键菜单之外的一种轻量入口）。
///
/// 与 `icon_button` 的区别是回调里额外拿到窗口坐标，调用方可以直接把菜单锚到光标处，
/// 不必再用固定偏移猜位置。放在行内时记得把它做成主点击区域的**兄弟节点**，
/// 否则点击会一路冒泡到整行的 on_click 上。
pub fn point_icon_button(
    icon: IconName,
    id: impl Into<ElementId>,
    handler: Box<dyn Fn(&mut DiscordApp, f32, f32, &mut Context<DiscordApp>) + 'static>,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    h_flex()
        .size(px(28.))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_color(rgb(palette::text_dim()))
        .hover(|el| {
            el.bg(rgb(palette::bg_channel_hover()))
                .text_color(rgb(palette::text_primary()))
        })
        .child(ico(icon, Size::XSmall))
        .id(id)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                handler(this, x, y, cx);
                cx.notify();
            }),
        )
}
