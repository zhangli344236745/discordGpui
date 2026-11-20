//! 邀请好友弹层：展示当前服务器的邀请链接，提交即复制到剪贴板。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::create_dialog::dialog;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{Size, Sizable};

/// 邀请弹层：链接可编辑（方便测试自定义邀请码），点「复制链接」写入剪贴板。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    dialog(
        format!("邀请加入 {}", app.active_server_ref().name),
        "把链接发给朋友，对方点击即可加入这个服务器。",
        "邀请链接",
        "复制链接",
        IconName::UserPlus,
        <Input as Sizable>::with_size(Input::new(&app.invite_input), Size::Large).w_full(),
        "invite",
        |this, _, cx| this.copy_invite(cx),
        |this, _, _| this.close_invite(),
        cx,
    )
}
