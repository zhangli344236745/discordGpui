use gpui_kit::*;
use gpui_kit::component::{h_flex, v_flex, Size};

use crate::app::DiscordApp;
use crate::data::MemberGroup;
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;

/// 单个成员行：头像 + 名字 + 状态，点击打开资料卡。
/// `identity` 是身份键（找资料卡 / 菜单用），`display` 是展示名（可能是昵称）。
fn member_row(
    cx: &mut Context<DiscordApp>,
    short: char,
    color: u32,
    online: bool,
    identity: &'static str,
    display: &'static str,
    index: usize,
) -> impl IntoElement {
    let member_name = identity;
    let member_id = format!("member-{index}");
    h_flex()
        .h(px(42.))
        .px_2()
        .gap_3()
        .rounded(px(4.))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .id(member_id)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_profile(member_name);
            cx.notify();
        }))
        // 右键打开成员上下文菜单，菜单跟着光标位置展开。
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                this.open_member_menu(member_name, x, y, cx);
            }),
        )
        .child(avatar(short, color, px(32.), online))
        .child(
            v_flex()
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if online {
                            rgb(color)
                        } else {
                            rgb(palette::text_dim())
                        })
                        .child(display),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_dim()))
                        .child(if online {
                            "正在使用 GPUI Kit"
                        } else {
                            "离线"
                        }),
                ),
        )
}

/// 分组标题：折叠箭头 + 角色名 + 人数，点击切换折叠。
fn group_header(
    group: &MemberGroup,
    collapsed: bool,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let key = group.key.clone();
    let arrow = if collapsed {
        IconName::ChevronRight
    } else {
        IconName::ChevronDown
    };
    h_flex()
        .w_full()
        .px_2()
        .pt_3()
        .pb_1()
        .gap_1()
        .items_center()
        .text_color(if collapsed {
            rgb(palette::text_muted())
        } else {
            rgb(group.color)
        })
        .hover(|el| el.text_color(rgb(palette::text_primary())))
        .id(format!("member-group-{}", group.key))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_role_group(key.clone());
            cx.notify();
        }))
        .child(ico(arrow, Size::XSmall))
        .child(
            div()
                .flex_1()
                .text_size(px(11.))
                .font_weight(FontWeight::BOLD)
                .truncate()
                .child(group.title.clone()),
        )
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(group.member_indices.len().to_string()),
        )
}

/// 右侧成员栏：按角色分组，可折叠，点击成员弹出资料卡。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let groups = app.member_groups();

    let mut panel = v_flex()
        .w(px(240.))
        .h_full()
        .flex_shrink_0()
        .bg(rgb(palette::bg_sidebar()))
        .px_2()
        .pt_4()
        .gap_1()
        .id("member-scroll")
        .overflow_y_scroll();

    for group in &groups {
        let collapsed = app.collapsed_roles.iter().any(|k| k == &group.key);
        panel = panel.child(group_header(group, collapsed, cx));
        if collapsed {
            continue;
        }
        for &index in &group.member_indices {
            if let Some(member) = app.members.get(index) {
                panel = panel.child(member_row(
                    cx,
                    member.display().chars().next().unwrap_or('?'),
                    member.color,
                    member.online,
                    member.name,
                    member.display(),
                    index,
                ));
            }
        }
    }

    panel
}
