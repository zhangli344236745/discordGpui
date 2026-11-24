//! 成员昵称弹层：复用 create_dialog 的通用模态骨架。
//! 留空保存等于清除昵称（回到原名），所以这里不额外提供「重置」按钮。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::create_dialog::dialog;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{Size, Sizable};

pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let target = app.member_nick_target.clone();
    let has_nick = app
        .members
        .iter()
        .find(|member| member.name == target)
        .map(|member| member.nick.is_some())
        .unwrap_or(false);
    let subtitle = if has_nick {
        format!("给 {target} 换一个昵称；留空保存即恢复原名。昵称只在服务器内展示。")
    } else {
        format!("给 {target} 设置一个昵称；留空保存即恢复原名。昵称只在服务器内展示。")
    };

    dialog(
        "修改服务器昵称",
        subtitle,
        "服务器昵称",
        "保存",
        IconName::Pen,
        <Input as Sizable>::with_size(Input::new(&app.member_nick_input), Size::Large).w_full(),
        "member-nick",
        |this, window, cx| this.save_member_nick(window, cx),
        |this, _, cx| this.close_member_nick(cx),
        cx,
    )
}
