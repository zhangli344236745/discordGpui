//! 好友分组菜单：两个独立浮层，都用「遮罩 + 卡片」兄弟节点的写法。
//!
//! - `render`：分组操作（重命名 / 上移 / 下移 / 删除），右键分组标题或点行尾省略号触发。
//! - `render_picker`：把某个好友移动到哪个分组，点好友行的文件夹按钮触发。

use gpui_kit::*;

use crate::app::{DiscordApp, MenuAnchor};
use crate::ui::theme::palette;
use crate::ui::{ico, menu_divider, menu_item};

use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, v_flex, Size};

/// 浮层卡片外框：位置由锚点换算，宽度固定，样式与频道 / 服务器菜单保持一致。
fn card_at(anchor: MenuAnchor, width: f32) -> Div {
    let (x, y) = match anchor {
        MenuAnchor::Point(x, y) => (x, y),
        // 分组菜单只有右键与行尾按钮两个入口，理论上不会走这里；留一个兜底位置。
        MenuAnchor::Header => (620., 120.),
    };
    v_flex()
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(width))
        .p_1()
        .gap(px(2.))
        .rounded(px(6.))
        .bg(rgb(palette::bg_footer()))
        .border_1()
        .border_color(rgb(palette::border_default()))
        .shadow_lg()
}

/// 卡片顶部信息行：图标 + 标题 + 说明（如「3 位好友」）。
fn card_header(icon: IconName, title: String, hint: String) -> Div {
    v_flex()
        .px_2()
        .py_1()
        .gap(px(2.))
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .text_color(rgb(palette::text_primary()))
                .child(ico(icon, Size::XSmall))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(title),
                ),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette::text_dim()))
                .truncate()
                .child(hint),
        )
}

/// 分组操作菜单。
pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(menu) = app.friend_group_menu.as_ref() else {
        return div().into_any_element();
    };

    let name = menu.name.clone();
    let anchor = menu.anchor;
    let count = app.friends_in_group(&name).len();
    let online = app.group_online_count(&name);
    // 顺序表里没有的分组说明还没规范化，上下移动会先把它补进表里，所以这里统一按展示顺序判断。
    let names = app.friend_group_names();
    let position = names.iter().position(|group| group == &name);
    let can_move_up = position.map(|pos| pos > 0).unwrap_or(false);
    let can_move_down = position
        .map(|pos| pos + 1 < names.len().saturating_sub(1))
        .unwrap_or(false);

    let up_name = name.clone();
    let down_name = name.clone();
    let rename_name = name.clone();
    let delete_name = name.clone();

    let card = card_at(anchor, 212.)
        .child(card_header(
            IconName::Folder,
            name.clone(),
            format!("{count} 位好友 · {online} 人在线"),
        ))
        .child(menu_divider())
        .child(menu_item(
            IconName::Pencil,
            "重命名分组",
            None,
            "friend-group-rename",
            Box::new(move |this, window, cx| {
                this.open_group_rename(rename_name.clone(), window, cx)
            }),
            cx,
        ))
        .child(menu_item(
            IconName::ArrowUp,
            "上移",
            (!can_move_up)
                .then(|| hint_text("已在最前"))
                .map(|el| el.into_any_element()),
            "friend-group-up",
            Box::new(move |this, _, cx| this.move_group(up_name.clone(), -1, cx)),
            cx,
        ))
        .child(menu_item(
            IconName::ArrowDown,
            "下移",
            (!can_move_down)
                .then(|| hint_text("已在最后"))
                .map(|el| el.into_any_element()),
            "friend-group-down",
            Box::new(move |this, _, cx| this.move_group(down_name.clone(), 1, cx)),
            cx,
        ))
        .child(menu_divider())
        .child(
            menu_item(
                IconName::Trash,
                "删除分组",
                Some(hint_text("成员回到未分组").into_any_element()),
                "friend-group-delete",
                Box::new(move |this, _, cx| this.delete_group(delete_name.clone(), cx)),
                cx,
            )
            .text_color(rgb(palette::danger())),
        );

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .id("friend-group-menu-backdrop")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.friend_group_menu = None;
                    cx.notify();
                })),
        )
        .child(card)
        .into_any_element()
}

/// 「移动到分组」菜单：列出全部分组，当前所在分组右侧打勾，底部可新建分组。
pub fn render_picker(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> AnyElement {
    let Some(picker) = app.friend_group_picker.as_ref() else {
        return div().into_any_element();
    };

    let friend = picker.friend.clone();
    let anchor = picker.anchor;
    let current = app.friend_group_of(&friend);

    let mut card = card_at(anchor, 224.)
        .child(card_header(
            IconName::FolderInput,
            "移动到分组".into(),
            format!("好友：{friend}"),
        ))
        .child(menu_divider());

    for (index, group) in app.friend_group_names().iter().enumerate() {
        let value = group.clone();
        let selected = &current == group;
        let label = if group.is_empty() {
            String::from("未分组")
        } else {
            group.clone()
        };
        let target_friend = friend.clone();
        card = card.child(menu_item(
            if group.is_empty() {
                IconName::Inbox
            } else {
                IconName::Folder
            },
            label,
            selected.then(|| ico(IconName::Check, Size::XSmall).into_any_element()),
            format!("friend-group-pick-{index}"),
            Box::new(move |this, _, cx| {
                this.set_friend_group(target_friend.clone(), value.clone(), cx)
            }),
            cx,
        ));
    }

    let card = card.child(menu_divider()).child(menu_item(
        IconName::FolderPlus,
        "新建分组…",
        None,
        "friend-group-pick-create",
        Box::new(|this, window, cx| this.open_create_group(window, cx)),
        cx,
    ));

    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .id("friend-group-picker-backdrop")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.friend_group_picker = None;
                    cx.notify();
                })),
        )
        .child(card)
        .into_any_element()
}

/// 菜单右侧的次要说明文字（如「已在最前」）。
fn hint_text(text: &'static str) -> Div {
    div()
        .text_size(px(10.))
        .text_color(rgb(palette::text_dim()))
        .child(text)
}
