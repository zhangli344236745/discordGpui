//! 服务器设置各分类内容：只渲染状态与调用业务方法，不持有自身状态。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

use crate::app::{DiscordApp, ServerSettingsSection};
use crate::ui::ico;
use crate::ui::theme::palette;

/// 可选的服务器图标颜色。
const SERVER_COLORS: [u32; 6] = [
    0x5865f2, 0xeb459e, 0x57f287, 0xfaa61c, 0xf23f43, 0x00a8fc,
];

/// 分类内容分发。各页面函数统一返回 Div，避免分支类型不一致。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    match app.server_settings_section {
        ServerSettingsSection::Overview => overview(app, cx),
        ServerSettingsSection::Roles => roles(app),
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

fn roles(app: &DiscordApp) -> Div {
    let groups = app.member_groups();
    let mut list = v_flex().w_full().gap(px(8.));

    for group in &groups {
        let names: Vec<&str> = group
            .member_indices
            .iter()
            .filter_map(|index| app.members.get(*index))
            .map(|member| member.name.split(" —— ").next().unwrap_or(member.name))
            .collect();

        list = list.child(
            h_flex()
                .w_full()
                .px(px(16.))
                .py(px(12.))
                .gap_3()
                .items_center()
                .rounded(px(8.))
                .bg(rgb(palette::bg_sidebar()))
                .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(rgb(group.color)))
                .child(
                    div()
                        .w(px(96.))
                        .flex_shrink_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(group.color))
                        .child(group.title.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(if names.is_empty() {
                            "暂无成员".to_string()
                        } else {
                            names.join("、")
                        }),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .px_2()
                        .h(px(20.))
                        .rounded_full()
                        .bg(rgb(palette::bg_rail()))
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_muted()))
                        .child(format!("{} 人", group.member_indices.len())),
                ),
        );
    }

    shell("角色", "角色按成员名后缀「 —— 角色」聚合，Demo 中只读写视图状态。").child(list)
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
    let code = app.invite_code();
    let copied = app.invite_copied;

    shell("邀请", "把链接发给朋友，他们就能加入这个服务器。")
        .child(
            card()
                .child(field_label("邀请链接"))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            h_flex()
                                .flex_1()
                                .min_w_0()
                                .h(px(32.))
                                .px_3()
                                .items_center()
                                .rounded(px(6.))
                                .bg(rgb(palette::bg_rail()))
                                .text_size(px(13.))
                                .text_color(rgb(palette::text_secondary()))
                                .child(format!("discord.gg/{code}")),
                        )
                        .child(
                            h_flex()
                                .h(px(32.))
                                .px_3()
                                .gap_1()
                                .items_center()
                                .rounded(px(6.))
                                .bg(if copied {
                                    rgb(palette::bg_rail())
                                } else {
                                    rgb(palette::accent())
                                })
                                .text_size(px(12.))
                                .text_color(if copied {
                                    rgb(palette::success())
                                } else {
                                    rgb(palette::WHITE)
                                })
                                .hover(|el| el.bg(rgb(palette::accent_hover())))
                                .child(ico(IconName::Copy, Size::XSmall))
                                .child(if copied { "已复制" } else { "复制链接" })
                                .id("server-invite-copy")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.copy_server_invite(cx);
                                })),
                        ),
                )
                .child(hint("链接由服务器名称生成：ASCII 名转小写，中文名用 gpui-N 兜底")),
        )
        .child(
            card()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("其他邀请方式"),
                )
                .child(
                    h_flex()
                        .h(px(32.))
                        .px_3()
                        .gap_2()
                        .items_center()
                        .rounded(px(6.))
                        .bg(rgb(palette::bg_rail()))
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_secondary()))
                        .hover(|el| {
                            el.bg(rgb(palette::bg_channel_hover()))
                                .text_color(rgb(palette::text_primary()))
                        })
                        .child(ico(IconName::UserPlus, Size::XSmall))
                        .child("打开邀请弹层")
                        .id("server-invite-open-dialog")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.open_invite(window, cx);
                        })),
                )
                .child(hint("打开弹层后复制链接会关闭设置页并回到主界面")),
        )
}
