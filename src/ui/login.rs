//! 启动 / 登录界面：健康检查、登录、注册与首屏同步态。

use gpui_kit::*;
use gpui_kit::component::input::Input;
use gpui_kit::component::{h_flex, v_flex, Size, Sizable};

use crate::app::{AuthStatus, DiscordApp, LoginMode};
use crate::ui::theme::palette;

pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let register = app.login_mode == LoginMode::Register;
    let (title, subtitle) = match app.auth_status {
        AuthStatus::Checking => ("正在连接", "检查后端是否可达…"),
        AuthStatus::Booting => ("正在同步", "拉取账号、服务器与私信数据"),
        AuthStatus::Login if register => ("创建账号", "只需要邮箱、密码和用户名"),
        AuthStatus::Login => ("欢迎回来", "登录后开始聊天"),
        AuthStatus::Demo | AuthStatus::Ready => ("Discord", ""),
    };

    let mut card = v_flex()
        .w(px(420.))
        .gap_4()
        .p_6()
        .rounded(px(12.))
        .bg(rgb(palette::bg_chat()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_size(px(22.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(palette::text_muted()))
                        .child(subtitle),
                ),
        );

    match app.auth_status {
        AuthStatus::Checking => {
            card = card.child(
                v_flex()
                    .gap_2()
                    .py_4()
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(rgb(palette::text_secondary()))
                            .child("正在探测 /api/v1/health…"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette::text_dim()))
                            .child("如果后端未启动，会自动进入离线演示模式。"),
                    ),
            );
        }
        AuthStatus::Booting => {
            card = card.child(
                v_flex()
                    .gap_2()
                    .py_4()
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(rgb(palette::text_secondary()))
                            .child(app.boot_message.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette::text_dim()))
                            .child("同步完成后进入主界面。"),
                    ),
            );
        }
        AuthStatus::Login => {
            card = card
                .child(field(
                    "邮箱",
                    <Input as Sizable>::with_size(Input::new(&app.login_email), Size::Small).w_full(),
                ))
                .child(field(
                    "密码",
                    <Input as Sizable>::with_size(Input::new(&app.login_password), Size::Small)
                        .w_full(),
                ));
            if register {
                card = card.child(field(
                    "用户名",
                    <Input as Sizable>::with_size(Input::new(&app.login_username), Size::Small)
                        .w_full(),
                ));
            }
            if let Some(error) = app.login_error.clone() {
                card = card.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::danger()))
                        .child(error),
                );
            }
            card = card
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .id("login-remember")
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_login_remember(cx)))
                        .child(
                            div()
                                .w(px(16.))
                                .h(px(16.))
                                .rounded(px(3.))
                                .border_1()
                                .border_color(rgb(if app.login_remember {
                                    palette::accent()
                                } else {
                                    palette::border_default()
                                }))
                                .bg(rgb(if app.login_remember {
                                    palette::accent()
                                } else {
                                    palette::bg_composer()
                                }))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.))
                                .text_color(rgb(palette::WHITE))
                                .child(if app.login_remember { "✓" } else { "" }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette::text_secondary()))
                                .child("记住我（保存 refresh token）"),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(38.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.))
                        .bg(rgb(if app.login_busy {
                            palette::bg_composer()
                        } else {
                            palette::accent()
                        }))
                        .text_color(rgb(palette::WHITE))
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .id("login-submit")
                        .on_click(cx.listener(|this, _, _, cx| this.submit_auth(cx)))
                        .child(if app.login_busy {
                            "请稍候…"
                        } else if register {
                            "注册并进入"
                        } else {
                            "登录"
                        }),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette::text_muted()))
                                .child(if register { "已有账号？" } else { "还没有账号？" }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette::accent()))
                                .id("login-toggle-mode")
                                .on_click(cx.listener(|this, _, _, cx| this.toggle_login_mode(cx)))
                                .child(if register { "去登录" } else { "注册" }),
                        ),
                );
            if app
                .login_error
                .as_deref()
                .is_some_and(|error| error.starts_with("首屏同步"))
            {
                card = card.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette::accent()))
                        .id("login-retry-boot")
                        .on_click(cx.listener(|this, _, _, cx| this.retry_boot(cx)))
                        .child("重试首屏同步"),
                );
            }
        }
        AuthStatus::Demo | AuthStatus::Ready => {}
    }

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(palette::bg_sidebar()))
        .text_color(rgb(palette::text_message()))
        .child(card)
        .into_any_element()
}

fn field(label: &'static str, input: impl IntoElement) -> Div {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(palette::text_secondary()))
                .child(label),
        )
        .child(input)
}
