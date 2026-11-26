//! 设置页各分类内容：只渲染状态与发起业务调用，不持有自身状态。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};
use gpui_kit::prelude::FluentBuilder as _;

use crate::app::{DiscordApp, SettingsSection, ThemeChoice};
use crate::ui::{avatar, toggle_row};
use crate::ui::ico;
use crate::ui::theme::palette;

/// 可选的头像颜色（与服务器图标色板同一套取值）。
const AVATAR_COLORS: [u32; 8] = [
    0x5865f2, 0xeb459e, 0x57f287, 0xfaa61c, 0xf23f43, 0x00a8fc, 0x9b59b6, 0x1abc9c,
];

/// 分类内容分发。各页面函数统一返回 Div，避免分支类型不一致。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    match app.settings_section {
        SettingsSection::Account => account(app, cx),
        SettingsSection::Privacy => privacy(app, cx),
        SettingsSection::Appearance => appearance(app, cx),
        SettingsSection::VoiceVideo => voice_video(app, cx),
        SettingsSection::Notifications => notifications(app, cx),
        SettingsSection::TextImage => text_image(app, cx),
        SettingsSection::Keybinds => keybinds(),
        SettingsSection::Language => language(),
        SettingsSection::Data => data(app, cx),
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

fn group(title: &'static str) -> Div {
    div().pt(px(8.)).child(crate::ui::section_label(title))
}

fn info_row(label: &'static str, value: impl Into<SharedString>) -> Div {
    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .px(px(16.))
        .py(px(12.))
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .child(
            div()
                .text_size(px(14.))
                .text_color(rgb(palette::text_secondary()))
                .child(label),
        )
        .child(
            div()
                .max_w(px(460.))
                .text_size(px(14.))
                .text_color(rgb(palette::text_primary()))
                .truncate()
                .child(value.into()),
        )
}

fn note_row(text: &'static str) -> Div {
    h_flex()
        .w_full()
        .gap(px(8.))
        .px(px(16.))
        .py(px(12.))
        .rounded(px(8.))
        .bg(rgb(palette::bg_sidebar()))
        .text_size(px(12.))
        .text_color(rgb(palette::text_muted()))
        .child(ico(IconName::Info, Size::XSmall))
        .child(text)
}

type ClickHandler = dyn Fn(&mut DiscordApp, &mut Window, &mut Context<DiscordApp>) + 'static;

fn small_button(
    symbol: &'static str,
    id: String,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    div()
        .w(px(26.))
        .h(px(26.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(rgb(palette::bg_channel_hover()))
        .text_size(px(14.))
        .text_color(rgb(palette::text_primary()))
        .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
        .child(symbol)
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))

}

/// 音量行：进度条 + 加减按钮，每次调整 5%。
fn volume_row(
    label: &'static str,
    value: u8,
    dec_id: String,
    inc_id: String,
    dec: Box<ClickHandler>,
    inc: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Div {
    let width = 160.0;
    let fill = width * value as f32 / 100.0;

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
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(palette::text_primary()))
                        .child(label),
                )
                .child(
                    div()
                        .w(px(width))
                        .h(px(6.))
                        .rounded_full()
                        .bg(rgb(palette::bg_toolbar_hover()))
                        .child(
                            div()
                                .w(px(fill))
                                .h(px(6.))
                                .rounded_full()
                                .bg(rgb(palette::accent())),
                        ),
                ),
        )
        .child(
            h_flex()
                .items_center()
                .gap(px(8.))
                .child(small_button("−", dec_id, dec, cx))
                .child(
                    h_flex()
                        .w(px(38.))
                        .justify_center()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(format!("{value}%")),
                )
                .child(small_button("+", inc_id, inc, cx)),
        )
}

// ---------------------------------------------------------------------------
// 各分类页面
// ---------------------------------------------------------------------------

fn account(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    // 头像颜色：与服务器图标色板同源的一套固定取值。
    let mut swatches = h_flex().gap_2();
    for (index, value) in AVATAR_COLORS.iter().enumerate() {
        let selected = *value == app.me.color;
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
                .id(format!("account-color-{index}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_avatar_color(picked, cx);
                })),
        );
    }

    let mut page = shell("账户", "管理你的个人信息与资料展示，改动保存后立刻反映到侧栏用户栏。");
    page = page.child(group("资料预览")).child(
        h_flex()
            .w_full()
            .gap(px(16.))
            .items_center()
            .px(px(16.))
            .py(px(20.))
            .rounded(px(8.))
            .bg(rgb(palette::bg_sidebar()))
            .child(avatar(app.me.initial(), app.me.color, px(72.), false))
            .child(
                v_flex()
                    .min_w_0()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(palette::text_primary()))
                            .child(app.me.display_name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette::text_muted()))
                            .child(app.me.username.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(rgb(palette::text_secondary()))
                            .child(if app.me.bio.is_empty() {
                                "还没有填写「关于我」".to_string()
                            } else {
                                app.me.bio.clone()
                            }),
                    )
                    .child(
                        h_flex()
                            .gap(px(6.))
                            .items_center()
                            .text_size(px(11.))
                            .text_color(rgb(palette::text_muted()))
                            .child(
                                div()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .rounded_full()
                                    .bg(rgb(app.my_status.color())),
                            )
                            .child(app.my_status.label()),
                    ),
            ),
    );

    page = page
        .child(group("头像颜色"))
        .child(swatches)
        .child(group("账户信息"))
        .child(form_field(
            "显示名称",
            "显示在消息、用户栏与资料卡上；留空会保存失败",
            <Input as Sizable>::with_size(Input::new(&app.account_name_input), Size::Small).w_full(),
        ))
        .child(form_field(
            "用户名",
            "全局唯一，好友可以通过用户名找到你",
            <Input as Sizable>::with_size(Input::new(&app.account_username_input), Size::Small)
                .w_full(),
        ))
        .child(form_field(
            "邮箱",
            "用于登录与通知，不会公开展示",
            <Input as Sizable>::with_size(Input::new(&app.account_email_input), Size::Small)
                .w_full(),
        ))
        .child(form_field(
            "关于我",
            "一句话介绍，显示在资料卡上",
            <Input as Sizable>::with_size(Input::new(&app.account_bio_input), Size::Small).w_full(),
        ))
        .child(info_row("手机号", "已绑定 · 不可修改（演示）"));

    let mut actions = h_flex().w_full().gap_2().items_center();
    actions = actions.child(action_button(
        "保存改动",
        true,
        "account-save",
        Box::new(|this, _, cx| this.save_account(cx)),
        cx,
    ));
    actions = actions.child(action_button(
        "重置",
        false,
        "account-reset",
        Box::new(|this, window, cx| this.reset_account_form(window, cx)),
        cx,
    ));
    if app.is_online() {
        actions = actions.child(action_button(
            "退出登录",
            false,
            "account-logout",
            Box::new(|this, _, cx| this.logout(cx)),
            cx,
        ));
    }
    if let Some(notice) = app.account_notice {
        actions = actions.child(
            div()
                .text_size(px(12.))
                .text_color(if notice == "已保存" {
                    rgb(palette::success())
                } else {
                    rgb(palette::text_muted())
                })
                .child(notice),
        );
    }

    page.child(actions)
        .child(note_row("资料只保存在内存里，重启应用会恢复初始值。"))
}

fn privacy(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("隐私与安全", "控制谁可以与你互动以及你分享的信息。");
    page = page.child(group("互动")).child(toggle_row(
        "允许 @提及",
        "被提及时在消息中高亮显示你的名字。",
        app.mention_pings,
        "settings-privacy-mentions",
        Box::new(|this, _, _| this.toggle_mention_pings()),
        cx,
    ));
    page = page.child(toggle_row(
        "私信通知",
        "收到新私信时显示系统通知。",
        app.notifications_on,
        "settings-privacy-dm-notify",
        Box::new(|this, _, _| this.toggle_notifications()),
        cx,
    ));
    page = page
        .child(group("说明"))
        .child(note_row("本 Demo 为纯前端模拟，数据仅保存在内存中。"));
    page
}

fn appearance(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("外观", "调整主题与消息的显示方式。");

    page = page.child(group("主题"));
    let mut themes = h_flex().w_full().gap(px(12.));
    let choices: [(ThemeChoice, IconName, &str); 3] = [
        (ThemeChoice::Dark, IconName::Moon, "dark"),
        (ThemeChoice::Light, IconName::Sun, "light"),
        (ThemeChoice::System, IconName::Monitor, "system"),
    ];
    for (choice, icon, slug) in choices {
        let selected = app.theme_choice == choice;
        themes = themes.child(
            v_flex()
                .w(px(120.))
                .h(px(84.))
                .gap(px(8.))
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .border_1()
                .border_color(if selected {
                    rgb(palette::accent())
                } else {
                    rgb(palette::border_default())
                })
                .bg(rgb(palette::bg_sidebar()))
                .text_color(if selected {
                    rgb(palette::text_link())
                } else {
                    rgb(palette::text_secondary())
                })
                .child(ico(icon, Size::Medium))
                .child(
                    div()
                        .text_size(px(13.))
                        .child(choice.label()),
                )
                .id(format!("settings-theme-{slug}"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.set_theme(choice, window, cx);
                })),
        );
    }
    page = page
        .child(themes)
        .child(note_row("深色 / 浅色会立即切换整界面配色；「跟随系统」在 Demo 中等同深色。"));

    page = page.child(group("消息")).child(toggle_row(
        "紧凑消息",
        "减少消息之间的间距，一屏显示更多内容。",
        app.compact_messages,
        "settings-appearance-compact",
        Box::new(|this, _, _| this.toggle_compact_messages()),
        cx,
    ));
    page = page.child(toggle_row(
        "显示时间戳",
        "在作者名旁显示消息发送时间。",
        app.show_timestamps,
        "settings-appearance-timestamps",
        Box::new(|this, _, _| this.toggle_show_timestamps()),
        cx,
    ));
    page = page.child(
        div()
            .mt(px(8.))
            .px(px(16.))
            .py(px(8.))
            .w(px(140.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .bg(rgb(palette::danger()))
            .text_color(rgb(palette::WHITE))
            .text_size(px(13.))
            .child("恢复默认设置")
            .id("settings-reset")
            .on_click(cx.listener(|this, _, _, cx| {
                this.reset_settings();
                cx.notify();
            })),
    );
    page
}

/// 设备行：左侧标签 + 当前设备名，右侧「切换」按钮。
fn device_row(
    label: &'static str,
    value: Option<String>,
    id: String,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Div {
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
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(palette::text_primary()))
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_secondary()))
                        .child(value.unwrap_or_else(|| "未检测到设备".into())),
                ),
        )
        .child(small_button("切换", id, handler, cx))
}

fn voice_video(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("声音与视频", "选择设备并调整输入与输出音量。");

    page = page.child(group("设备")).child(
        v_flex()
            .gap(px(8.))
            .child(device_row(
                "输入设备",
                app.voice_input_device.as_ref().map(|(_, name)| name.clone()),
                "settings-input-device".into(),
                Box::new(|this, _, cx| this.cycle_input_device(cx)),
                cx,
            ))
            .child(device_row(
                "输出设备",
                app.voice_output_device.as_ref().map(|(_, name)| name.clone()),
                "settings-output-device".into(),
                Box::new(|this, _, cx| this.cycle_output_device(cx)),
                cx,
            ))
            .child(
                div()
                    .px(px(16.))
                    .text_size(px(12.))
                    .text_color(rgb(palette::text_dim()))
                    .child(
                        app.voice_device_note
                            .clone()
                            .unwrap_or_else(|| {
                                "设备列表在加入语音后可用；若系统拒绝麦克风权限，请到 系统设置 → 隐私与安全性 → 麦克风 开启。"
                                    .into()
                            }),
                    ),
            ),
    );

    page = page.child(group("音量")).child(volume_row(
        "输入音量",
        app.input_volume,
        "settings-input-volume-dec".into(),
        "settings-input-volume-inc".into(),
        Box::new(|this, _, _| this.adjust_input_volume(-5)),
        Box::new(|this, _, _| this.adjust_input_volume(5)),
        cx,
    ));
    page = page.child(volume_row(
        "输出音量",
        app.output_volume,
        "settings-output-volume-dec".into(),
        "settings-output-volume-inc".into(),
        Box::new(|this, _, _| this.adjust_output_volume(-5)),
        Box::new(|this, _, _| this.adjust_output_volume(5)),
        cx,
    ));
    page = page.child(group("通知")).child(toggle_row(
        "通知音效",
        "收到消息或来电时播放提示音。",
        app.notification_sound,
        "settings-voice-sound",
        Box::new(|this, _, _| this.toggle_notification_sound()),
        cx,
    ));
    page
}

fn notifications(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("通知", "选择你希望收到哪些提醒。");
    page = page.child(toggle_row(
        "桌面通知",
        "在新消息到达时显示系统通知。",
        app.notifications_on,
        "settings-notify-desktop",
        Box::new(|this, _, _| this.toggle_notifications()),
        cx,
    ));
    page = page.child(toggle_row(
        "通知音效",
        "播放提示音前检查系统音量。",
        app.notification_sound,
        "settings-notify-sound",
        Box::new(|this, _, _| this.toggle_notification_sound()),
        cx,
    ));
    page = page.child(toggle_row(
        "@提及提醒",
        "被其他成员提及时特别提醒。",
        app.mention_pings,
        "settings-notify-mentions",
        Box::new(|this, _, _| this.toggle_mention_pings()),
        cx,
    ));
    page
}

fn text_image(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("文字与图片", "控制聊天区中内容与媒体的展示。");
    page = page.child(toggle_row(
        "显示表情回应",
        "在消息下方显示已有的表情回应。",
        app.show_emoji,
        "settings-text-emoji",
        Box::new(|this, _, _| this.toggle_show_emoji()),
        cx,
    ));
    page = page.child(toggle_row(
        "显示时间戳",
        "在消息作者旁显示发送时间。",
        app.show_timestamps,
        "settings-text-timestamps",
        Box::new(|this, _, _| this.toggle_show_timestamps()),
        cx,
    ));
    page = page
        .child(group("说明"))
        .child(note_row("图片与附件的自动预览将在后续版本支持。"));
    page
}

fn keybinds() -> Div {
    let rows: [(&str, &str); 11] = [
        ("Enter", "发送消息 / 保存消息编辑 / 发送话题回复 / 发送首个搜索结果"),
        ("Shift + Enter", "输入框内换行"),
        ("Ctrl/⌘ + K", "打开频道 / 私信快速切换器"),
        ("↑ / ↓", "在快速切换器里循环选择目标"),
        ("Esc", "按最上层优先关闭浮层或页面"),
        ("点击作者名", "打开用户资料卡"),
        ("点击齿轮", "打开用户设置"),
        ("点击 ⋯", "打开消息操作菜单"),
        ("右键消息", "打开消息菜单 / 创建话题"),
        ("点击 GIF 图标", "搜索并发送 GIF"),
        ("点击标题栏 ?", "打开帮助中心"),
    ];
    let mut page = shell("快捷键", "常用操作对应的键盘快捷键。").child(group("键盘"));
    for (keys, desc) in rows {
        page = page.child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .px(px(16.))
                .py(px(12.))
                .rounded(px(8.))
                .bg(rgb(palette::bg_sidebar()))
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(palette::text_message()))
                        .child(desc),
                )
                .child(
                    div()
                        .px(px(8.))
                        .py(px(4.))
                        .rounded(px(4.))
                        .bg(rgb(palette::bg_channel_hover()))
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_primary()))
                        .child(keys),
                ),
        );
    }
    page
}

fn language() -> Div {
    let mut page = shell("语言", "选择界面显示语言。").child(group("界面语言"));
    for (name, selected) in [("简体中文（中国大陆）", true), ("English (US)", false)] {
        page = page.child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .px(px(16.))
                .py(px(14.))
                .rounded(px(8.))
                .bg(rgb(palette::bg_sidebar()))
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(if selected {
                            rgb(palette::text_primary())
                        } else {
                            rgb(palette::text_muted())
                        })
                        .child(name),
                )
                .when(selected, |el| {
                    el.child(ico(IconName::Check, Size::Small).text_color(rgb(palette::accent())))
                }),
        );
    }
    page = page.child(note_row("更多语言将在后续版本加入。"));
    page
}

/// 带标题与说明的输入行。
/// 数据与存储：本地快照的位置、规模，以及手动保存 / 恢复示例数据。
fn data(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let (messages, channels) = app.snapshot_size();
    let note = app
        .persist_note
        .unwrap_or("每次内容变化后会自动写入快照，无需手动保存");

    let mut page = shell(
        "数据与存储",
        "消息、频道、好友与个人资料保存在本机 JSON 快照里，重启后自动恢复。",
    )
    .child(group("本地快照"));

    page = page
        .child(info_row("快照文件", app.snapshot_path()))
        .child(info_row(
            "内容规模",
            format!("{messages} 条消息 · {channels} 个频道"),
        ))
        .child(info_row("自动保存", "已开启（内容变化即写入）"))
        .child(info_row("最近一次操作", note));

    page = page.child(group("维护")).child(
        h_flex()
            .w_full()
            .gap(px(12.))
            .child(action_button(
                "立即保存",
                true,
                "settings-data-save",
                Box::new(|this, _, cx| this.persist_now(cx)),
                cx,
            ))
            .child(action_button(
                "恢复示例数据",
                false,
                "settings-data-reset",
                Box::new(|this, window, cx| this.reset_local_data(window, cx)),
                cx,
            )),
    );

    page.child(note_row(
        "恢复示例数据会删除本地快照并清空所有改动（消息、频道、好友、资料），且无法撤销。",
    ))
    .child(note_row(
        "浮层展开状态、输入框焦点、正在播放的语音连接不做持久化；草稿与通话状态也只在本次运行内有效。",
    ))
}

fn form_field(label: &'static str, hint: &'static str, input: impl IntoElement) -> Div {
    v_flex()
        .w_full()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_secondary()))
                .child(label),
        )
        .child(
            div()
                .w_full()
                .rounded(px(6.))
                .bg(rgb(palette::bg_composer()))
                .child(input),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_muted()))
                .child(hint),
        )
}

/// 表单动作按钮：主按钮用品牌色。
fn action_button(
    label: &'static str,
    primary: bool,
    id: &'static str,
    handler: Box<ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> Stateful<Div> {
    div()
        .h(px(34.))
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD)
        .bg(rgb(if primary {
            palette::accent()
        } else {
            palette::bg_channel_hover()
        }))
        .text_color(rgb(if primary {
            palette::WHITE
        } else {
            palette::text_primary()
        }))
        .hover(|el| {
            el.bg(rgb(if primary {
                palette::accent_hover()
            } else {
                palette::bg_toolbar_hover()
            }))
        })
        .child(label)
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}
