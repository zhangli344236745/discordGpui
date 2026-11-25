//! 消息操作菜单：省略号按钮打开时贴在消息行内，右键消息时跟随光标浮在根层级。
//! 两个入口共用同一份操作项（回应 / 回复 / 编辑 / 置顶 / 复制 / 删除）。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};

use crate::data::DeliveryStatus;
use gpui_kit::prelude::FluentBuilder as _;

/// 菜单项：图标 + 文案 + 唯一 id + 点击行为。
fn menu_row(
    icon: IconName,
    label: impl Into<SharedString>,
    id: String,
    danger: bool,
    cx: &mut Context<DiscordApp>,
    handler: impl Fn(&mut DiscordApp, &mut Window, &mut Context<DiscordApp>) + 'static,
) -> impl IntoElement {
    div()
        .h(px(30.))
        .px_2()
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(4.))
        .text_size(px(13.))
        .text_color(if danger {
            rgb(palette::danger())
        } else {
            rgb(palette::text_secondary())
        })
        .hover(move |el| {
            el.bg(rgb(palette::bg_channel_hover()))
                .text_color(if danger {
                    rgb(palette::danger())
                } else {
                    rgb(palette::text_primary())
                })
        })
        .child(ico(icon, Size::XSmall))
        .child(label.into())
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| handler(this, window, cx)))
}

/// 操作项集合；`index` 是当前会话里的消息下标。
fn menu_items(index: usize, app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let pinned = app
        .current_messages()
        .get(index)
        .map(|m| m.pinned)
        .unwrap_or(false);
    let thread_count = app.thread_count(index);
    let bookmarked = app.is_bookmarked(index);

    v_flex()
        .gap(px(2.))
        .child(menu_row(
            IconName::FaceSlightlySmiling,
            "添加回应",
            format!("msg-menu-react-{index}"),
            false,
            cx,
            move |this, _, cx| {
                this.toggle_emoji_picker(Some(index));
                this.close_message_menu();
                cx.notify();
            },
        ))
        .child(menu_row(
            IconName::Reply,
            "回复",
            format!("msg-menu-reply-{index}"),
            false,
            cx,
            move |this, _, cx| {
                this.toggle_reply_at(index, cx);
                this.close_message_menu();
            },
        ))
        .child(menu_row(
            if thread_count > 0 {
                IconName::MessageSquare
            } else {
                IconName::MessageSquarePlus
            },
            if thread_count > 0 { "查看话题" } else { "创建话题" },
            format!("msg-menu-thread-{index}"),
            false,
            cx,
            move |this, window, cx| {
                this.close_message_menu();
                this.toggle_thread(index, window, cx);
            },
        ))
        .child(menu_row(
            IconName::Forward,
            "转发消息",
            format!("msg-menu-forward-{index}"),
            false,
            cx,
            move |this, window, cx| this.open_forward(index, window, cx),
        ))
        .when(
            app.current_messages()
                .get(index)
                .is_some_and(|message| message.delivery == DeliveryStatus::Failed),
            |el| {
                el.child(menu_row(
                    IconName::RefreshCw,
                    "重试发送",
                    format!("msg-menu-retry-{index}"),
                    false,
                    cx,
                    move |this, window, cx| this.retry_failed_message(index, window, cx),
                ))
            },
        )
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::bg_divider())))
        .when(app.is_mine(index), |el| {
            el.child(menu_row(
                IconName::SquarePen,
                "编辑消息",
                format!("msg-menu-edit-{index}"),
                false,
                cx,
                move |this, window, cx| this.start_edit(index, window, cx),
            ))
        })
        .when(
            app.current_messages()
                .get(index)
                .is_some_and(|message| message.is_edited()),
            |el| {
                el.child(menu_row(
                    IconName::Clock,
                    "编辑历史",
                    format!("msg-menu-history-{index}"),
                    false,
                    cx,
                    move |this, _, cx| this.open_edit_history_from_menu(index, cx),
                ))
            },
        )
        .child(menu_row(
            if pinned {
                IconName::PinOff
            } else {
                IconName::Pin
            },
            if pinned { "取消置顶" } else { "置顶消息" },
            format!("msg-menu-pin-{index}"),
            false,
            cx,
            move |this, _, cx| this.toggle_pin(index, cx),
        ))
        .child(menu_row(
            IconName::Copy,
            "复制文本",
            format!("msg-menu-copy-{index}"),
            false,
            cx,
            move |this, _, cx| this.copy_message(index, cx),
        ))
        .child(menu_row(
            if bookmarked {
                IconName::BookmarkMinus
            } else {
                IconName::BookmarkPlus
            },
            if bookmarked { "取消保存" } else { "保存消息" },
            format!("msg-menu-save-{index}"),
            false,
            cx,
            move |this, _, cx| this.toggle_bookmark(index, cx),
        ))
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::bg_divider())))
        .child(menu_row(
            IconName::Trash,
            "删除消息",
            format!("msg-menu-delete-{index}"),
            true,
            cx,
            move |this, _, cx| this.delete_message(index, cx),
        ))
}

/// 行内菜单：由消息工具栏的省略号按钮触发，贴在消息右上角。
pub fn render(index: usize, app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    v_flex()
        .absolute()
        .right(px(12.))
        .top(px(8.))
        .w(px(176.))
        .p_1()
        .rounded(px(6.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(menu_items(index, app, cx))
}

/// 光标浮层：右键消息触发，带透明遮罩，点外部或再次右键即关闭。
pub fn render_floating(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let (Some(index), Some((x, y))) = (app.menu_message, app.menu_message_point) else {
        return div().into_any_element();
    };

    // 靠近窗口右/下边缘时往回收，避免菜单被裁掉。
    let left = (x - 8.).min(1264.);
    let top = (y - 8.).min(768.);

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .id("msg-menu-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_message_menu();
            cx.notify();
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, _, _, cx| {
                this.close_message_menu();
                cx.notify();
            }),
        )
        .child(
            v_flex()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(176.))
                .p_1()
                .rounded(px(6.))
                .bg(rgb(palette::bg_footer()))
                .border_1()
                .border_color(rgb(palette::border_default()))
                .shadow_lg()
                .child(menu_items(index, app, cx)),
        )
        .into_any_element()
}
