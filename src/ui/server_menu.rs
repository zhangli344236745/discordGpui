//! 服务器下拉菜单：点频道侧栏头部展开。
//! 提供「邀请好友 / 静音服务器 / 标记为已读 / 创建频道 / 离开服务器」，遮罩点击关闭。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;
use crate::ui::ico;

use gpui_kit::assets::IconName;
use gpui_kit::component::{v_flex, Size};

/// 渲染服务器菜单；未打开时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    if !app.show_server_menu {
        return div().into_any_element();
    }

    let server = app.active_server_ref();
    let name = server.name.clone();
    let muted = server.muted;
    let members = app.members.len();
    let mute_label = if muted { "取消静音服务器" } else { "静音服务器" };
    let mute_icon = if muted {
        IconName::Bell
    } else {
        IconName::BellOff
    };

    let card = v_flex()
        .absolute()
        // 频道侧栏宽 240、头部高 48，菜单贴在头部下方。
        .left(px(8.))
        .top(px(44.))
        .w(px(224.))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::BG_FOOTER))
        .border_1()
        .border_color(rgb(palette::BORDER_DEFAULT))
        .shadow_lg()
        .child(
            v_flex()
                .px_2()
                .py_1()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::TEXT_PRIMARY))
                        .truncate()
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::TEXT_DIM))
                        .child(format!(
                            "{members} 名成员 · {}",
                            if muted { "已静音" } else { "通知开启" }
                        )),
                ),
        )
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::BG_DIVIDER)))
        .child(
            menu_item("服务器设置", IconName::Settings, false)
                .id("server-menu-settings")
                .on_click(cx.listener(|this, _, window, cx| this.open_server_settings(window, cx))),
        )
        .child(
            menu_item("邀请好友", IconName::UserPlus, false)
                .id("server-menu-invite")
                .on_click(cx.listener(|this, _, window, cx| this.open_invite(window, cx))),
        )
        .child(
            menu_item(mute_label, mute_icon, false)
                .id("server-menu-mute")
                .on_click(cx.listener(|this, _, _, cx| this.toggle_server_mute(cx))),
        )
        .child(
            menu_item("标记服务器已读", IconName::CheckCheck, false)
                .id("server-menu-mark-read")
                .on_click(cx.listener(|this, _, _, cx| this.mark_server_read(cx))),
        )
        .child(
            menu_item("创建频道", IconName::Plus, false)
                .id("server-menu-create-channel")
                .on_click(cx.listener(|this, _, window, cx| this.open_create_channel(window, cx))),
        )
        .child(
            menu_item("新建分类", IconName::FolderPlus, false)
                .id("server-menu-create-category")
                .on_click(cx.listener(|this, _, window, cx| this.open_create_category(window, cx))),
        )
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::BG_DIVIDER)))
        .child(
            menu_item("离开服务器", IconName::LogOut, true)
                .id("server-menu-leave")
                .on_click(cx.listener(|this, _, window, cx| this.leave_server(window, cx))),
        );

    // 透明遮罩：点菜单外任意位置关闭。
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .id("server-menu-backdrop")
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_server_menu();
            cx.notify();
        }))
        .child(card)
        .into_any_element()
}

/// 菜单项统一样式；danger 为危险操作（红色）。
fn menu_item(label: &'static str, icon: IconName, danger: bool) -> Div {
    let color = if danger {
        palette::DANGER
    } else {
        palette::TEXT_SECONDARY
    };
    div()
        .h(px(30.))
        .px_2()
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(4.))
        .text_size(px(13.))
        .text_color(rgb(color))
        .hover(move |el| {
            el.bg(rgb(palette::BG_CHANNEL_HOVER)).text_color(rgb(if danger {
                palette::DANGER
            } else {
                palette::TEXT_PRIMARY
            }))
        })
        .child(ico(icon, Size::XSmall))
        .child(label)
}
