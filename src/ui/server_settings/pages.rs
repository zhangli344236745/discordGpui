//! 服务器设置各分类内容：只渲染状态与调用业务方法，不持有自身状态。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

use crate::app::{DiscordApp, MemberFilter, ROLE_COLORS, ServerSettingsSection};
use crate::data::{
    AuditKind, AutoModAction, InviteExpiry, PermissionGroup, PermissionState, BOOST_TIERS,
    PERMISSIONS,
};
use crate::ui::theme::palette;
use crate::ui::{avatar, danger_icon_button, filter_chip, ico, icon_button};

/// 可选的服务器图标颜色。
const SERVER_COLORS: [u32; 6] = [
    0x5865f2, 0xeb459e, 0x57f287, 0xfaa61c, 0xf23f43, 0x00a8fc,
];

/// 分类内容分发。各页面函数统一返回 Div，避免分支类型不一致。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    match app.server_settings_section {
        ServerSettingsSection::Overview => overview(app, cx),
        ServerSettingsSection::Roles => roles(app, cx),
        ServerSettingsSection::Permissions => permissions(app, cx),
        ServerSettingsSection::AutoMod => automod(app, cx),
        ServerSettingsSection::Emoji => emoji(app, cx),
        ServerSettingsSection::Invites => invites(app, cx),
        ServerSettingsSection::Members => members(app, cx),
        ServerSettingsSection::Commands => commands(app, cx),
        ServerSettingsSection::Integrations => integrations(app, cx),
        ServerSettingsSection::Audit => audit(app, cx),
        ServerSettingsSection::Boost => boost(app, cx),
    }
}

// ---------------------------------------------------------------------------
// 公共片段
// ---------------------------------------------------------------------------

fn shell(title: &'static str, desc: &'static str) -> Div {
    v_flex()
        .w_full()
        .p(px(24.))
        .gap(px(16.))
        .child(
            v_flex()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(20.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_muted()))
                        .child(desc),
                ),
        )
}

/// 设置卡片容器。
fn card() -> Div {
    v_flex()
        .w_full()
        .p(px(16.))
        .gap(px(12.))
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
}

fn field_label(text: &'static str) -> Div {
    h_flex()
        .gap_1()
        .text_size(px(11.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(palette::text_muted()))
        .child(text)
        .child(
            div()
                .text_color(rgb(palette::danger()))
                .child("*"),
        )
}

fn hint(text: &'static str) -> Div {
    div()
        .text_size(px(11.))
        .text_color(rgb(palette::text_dim()))
        .child(text)
}

/// 表单分组标签：比 `field_label` 轻，用于同一卡片里的子选项（频道 / 有效期等）。
fn chip_label(text: &'static str) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(palette::text_muted()))
        .child(text)
}

// ---------------------------------------------------------------------------
// 概览
// ---------------------------------------------------------------------------

fn overview(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let server = app.active_server_ref();
    let badge = server.badge.clone();
    let color = server.color;

    let mut swatches = h_flex().gap_2();
    for (index, value) in SERVER_COLORS.iter().enumerate() {
        let selected = *value == color;
        let picked = *value;
        swatches = swatches.child(
            div()
                .w(px(28.))
                .h(px(28.))
                .rounded_full()
                .bg(rgb(picked))
                .border_2()
                .border_color(if selected {
                    rgb(palette::text_primary())
                } else {
                    rgb(palette::bg_sidebar())
                })
                .hover(|el| el.border_color(rgb(palette::text_secondary())))
                .id(format!("server-color-{index}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_server_color(picked, cx);
                })),
        );
    }

    shell(
        "概览",
        "服务器名称与图标会立即反映到左侧服务器栏与侧栏标题。",
    )
    .child(
        card()
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .w(px(56.))
                            .h(px(56.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(16.))
                            .bg(rgb(color))
                            .text_size(px(22.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(palette::WHITE))
                            .child(badge),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(palette::text_primary()))
                                    .child("服务器图标"),
                            )
                            .child(hint("徽标取名称首字符，改名后自动更新")),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(field_label("服务器名称"))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .rounded(px(6.))
                                    .bg(rgb(palette::bg_rail()))
                                    .child(
                                        <Input as Sizable>::with_size(
                                            Input::new(&app.server_name_edit),
                                            Size::Small,
                                        )
                                        .w_full(),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(32.))
                                    .px_4()
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.))
                                    .bg(rgb(palette::accent()))
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(palette::WHITE))
                                    .hover(|el| el.bg(rgb(palette::accent_hover())))
                                    .child("保存")
                                    .id("server-overview-save")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.save_server_overview(cx);
                                    })),
                            ),
                    )
                    .child(hint("按 Enter 或点保存生效；改名会同步迁移该服务器的消息记录与分类折叠状态")),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(field_label("服务器简介"))
                    .child(
                        div()
                            .w_full()
                            .rounded(px(6.))
                            .bg(rgb(palette::bg_rail()))
                            .child(
                                <Input as Sizable>::with_size(
                                    Input::new(&app.server_desc_edit),
                                    Size::Small,
                                )
                                .w_full(),
                            ),
                    )
                    .child(hint("显示在邀请页与服务器信息中，留空则保持原简介")),
            ),
    )
    .child(
        card()
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(palette::text_primary()))
                    .child("图标颜色"),
            )
            .child(swatches)
            .child(hint("点击色块立即生效，无需保存")),
    )
}

// ---------------------------------------------------------------------------
// 角色
// ---------------------------------------------------------------------------

fn roles(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let selected = app.selected_role.clone();

    // 左栏：角色列表（顺序即成员栏分组顺序）+ 新建。
    let mut list = v_flex().w_full().gap(px(2.));
    for (index, role) in app.roles.iter().enumerate() {
        let name = role.name.clone();
        let is_selected = selected.as_deref() == Some(role.name.as_str());
        let count = app.role_member_count(&role.name);
        list = list.child(
            h_flex()
                .w_full()
                .h(px(40.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(6.))
                .bg(rgb(if is_selected {
                    palette::bg_rail()
                } else {
                    palette::bg_sidebar()
                }))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(rgb(role.color)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(rgb(role.color))
                        .child(role.name.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_muted()))
                        .child(format!("{count} 人")),
                )
                .id(format!("role-row-{index}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.select_role(name.clone(), window, cx);
                })),
        );
    }

    let left = v_flex()
        .w(px(228.))
        .flex_shrink_0()
        .gap_2()
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("角色"),
                )
                .child(
                    h_flex()
                        .h(px(24.))
                        .px_2()
                        .gap_1()
                        .items_center()
                        .rounded(px(4.))
                        .bg(rgb(palette::accent()))
                        .text_size(px(11.))
                        .text_color(rgb(palette::WHITE))
                        .hover(|el| el.bg(rgb(palette::accent_hover())))
                        .child(ico(IconName::Plus, Size::XSmall))
                        .child("新建")
                        .id("role-create")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.create_role(window, cx);
                        })),
                ),
        )
        .child(list);

    // 右栏：选中角色的详情（名称 / 颜色 / 成员归属 / 删除）。
    let mut detail = card();
    match selected.as_deref() {
        None => {
            detail = detail.child(hint("左侧选择一个角色，或点「新建」创建一个新角色"));
        }
        Some(current) => {
            let color = app.role_color(current);
            let count = app.role_member_count(current);

            let mut swatches = h_flex().flex_wrap().gap_2();
            for (index, value) in ROLE_COLORS.iter().enumerate() {
                let picked = *value;
                let active = picked == color;
                swatches = swatches.child(
                    div()
                        .w(px(26.))
                        .h(px(26.))
                        .rounded_full()
                        .bg(rgb(picked))
                        .border_2()
                        .border_color(rgb(if active {
                            palette::text_primary()
                        } else {
                            palette::bg_sidebar()
                        }))
                        .hover(|el| el.border_color(rgb(palette::text_secondary())))
                        .id(format!("role-color-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_role_color(picked, cx);
                        })),
                );
            }

            // 成员归属：点一行即在「当前角色 / 无角色」之间切换。
            let mut members = v_flex().w_full().gap(px(2.));
            for (index, member) in app.members.iter().enumerate() {
                let member_name = member.name;
                let assigned = member.role == Some(current);
                let current_role = member.role.unwrap_or("无角色");

                let mut check = div()
                    .w(px(16.))
                    .h(px(16.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .bg(rgb(if assigned {
                        color
                    } else {
                        palette::bg_rail()
                    }))
                    .text_color(rgb(palette::WHITE));
                if assigned {
                    check = check.child(ico(IconName::Check, Size::XSmall));
                }

                members = members.child(
                    h_flex()
                        .w_full()
                        .h(px(38.))
                        .px_2()
                        .gap_3()
                        .items_center()
                        .rounded(px(6.))
                        .hover(|el| el.bg(rgb(palette::bg_rail())))
                        .child(avatar(
                            member.name.chars().next().unwrap_or('?'),
                            member.color,
                            px(24.),
                            member.online,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_primary()))
                                .child(member.name),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_muted()))
                                .child(current_role),
                        )
                        .child(check)
                        .id(format!("role-member-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_member_role(member_name, cx);
                        })),
                );
            }

            detail = detail
                .child(
                    v_flex()
                        .gap_1()
                        .child(field_label("角色名称"))
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .rounded(px(6.))
                                        .bg(rgb(palette::bg_rail()))
                                        .child(
                                            <Input as Sizable>::with_size(
                                                Input::new(&app.role_name_edit),
                                                Size::Small,
                                            )
                                            .w_full(),
                                        ),
                                )
                                .child(
                                    div()
                                        .h(px(32.))
                                        .px_4()
                                        .flex()
                                        .items_center()
                                        .rounded(px(6.))
                                        .bg(rgb(palette::accent()))
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(palette::WHITE))
                                        .hover(|el| el.bg(rgb(palette::accent_hover())))
                                        .child("保存")
                                        .id("role-name-save")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.save_role_name(cx);
                                        })),
                                ),
                        )
                        .child(hint(app.role_notice.unwrap_or(
                            "按 Enter 或点保存生效；改名会同步到成员与成员栏分组",
                        ))),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(chip_label("角色颜色"))
                        .child(swatches),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(chip_label("成员"))
                                .child(
                                    div()
                                        .px_2()
                                        .h(px(18.))
                                        .flex()
                                        .items_center()
                                        .rounded_full()
                                        .bg(rgb(palette::bg_rail()))
                                        .text_size(px(11.))
                                        .text_color(rgb(palette::text_muted()))
                                        .child(format!("{count} 人")),
                                ),
                        )
                        .child(members)
                        .child(hint("点成员行加入 / 移除该角色；未分配角色的成员归入「在线 / 离线」")),
                )
                .child(
                    h_flex()
                        .h(px(32.))
                        .px_3()
                        .gap_1()
                        .items_center()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_rail()))
                        .text_size(px(12.))
                        .text_color(rgb(palette::danger()))
                        .hover(|el| {
                            el.bg(rgb(palette::danger()))
                                .text_color(rgb(palette::WHITE))
                        })
                        .child(ico(IconName::Trash, Size::XSmall))
                        .child("删除角色")
                        .id("role-delete")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.delete_role(cx);
                        })),
                );
        }
    }

    shell(
        "角色",
        "角色决定成员栏的分组与颜色；改名会同步到所有成员。",
    )
    .child(
        h_flex()
            .w_full()
            .gap_4()
            .items_start()
            .child(left)
            .child(div().flex_1().min_w_0().child(detail)),
    )
}

// ---------------------------------------------------------------------------
// 权限
// ---------------------------------------------------------------------------

/// 单个权限行的三态按钮：继承（灰）/ 允许（绿勾）/ 拒绝（红叉），点击循环切换。
fn permission_state_button(
    role: &str,
    key: &'static str,
    label: &'static str,
    state: PermissionState,
    id: String,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    let (bg, icon, icon_color) = match state {
        PermissionState::Unset => (palette::bg_rail(), IconName::Minus, palette::text_muted()),
        PermissionState::Allow => (palette::success(), IconName::Check, palette::WHITE),
        PermissionState::Deny => (palette::danger(), IconName::X, palette::WHITE),
    };
    let owned_role = role.to_string();
    div()
        .w(px(76.))
        .h(px(28.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .rounded(px(5.))
        .bg(rgb(bg))
        .hover(|el| el.opacity(0.85))
        .child(ico(icon, Size::XSmall).text_color(rgb(icon_color)))
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(icon_color))
                .child(state.label()),
        )
        .id(id)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.cycle_permission(owned_role.clone(), key, label, cx);
        }))
}

fn permissions(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    // 顶部角色 tabs：复用 selected_role，点击只切聚焦角色。
    let mut tabs = h_flex().w_full().gap_2().flex_wrap();
    let current = app.permission_role();
    for (index, role) in app.roles.iter().enumerate() {
        let active = current.as_deref() == Some(role.name.as_str());
        let name = role.name.clone();
        let count = app.role_member_count(&role.name);
        tabs = tabs.child(
            h_flex()
                .h(px(34.))
                .px_3()
                .gap_2()
                .items_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(if active {
                    palette::accent()
                } else {
                    palette::border_default()
                }))
                .bg(rgb(if active {
                    palette::bg_channel_selected()
                } else {
                    palette::bg_sidebar()
                }))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(rgb(role.color)))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(palette::text_primary()))
                        .child(name.clone()),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette::text_muted()))
                        .child(format!("{count}")),
                )
                .id(format!("permission-role-tab-{index}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_permission_role(name.clone(), cx);
                })),
        );
    }

    let mut body = v_flex().w_full().gap(px(14.));
    if let Some(role) = current.as_deref() {
        let role_color = app.role_color(role);
        // 按权限分组渲染：每组一个标题 + 若干权限行。
        for group in PermissionGroup::ALL {
            let mut group_card = card().child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w(px(4.))
                            .h(px(14.))
                            .rounded(px(2.))
                            .bg(rgb(role_color)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(palette::text_primary()))
                            .child(group.label()),
                    ),
            );
            for def in PERMISSIONS.iter().filter(|def| def.group == group) {
                let state = app.permission_state(role, def.key);
                let row = h_flex()
                    .w_full()
                    .min_h(px(40.))
                    .px_2()
                    .gap_3()
                    .items_center()
                    .justify_between()
                    .rounded(px(6.))
                    .hover(|el| el.bg(rgb(palette::bg_message_hover())))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(palette::text_primary()))
                                    .child(def.label),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette::text_dim()))
                                    .child(def.hint),
                            ),
                    )
                    .child(permission_state_button(
                        role,
                        def.key,
                        def.label,
                        state,
                        format!("permission-toggle-{}-{}", role, def.key),
                        cx,
                    ));
                group_card = group_card.child(row);
            }
            body = body.child(group_card);
        }
    } else {
        body = body.child(card().child(hint("当前服务器没有角色，请先在「角色」分类创建")));
    }

    shell(
        "权限",
        "按角色配置权限位：继承跟随服务器默认，允许 / 拒绝显式覆盖（演示数据，不影响真实操作）。",
    )
    .child(
        card()
            .child(chip_label("选择角色"))
            .child(tabs)
            .child(hint(app.permission_notice.unwrap_or(
                "点每行右侧按钮循环切换：继承 → 允许 → 拒绝；变更会写入审核日志。",
            ))),
    )
    .child(body)
}

// ---------------------------------------------------------------------------
// 自动审核
// ---------------------------------------------------------------------------

fn automod(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let focus = app.automod_rule_focus();

    // 左栏：规则列表 + 新建输入。
    let mut list = v_flex().w_full().gap(px(2.));
    for (index, rule) in app.automod_rules.iter().enumerate() {
        let active = focus.as_deref() == Some(rule.name.as_str());
        let name = rule.name.clone();
        let enabled = rule.enabled;
        list = list.child(
            h_flex()
                .w_full()
                .min_h(px(40.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(6.))
                .bg(rgb(if active {
                    palette::bg_rail()
                } else {
                    palette::bg_sidebar()
                }))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(
                    div()
                        .w(px(8.))
                        .h(px(8.))
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(rgb(if enabled {
                            palette::success()
                        } else {
                            palette::text_dim()
                        })),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(1.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_primary()))
                                .child(rule.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette::text_muted()))
                                .child(format!(
                                    "{} · {} 个关键词",
                                    rule.action.label(),
                                    rule.keywords.len()
                                )),
                        ),
                )
                .id(format!("automod-rule-{index}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.select_automod_rule(name.clone(), window, cx);
                })),
        );
    }

    let left = v_flex()
        .w(px(228.))
        .flex_shrink_0()
        .gap_2()
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_primary()))
                .child("规则"),
        )
        .child(list)
        .child(
            v_flex()
                .gap_1()
                .child(chip_label("新建规则"))
                .child(
                    div()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_rail()))
                        .child(
                            <Input as Sizable>::with_size(
                                Input::new(&app.automod_name_edit),
                                Size::Small,
                            )
                            .w_full(),
                        ),
                )
                .child(
                    div()
                        .h(px(30.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .bg(rgb(palette::accent()))
                        .text_size(px(12.))
                        .text_color(rgb(palette::WHITE))
                        .hover(|el| el.bg(rgb(palette::accent_hover())))
                        .child("创建规则")
                        .id("automod-create")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.create_automod_rule(window, cx);
                        })),
                ),
        );

    // 右栏：选中规则的详情。
    let mut detail = card();
    match focus.as_deref() {
        None => {
            detail = detail.child(hint("左侧选择或新建一条规则"));
        }
        Some(current) => {
            let rule = app
                .automod_rules
                .iter()
                .find(|rule| rule.name == current);
            if let Some(rule) = rule {
                // 启用开关行。
                let toggle_icon = if rule.enabled {
                    IconName::CircleCheck
                } else {
                    IconName::Circle
                };
                let enabled = rule.enabled;
                let action = rule.action;
                let keyword_count = rule.keywords.len();

                detail = detail
                    .child(
                        v_flex()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(palette::text_primary()))
                                    .child(rule.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette::text_muted()))
                                    .child(rule.description.clone()),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .h(px(38.))
                            .px_2()
                            .gap_2()
                            .items_center()
                            .justify_between()
                            .rounded(px(6.))
                            .bg(rgb(palette::bg_rail()))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(palette::text_primary()))
                                    .child(if enabled { "已启用" } else { "已停用" }),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(ico(toggle_icon, Size::XSmall))
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(if enabled {
                                                palette::success()
                                            } else {
                                                palette::text_muted()
                                            }))
                                            .child(if enabled { "点击停用" } else { "点击启用" }),
                                    )
                            .id("automod-toggle-enabled")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_automod_rule(cx);
                            })),
                            ),
                    )
                    // 动作切换：拦截 / 警告 两个按钮。
                    .child(
                        v_flex()
                            .gap_2()
                            .child(chip_label("命中后动作"))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(action_button(
                                        "拦截消息",
                                        "命中关键词时阻止发送",
                                        action == AutoModAction::Block,
                                        "automod-action-block",
                                        AutoModAction::Block,
                                        cx,
                                    ))
                                    .child(action_button(
                                        "发出警告",
                                        "照常发送并提示",
                                        action == AutoModAction::Warn,
                                        "automod-action-warn",
                                        AutoModAction::Warn,
                                        cx,
                                    )),
                            ),
                    )
                    // 关键词 chips：每个可点 X 删除。
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(chip_label("关键词"))
                                    .child(
                                        div()
                                            .px_2()
                                            .h(px(18.))
                                            .flex()
                                            .items_center()
                                            .rounded_full()
                                            .bg(rgb(palette::bg_rail()))
                                            .text_size(px(11.))
                                            .text_color(rgb(palette::text_muted()))
                                            .child(format!("{keyword_count}")),
                                    ),
                            )
                            .child({
                                let mut chips = h_flex().gap_2().flex_wrap();
                                for (index, keyword) in rule.keywords.iter().enumerate() {
                                    let word = keyword.clone();
                                    chips = chips.child(
                                        h_flex()
                                            .h(px(28.))
                                            .px_2()
                                            .gap_1()
                                            .items_center()
                                            .rounded(px(6.))
                                            .bg(rgb(palette::bg_rail()))
                                            .border_1()
                                            .border_color(rgb(palette::border_default()))
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(rgb(palette::text_primary()))
                                                    .child(keyword.clone()),
                                            )
                                            .child(
                                                div()
                                                    .w(px(16.))
                                                    .h(px(16.))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(px(3.))
                                                    .hover(|el| {
                                                        el.bg(rgb(palette::danger()))
                                                            .text_color(rgb(palette::WHITE))
                                                    })
                                                    .child(ico(IconName::X, Size::XSmall))
                                                    .id(format!("automod-keyword-remove-{index}"))
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.remove_automod_keyword(
                                                                word.clone(),
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                            ),
                                    );
                                }
                                if keyword_count == 0 {
                                    chips.child(hint("还没有关键词，在下方输入添加"))
                                } else {
                                    chips
                                }
                            })
                            .child(
                                div()
                                    .rounded(px(6.))
                                    .bg(rgb(palette::bg_rail()))
                                    .child(
                                        <Input as Sizable>::with_size(
                                            Input::new(&app.automod_keyword_edit),
                                            Size::Small,
                                        )
                                        .w_full(),
                                    ),
                            )
                            .child(hint("回车添加关键词；消息命中任一关键词即触发动作")),
                    )
                    // 删除规则。
                    .child(
                        h_flex()
                            .h(px(32.))
                            .px_3()
                            .gap_1()
                            .items_center()
                            .rounded(px(6.))
                            .bg(rgb(palette::bg_rail()))
                            .text_size(px(12.))
                            .text_color(rgb(palette::danger()))
                            .hover(|el| {
                                el.bg(rgb(palette::danger()))
                                    .text_color(rgb(palette::WHITE))
                            })
                            .child(ico(IconName::Trash, Size::XSmall))
                            .child("删除规则")
                            .id("automod-delete")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.delete_automod_rule(cx);
                            })),
                    );
            }
        }
    }

    let notice = app.automod_notice.clone().unwrap_or_else(|| {
        "新建规则并添加关键词后即可生效；变更写入审核日志。".to_string()
    });
    shell(
        "自动审核",
        "规则命中关键词时拦截或提醒消息（仅对服务器频道生效，不会写入真实后端）。",
    )
    .child(
        div()
            .text_size(px(11.))
            .text_color(rgb(palette::text_dim()))
            .child(notice),
    )
    .child(
        h_flex()
            .w_full()
            .gap_4()
            .items_start()
            .child(left)
            .child(div().flex_1().min_w_0().child(detail)),
    )
}

/// 动作二选一按钮。
fn integrations(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let server = app.active_server_name();
    let bots = app.bots_by_server.get(&server).cloned().unwrap_or_default();
    let webhooks = app.webhooks_by_server.get(&server).cloned().unwrap_or_default();
    let text_channels: Vec<String> = app
        .servers
        .get(app.active_server)
        .map(|server| {
            server
                .categories
                .iter()
                .flat_map(|category| category.channels.iter())
                .filter(|channel| channel.kind == crate::data::ChannelKind::Text)
                .map(|channel| channel.name.clone())
                .collect()
        })
        .unwrap_or_default();

    let mut page = shell(
        "机器人 / Webhook",
        "Bot 用 `Authorization: Bot <token>` 调用接口；Webhook 用生成的 URL 直接投递消息。",
    );

    if let Some(note) = &app.integrations_notice {
        page = page.child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::text_secondary()))
                .child(note.clone()),
        );
    }

    // ---- Bot ----
    let mut bot_card = card()
        .child(field_label("机器人"))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div().w(px(240.)).child(
                        <Input as Sizable>::with_size(Input::new(&app.bot_name_edit), Size::Small)
                            .w_full(),
                    ),
                )
                .child(command_action(
                    "创建机器人",
                    "server-bot-create".to_string(),
                    false,
                    Box::new(|this, window, cx| this.create_bot(window, cx)),
                    cx,
                )),
        );

    if let Some((name, token)) = &app.bot_secret {
        let value = token.clone();
        bot_card = bot_card.child(
            h_flex()
                .gap_2()
                .items_center()
                .px_2()
                .py_2()
                .rounded(px(6.))
                .bg(rgb(palette::bg_rail()))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette::success()))
                                .child(format!("{name} 的 token（只显示一次）")),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_secondary()))
                                .child(token.clone()),
                        ),
                )
                .child(command_action(
                    "复制",
                    "server-bot-copy".to_string(),
                    false,
                    Box::new(move |this, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(value.clone()));
                        this.integrations_notice = Some("token 已复制".into());
                        cx.notify();
                    }),
                    cx,
                )),
        );
    }

    if bots.is_empty() {
        bot_card = bot_card.child(hint("还没有机器人。"));
    }
    for (index, bot) in bots.into_iter().enumerate() {
        let bot_id = bot.id.clone();
        bot_card = bot_card.child(
            h_flex()
                .gap_3()
                .items_center()
                .px_2()
                .py_2()
                .rounded(px(6.))
                .bg(rgb(palette::bg_rail()))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_primary()))
                        .child(format!("{}#{}", bot.name, bot.tag)),
                )
                .child(command_action(
                    "撤销",
                    format!("server-bot-revoke-{index}"),
                    true,
                    Box::new(move |this, _, cx| this.revoke_bot(bot_id.clone(), cx)),
                    cx,
                )),
        );
    }

    // ---- Webhook ----
    let mut webhook_card = card().child(field_label("Webhook"));
    let mut chips = h_flex().gap_1().flex_wrap();
    for (index, channel) in text_channels.iter().enumerate() {
        let selected = app.webhook_channel.as_deref() == Some(channel.as_str());
        let name = channel.clone();
        chips = chips.child(
            div()
                .px_2()
                .py_1()
                .rounded(px(4.))
                .text_size(px(11.))
                .bg(rgb(if selected {
                    palette::accent()
                } else {
                    palette::bg_rail()
                }))
                .text_color(if selected {
                    rgb(palette::WHITE)
                } else {
                    rgb(palette::text_secondary())
                })
                .child(format!("# {name}"))
                .id(format!("webhook-channel-{index}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_webhook_channel(name.clone(), cx)
                })),
        );
    }
    webhook_card = webhook_card
        .child(chips)
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div().w(px(240.)).child(
                        <Input as Sizable>::with_size(
                            Input::new(&app.webhook_name_edit),
                            Size::Small,
                        )
                        .w_full(),
                    ),
                )
                .child(command_action(
                    "创建 Webhook",
                    "server-webhook-create".to_string(),
                    false,
                    Box::new(|this, window, cx| this.create_webhook(window, cx)),
                    cx,
                )),
        );

    if let Some((name, url)) = &app.webhook_secret {
        let value = url.clone();
        webhook_card = webhook_card.child(
            h_flex()
                .gap_2()
                .items_center()
                .px_2()
                .py_2()
                .rounded(px(6.))
                .bg(rgb(palette::bg_rail()))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette::success()))
                                .child(format!("{name} 的 URL（只显示一次）")),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_secondary()))
                                .child(url.clone()),
                        ),
                )
                .child(command_action(
                    "复制",
                    "server-webhook-copy".to_string(),
                    false,
                    Box::new(move |this, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(value.clone()));
                        this.integrations_notice = Some("Webhook URL 已复制".into());
                        cx.notify();
                    }),
                    cx,
                )),
        );
    }

    if webhooks.is_empty() {
        webhook_card = webhook_card.child(hint("还没有 Webhook。"));
    }
    for (index, hook) in webhooks.into_iter().enumerate() {
        let hook_id = hook.id.clone();
        let channel_name = app
            .remote
            .channels
            .iter()
            .find(|(_, id)| id.as_str() == hook.channel_id)
            .and_then(|(key, _)| key.split_once('/').map(|(_, name)| name.to_string()))
            .unwrap_or_else(|| "未知频道".into());
        webhook_card = webhook_card.child(
            h_flex()
                .gap_3()
                .items_center()
                .px_2()
                .py_2()
                .rounded(px(6.))
                .bg(rgb(palette::bg_rail()))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_primary()))
                        .child(format!("{} → #{}", hook.name, channel_name)),
                )
                .child(command_action(
                    "撤销",
                    format!("server-webhook-revoke-{index}"),
                    true,
                    Box::new(move |this, _, cx| this.revoke_webhook(hook_id.clone(), cx)),
                    cx,
                )),
        );
    }

    page.child(bot_card).child(webhook_card)
}

fn commands(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let server = app.active_server_name();
    let rows = app.commands_by_server.get(&server).cloned().unwrap_or_default();

    let mut page = shell(
        "斜杠命令",
        "自定义命令需要 MANAGE_SERVER；内置 /help 与 /shrug 对所有服务器可用。",
    );

    if let Some(note) = &app.commands_notice {
        page = page.child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::text_secondary()))
                .child(note.clone()),
        );
    }

    let form = card()
        .child(field_label("新建命令"))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .w(px(190.))
                        .min_w_0()
                        .child(
                            <Input as Sizable>::with_size(
                                Input::new(&app.command_name_edit),
                                Size::Small,
                            )
                            .w_full(),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            <Input as Sizable>::with_size(
                                Input::new(&app.command_response_edit),
                                Size::Small,
                            )
                            .w_full(),
                        ),
                )
                .child(command_action(
                    "创建",
                    "server-command-create".to_string(),
                    false,
                    Box::new(|this, window, cx| this.create_server_command(window, cx)),
                    cx,
                )),
        )
        .child(hint("回复模板可用 {args} 插入参数，例如：开始部署 {args}"));

    let mut list = card().child(field_label("现有命令"));
    if rows.is_empty() {
        list = list.child(hint("还没有命令，或正在加载…"));
    }
    for (index, row) in rows.into_iter().enumerate() {
        let kind_label = if row.kind == "builtin" { "内置" } else { "自定义" };
        let detail = if !row.description.is_empty() {
            row.description.clone()
        } else if !row.response.is_empty() {
            row.response.clone()
        } else {
            "—".to_string()
        };
        let mut line = h_flex()
            .gap_3()
            .items_center()
            .px_2()
            .py_2()
            .rounded(px(6.))
            .bg(rgb(palette::bg_rail()))
            .child(
                div()
                    .w(px(150.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(palette::text_primary()))
                    .child(format!("/{}", row.name)),
            )
            .child(
                div()
                    .w(px(48.))
                    .text_size(px(11.))
                    .text_color(rgb(palette::text_dim()))
                    .child(kind_label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(rgb(palette::text_secondary()))
                    .child(detail),
            );
        if row.kind != "builtin" {
            let command_id = row.id.clone();
            line = line.child(command_action(
                "删除",
                format!("server-command-delete-{index}"),
                true,
                Box::new(move |this, _, cx| this.delete_server_command(command_id.clone(), cx)),
                cx,
            ));
        }
        list = list.child(line);
    }

    page.child(form).child(list)
}

/// 小型动作按钮（创建 / 删除）。
fn command_action(
    label: &'static str,
    id: String,
    danger: bool,
    handler: Box<dyn Fn(&mut DiscordApp, &mut Window, &mut Context<DiscordApp>)>,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    div()
        .px_3()
        .h(px(30.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .bg(rgb(if danger {
            palette::danger()
        } else {
            palette::accent()
        }))
        .text_color(rgb(palette::WHITE))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .hover(|el| el.opacity(0.85))
        .child(label)
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| handler(this, window, cx)))
}

fn action_button(
    title: &'static str,
    desc: &'static str,
    active: bool,
    id: &'static str,
    action: AutoModAction,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    div()
        .flex_1()
        .px_3()
        .py_2()
        .gap(px(2.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if active {
            palette::accent()
        } else {
            palette::border_default()
        }))
        .bg(rgb(if active {
            palette::bg_channel_selected()
        } else {
            palette::bg_sidebar()
        }))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(palette::text_primary()))
                .child(title),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child(desc),
        )
        .id(id)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_automod_action(action, cx);
        }))
}

// ---------------------------------------------------------------------------
// 表情
// ---------------------------------------------------------------------------

fn emoji(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut grid = v_flex().gap_2();
    for row in app.server_emojis.chunks(8) {
        let mut line = h_flex().gap_2();
        for &item in row {
            line = line.child(
                h_flex()
                    .w(px(64.))
                    .h(px(56.))
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .rounded(px(8.))
                    .bg(rgb(palette::bg_sidebar()))
                    .text_size(px(20.))
                    .child(item)
                    .child(
                        div()
                            .w(px(16.))
                            .h(px(16.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.))
                            .text_color(rgb(palette::text_dim()))
                            .hover(|el| {
                                el.bg(rgb(palette::danger()))
                                    .text_color(rgb(palette::WHITE))
                            })
                            .child(ico(IconName::X, Size::XSmall))
                            .id(format!("server-emoji-remove-{item}"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_server_emoji(item, cx);
                            })),
                    ),
            );
        }
        grid = grid.child(line);
    }

    let mut removed = h_flex().flex_wrap().gap_2();
    if app.removed_server_emojis.is_empty() {
        removed = removed.child(hint("暂无已移除的表情"));
    } else {
        for &item in &app.removed_server_emojis {
            removed = removed.child(
                h_flex()
                    .h(px(28.))
                    .px_2()
                    .gap_1()
                    .items_center()
                    .rounded(px(6.))
                    .bg(rgb(palette::bg_sidebar()))
                    .text_size(px(13.))
                    .text_color(rgb(palette::text_muted()))
                    .child(item)
                    .child(ico(IconName::RotateCcw, Size::XSmall))
                    .hover(|el| el.text_color(rgb(palette::text_primary())))
                    .id(format!("server-emoji-restore-{item}"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.restore_server_emoji(item, cx);
                    })),
            );
        }
    }

    shell("表情", "服务器自定义表情，点右上角 X 移除，可在下方恢复。")
        .child(card().child(grid))
        .child(
            card()
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child("已移除"),
                        )
                        .child(hint("点击表情恢复")),
                )
                .child(removed),
        )
}

// ---------------------------------------------------------------------------
// 邀请
// ---------------------------------------------------------------------------

fn invites(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let list = app.server_invites();
    let copied_code = app.invite_copied_code.clone();

    // 邀请频道：链接的落地页，只列文字频道。
    let mut channels = h_flex().flex_wrap().gap_2();
    for (index, name) in app.invite_channel_options().iter().enumerate() {
        let selected = app.invite_channel == *name;
        let target = name.clone();
        channels = channels.child(filter_chip(
            format!("#{name}"),
            selected,
            format!("invite-channel-{index}"),
            Box::new(move |this: &mut DiscordApp, _, cx| {
                this.set_invite_channel(target.clone(), cx);
            }),
            cx,
        ));
    }

    let mut expiries = h_flex().flex_wrap().gap_2();
    for expiry in InviteExpiry::ALL {
        let selected = app.invite_expiry == expiry;
        expiries = expiries.child(filter_chip(
            expiry.label(),
            selected,
            format!("invite-expiry-{}", expiry.label()),
            Box::new(move |this: &mut DiscordApp, _, cx| {
                this.set_invite_expiry(expiry, cx);
            }),
            cx,
        ));
    }

    let mut limits = h_flex().flex_wrap().gap_2();
    for max in [0u32, 1, 5, 10, 25] {
        let selected = app.invite_max_uses == max;
        limits = limits.child(filter_chip(
            if max == 0 {
                "不限".to_string()
            } else {
                format!("{max} 次")
            },
            selected,
            format!("invite-limit-{max}"),
            Box::new(move |this: &mut DiscordApp, _, cx| {
                this.set_invite_max_uses(max, cx);
            }),
            cx,
        ));
    }

    let create_button = h_flex()
        .h(px(32.))
        .px_4()
        .gap_1()
        .items_center()
        .rounded(px(6.))
        .bg(rgb(palette::accent()))
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(palette::WHITE))
        .hover(|el| el.bg(rgb(palette::accent_hover())))
        .child(ico(IconName::Plus, Size::XSmall))
        .child("生成并复制链接")
        .id("invite-create")
        .on_click(cx.listener(|this, _, _, cx| {
            this.create_invite(cx);
        }));

    // 已有邀请：一行一条，右侧复制与撤销。
    let mut rows = v_flex().w_full().gap(px(2.));
    if list.is_empty() {
        rows = rows.child(
            h_flex()
                .w_full()
                .py(px(20.))
                .justify_center()
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child("还没有邀请链接，先在上面生成一条吧"),
        );
    }
    for (index, invite) in list.iter().enumerate() {
        let code = invite.code.clone();
        let copied = copied_code.as_deref() == Some(code.as_str());
        let copy_code = code.clone();
        let revoked_code = code.clone();

        let mut uses = v_flex().w(px(80.)).flex_shrink_0().gap(px(2.)).child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::text_secondary()))
                .child(invite.uses_label()),
        );
        if let Some(status) = invite.status() {
            uses = uses.child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(palette::danger()))
                    .child(status),
            );
        }

        rows = rows.child(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .px(px(12.))
                .py(px(10.))
                .rounded(px(6.))
                .hover(|el| el.bg(rgb(palette::bg_rail())))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .text_color(rgb(palette::accent()))
                                .child(invite.link()),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_muted()))
                                .child(format!("{} 邀请 · #{}", invite.inviter, invite.channel)),
                        ),
                )
                .child(uses)
                .child(
                    div()
                        .w(px(116.))
                        .flex_shrink_0()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(if invite.expired() {
                            rgb(palette::danger())
                        } else {
                            rgb(palette::text_muted())
                        })
                        .child(invite.expiry_label()),
                )
                .child(
                    h_flex()
                        .h(px(26.))
                        .px_2()
                        .gap_1()
                        .items_center()
                        .rounded(px(4.))
                        .text_size(px(11.))
                        .text_color(if copied {
                            rgb(palette::success())
                        } else {
                            rgb(palette::text_secondary())
                        })
                        .hover(|el| {
                            el.bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_primary()))
                        })
                        .child(ico(IconName::Copy, Size::XSmall))
                        .child(if copied { "已复制" } else { "复制" })
                        .id(format!("invite-copy-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.copy_invite_link(&copy_code, cx);
                        })),
                )
                .child(danger_icon_button(
                    IconName::Trash,
                    Size::XSmall,
                    px(26.),
                    format!("invite-revoke-{index}"),
                    Box::new(move |this: &mut DiscordApp, _, cx| {
                        this.revoke_invite(&revoked_code, cx);
                    }),
                    cx,
                )),
        );
    }

    shell("邀请", "生成邀请链接发给朋友，他们就能加入这个服务器。")
        .child(
            card()
                .child(field_label("创建邀请链接"))
                .child(
                    v_flex()
                        .gap(px(14.))
                        .child(
                            v_flex()
                                .gap_2()
                                .child(chip_label("邀请频道"))
                                .child(channels),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(chip_label("有效期"))
                                .child(expiries),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(chip_label("最大使用次数"))
                                .child(limits),
                        ),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(create_button)
                        .child(hint("生成后链接直接写入剪贴板")),
                ),
        )
        .child(
            card()
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(palette::text_primary()))
                                        .child("已创建的邀请"),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .h(px(18.))
                                        .flex()
                                        .items_center()
                                        .rounded_full()
                                        .bg(rgb(palette::bg_rail()))
                                        .text_size(px(11.))
                                        .text_color(rgb(palette::text_muted()))
                                        .child(format!("{} 条", list.len())),
                                ),
                        )
                        .child(
                            h_flex()
                                .h(px(28.))
                                .px_2()
                                .gap_1()
                                .items_center()
                                .rounded(px(4.))
                                .text_size(px(12.))
                                .text_color(rgb(palette::text_secondary()))
                                .hover(|el| {
                                    el.bg(rgb(palette::bg_channel_hover()))
                                        .text_color(rgb(palette::text_primary()))
                                })
                                .child(ico(IconName::UserPlus, Size::XSmall))
                                .child("邀请弹层")
                                .id("server-invite-open-dialog")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_invite(window, cx);
                                })),
                        ),
                )
                .child(rows)
                .child(hint("撤销后链接立即失效；已过期 / 已用完的记录只作为留档")),
        )
}

// ---------------------------------------------------------------------------
// 审核日志
// ---------------------------------------------------------------------------

/// 类别对应的行首图标。
fn audit_icon(kind: AuditKind) -> IconName {
    match kind {
        AuditKind::Server => IconName::Server,
        AuditKind::Channel => IconName::Hash,
        AuditKind::Role => IconName::Shield,
        AuditKind::Member => IconName::Users,
        AuditKind::Invite => IconName::Link,
        AuditKind::Emoji => IconName::FaceSlightlySmiling,
    }
}

fn audit(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let newest = app.audit_newest_seq();
    let total = app.audit_count(None);
    let entries = app.audit_visible();

    // 筛选条：全部 + 五个类别，各带条数。
    let mut filters = h_flex().flex_wrap().gap_2();
    filters = filters.child(filter_chip(
        format!("全部 {total}"),
        app.audit_filter.is_none(),
        "audit-filter-all",
        Box::new(|this: &mut DiscordApp, _, cx| {
            this.set_audit_filter(None, cx);
        }),
        cx,
    ));
    for (index, kind) in AuditKind::ALL.into_iter().enumerate() {
        filters = filters.child(filter_chip(
            format!("{} {}", kind.label(), app.audit_count(Some(kind))),
            app.audit_filter == Some(kind),
            format!("audit-filter-{index}"),
            Box::new(move |this: &mut DiscordApp, _, cx| {
                this.set_audit_filter(Some(kind), cx);
            }),
            cx,
        ));
    }

    // 标题行：左侧条数说明，右侧清空按钮。
    let mut header = h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(if entries.len() == total {
                            format!("共 {total} 条")
                        } else {
                            format!("{} / {total} 条", entries.len())
                        }),
                )
                .child(hint("只保留最近 200 条")),
        );
    if total > 0 {
        header = header.child(
            h_flex()
                .h(px(28.))
                .px_2()
                .gap(px(6.))
                .items_center()
                .rounded(px(4.))
                .text_size(px(12.))
                .text_color(rgb(palette::danger()))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(ico(IconName::Trash, Size::XSmall))
                .child("清空日志")
                .id("audit-clear")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.clear_audit(cx);
                })),
        );
    }

    // 日志列表：一行一条，最新的在最上面。
    let mut rows = v_flex().w_full().gap(px(2.));
    if entries.is_empty() {
        rows = rows.child(
            h_flex()
                .w_full()
                .py(px(24.))
                .justify_center()
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child(if total == 0 {
                    "日志是空的：改名、建频道、改角色这些操作都会记在这里"
                } else {
                    "这个筛选下还没有记录"
                }),
        );
    }
    for entry in &entries {
        rows = rows.child(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .px(px(12.))
                .py(px(10.))
                .rounded(px(6.))
                .hover(|el| el.bg(rgb(palette::bg_rail())))
                .child(
                    div()
                        .w(px(30.))
                        .h(px(30.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.))
                        .bg(rgb(entry.kind.color()))
                        .text_color(rgb(palette::WHITE))
                        .child(ico(audit_icon(entry.kind), Size::Small)),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .text_size(px(13.))
                                        .text_color(rgb(palette::text_primary()))
                                        .child(entry.action.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.))
                                        .text_color(rgb(palette::text_muted()))
                                        .child(entry.detail.clone()),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(entry.actor.clone()),
                                )
                                .child("·")
                                .child(entry.time_label(newest)),
                        ),
                ),
        );
    }

    shell(
        "审核日志",
        "记录本机的服务器、频道、角色、邀请与表情变更，按操作时间倒序排列。",
    )
    .child(card().child(filters).child(header).child(rows))
}

// ---------------------------------------------------------------------------
// 成员
// ---------------------------------------------------------------------------

fn members(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let query = app.member_search.read(cx).value().trim().to_string();
    let visible = app.filtered_members(cx);
    let total = app.members.len();

    // 搜索行：图标 + 输入框 + 条数（有内容时多一个清除按钮）。
    let mut search = h_flex()
        .w_full()
        .gap_2()
        .items_center()
        .px_3()
        .h(px(36.))
        .rounded(px(6.))
        .bg(rgb(palette::bg_rail()))
        .child(ico(IconName::Search, Size::Small))
        .child(<Input as Sizable>::with_size(Input::new(&app.member_search), Size::Small).flex_1())
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(12.))
                .text_color(rgb(palette::text_muted()))
                .child(format!("共 {total} 位成员")),
        );
    if !query.is_empty() {
        search = search.child(
            div()
                .w(px(22.))
                .h(px(22.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(3.))
                .text_color(rgb(palette::text_dim()))
                .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                .child(ico(IconName::X, Size::XSmall))
                .id("member-search-clear")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.clear_member_search(window, cx);
                })),
        );
    }

    // 筛选条：全部 / 每个角色 / 无角色，各带人数。
    let mut filters = h_flex().flex_wrap().gap_2();
    filters = filters.child(filter_chip(
        format!("全部 {}", app.member_count_of(&MemberFilter::All)),
        app.member_filter == MemberFilter::All,
        "member-filter-all",
        Box::new(|this: &mut DiscordApp, _, cx| {
            this.set_member_filter(MemberFilter::All, cx);
        }),
        cx,
    ));
    for (index, role) in app.roles.iter().enumerate() {
        let name = role.name.clone();
        let selected = app.member_filter == MemberFilter::Role(name.clone());
        let count = app.member_count_of(&MemberFilter::Role(name.clone()));
        let target = name.clone();
        filters = filters.child(filter_chip(
            format!("{name} {count}"),
            selected,
            format!("member-filter-role-{index}"),
            Box::new(move |this: &mut DiscordApp, _, cx| {
                this.set_member_filter(MemberFilter::Role(target.clone()), cx);
            }),
            cx,
        ));
    }
    filters = filters.child(filter_chip(
        format!("无角色 {}", app.member_count_of(&MemberFilter::NoRole)),
        app.member_filter == MemberFilter::NoRole,
        "member-filter-none",
        Box::new(|this: &mut DiscordApp, _, cx| {
            this.set_member_filter(MemberFilter::NoRole, cx);
        }),
        cx,
    ));

    // 成员行：头像 + 昵称/原名 + 角色 badge + 加入时间 + 三个操作。
    let mut rows = v_flex().w_full().gap(px(2.));
    if visible.is_empty() {
        rows = rows.child(
            h_flex()
                .w_full()
                .py(px(24.))
                .justify_center()
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child(if total == 0 {
                    "服务器里还没有成员"
                } else {
                    "没有匹配的成员，换个关键词或筛选试试"
                }),
        );
    }
    for &index in &visible {
        let Some(member) = app.members.get(index) else {
            continue;
        };
        let name = member.name.to_string();
        let nick_name = name.clone();
        let kick_name = name.clone();
        let ban_name = name.clone();
        let aliased = member.nick.is_some();

        let mut title_row = h_flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(palette::text_primary()))
                    .child(member.display().to_string()),
            );
        if aliased {
            title_row = title_row.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(11.))
                    .text_color(rgb(palette::text_dim()))
                    .child(format!("@{name}")),
            );
        }
        if let Some(role) = member.role {
            title_row = title_row.child(
                h_flex()
                    .flex_shrink_0()
                    .h(px(18.))
                    .px_2()
                    .gap_1()
                    .items_center()
                    .rounded_full()
                    .bg(rgb(palette::bg_rail()))
                    .child(
                        div()
                            .w(px(6.))
                            .h(px(6.))
                            .rounded_full()
                            .bg(rgb(app.role_color(role))),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette::text_secondary()))
                            .child(role.to_string()),
                    ),
            );
        }

        rows = rows.child(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(6.))
                .hover(|el| el.bg(rgb(palette::bg_rail())))
                .child(avatar(
                    member.name.chars().next().unwrap_or('?'),
                    member.color,
                    px(32.),
                    member.online,
                ))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(title_row)
                        .child(
                            h_flex()
                                .gap_1()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(member.joined_label())
                                .child("·")
                                .child(if member.online { "在线" } else { "离线" }),
                        ),
                )
                .child(icon_button(
                    IconName::Pen,
                    Size::XSmall,
                    px(28.),
                    format!("member-nick-{index}"),
                    false,
                    Box::new(move |this: &mut DiscordApp, window, cx| {
                        this.open_member_nick(nick_name.clone(), window, cx);
                    }),
                    cx,
                ))
                .child(danger_icon_button(
                    IconName::UserMinus,
                    Size::XSmall,
                    px(28.),
                    format!("member-kick-{index}"),
                    Box::new(move |this: &mut DiscordApp, _, cx| {
                        this.kick_member(&kick_name, cx);
                    }),
                    cx,
                ))
                .child(danger_icon_button(
                    IconName::Ban,
                    Size::XSmall,
                    px(28.),
                    format!("member-ban-{index}"),
                    Box::new(move |this: &mut DiscordApp, _, cx| {
                        this.ban_member(&ban_name, cx);
                    }),
                    cx,
                )),
        );
    }

    // 封禁列表：一行一条，右侧撤销。
    let mut ban_rows = v_flex().w_full().gap(px(2.));
    if app.bans.is_empty() {
        ban_rows = ban_rows.child(
            h_flex()
                .w_full()
                .py(px(16.))
                .justify_center()
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child("没有被封禁的用户"),
        );
    }
    for (index, ban) in app.bans.iter().enumerate() {
        let target = ban.name.clone();
        ban_rows = ban_rows.child(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(6.))
                .hover(|el| el.bg(rgb(palette::bg_rail())))
                .child(avatar(
                    ban.name.chars().next().unwrap_or('?'),
                    0x80848e,
                    px(28.),
                    false,
                ))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_primary()))
                                .child(ban.name.clone()),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(ban.reason_label().to_string())
                                .child("·")
                                .child(ban.time_label())
                                .child("·")
                                .child(format!("by {}", ban.by)),
                        ),
                )
                .child(
                    div()
                        .h(px(26.))
                        .px_2()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .rounded(px(4.))
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_link()))
                        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                        .child("撤销封禁")
                        .id(format!("member-unban-{index}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.unban_member(&target, cx);
                        })),
                ),
        );
    }

    shell(
        "成员",
        "搜索、改昵称、移出或封禁成员；封禁记录只在本机留存，可随时撤销。",
    )
    .child(card().child(search).child(filters).child(rows))
    .child(
        card()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(palette::text_primary()))
                            .child("封禁列表"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette::text_muted()))
                            .child(format!("{} 人", app.bans.len())),
                    ),
            )
            .child(ban_rows),
    )
}

// ---------------------------------------------------------------------------
// 服务器加成
// ---------------------------------------------------------------------------

/// 三个等级的门槛与特权。门槛复用 `BOOST_TIERS`，改数值只改一处。
const BOOST_PERKS: [(u32, &str, [&str; 3]); 3] = [
    (
        BOOST_TIERS[0],
        "等级 1",
        ["自定义服务器邀请链接", "128 kbps 语音音质", "50 个自定义表情槽位"],
    ),
    (
        BOOST_TIERS[1],
        "等级 2",
        ["1080p 60fps 画面分享", "服务器横幅与背景", "100 个自定义表情槽位"],
    ),
    (
        BOOST_TIERS[2],
        "等级 3",
        ["384 kbps 语音音质", "动画服务器图标", "无限自定义表情槽位"],
    ),
];

fn boost(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let boost = app.active_boost();
    let count = boost.count();
    let level = boost.level();
    let remaining = boost.remaining();
    let mine = boost.boosted_by("你");

    let next_text = match remaining {
        Some(left) => format!("再 {left} 次加成升到等级 {}", level + 1),
        None => "已经是最高等级，感谢每一位加成者".to_string(),
    };
    let bar = div()
        .relative()
        .w_full()
        .h(px(8.))
        .rounded(px(4.))
        .bg(rgb(palette::bg_divider()))
        .child(
            div()
                .w(relative(boost.progress().clamp(0.0, 1.0)))
                .h_full()
                .rounded(px(4.))
                .bg(rgb(palette::accent())),
        );

    // 主按钮：没加成时显示「加成此服务器」，已加成时可撤销。
    let (label, bg, fg) = if mine {
        ("撤销我的加成", palette::bg_channel_selected(), palette::text_primary())
    } else {
        ("加成此服务器", palette::accent(), palette::WHITE)
    };
    let toggle = div()
        .flex_shrink_0()
        .h(px(36.))
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(rgb(bg))
        .hover(|el| el.opacity(0.9))
        .text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(fg))
        .child(label)
        .id("boost-toggle")
        .on_click(cx.listener(|this, _, _, cx| this.toggle_boost(cx)));

    let mut page = shell(
        "加成",
        "成员用加成把服务器升到更高等级，解锁更好的音质、画质与表情槽位。",
    );

    if let Some(notice) = app.boost_notice.clone() {
        page = page.child(
            h_flex()
                .w_full()
                .px_3()
                .h(px(32.))
                .gap_2()
                .items_center()
                .rounded(px(6.))
                .bg(rgb(palette::bg_sidebar()))
                .border_1()
                .border_color(rgb(palette::accent()))
                .text_size(px(12.))
                .text_color(rgb(palette::text_primary()))
                .child(ico(IconName::Sparkles, Size::XSmall).text_color(rgb(palette::accent())))
                .child(div().flex_1().child(notice))
                .child(
                    div()
                        .w(px(18.))
                        .h(px(18.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.))
                        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                        .child(ico(IconName::X, Size::XSmall))
                        .id("boost-notice-close")
                        .on_click(cx.listener(|this, _, _, cx| this.dismiss_boost_notice(cx))),
                ),
        );
    }

    // 等级总览：大数字 + 进度条 + 加成按钮。
    page = page.child(
        div()
            .w_full()
            .p(px(20.))
            .rounded(px(10.))
            .bg(rgb(palette::bg_reaction_mine()))
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .w(px(56.))
                            .h(px(56.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(16.))
                            .bg(rgb(palette::accent()))
                            .child(
                                ico(IconName::Sparkles, Size::Large)
                                    .text_color(rgb(palette::WHITE)),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(18.))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(palette::text_primary()))
                                            .child(format!("等级 {level}")),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .py(px(1.))
                                            .rounded(px(10.))
                                            .bg(rgb(palette::accent()))
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(palette::WHITE))
                                            .child(format!("{count} 次加成")),
                                    ),
                            )
                            .child(bar)
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette::text_secondary()))
                                    .child(next_text),
                            ),
                    )
                    .child(toggle),
            ),
    );

    // 三级特权：已达成的卡片用品牌色描边并列出已解锁。
    let mut tiers = h_flex().w_full().gap_3();
    for (threshold, title, perks) in BOOST_PERKS.iter() {
        let reached = count >= *threshold;
        let left = threshold.saturating_sub(count);
        let mut list = v_flex().gap_1();
        for perk in perks.iter() {
            list = list.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        ico(
                            if reached {
                                IconName::Check
                            } else {
                                IconName::Circle
                            },
                            Size::XSmall,
                        )
                        .text_color(rgb(if reached {
                            palette::success()
                        } else {
                            palette::text_dim()
                        })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(if reached {
                                palette::text_secondary()
                            } else {
                                palette::text_muted()
                            }))
                            .child(*perk),
                    ),
            );
        }

        tiers = tiers.child(
            v_flex()
                .flex_1()
                .p(px(14.))
                .gap_3()
                .rounded(px(8.))
                .bg(rgb(palette::bg_sidebar()))
                .border_1()
                .border_color(rgb(if reached {
                    palette::accent()
                } else {
                    palette::border_default()
                }))
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(palette::text_primary()))
                                .child(*title),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_muted()))
                                .child(format!("{threshold} 次加成")),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(if reached {
                            palette::success()
                        } else {
                            palette::text_dim()
                        }))
                        .child(if reached {
                            "已解锁".to_string()
                        } else {
                            format!("还差 {left} 次")
                        }),
                )
                .child(list),
        );
    }
    page = page.child(tiers);

    // 加成者列表。
    let mut people = h_flex().flex_wrap().gap_2();
    for booster in boost.boosters.iter() {
        let letter = booster
            .name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .next()
            .unwrap_or('?');
        people = people.child(
            h_flex()
                .w(px(160.))
                .gap_2()
                .items_center()
                .px_2()
                .py(px(6.))
                .rounded(px(6.))
                .bg(rgb(palette::bg_sidebar()))
                .child(avatar(letter, booster.color, px(28.), true))
                .child(
                    v_flex()
                        .min_w_0()
                        .gap(px(1.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(palette::text_primary()))
                                .child(booster.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_muted()))
                                .child(booster.time_label()),
                        ),
                ),
        );
    }
    if count == 0 {
        people = people.child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::text_muted()))
                .child("还没有人加成，服务器停在等级 0。"),
        );
    }

    page.child(
        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(palette::text_primary()))
                            .child("加成者"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette::text_muted()))
                            .child(format!("共 {count} 人次")),
                    ),
            )
            .child(people)
            .child(hint("Demo 里只有你能加成或撤销，其他人是种子数据。")),
    )
}
