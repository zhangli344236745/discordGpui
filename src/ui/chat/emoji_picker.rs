use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::theme::palette;

use gpui_kit::component::{h_flex, v_flex};

/// 常用表情列表。点击后插入输入框草稿或作为消息回应。
const EMOJIS: &[&str] = &[
    "😀", "😂", "🥹", "😍", "🤔", "😎", "🥳", "😭", "👍", "👎", "👏", "🙌", "🤝", "💪", "🙏",
    "❤️", "🔥", "✨", "🎉", "💡", "🚀", "👀", "✅", "🎨", "😅", "🤣", "😊", "😉", "🥺", "😴",
];

/// 表情选择器弹层：绝对定位在输入框上方，点击表情触发 app.pick_emoji。
pub fn render(_app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let mut grid = v_flex().gap_1();

    for row in EMOJIS.chunks(8) {
        let mut line = h_flex().gap_1();
        for &emoji in row {
            let picked = emoji;
            line = line.child(
                div()
                    .w(px(34.))
                    .h(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .text_size(px(20.))
                    .hover(|el| el.bg(rgb(palette::bg_channel_hover())))
                    .id(format!("emoji-{picked}"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick_emoji(picked, window, cx);
                    })),
            );
        }
        grid = grid.child(line);
    }

    div()
        .absolute()
        .bottom(px(52.))
        .right(px(16.))
        .w(px(320.))
        .p_2()
        .rounded(px(10.))
        .bg(rgb(palette::bg_sidebar()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
        .child(grid)
}
