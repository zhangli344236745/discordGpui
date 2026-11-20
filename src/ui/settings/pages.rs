//! 设置页各分类内容：只渲染状态与发起业务调用，不持有自身状态。

use gpui_kit::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

use crate::app::{DiscordApp, SettingsSection, ThemeChoice};
use crate::ui::avatar;
use crate::ui::ico;
use crate::ui::theme::palette;

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
    div()
        .pt(px(8.))
        .text_size(px(12.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(palette::text_muted()))
        .child(title)
}

fn info_row(label: &'static str, value: &'static str) -> Div {
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
                .text_size(px(14.))
                .text_color(rgb(palette::text_primary()))
                .child(value),
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

/// 一行开关：左侧文案 + 右侧拨动条，点击整行生效。
fn switch_row(
    label: &'static str,
    desc: &'static str,
    on: bool,
    id: String,
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
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::text_muted()))
                        .child(desc),
                ),
        )
        .child(track.child(knob))
        .id(id)
        .on_click(cx.listener(move |this, _, window, cx| {
            handler(this, window, cx);
            cx.notify();
        }))
}

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

fn account(app: &DiscordApp, _cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("账户", "管理你的个人信息与资料展示。");
    page = page.child(group("个人资料")).child(
        v_flex()
            .w_full()
            .items_center()
            .gap(px(8.))
            .px(px(16.))
            .py(px(24.))
            .rounded(px(8.))
            .bg(rgb(palette::bg_sidebar()))
            .child(avatar('R', palette::accent(), px(72.), false))
            .child(
                div()
                    .text_size(px(20.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(palette::text_primary()))
                    .child("Riven"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette::text_muted()))
                    .child(format!("当前状态：{}", app.my_status.label())),
            ),
    );
    page = page
        .child(group("账户信息"))
        .child(info_row("用户名", "riven#0001"))
        .child(info_row("邮箱", "riven@example.com"))
        .child(info_row("手机号", "已绑定"));
    page
}

fn privacy(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("隐私与安全", "控制谁可以与你互动以及你分享的信息。");
    page = page.child(group("互动")).child(switch_row(
        "允许 @提及",
        "被提及时在消息中高亮显示你的名字。",
        app.mention_pings,
        "settings-privacy-mentions".into(),
        Box::new(|this, _, _| this.toggle_mention_pings()),
        cx,
    ));
    page = page.child(switch_row(
        "私信通知",
        "收到新私信时显示系统通知。",
        app.notifications_on,
        "settings-privacy-dm-notify".into(),
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

    page = page.child(group("消息")).child(switch_row(
        "紧凑消息",
        "减少消息之间的间距，一屏显示更多内容。",
        app.compact_messages,
        "settings-appearance-compact".into(),
        Box::new(|this, _, _| this.toggle_compact_messages()),
        cx,
    ));
    page = page.child(switch_row(
        "显示时间戳",
        "在作者名旁显示消息发送时间。",
        app.show_timestamps,
        "settings-appearance-timestamps".into(),
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

fn voice_video(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("声音与视频", "调整输入与输出设备音量。");
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
    page = page.child(group("通知")).child(switch_row(
        "通知音效",
        "收到消息时播放提示音。",
        app.notification_sound,
        "settings-voice-sound".into(),
        Box::new(|this, _, _| this.toggle_notification_sound()),
        cx,
    ));
    page
}

fn notifications(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("通知", "选择你希望收到哪些提醒。");
    page = page.child(switch_row(
        "桌面通知",
        "在新消息到达时显示系统通知。",
        app.notifications_on,
        "settings-notify-desktop".into(),
        Box::new(|this, _, _| this.toggle_notifications()),
        cx,
    ));
    page = page.child(switch_row(
        "通知音效",
        "播放提示音前检查系统音量。",
        app.notification_sound,
        "settings-notify-sound".into(),
        Box::new(|this, _, _| this.toggle_notification_sound()),
        cx,
    ));
    page = page.child(switch_row(
        "@提及提醒",
        "被其他成员提及时特别提醒。",
        app.mention_pings,
        "settings-notify-mentions".into(),
        Box::new(|this, _, _| this.toggle_mention_pings()),
        cx,
    ));
    page
}

fn text_image(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> Div {
    let mut page = shell("文字与图片", "控制聊天区中内容与媒体的展示。");
    page = page.child(switch_row(
        "显示表情回应",
        "在消息下方显示已有的表情回应。",
        app.show_emoji,
        "settings-text-emoji".into(),
        Box::new(|this, _, _| this.toggle_show_emoji()),
        cx,
    ));
    page = page.child(switch_row(
        "显示时间戳",
        "在消息作者旁显示发送时间。",
        app.show_timestamps,
        "settings-text-timestamps".into(),
        Box::new(|this, _, _| this.toggle_show_timestamps()),
        cx,
    ));
    page = page
        .child(group("说明"))
        .child(note_row("图片与附件的自动预览将在后续版本支持。"));
    page
}

fn keybinds() -> Div {
    let rows: [(&str, &str); 5] = [
        ("Enter", "发送消息 / 保存消息编辑"),
        ("Shift + Enter", "输入框内换行"),
        ("点击作者名", "打开用户资料卡"),
        ("点击齿轮", "打开用户设置"),
        ("点击 ⋯", "打开消息操作菜单"),
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
