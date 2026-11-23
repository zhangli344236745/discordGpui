//! 成员上下文菜单：右键成员行打开，提供资料卡 / 私信 / 提及 / 复制名 / 屏蔽。
//! 带透明遮罩实现点击外部关闭，样式与频道菜单保持一致。

use gpui_kit::*;

use crate::app::{DiscordApp, MenuAnchor};
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 菜单项：图标 + 文案 + 可选危险色 + 点击回调由调用方补齐。
fn menu_item(label: &'static str, icon: IconName, danger: bool) -> Div {
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
            el.bg(rgb(palette::bg_channel_hover())).text_color(if danger {
                rgb(palette::danger())
            } else {
                rgb(palette::text_primary())
            })
        })
        .child(ico(icon, Size::XSmall))
        .child(label)
}

/// 渲染成员上下文菜单；没有打开菜单时返回空元素。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(menu) = app.member_menu.as_ref() else {
        return div().into_any_element();
    };

    let name = menu.name;
    // 角色已独立到 Member.role；这里取短名并兼容旧的「名字 —— 角色」展示格式。
    let short = name.split(" —— ").next().unwrap_or(name);
    let role = app.member_menu_role();
    let member = app.members.iter().find(|item| item.name == name);
    let color = member.map(|item| item.color).unwrap_or(palette::accent());
    let online = member.map(|item| item.online).unwrap_or(false);
    let blocked = app.member_menu_blocked();

    let (left, top) = match menu.anchor {
        MenuAnchor::Point(x, y) => (x, y),
        // 目前只有右键入口；Header 分支仅为穷尽枚举。
        MenuAnchor::Header => (240., 96.),
    };

    let header = h_flex()
        .px_2()
        .py_1()
        .gap_2()
        .items_center()
        .child(avatar(short.chars().next().unwrap_or('?'), color, px(28.), online))
        .child(
            v_flex()
                .min_w_0()
                .gap(px(1.))
                .child(
                    div()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(short),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(match role {
                            Some(role) => role.to_string(),
                            None if online => "在线".to_string(),
                            None => "离线".to_string(),
                        }),
                ),
        );

    let card = v_flex()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(196.))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(header)
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::bg_divider())))
        .child(
            menu_item("查看资料卡", IconName::User, false)
                .id("member-menu-profile")
                .on_click(cx.listener(|this, _, _, cx| this.member_open_profile(cx))),
        )
        .child(
            menu_item("发送私信", IconName::MessageSquare, false)
                .id("member-menu-dm")
                .on_click(cx.listener(|this, _, window, cx| this.member_send_dm(window, cx))),
        )
        .child(
            menu_item("在输入框提及", IconName::AtSign, false)
                .id("member-menu-mention")
                .on_click(cx.listener(|this, _, window, cx| this.member_mention(window, cx))),
        )
        .child(
            menu_item("复制用户名", IconName::Copy, false)
                .id("member-menu-copy")
                .on_click(cx.listener(|this, _, _, cx| this.member_copy_name(cx))),
        )
        .child(div().h(px(1.)).my(px(2.)).bg(rgb(palette::bg_divider())))
        .child(
            menu_item(
                if blocked { "取消屏蔽" } else { "屏蔽成员" },
                if blocked {
                    IconName::UserCheck
                } else {
                    IconName::UserX
                },
                true,
            )
            .id("member-menu-block")
            .on_click(cx.listener(|this, _, _, cx| this.member_toggle_block(cx))),
        );

    // 透明遮罩：点菜单外任意位置关闭。
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .id("member-menu-backdrop")
        .on_click(cx.listener(|this, _, _, cx| this.close_member_menu(cx)))
        .child(card)
        .into_any_element()
}
