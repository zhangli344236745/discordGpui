//! 服务器设置各分类内容：只渲染状态与调用业务方法，不持有自身状态。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

use crate::app::{DiscordApp, ROLE_COLORS, ServerSettingsSection};
use crate::data::InviteExpiry;
use crate::ui::theme::palette;
use crate::ui::{avatar, danger_icon_button, filter_chip, ico};

/// 可选的服务器图标颜色。
const SERVER_COLORS: [u32; 6] = [
    0x5865f2, 0xeb459e, 0x57f287, 0xfaa61c, 0xf23f43, 0x00a8fc,
];

/// 分类内容分发。各页面函数统一返回 Div，避免分支类型不一致。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    match app.server_settings_section {
        ServerSettingsSection::Overview => overview(app, cx),
        ServerSettingsSection::Roles => roles(app, cx),
        ServerSettingsSection::Emoji => emoji(app, cx),
        ServerSettingsSection::Invites => invites(app, cx),
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
