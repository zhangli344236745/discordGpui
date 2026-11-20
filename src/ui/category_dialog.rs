//! 分类弹层：新建分类 / 重命名分类，复用 create_dialog 的通用模态骨架。
//! 两者共用 `category_input`，按 `show_create_category` 决定文案与提交动作。

use gpui_kit::*;

use crate::app::DiscordApp;
use crate::ui::create_dialog::dialog;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::{Size, Sizable};

/// 渲染分类弹层：新建或重命名二选一，由 app 层状态决定。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement {
    let creating = app.show_create_category;
    let title = if creating {
        "新建分类"
    } else {
        "重命名分类"
    };
    let subtitle = if creating {
        String::from("分类用于把频道分组，创建后可以随时重命名或删除。")
    } else {
        format!(
            "分类「{}」下有 {} 个频道，改名不会影响其中的频道。",
            app.rename_category_source,
            app.category_channel_count(&app.rename_category_source)
        )
    };

    dialog(
        title,
        subtitle,
        "分类名称",
        if creating { "创建" } else { "保存" },
        IconName::FolderPlus,
        <Input as Sizable>::with_size(Input::new(&app.category_input), Size::Large).w_full(),
        // 两种模式用不同的元素 ID 前缀，避免切换弹层时复用旧的输入焦点状态。
        if creating {
            "create-category"
        } else {
            "rename-category"
        },
        move |this, window, cx| {
            if creating {
                this.create_category(window, cx);
            } else {
                this.save_category_rename(window, cx);
            }
        },
        move |this, _, _| {
            if creating {
                this.close_create_category();
            } else {
                this.close_category_rename();
            }
        },
        cx,
    )
}
