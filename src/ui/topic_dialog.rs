//! 频道主题编辑弹层：复用 create_dialog 的通用模态骨架。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::create_dialog::dialog;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{Size, Sizable};

/// 编辑频道主题：Enter 或点保存写回，取消关闭。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    dialog(
        "编辑频道主题",
        format!(
            "#{} 的主题会显示在标题栏，用一句话说明这个频道聊什么。",
            app.edit_topic_target
        ),
        "频道主题",
        "保存",
        IconName::SquarePen,
        <Input as Sizable>::with_size(Input::new(&app.topic_input), Size::Large).w_full(),
        "edit-topic",
        |this, window, cx| this.save_channel_topic(window, cx),
        |this, _, _| this.close_topic_editor(),
        cx,
    )
}
