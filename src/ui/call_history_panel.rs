//! 通话记录面板：私信列表右上入口打开，展示最近 30 条本地记录。

use gpui_kit::*;

use crate::app::{CallMode, DiscordApp};
use crate::data::CallRecord;
use crate::ui::theme::palette;
use crate::ui::{avatar, ico};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 相对时间：刚刚 / N 分钟前 / N 小时前 / N 天前。
fn relative_time(at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(at);
    let delta = now.saturating_sub(at);
    if delta < 60 {
        "刚刚".into()
    } else if delta < 3600 {
        format!("{} 分钟前", delta / 60)
    } else if delta < 86_400 {
        format!("{} 小时前", delta / 3600)
    } else {
        format!("{} 天前", delta / 86_400)
    }
}

fn outcome_color(outcome: &str) -> u32 {
    match outcome {
        "connected" => palette::success(),
        "missed" => palette::danger(),
        _ => palette::text_dim(),
    }
}

fn row(record: &CallRecord, index: usize, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let friend = record.friend.clone();
    let mode = if record.video {
        CallMode::Video
    } else {
        CallMode::Voice
    };
    let label = format!(
        "{} · {}",
        if record.video { "视频" } else { "语音" },
        record.outcome_label()
    );
    let color = outcome_color(&record.outcome);
    let time = relative_time(record.at);
    let incoming = record.incoming;

    h_flex()
        .gap_2()
        .px_2()
        .py_2()
        .rounded(px(6.))
        .bg(rgb(palette::bg_sidebar()))
        .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
        .child(avatar(
            record.friend.chars().next().unwrap_or('?'),
            record.color,
            px(28.),
            false,
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
                        .text_color(rgb(palette::text_primary()))
                        .child(record.friend.clone()),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(ico(
                            if incoming {
                                IconName::PhoneIncoming
                            } else {
                                IconName::PhoneOutgoing
                            },
                            Size::XSmall,
                        ))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(color))
                                .child(label),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .child(time),
                        ),
                ),
        )
        .id(format!("call-history-{index}"))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.start_dm_call(friend.clone(), mode, cx);
            this.show_call_history = false;
            cx.notify();
        }))
}

pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    if !app.show_call_history {
        return div().into_any_element();
    }

    let mut list = v_flex()
        .gap_1()
        .max_h(px(320.))
        .id("call-history-scroll")
        .overflow_y_scroll();
    if app.call_history.is_empty() {
        list = list.child(
            div()
                .p_3()
                .text_size(px(12.))
                .text_color(rgb(palette::text_dim()))
                .child("还没有通话记录。"),
        );
    }
    for (index, record) in app.call_history.iter().enumerate() {
        list = list.child(row(record, index, cx));
    }

    v_flex()
        .absolute()
        .right(px(16.))
        .top(px(64.))
        .w(px(340.))
        .p_2()
        .gap_2()
        .rounded(px(10.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(
            h_flex()
                .px_1()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(palette::text_primary()))
                        .child("通话记录"),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded(px(4.))
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_dim()))
                                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                                .child("清空")
                                .id("call-history-clear")
                                .on_click(cx.listener(|this, _, _, cx| this.clear_call_history(cx))),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded(px(4.))
                                .text_size(px(11.))
                                .text_color(rgb(palette::text_secondary()))
                                .hover(|el| el.bg(rgb(palette::bg_toolbar_hover())))
                                .child("关闭")
                                .id("call-history-close")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_call_history = false;
                                    cx.notify();
                                })),
                        ),
                ),
        )
        .child(list)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::relative_time;

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default()
    }

    #[test]
    fn formats_relative_time() {
        let now = now();
        assert_eq!(relative_time(now), "刚刚");
        assert_eq!(relative_time(now.saturating_sub(120)), "2 分钟前");
        assert_eq!(relative_time(now.saturating_sub(7200)), "2 小时前");
        assert_eq!(relative_time(now.saturating_sub(2 * 86_400)), "2 天前");
    }
}
