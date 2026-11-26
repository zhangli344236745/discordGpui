//! 私信语音 / 视频通话浮窗：跨会话与视图保持，直到挂断或改连服务器语音。
//! Demo 用「模拟接通」推进呼叫状态，画面区是占位块，不做真实音视频采集。

use gpui_kit::*;

use crate::app::{CallMode, DiscordApp};
use crate::ui::theme::palette;
use crate::ui::{avatar, danger_icon_button, icon_button, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};
use gpui_kit::prelude::FluentBuilder as _;

/// 通话控件：图标在上、文字在下，统一走公共图标按钮。
fn control(
    icon: IconName,
    label: &'static str,
    id: impl Into<ElementId>,
    active: bool,
    danger: bool,
    handler: Box<crate::ui::widgets::ClickHandler>,
    cx: &mut Context<DiscordApp>,
) -> impl IntoElement {
    let button = if danger {
        danger_icon_button(icon, Size::Small, px(32.), id, handler, cx)
            .bg(rgb(palette::danger()))
            .text_color(rgb(palette::WHITE))
            .into_any_element()
    } else {
        icon_button(icon, Size::Small, px(32.), id, active, handler, cx).into_any_element()
    };

    v_flex()
        .flex_1()
        .items_center()
        .gap_1()
        .child(button)
        .child(
            div()
                .text_size(px(10.))
                .text_color(if danger {
                    rgb(palette::danger())
                } else {
                    rgb(palette::text_dim())
                })
                .child(label),
        )
}

/// 本地预览小窗的拖拽载荷。
struct PipDrag;

struct PipGhost;

impl Render for PipGhost {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .w(px(96.))
            .h(px(54.))
            .rounded(px(4.))
            .bg(rgba(0x00000066))
            .border_1()
            .border_color(rgb(palette::text_primary()))
    }
}

/// 视频/屏幕分享预览区：只画占位块，明确标注 Demo 语义。
fn preview(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let Some(call) = app.dm_call.as_ref() else {
        return div().into_any_element();
    };

    if !call.connected {
        return v_flex()
            .h(px(112.))
            .items_center()
            .justify_center()
            .gap_2()
            .bg(rgb(palette::bg_rail()))
            .child(avatar(
                call.friend.chars().next().unwrap_or('?'),
                call.color,
                px(48.),
                call.online,
            ))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette::text_secondary()))
                    .child(format!("正在呼叫 @{}…", call.friend)),
            )
            .into_any_element();
    }

    // 远端真实画面优先：屏幕 > 摄像头；本端画面以画中画小窗叠加
    let (tiles, _) = app.video_tiles(6);
    let remote = tiles.iter().find(|tile| !tile.local).cloned();
    let local = tiles.iter().find(|tile| tile.local).cloned();

    if let Some(remote) = remote {
        let remote_key = (remote.identity.clone(), remote.kind.clone());
        let focused = app.video_focus.as_ref() == Some(&remote_key);
        let stage_height = if focused { 260.0 } else { 180.0 };
        let key_for_click = remote_key.clone();
        let mut stage = div()
            .relative()
            .w_full()
            .h(px(stage_height))
            .overflow_hidden()
            .id("dm-video-stage")
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_video_focus(&key_for_click.0, &key_for_click.1, cx)
            }))
            .child(
                img(ImageSource::Render(remote.frame.clone()))
                    .w_full()
                    .h(px(stage_height))
                    .object_fit(ObjectFit::Cover),
            )
            .child(
                div()
                    .absolute()
                    .left(px(8.))
                    .top(px(8.))
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(3.))
                    .bg(rgb(palette::bg_rail()))
                    .text_size(px(10.))
                    .text_color(rgb(palette::text_primary()))
                    .child(if focused { "点击还原" } else { "点击放大" }),
            );
        if let Some(local) = local {
            let (pip_x, pip_y) = app.pip_position;
            stage = stage
                .on_drag_move::<PipDrag>(cx.listener(
                    |this, event: &DragMoveEvent<PipDrag>, _window, cx| {
                    let size = event.bounds.size;
                    if size.width > px(0.) && size.height > px(0.) {
                        let offset = event.event.position - event.bounds.origin;
                        this.set_pip_position(offset.x / size.width, offset.y / size.height, cx);
                    }
                },
                ))
                .child(
                    div()
                        .id("dm-call-pip")
                        .absolute()
                        .left(relative(pip_x))
                        .top(relative(pip_y))
                        .ml(px(-48.))
                        .mt(px(-27.))
                        .w(px(96.))
                        .h(px(54.))
                        .rounded(px(4.))
                        .overflow_hidden()
                        .border_1()
                        .border_color(rgb(palette::text_primary()))
                        .on_drag(PipDrag, |_drag, _offset, _window, cx| {
                            cx.new(|_| PipGhost)
                        })
                        // 吞掉点击，避免拖拽小窗时触发舞台的放大/还原
                        .on_click(cx.listener(|_, _, _, _| {}))
                        .child(
                            img(ImageSource::Render(local.frame.clone()))
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Cover),
                        ),
                );
        }
        return stage.into_any_element();
    }

    // 没有远端画面但有本端预览：直接展示本端
    if let Some(local) = local {
        return div()
            .relative()
            .w_full()
            .h(px(180.))
            .overflow_hidden()
            .child(
                img(ImageSource::Render(local.frame.clone()))
                    .w_full()
                    .h(px(180.))
                    .object_fit(ObjectFit::Cover),
            )
            .child(
                div()
                    .absolute()
                    .left(px(8.))
                    .top(px(8.))
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(3.))
                    .bg(rgb(palette::bg_rail()))
                    .text_size(px(10.))
                    .text_color(rgb(palette::text_primary()))
                    .child("你"),
            )
            .into_any_element();
    }

    if call.screen_sharing {
        return v_flex()
            .h(px(160.))
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .bg(rgb(palette::bg_chat()))
            .child(ico(IconName::ScreenShare, Size::Large))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette::text_dim()))
                    .child("屏幕分享中，等待对方画面…"),
            )
            .into_any_element();
    }

    if call.camera_on {
        return v_flex()
            .h(px(160.))
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .bg(rgb(palette::bg_chat()))
            .child(avatar(
                call.friend.chars().next().unwrap_or('?'),
                call.color,
                px(56.),
                call.online,
            ))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette::text_dim()))
                    .child("摄像头已开启，等待对方画面…"),
            )
            .into_any_element();
    }

    v_flex()
        .h(px(112.))
        .items_center()
        .justify_center()
        .gap_2()
        .bg(rgb(palette::bg_rail()))
        .child(ico(IconName::VideoOff, Size::Large))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette::text_secondary()))
                .child(format!("与 @{} 通话中 · 摄像头已关闭", call.friend)),
        )
        .id("dm-call-preview-off")
        .on_click(cx.listener(|this, _, _, cx| this.toggle_dm_call_camera(cx)))
        .into_any_element()
}

/// 通话浮窗：呼叫中显示「模拟接通 / 取消」，接通后显示麦克风、摄像头、屏幕分享与挂断。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(call) = app.dm_call.as_ref() else {
        return div().into_any_element();
    };
    let friend = call.friend.clone();
    let mode = call.mode;
    let connected = call.connected;
    let incoming = call.incoming;
    let mic_on = call.mic_on;
    let camera_on = call.camera_on;
    let screen_sharing = call.screen_sharing;
    let online = call.online;

    let header = h_flex()
        .h(px(44.))
        .flex_shrink_0()
        .px_3()
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(rgb(palette::bg_header_border()))
        .child(ico(
            if mode == CallMode::Video {
                IconName::Video
            } else {
                IconName::PhoneCall
            },
            Size::Small,
        ))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    div()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child(format!("{} · {friend}", mode.label())),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(
                            div()
                                .w(px(6.))
                                .h(px(6.))
                                .rounded_full()
                                .bg(rgb(if connected {
                                    palette::success()
                                } else {
                                    palette::text_dim()
                                })),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(if connected {
                                    palette::success()
                                } else {
                                    palette::text_muted()
                                }))
                                .child(if connected {
                                    "通话中"
                                } else if incoming {
                                    "来电"
                                } else {
                                    "正在呼叫"
                                }),
                        ),
                ),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette::text_dim()))
                .child(if online { "在线" } else { "离线" }),
        );

    let footer = if connected {
        h_flex()
            .flex_shrink_0()
            .px_3()
            .py_3()
            .gap_2()
            .child(control(
                if mic_on { IconName::Mic } else { IconName::MicOff },
                if mic_on { "麦克风" } else { "已静音" },
                "dm-call-toggle-mic",
                mic_on,
                false,
                Box::new(|this, _, cx| this.toggle_dm_call_mic(cx)),
                cx,
            ))
            .child(control(
                if camera_on {
                    IconName::Video
                } else {
                    IconName::VideoOff
                },
                if camera_on { "摄像头" } else { "关画面" },
                "dm-call-toggle-camera",
                camera_on,
                false,
                Box::new(|this, _, cx| this.toggle_dm_call_camera(cx)),
                cx,
            ))
            .child(control(
                if screen_sharing {
                    IconName::ScreenShareOff
                } else {
                    IconName::ScreenShare
                },
                if screen_sharing { "停分享" } else { "共享屏" },
                "dm-call-toggle-screen",
                screen_sharing,
                false,
                Box::new(|this, _, cx| this.toggle_dm_call_screen(cx)),
                cx,
            ))
            .child(control(
                IconName::PhoneOff,
                "挂断",
                "dm-call-hangup",
                true,
                true,
                Box::new(|this, _, cx| this.end_dm_call(cx)),
                cx,
            ))
    } else {
        h_flex()
            .flex_shrink_0()
            .px_3()
            .py_3()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .h(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .bg(rgb(palette::accent()))
                    .text_color(rgb(palette::WHITE))
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .hover(|el| el.bg(rgb(palette::accent_hover())))
                    .child(if incoming { "接听" } else { "等待接听…" })
                    .when(!incoming, |el| el.opacity(0.6))
                    .id("dm-call-connect")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if incoming {
                            this.connect_dm_call(cx);
                        }
                    })),
            )
            .child(control(
                IconName::PhoneOff,
                if incoming { "拒绝" } else { "取消" },
                "dm-call-cancel",
                true,
                true,
                Box::new(move |this, _, cx| {
                    if incoming {
                        this.decline_dm_call(cx);
                    } else {
                        this.end_dm_call(cx);
                    }
                }),
                cx,
            ))
    };

    div()
        .absolute()
        .right(px(24.))
        .bottom(px(72.))
        .w(px(320.))
        .rounded(px(10.))
        .overflow_hidden()
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .id("dm-call-panel")
        .child(header)
        .child(preview(app, cx))
        .child(footer)
        .child(
            div()
                .px_3()
                .pb_2()
                .text_size(px(10.))
                .text_color(rgb(palette::text_dim()))
                .child(format!("@{friend} · 语音/视频通话")),
        )
        .into_any_element()
}
