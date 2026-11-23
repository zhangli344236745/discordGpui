//! 好友分组弹层：新建分组 / 重命名分组，复用 create_dialog 的通用模态骨架。
//! 两者共用 `group_input`，按 `show_create_group` 决定文案与提交动作。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::create_dialog::dialog;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{Size, Sizable};

/// 渲染分组弹层：新建或重命名二选一，由 app 层状态决定。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let creating = app.show_create_group;
    let title = if creating { "新建分组" } else { "重命名分组" };
    let subtitle = if creating {
        String::from("分组只影响好友列表的展示顺序，随时可以重命名或删除。")
    } else {
        format!(
            "分组「{}」下有 {} 位好友，改名不会影响成员关系。",
            app.rename_group_source,
            app.friends_in_group(&app.rename_group_source).len()
        )
    };

    dialog(
        title,
        subtitle,
        "分组名称",
        if creating { "创建" } else { "保存" },
        IconName::FolderPlus,
        <Input as Sizable>::with_size(Input::new(&app.group_input), Size::Large).w_full(),
        // 两种模式用不同的元素 ID 前缀，避免切换弹层时复用旧的输入焦点状态。
        if creating {
            "create-friend-group"
        } else {
            "rename-friend-group"
        },
        move |this, window, cx| {
            if creating {
                this.create_group(window, cx);
            } else {
                this.save_group_rename(window, cx);
            }
        },
        move |this, _, cx| this.close_group_dialog(cx),
        cx,
    )
}
