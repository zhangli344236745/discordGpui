# agent.md — discord-gpui 开发规范

本文件是本项目的**开发契约**。后续任何人（包括 AI Agent）在此项目内新增功能、修改代码或重构时，必须先遵守本规范。规范的目标是：单一职责、依赖清晰、可编译验证、不重复踩 gpui-kit 的 API 坑。

---

## 1. 项目定位

- **产品**：一个 Discord 风格的桌面 IM 界面 Demo，纯前端模拟数据，无后端、无持久化。
- **优先级**：界面与交互功能优先；数据层保持可替换，方便未来接入真实 API。
- **技术栈**：Rust 2024 edition + gpui-kit `0.10.0`（单一依赖）。
- **配色**：Discord 深色主题；涨红跌绿等金融约定与本项目无关。

---

## 2. 目录结构与职责边界

```text
src/
├── main.rs                   # 仅：应用初始化、主题、窗口配置、打开窗口
├── app.rs                    # DiscordApp 状态 + ViewMode（Server/DirectMessage/Friends/Discover）+ 业务方法 + 顶层 Render 分发
├── data/
│   ├── mod.rs                # 仅 re-export models::* 与 seed::*
│   ├── models.rs             # 纯数据结构与构造函数（含 Friend / DmConversation / UserStatus / Attachment）
│   └── seed.rs               # 模拟数据工厂（服务器/频道/消息/成员/好友/私信/可选附件清单）
└── ui/
    ├── mod.rs                # 声明子模块，re-export ico / avatar / unread_badge
    ├── theme.rs              # 色板常量 palette + ico() 图标工具
    ├── widgets.rs            # 可复用无状态组件：avatar / unread_badge
    ├── server_rail.rs        # 服务器栏视图（含私信主页入口、探索入口）
    ├── discover.rs           # 探索页（整屏：搜索 + 分类筛选 + 公开服务器卡片 + 加入）
    ├── settings/             # 用户设置页（全屏覆盖主界面）
    │   ├── mod.rs            # 分类数据源 SECTIONS + 全屏骨架 + 顶栏关闭
    │   ├── sidebar.rs        # 左侧设置分类栏
    │   └── pages.rs          # 各分类内容 + 公共片段（switch_row/volume_row 等）
    ├── server_settings/      # 服务器设置页（全屏覆盖主界面，与用户设置页互斥）
    │   ├── mod.rs            # 分类数据源 SECTIONS + 顶栏（服务器徽标/名称）+ 关闭
    │   ├── sidebar.rs        # 左侧分类栏（概览/角色/表情/邀请）
    │   └── pages.rs          # 四个页面：概览改名换色、角色汇总、表情增删、邀请链接
    ├── status_menu.rs        # 在线状态选择弹层（绝对定位）
    ├── channel_sidebar.rs    # 频道侧栏视图（服务器菜单入口 + 分类 + 新建频道入口 + 频道/分类右键菜单）
    ├── channel_menu.rs       # 频道操作菜单浮层（标记已读/编辑主题/删除频道，含透明遮罩）
    ├── category_menu.rs      # 分类操作菜单浮层（新建频道/标记已读/重命名/删除，含透明遮罩）
    ├── category_dialog.rs    # 分类弹层（新建 / 重命名，复用 create_dialog::dialog）
    ├── server_menu.rs        # 服务器下拉菜单（邀请/静音/标记已读/创建频道/离开，贴侧栏头部下方）
    ├── invite_dialog.rs      # 邀请好友弹层（展示邀请链接，提交即复制到剪贴板）
    ├── create_dialog.rs      # 新建服务器 / 新建频道居中模态（通用 dialog 骨架，可复用）
    ├── topic_dialog.rs       # 频道主题编辑弹层（复用 create_dialog::dialog）
    ├── dm_sidebar.rs         # 私信侧栏（真实搜索框 + 好友入口 + 会话列表 + 发起对话）
    ├── friends.rs            # 好友页（在线/待处理/全部/已屏蔽/添加好友 五个标签，点击发起私信）
    ├── user_footer.rs        # 侧栏底部用户控制条（频道栏/私信栏共用）
    ├── member_list.rs        # 成员栏视图（按角色分组 + 折叠 + 点击成员弹资料卡 + 右键上下文菜单）
    ├── member_menu.rs        # 成员上下文菜单（资料卡/私信/提及/复制名/屏蔽，光标锚点 + 透明遮罩）
    ├── profile_card.rs       # 用户资料卡弹层（绝对定位）
    ├── voice_status.rs       # 语音连接状态条（绝对定位，跨视图保持，点击打开语音房间）
    ├── voice_room.rs         # 语音房间面板（参与者/静音/移出/本机麦克风/断开）
    ├── search_center.rs      # 搜索中心（接管聊天主区域：范围 chips + 跨频道/私信结果 + 命中高亮 + 跳转）
    └── chat/
        ├── mod.rs            # 声明 attachment_picker / composer / emoji_picker / header / inbox_panel / messages / message_menu / pinned_panel
        ├── header.rs         # 聊天标题栏（频道/私信自适应）、就地过滤输入框 + 放大镜进搜索中心、置顶/收件箱开关（带未读角标）
        ├── messages.rs       # 消息列表、搜索过滤、表情回应、附件卡片、消息工具栏、作者点击
        ├── composer.rs       # 输入框、待发附件条、回复提示条、发送按钮、@提及补全浮层
        ├── attachment_picker.rs # 附件选择弹层（模拟文件清单，选中后成为待发附件）
        ├── emoji_picker.rs   # 表情选择器弹层
        ├── message_menu.rs   # 消息操作菜单（行内省略号入口 + 右键光标浮层，共用一份操作项）
        ├── forward_dialog.rs # 转发消息弹层（搜索目标频道/私信，点击即转发）
        ├── sticker_picker.rs # 贴纸弹层（包标签 + 贴纸网格，点击即发送）
        ├── pinned_panel.rs   # 置顶消息面板（绝对定位，覆盖聊天区右侧）
        └── inbox_panel.rs    # 收件箱/通知中心（绝对定位，聚合提及/未读/私信/好友请求）
```

### 视图模式（ViewMode）

`app.rs` 中的 `ViewMode` 决定主布局：

- `Server`：服务器栏 + 频道侧栏 + 聊天区 + 成员栏。
- `DirectMessage`：服务器栏 + 私信侧栏 + 聊天区（无成员栏），标题用 `@好友名`。
- `Friends`：服务器栏 + 私信侧栏 + 好友页。
- `Discover`：服务器栏 + 探索页（独立整屏页面，不渲染频道/私信侧栏与成员栏；`render_main` 在最前面单独分支返回）。

另有全屏覆盖层「用户设置页」与「服务器设置页」，由 `app::show_settings` / `app::show_server_settings` 控制，都不占用 `ViewMode`：用户设置页由底部齿轮 `open_settings` 打开（X `close_settings`），服务器设置页由服务器菜单「服务器设置」`open_server_settings` 打开（X `close_server_settings`）。两者互斥，`Render::render` 的分发顺序是 `show_server_settings` → `show_settings` → 主界面。设置页状态集中在 `app.rs`（`SettingsSection` / `ThemeChoice` / 各开关字段），`ui/settings` 只渲染。`show_timestamps` / `show_emoji` / `compact_messages` 已在 `chat/messages.rs` 真正生效。

会话切换统一走 `app::reset_session`（清搜索、关表情层、清回复、更新输入框 placeholder、滚动到底）。消息访问统一走 `app::current_messages()`，不要直接读 `messages` / `dm_messages` 字段。

### 依赖方向

```text
main.rs ──> app.rs ──> data/          （app 读取种子数据构造状态）
              │
              └────> ui/ 的 render 函数（app 只负责调用，不写具体 widget）

ui/ 的每个视图模块 ──> app::DiscordApp（读状态、调业务方法）
                  ──> data/（读模型类型）
                  ──> ui/theme.rs + ui/widgets.rs（公共能力）

data/ ──> 只依赖 gpui 的 Rgba/rgb 等基础类型，禁止依赖 app 或 ui
```

> **注意**：`app` 与 `ui` 在模块层面是双向引用（Rust 同 crate 内允许）。这是刻意设计：**结构上双向、逻辑上单向**——`app` 只调用 `ui::*` 的 `render` 组装布局，`ui` 视图只读状态并调用 `app` 的业务方法。禁止在 `ui` 视图里直接写 `app` 的字段变更逻辑。

### 各层硬规则

1. **`main.rs`**：不允许出现任何 widget 代码或业务状态。只允许 `application().with_assets(...).run(...)`、`Theme::change`、`WindowOptions`、`open_window`。
2. **`app.rs`**：不允许出现具体的 `div()` 嵌套细节（除顶层四栏骨架外）。所有状态变更必须是 `DiscordApp` 的方法，方法名用动词：`send_message` / `select_channel` / `toggle_*` / `clear_search`。
3. **`ui/` 视图模块**：导出 `pub fn render(...)`，返回 `impl IntoElement` 或 `Div`/`Stateful<Div>`。不持有自身状态，不直接 `cx.notify()`（由 `app` 的业务方法负责，或在无法下沉时显式调用）。
4. **`data/`**：纯数据，不引用 `crate::app` 或 `crate::ui`。

---

## 3. gpui-kit 0.10.0 API 约定（务必遵守，都是踩过的坑）

crates.io 最新版本是 `0.10.0`。网上常见的 `0.7` 文档已经过时，**不要照抄旧文档**。以 `~/.cargo/registry/src/*/gpui-kit-0.10.0/` 与 `gpui-component-0.10.0/` 源码为准。

### 3.1 导入位置

```rust
// h_flex / v_flex 在 component 根，不在 gpui_kit 根
use gpui_kit::component::{h_flex, v_flex};

// Icon 从 component 根导出；component::icon 是私有模块
use gpui_kit::component::Icon;

// 输入组件
use gpui_kit::component::input::{Input, InputEvent, InputState};

// 图标名枚举
use gpui_kit::assets::IconName;
```

### 3.2 trait 必须在作用域内（否则方法"不存在"）

| 想调用的方法 | 需要的 trait | 推荐导入 |
|---|---|---|
| `.hover()`、`.when()` | `FluentBuilder` / `StyledExt` | `use gpui_kit::prelude::FluentBuilder as _;` 或 `use gpui_kit::*;` |
| `.relative()` `.absolute()` `.min_w()` `.bg()` `.w()` 等 | `Styled` | `use gpui_kit::Styled as _;`（或 `use gpui_kit::*;`） |
| `.child()` | `ParentElement` | `use gpui_kit::ParentElement as _;`（或 `use gpui_kit::*;`） |
| `.id()` `.on_click()` | `InteractiveElement` / `StatefulInteractiveElement` | `use gpui_kit::*;` 已包含 |
| `.flex_1()` `.h_full()` 等布局 | `Styled` | 同上 |

> **实践结论**：视图模块统一用 `use gpui_kit::*;` 打底，再按需从 `component` 根显式导入 `h_flex` / `v_flex` / `Size` / `Sizable` / `Input` 等。`widgets.rs` 这种不写 `use gpui_kit::*` 的模块，必须自己补 `Styled` + `ParentElement` + `FluentBuilder`。

### 3.3 图标尺寸必须走 UFCS

```rust
// 错误：.size(Size::Small) 会被 GPUI 的 Styled::size 抢占，语义变成"整体尺寸"
Icon::new(name).size(Size::Small)

// 正确：Sizable 的 trait 方法名是 with_size
<Icon as Sizable>::with_size(Icon::new(name), Size::Small)

// 项目已封装为 ico()，一律用它
ico(IconName::Hash, Size::Small)
```

`Input` 同理：

```rust
<Input as Sizable>::with_size(Input::new(&app.composer), Size::Large)
```

### 3.4 交互元素必须先有 id

`.on_click()`、`.track_scroll()`、`.overflow_y_scroll()` 等方法要求接收者是 `Stateful<Div>`，因此**链式调用中 `.id(...)` 必须排在它们之前**：

```rust
div()
    ...
    .id("composer-send")            // 先
    .on_click(cx.listener(...))     // 后
```

**ID 必须全局唯一**，且语义化、稳定。命名规则：

- 固定控件：`rail-home`、`sidebar-toggle-mic`、`chat-toggle-members`、`composer-send`
- 列表项：`rail-server-{index}`、`sidebar-channel-{serverIndex}-{name}`、`reaction-{msgIndex}-{reactionIndex}`
- 禁止使用递增的 `click-1`、`click-2` 这类无语义 ID。

### 3.5 输入框事件

```rust
cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
    match event {
        InputEvent::PressEnter { secondary: false, .. } => { /* 发送 */ }
        InputEvent::Change => { /* 读取 this.input.read(cx).value() 并 cx.notify() */ }
        _ => {}
    }
}).detach();
```

清空输入：`input.update(cx, |state, cx| state.set_value("", window, cx))`。

### 3.6 主题与资源

```rust
gpui_kit::application()
    .with_assets(gpui_kit::assets::AllAssets)   // 加载内置 Lucide 图标
    .run(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);   // 启动默认深色；切主题见「主题切换约定」
        // ...
    });
```

新增图标前，先确认它存在于 `gpui-kit-assets-0.10.0/assets/icons/<name>.svg`，文件名转 `IconName` 时连字符转驼峰（如 `circle-question-mark` → `IconName::CircleQuestionMark`）。

---

## 4. 编码规范

- **颜色**：一律使用 `ui/theme.rs::palette` 的 token，通过 `rgb(palette::xxx())` 使用（**带括号**，它们是随主题切换的函数而不是常量）。禁止在视图里写裸 `0xRRGGBB`。新增颜色先在 `palette` 的 `tokens!` 表里补一对「深色 => 浅色」。
- **纯白 `palette::WHITE`**：唯一保留的常量，只允许用在「彩色底上的文字/图标」（徽标字、按钮图标、开关滑块），不要拿它当背景色。
- **公共组件**：头像、徽标、图标等可复用片段放 `ui/widgets.rs`；只服务单一视图的片段留在该视图模块内作为私有辅助函数。
- **模型构造**：用 `Message::new(...).grouped().reaction(...)` 这类 builder 风格；新增字段时保持构造函数向后兼容。
- **状态字段**：`DiscordApp` 的字段一律 `pub(crate)`；外部读取通过字段，修改通过方法。
- **消息索引键**：统一为 `format!("{}/{}", server_name, channel_name)`，只在 `active_key()` 里生成，不要在别处拼字符串。
- **注释**：模块顶部用一行中文注释说明职责；复杂 API 约定可就近写简短注释，但不要逐行解释代码。
- **格式化**：保持 rustfmt 风格（4 空格、链式调用每行一个方法）。

### 弹层 / 浮层组件约定

新增弹层（资料卡、表情选择器、消息菜单、置顶面板、语音状态条）时：

- 一律用 `.absolute()` 定位，由 `app.rs` 顶层 Render 的根 `div()` 提供 `.relative()` 容器。
- 弹层显隐由 `DiscordApp` 的状态字段控制（`profile_target` / `emoji_picker_open` / `menu_message` / `show_pins` / `voice_channel`），视图模块只按状态条件挂载，不自持状态。
- 关闭弹层走 `app` 的业务方法（`close_profile` / `close_emoji_picker` / `toggle_*`），同样触发 `cx.notify()`。
- 切换会话统一在 `app::reset_session` 里清理所有弹层状态，但 `voice_channel` 例外——语音连接跨会话保持。
- 置顶面板与收件箱面板互斥：`toggle_pins` 打开时关 inbox，`toggle_inbox` 打开时关 pins。

### 收件箱 / 通知中心约定

- data 层 `Notification { kind, title, body, time, color, target }`，`NotificationKind`（Mention/UnreadChannel/Dm/FriendRequest）、`NotificationTarget`（Channel/Dm/Friends）；因含 `String` 只 derive `Clone`。
- `app::notifications()` 从四类数据聚合：@已知好友的消息、unread>0 的文字频道、unread>0 的私信会话、incoming 好友请求。key 由 `notification_key()` 按目标生成（channel/server/channel、dm/好友、friends）。
- 已读存 `read_notifications: Vec<String>`；`unread_notification_count()` / `is_notification_unread(index)` 供角标与行高亮；`mark_all_notifications_read` 全部已读。
- 点击通知走 `app::open_notification(index, window, cx)`：标记已读、关面板、按 target 跳转（频道直接设 active_server/active_channel 后 reset_session；私信走 open_dm；好友请求切 Friends + FriendTab::Pending）。
- header 的 Inbox 图标挂 `chat-open-inbox` → `toggle_inbox`，未读时显示红色数字角标；面板在 `chat/inbox_panel.rs`，`toggle_inbox` 与 `toggle_pins` 互斥。

### 用户资料卡数据流

`ui/profile_card.rs` 不直接读 friends/members，统一调用 `app.profile_info(name: &'static str) -> ProfileInfo` 聚合展示数据：

- 用户名可能是基础名（消息作者如「Nova」）或带角色后缀的成员名（「Nova —— 管理员」），用 `app::base_name` 归一。
- 颜色/在线状态优先取 friend，其次 member；活动取 friend.activity；角色取 ` —— ` 后缀，缺省「成员」。
- `ProfileInfo` 定义在 `app.rs`，字段均为 `pub(crate)`。

### 成员栏角色分组约定

- `app::member_groups() -> Vec<MemberGroup>`（结构定义在 data 层）按成员名 ` —— ` 后缀的角色聚合：有角色的按首次出现顺序成组，无角色的按在线状态归入「在线」「离线」兜底组；组内含 `member_indices`（指向 app.members 的索引）。
- 角色高亮色由 `app::role_group_color(role)` 映射（管理员红 / 版主绿 / 贡献者蓝 / 未知 ACCENT），在线/离线兜底组用 TEXT_MUTED/TEXT_DIM。
- 折叠状态存 `collapsed_roles: Vec<String>`，标题点击走 `app::toggle_role_group(key)`；渲染时 `app.collapsed_roles.iter().any(|k| k == &group.key)` 判断是否展开。
- `member_list.rs` 把成员行抽成 `member_row()`、分组标题抽成 `group_header(group, collapsed, cx)`；`Size` 必须从 `gpui_kit::component` 导入（`group_header` 的 `on_click` 用 `cx.listener` 四参闭包）。

### 在线状态与消息文本增强

- 本人在线状态存于 `my_status: UserStatus`（data 层枚举：Online/Idle/Dnd/Invisible，含 `label()` / `color()`）；切换走 `app::set_my_status`，弹层显隐走 `toggle_status_menu`。
- 头像状态点用 `ui::avatar_status(letter, color, size, status_color: Option<u32>)`；原 `avatar(bool)` 委托给它（绿点/隐藏）。底部用户栏的信息区可点击打开状态菜单。
- @提及：`messages::render_text` 把文本中的 `@token` 切出，用 `app::resolve_mention(token) -> Option<&'static str>` 校验（仅匹配已知好友/成员），命中则渲染可点击高亮片段并打开资料卡。
- @提及自动补全：composer 订阅 `InputEvent::Change` → `app::refresh_mention` 取输入末尾以空白分隔的 token，若以 `@` 开头则按前缀（不区分大小写）匹配好友名 + 成员 `base_name`，去重后存 `mention_candidates`；`mention_prefix` 记录前缀。composer 在输入框上方渲染 `mention_overlay`（绝对定位），点候选走 `app::apply_mention`（用 `set_value` 把末尾 `@token` 替换为 `@完整名 ` 并清空候选）。`send_message` / `reset_session` 都会清空补全状态。
- 消息编辑：仅本人消息（`app::is_mine(index)`）在菜单里显示「编辑」；`start_edit` 把原文载入独立 `editor: Entity<InputState>` 并聚焦，`commit_edit`（Enter）覆盖原文，`cancel_edit` 放弃。`reset_session` 会取消编辑。

### 好友页约定

- `FriendTab`（data 层枚举：Online/Pending/All/Blocked/AddFriend）决定好友页内容，`friend_tab` 存于 app，切换走 `set_friend_tab`。标签数据源在 `friends.rs::TABS`。
- `FriendRequest { name: String, color, incoming }`：`incoming=true` 为收到的请求（可 接受/拒绝），否则为发出的请求（显示等待中）。`accept_request` 把动态名用 `Box::leak` 转为 `&'static str` 后加入好友（Demo 纯前端静态模型）；`reject_request` 直接移除。
- 添加好友：输入框 `add_friend_input` 接受「用户名#标签」，Enter 或按钮触发 `send_friend_request`，按基础名去重，成功后切到 Pending 标签。屏蔽好友从 `friends` 移除并记入 `blocked_users`，`unblock_friend` 反向恢复。
- 屏蔽入口在好友行右侧的 Ban 图标，与整行的「发起私信」点击共存（GPUI 事件不冒泡，各自独立 id）。

### 私信侧栏搜索约定

- 顶部搜索为真实 `InputState`：`dm_search: Entity<InputState>` + `dm_query: String`，订阅 `InputEvent::Change` 更新 `dm_query`。有内容时显示 X 清除按钮（`clear_dm_search`）。
- 搜索按好友名或消息预览（不区分大小写）过滤会话；名字匹配但**尚无会话**的好友显示在「发起对话」分组，点击走 `app::start_dm`。
- `start_dm(friend: &'static str)`：若会话不存在则补建 `DmConversation` 条目（颜色/在线状态取好友）并初始化空 `dm_messages`，再走 `open_dm` → `reset_session`（会清空 dm 搜索）。`reset_session` 同时调用 `clear_search` 与 `clear_dm_search`。
- 搜索无结果时显示空状态；搜索模式隐藏好友入口与常规标题。会话行抽成 `conversation_row()` 复用，不依赖 `app`（只取渲染所需参数）。

### 新建服务器 / 频道约定

- `Server.name` / `Server.badge` 为 `String`（支持运行时新建），`Server::new(name, color)` 生成带默认「文字频道/闲聊」的服务器，徽标取名称首字符。
- 服务器栏 `+`（`rail-create-server`）→ `app::open_create_server`；频道分类 `+` → `app::open_create_channel`（仅服务器视图可用）。两者共用 `create_dialog.rs` 的 `dialog()` 骨架，由 `show_create_server` / `show_create_channel` 控制显隐，互斥。
- 输入草稿用独立 `Entity<InputState>`（server_name_input / channel_name_input），订阅 Enter 提交：`create_server` / `create_channel`。两个方法都在成功后调用 `select_server` / `select_channel` 走统一的 `reset_session`（会关闭弹层、清搜索、滚动到底）。
- `create_channel` 把频道加入目标分类（`create_channel_category`，默认为第一个分类），同名频道在服务器内全局拦截。新建服务器配色由 `next_server_color()` 按数量轮换。
- 弹层关闭有三个入口：遮罩点击、右上角 X、取消按钮；三个 handler 共享同一个闭包，用 `Rc` 包装避免 Rust 2024 下的重复移动错误。

### 附件消息约定

- `Message.attachment: Option<Attachment>` 由 builder `.attachment(a)` 设置；`Attachment { name: String, size: &'static str, kind: AttachmentKind }`，构造用 `Attachment::image` / `Attachment::file`，`is_image()` 判断展示形态。Demo 不做真实上传，只保留文件名与大小文案。
- 可选文件清单在 `data/seed.rs::seed_attachments()`（图片 + 文档混合共 5 条）；`ui/chat/attachment_picker.rs` 直接渲染该清单，不要硬编码第二份。
- 待发附件与弹层显隐：`app.attachment_draft: Option<Attachment>` + `app.show_attachment_picker: bool`；入口是 composer 左侧 `+`（`composer-attach`）→ `toggle_attachment_picker`，选中走 `pick_attachment`（写入草稿、关弹层、聚焦输入框），移除走 `clear_attachment`。
- `send_message` 允许「只有附件、没有文字」；发送时 `attachment_draft.take()` 挂到消息上。文字为空时 `messages.rs` 跳过文本节点，只渲染 `attachment_card()`。
- 弹层互斥：打开附件层收起表情层/状态菜单/各类菜单/资料卡/置顶/收件箱；打开表情层会关附件层。`reset_session` 与 `open_settings` 都会清空草稿并关弹层。
- 附件卡片宽 300：图片走 `BG_ATTACHMENT_PREVIEW` 占位块 + 名称/大小；文件走图标 + 名称/大小的横向卡片。新增附件样式先加 palette 颜色，不要在视图里写裸色值。

### 探索页约定

- 入口是服务器栏底部指南针（`rail-discover`）→ `app::open_discover`：切到 `ViewMode::Discover`、清空搜索与分类筛选、收起所有浮层，但**保留** `active_server` / `active_channel`，点右上角 X（`discover-close` → `close_discover`）可原样回到频道视图。
- `render_main` 对 `ViewMode::Discover` 单独早期返回：只挂服务器栏 + `discover::render` + 语音状态条；`side`/`main` 里的 match 仍保留 Discover 兜底分支（不会走到，但要求穷尽）。
- 数据在 `data::DiscoverServer { name, badge, color, category, members, description, joined }`，种子 `seed_discover_servers()`（8 个，覆盖游戏/科技/设计/音乐/学习/影视）。
- 搜索框 `discover_search` 订阅 Change 写入 `discover_filter`；分类 chips 数据源是 `ui/discover.rs::CATEGORIES`（「全部」= `discover_category: None`），点击走 `set_discover_category`。视图内自行做「名字/简介/分类」三字段大小写不敏感过滤，不新增 app 查询方法。
- 加入走 `join_discover_server(index)`：已在 `servers` 里同名就直接 `select_server`；否则用它构造 `Server::new`（并写入简介）追加到服务器栏、标记 `joined`，再 `select_server` 进入，天然回到 `ViewMode::Server`。
- 卡片宽度固定 252，简介两行高度 34 用 `overflow_hidden` 截断；「已加入」判定是 `entry.joined || servers` 里存在同名。

### 服务器设置页约定

- 入口：服务器下拉菜单第一项「服务器设置」（`server-menu-settings`）→ `app::open_server_settings`；它会把当前服务器名称/简介预填到 `server_name_edit` / `server_desc_edit`，并收起所有浮层（含用户设置页）。
- 分类：`ServerSettingsSection`（Overview/Roles/Emoji/Invites），文案与图标唯一数据源是 `ui/server_settings/mod.rs::SECTIONS`，切换走 `select_server_settings_section`。
- 概览：名称与简介各一个 Input，Enter（订阅在 `new` 里）或「保存」按钮都调用 `save_server_overview`。改名是**带迁移**的：同步搬走 `messages` 里 `旧名/` 前缀的键、重写 `collapsed` 里的折叠键，并按新名首字更新 `badge`；空名或未改名不生效。图标颜色由 6 个色块 `set_server_color` 点击即生效。
- 角色：只读汇总，直接复用 `app.member_groups()`，成员名按 ` —— ` 取前缀展示；不在视图里自己聚合。
- 表情：`server_emojis` / `removed_server_emojis` 两个向量，`remove_server_emoji` 与 `restore_server_emoji` 互转；Demo 不做真实上传。
- 邀请：链接由 `app.invite_code()`（已 `pub(crate)`）+ `discord.gg/` 拼出，`copy_server_invite` 写剪贴板并把 `invite_copied` 置真（按钮变「已复制」）。「打开邀请弹层」走 `open_invite`，该弹层挂在主界面上，因此 `open_invite` 会顺带关掉服务器设置页。
- 打开用户设置页（`open_settings`）会关闭服务器设置页；切换分类时 `invite_copied` 复位。

### 贴纸消息约定

- 数据：`data::Sticker { emoji, label, pack }`（Demo 用 emoji 字形代替贴纸图），`Message.sticker: Option<Sticker>` + `.sticker(s)` builder；种子 `seed_stickers()` 24 张分三包（日常 / 猫咪 / 礼物），`sticker_packs()` 给出标签顺序。
- 状态：`app.stickers` / `app.sticker_packs` / `app.sticker_pack`（当前包）/ `app.show_sticker_picker`。
- 入口两个，都走 `toggle_sticker_picker(Option<&'static str>, cx)`：输入框右侧**礼物按钮**（`composer-gift`）传 `Some("礼物")` 直达礼物包，**贴纸按钮**（`composer-sticker`）传 `None` 打开当前包；同一个包再点一次收起。
- 打开时与表情层 / 附件层 / 各类菜单 / 资料卡互斥（反过来打开表情层或附件层也会置 `show_sticker_picker = false`）；`reset_session` 与打开设置 / 服务器设置 / 探索页 / 搜索中心时统一 `close_sticker_picker()`。
- 点击贴纸走 `send_sticker`：生成 `Message::new("你", "刚刚", "", palette::ACCENT).sticker(s)` 直接入列、清回复态、滚到底部并把焦点还给输入框（好友视图直接返回）。
- 视图 `ui/chat/sticker_picker.rs`：底部 52 / 右侧 16、宽 348 的浮卡；顶部包标签（数据源 `app.sticker_packs`）+ 分隔线 + 4 列贴纸格（emoji 30px + 名称 10px）+ 底部「当前包 · 共 N 张」。切包走 `set_sticker_pack`。
- 消息里的贴纸由 `messages.rs::sticker_card` 渲染：132×132 圆角块 + 54px emoji + 名称；正文为空时不渲染文本节点，因此贴纸消息只有卡片。

### 主题切换约定

- 色板不再是常量：`ui/theme.rs::palette` 里每个 token 都是 `fn() -> u32`，深/浅两套值写在 `tokens!` 宏表里（`名称 = 深色 => 浅色`）。主题状态是一个 `AtomicBool`，由 `palette::set_light(bool)` 写入、渲染时读取，所以**不需要**把主题逐层透传进视图。
- 读取一律写 `palette::bg_chat()` **带括号**；`palette::WHITE` 是唯一常量（彩色底上的文字/图标），不要当背景。
- 切换入口：设置页「外观」三张卡片 → `app::set_theme(theme, window, cx)`。它做三件事：写 `theme_choice`、`palette::set_light(...)`、`Theme::change(ThemeMode::…)`（gpui-component 的输入框等控件读这套组件主题，漏掉会让输入框在浅色下变成浅底浅字）。最后 `cx.notify()` 让整棵树按新色板重建。
- `ThemeChoice::System` 用 `window.appearance()` 判定（`Light` / `VibrantLight` 视为浅色），所以「跟随系统」在启动后第一次点击时就能生效；`reset_settings` 复位为深色并同步 `palette::set_light(false)`。
- 新增颜色时必须同时给出浅色值，并保证正文对比度 ≥ 4.5、图标 ≥ 3.0；浅色下不要复用「深色底上的白条」这类写法（服务器栏的选中指示条已改用 `text_primary()`，深浅两套都能看见）。

### 消息转发约定

- 数据：`data::ForwardOrigin { label, is_dm, server: Option<String>, channel }`（构造用 `ForwardOrigin::channel(server, channel)` / `::dm(friend)`），`Message.forwarded: Option<ForwardOrigin>` + `.forwarded(origin)` builder。`data::ForwardTarget { server, channel, is_dm, color }` 是转发弹层里的候选目标，带 `label()`（`#频道` / `@好友`）与 `scope()`（服务器名 / 私信）两个展示方法。
- 状态：`app.forward_source: Option<usize>`（源消息在当前会话的下标）、`show_forward`、`forward_search` / `forward_query`、`forward_notice: Option<String>`（成功提示条）。
- 打开走 `open_forward(index, window, cx)`：先校验下标存在，再清空搜索词、聚焦搜索框，并收起消息菜单 / 置顶 / 收件箱 / 资料卡 / 表情层 / 贴纸层 / 附件层。关闭统一走 `close_forward()`（清 `show_forward` + `forward_source` + 搜索词），遮罩、X、取消三处共用。
- 目标列表只在 app 里算：`forward_targets()` 遍历所有服务器的 `ChannelKind::Text` 频道 + 全部私信会话，`filtered_forward_targets()` 再按 `forward_query` 过滤（匹配频道名/好友名与服务器名）。视图不要自己拼目标。
- 转发动作 `forward_message(target, cx)`：克隆源消息的 `author` / `color` / `text` / `attachment` / `sticker`，**不**带走 `pinned` / `reactions`，`grouped` 置 false，补上 `forwarded` 来源；私信写 `dm_messages` 并把 `dm_conversations.preview` 改成「转发了一条消息」，频道写 `messages` 的 `服务器名/频道名` 键（键格式必须与 `active_key()` 一致）。
- 转发到当前会话时滚到底部，否则不动滚动位置；随后置 `forward_notice = Some("已转发到 #xxx")` 并关闭弹层。提示条由 `messages.rs::notice_bar` 渲染在列表顶部，`dismiss_forward_notice(cx)` 关闭，`reset_session` 会清空。
- 消息里的来源行由 `messages.rs::forward_origin_row` 渲染（转发图标 + 「转发自 #频道/ @好友」，链接色），点击走 `jump_to_origin(origin, window, cx)`：私信转 `open_dm`，频道按服务器名定位索引后写 `active_server` / `active_channel` 并 `reset_session`。来源行的 id 必须带消息下标，避免同一来源重复出现时冲突。
- 弹层 `ui/chat/forward_dialog.rs` 是独立组件（**不**复用 `create_dialog::dialog`）：宽 468 居中卡片 = 标题 + 来源消息预览（作者 + 80 字摘要，贴纸/附件退化为 `[贴纸] / [附件]` 文案）+ 搜索框 + `max_h 268` 可滚动目标列表 + 底部说明/取消。目标行为 44 高，行首彩色圆标（Hash / AtSign）、名称 + 归属、行尾「转发到 …」。空结果显示「没有匹配的频道或私信」。

### 消息操作菜单约定

- 两个入口共用一份操作项 `ui/chat/message_menu.rs::menu_items(index, app, cx)`：**省略号按钮**（`toggle_message_menu`，菜单贴消息行右上角）与**消息行右键**（`open_message_menu_at(index, x, y, cx)`，菜单跟随光标、渲染在根层级）。
- 区分方式是 `app.menu_message_point: Option<(f32, f32)>`：`Some` 表示光标浮层，`None` 表示行内。`messages.rs` 里行内菜单的挂载条件是 `menu_open && app.menu_message_point.is_none()`，`render_main` 里另外挂 `render_floating`（带 `msg-menu-backdrop`，点外部或再次右键关闭）。
- 关闭**一律**走 `app::close_message_menu()`（同时清 `menu_message` 与 `menu_message_point`），不要在别处只置 `menu_message = None`，否则浮层锚点会残留。
- 操作项：添加回应（`toggle_emoji_picker(Some(index))` + 关闭菜单）、回复（`toggle_reply(author)` + 关闭菜单）、转发消息（`open_forward(index, window, cx)`，见「消息转发约定」）、编辑消息（仅 `is_mine`）、置顶 / 取消置顶、复制文本、删除消息；图标随状态在 `Pin` / `PinOff` 间切换，删除项用危险色。
- 右键坐标取 `event.position.x/y` 转 `f32`，浮层用 `left = x - 8`、`top = y - 8` 并 `min` 到 1264 / 768 兜底，避免贴边被裁。
- 打开右键菜单会收起资料卡 / 成员菜单 / 频道菜单 / 分类菜单 / 服务器菜单 / 收件箱 / 状态菜单 / 附件层 / 表情层。

### 频道操作菜单约定

- 入口有两个：频道行**右键**（`on_mouse_down(MouseButton::Right)`，用 `event.position` 取光标坐标）与**标题栏铃铛**（`chat-channel-menu`，锚点 `MenuAnchor::Header`）。两者都调 `app::open_channel_menu(name, anchor)`。
- 菜单状态：`app.channel_menu: Option<ChannelMenuState { name, anchor }>`；`MenuAnchor::Point(x, y)` 时 `.left(x).top(y)`，`Header` 时 `.right(...).top(...)`（`channel_menu.rs` 里统一换算）。
- 菜单项：标记为已读（`mark_channel_read`）、编辑频道主题（`open_topic_editor`，仅文字频道）、删除频道（`delete_channel`，危险色）。
- 关闭方式：透明全屏遮罩 `.id("channel-menu-backdrop")` 点击关闭 + 每个动作执行后 `self.channel_menu.take()`/`None`。菜单打开时会收起资料卡、消息菜单、置顶/收件箱/状态菜单、表情层，避免叠层。
- `delete_channel` 的硬约束：至少保留一个文字频道；删掉的若是当前频道就回退到 `first_text_channel()`；删掉的若是正在连接的语音频道先断开；空分类一并移除；同时清掉 `messages` 里 `服务器/频道` 的历史键。
- 主题编辑走 `topic_dialog.rs`，复用 `create_dialog::dialog()`（该函数已 `pub(crate)`），输入 `topic_input`，Enter/保存写回 `Channel.topic`，标题栏的 `active_topic()` 立即刷新。
- 频道定位统一用 `channel_position(name) -> Option<(分类下标, 频道下标)>`，不要在 UI 层自己遍历服务器。

### 服务器菜单约定

- 入口：频道侧栏头部（服务器名 + ⌄，`sidebar-server-menu`）→ `app::toggle_server_menu(cx)`。菜单用固定定位贴在头部下方（`left 8 / top 44`，宽 224），不跟随光标。
- 显隐状态只有 `app.show_server_menu: bool`；`server_menu.rs` 顶部 `if !app.show_server_menu { return div().into_any_element(); }`，顶层用 `.when(...)` 挂载。
- 菜单项：邀请好友（`open_invite`）、静音/取消静音（`toggle_server_mute`）、标记服务器已读（`mark_server_read`）、创建频道（`open_create_channel`）、离开服务器（`leave_server`，危险色 + 遮罩分隔）。
- `Server.muted: bool` 是模型字段（新建服务器默认 false，seed 全 false）；静音后侧栏标题右侧显示 `BellOff` 图标，菜单文案变「取消静音服务器」。
- 邀请弹层 `invite_dialog.rs` 复用 `create_dialog::dialog()`：链接由 `invite_code()` 生成（ASCII 名转小写 slug，纯中文名用 `gpui-{序号}` 兜底），提交按钮即复制到剪贴板并关闭。
- `leave_server` 的硬约束：至少保留一个服务器（`servers.len() <= 1` 直接返回）；删除后 `active_server` 用 `min(len-1)` 收敛，清理该服务器 `messages` 里 `服务器/` 前缀的历史键；若语音连接在被删服务器上则断开，若在其后则下标前移一位。
- 菜单互斥：`toggle_server_menu` 会关掉频道菜单/资料卡/消息菜单/置顶/收件箱/状态菜单/表情层；`open_create_server`、`open_create_channel`、`open_invite`、`open_channel_menu`、`reset_session` 都会把 `show_server_menu` 置 false。

### 分类操作菜单约定

- `Category.name` 是 `String`（不是 `&'static str`），支持运行时重命名；新建分类用 `Category::new(name, channels)`。
- 入口：分类标题行**右键**（`on_mouse_down(MouseButton::Right)`，取 `event.position`）→ `app::open_category_menu(name, MenuAnchor::Point(x, y))`。分类没有标题栏入口，不需要 `MenuAnchor::Header`（`category_menu.rs` 里保留兜底坐标）。
- 菜单状态：`app.category_menu: Option<CategoryMenuState { name, anchor }>`；分类按名称在**当前服务器**内定位（`category_position(name) -> Option<usize>`），不跨服务器。
- 菜单项：在该分类中创建频道（`open_category_create_channel` → 记 `create_channel_category` 后复用 `open_create_channel`）、标记为已读（`mark_category_read` 清该分类全部未读）、重命名分类（`open_category_rename`）、删除分类（`delete_category`，不可删时置灰并提示「至少保留一个文字分类」）。
- 重命名：`category_dialog.rs` 复用 `create_dialog::dialog()`，输入 `category_input`，`rename_category_source` 记录原名用于定位；空名、未改名、与现有分类重名都不生效；改名后要把 `collapsed` 里的 `服务器/旧分类名` key 迁移成新 key，否则展开状态会丢。
- 新建分类：入口在**服务器菜单**的「新建分类」（`server-menu-create-category`）→ `open_create_category`；同样复用 `category_dialog.rs` + `category_input`，由 `show_create_category` 决定标题/副标题/确认按钮文案与提交动作。`create_category` 追加一个空分类到服务器末尾，空名或重名不生效。新建的空分类没有频道，仍可被右键重命名或删除。
- 同一个 `category_input` 服务两个弹层，Enter 在订阅里按 `show_create_category` / `show_rename_category` 分发；打开任一弹层时要先清掉另一个的标志位，避免两边文案串味。
- `delete_category` 的硬约束：`category_deletable(name)` 为假时不执行——要求至少存在**另一个包含文字频道**的分类；删除时断开会话中落在该分类的语音频道、清掉分类内所有 `服务器/频道` 历史消息、当前频道被删则回退 `first_text_channel()`、并移除该分类的折叠 key。
- 与频道菜单互斥：`open_category_menu` 会关掉频道菜单/服务器菜单/资料卡/消息菜单/置顶/收件箱/状态菜单/表情层；`open_channel_menu`、`toggle_server_menu`、`reset_session`、`open_settings` 都会清 `category_menu` 与重命名弹层。

### 语音频道约定

- 语音频道的点击走 `app::toggle_voice(server_index, channel)`，**不**调用 `select_channel`，不切换聊天区消息。
- 连接态存于 `voice_channel: Option<(usize, String)>`，`voice_label()` 返回 `(服务器名, 频道名)` 供状态条展示。
- 频道栏中语音频道用 `voice_joined` 判断是否高亮（与文字频道的 `active_channel` 逻辑分开）。
- 断开语音统一走 `app::leave_voice()`（清 `voice_channel` + `voice_participants` + `show_voice_room`）；`delete_channel` / `delete_category` / `leave_server` 里原本直接置 `voice_channel = None` 的地方都要改调它，避免房间面板残留。

### 搜索中心约定

- 入口是标题栏搜索框右侧放大镜（`chat-open-search-center`）→ `app::open_search_center`：它继承标题栏搜索框的关键词作为初始 query（写进 `search_center_input`），按视图选默认范围（私信视图 → `Dm`，其余 → `CurrentServer`），收起其他浮层后调 `run_search`。
- 搜索中心**不占** `ViewMode`，只是一个 `show_search_center: bool`。`render_main` 里当 `show_search_center && matches!(view_mode, Server | DirectMessage)` 时用 `search_center::render` 替换整个聊天主区域，服务器栏与左侧栏保留，便于边搜边切。`reset_session` 会关掉它（切频道 / 开私信自动退出），`open_settings` / `open_server_settings` / `open_discover` 也会收起。
- 范围是 `app::SearchScope`（`CurrentServer` / `AllServers` / `Dm` / `Everything`），视图侧文案唯一数据源是 `ui/search_center.rs::SCOPES`，切换走 `set_search_scope`（值变了才重跑）。
- 检索逻辑只在 `app::run_search` 里：服务器结果按 `servers → categories → channels` 顺序遍历、跳过 `ChannelKind::Voice`、键用 `服务器名/频道名` 去 `messages` 取；私信结果遍历 `dm_conversations` 取 `dm_messages`。命中判定是自由函数 `hit_matches`（正文或作者名，调用方已转小写），结果 `truncate(60)`。
- `data::SearchHit { server, channel, is_dm, author, time, text, color }` + `scope_label()`（`服务器 ▸ #频道` / `私信 ▸ 好友`）。`color` 是 `Rgba`（直接来自 `Message`），视图用 `into()` 传给 `avatar`。
- 跳转 `open_search_hit(index)`：私信走 `open_dm`，频道先按名字找到服务器索引写入 `active_server` 再 `select_channel`；两者都会经 `reset_session` 关掉搜索中心。
- 命中高亮由视图内 `highlight()` 完成：`to_lowercase()` 后按字节找关键词，**先校验字节长度未变且落在 char boundary**，否则退化为整段普通文本（避免中文切片 panic）。

### 成员上下文菜单约定

- 入口：成员行 `member_row()` 的 `on_mouse_down(MouseButton::Right, ...)` → `app::open_member_menu(name, x, y, cx)`，坐标取 `event.position.x/y` 转 `f32` 存进 `MenuAnchor::Point`（与频道/分类菜单同一套锚点）。
- 状态只有 `app.member_menu: Option<MemberMenuState { name: &'static str, anchor }>`（`name` 是成员展示名，可能含「 —— 角色」）。打开时会收起资料卡 / 消息菜单 / 频道菜单 / 分类菜单 / 置顶 / 收件箱 / 服务器菜单 / 状态菜单 / 表情层；`reset_session`、`open_settings`、`open_server_settings`、`open_discover`、`open_search_center` 都会把它清掉。
- 角色解析只走 `app.member_menu_role()`（按「 —— 」取后半段），视图不要自己 split 角色；短名用 `base_name` 语义（视图内 `split(" —— ").next()`），屏蔽名单里存的也是短名。
- 菜单动作：`member_open_profile`（转 `toggle_profile`）、`member_send_dm`（转 `start_dm`，会自动补建会话；`start_dm` 找不到好友时回退到 `members` 取颜色与在线态）、`member_mention`（把「@短名 」追加到 composer 并聚焦）、`member_copy_name`（复制 `@短名`）、`member_toggle_block`（**只动 `blocked_users`**，不像 `block_friend` 那样从好友列表移除——成员未必是好友）。
- 屏蔽后好友页「已屏蔽」分组会列出该名字并支持取消屏蔽，形成闭环；菜单文案与图标随 `member_menu_blocked()` 在「屏蔽成员 / 取消屏蔽」间切换（`UserX` / `UserCheck`，危险色）。
- 视图 `ui/member_menu.rs` 用绝对定位卡片（`left/top` 取锚点、宽 196）+ 全屏透明遮罩 `member-menu-backdrop`，点外部空白即 `close_member_menu`。

### 语音房间面板约定

- 连接语音时 `toggle_voice` 会调 `seed_voice_participants()`：从在线成员里抽三位（排除「你」，名字按 ` —— ` 取前缀）生成 `data::VoiceParticipant { name, color, muted }` 存入 `app.voice_participants`。
- 面板显隐只有 `app.show_voice_room: bool`，且**必须**与 `voice_channel.is_some()` 同时成立才挂载（`render_main` 两个分支都要挂，探索页也显示）。
- 入口：语音状态条文字区 `voice-open-room` → `toggle_voice_room`；面板右上角 X → `close_voice_room`。
- 面板定位固定 `left 72 / bottom 108 / w 300`，贴在 52 高的状态条上方，样式对齐 `BG_FOOTER` 浮卡。
- 行内操作：麦克风图标 `toggle_participant_mute`（静音态红色 + 行尾「已静音」）、`UserX` 图标 `remove_voice_participant`（仅从列表移除，Demo 不涉及权限）；底部两个按钮分别是本机 `toggle_mic` 与 `leave_voice`。
- 打开用户设置 / 服务器设置 / 探索页都会收起面板；连接态本身跨会话保持（与 `voice_channel` 一致）。

---

## 5. 常见任务操作指引

### 新增一个模拟服务器

1. 在 `data/seed.rs::seed_servers()` 的向量里追加一个 `Server`。
2. 若需要新频道专属消息，在同一文件的 `seed_messages()` 里以 `"服务器名/频道名"` 为键插入。
3. 不需要动 `app.rs` 或 `ui/`——服务器栏与频道栏都是遍历渲染。

### 新增一个 UI 组件

1. 若是通用组件：在 `ui/widgets.rs` 新建 `pub fn xxx(...) -> Div`，并在 `ui/mod.rs` re-export。
2. 若只属于某个区域：直接写在该视图模块里，不 `pub`。
3. 组件本身不读 `DiscordApp`，通过参数接收所需数据。

### 新增一个交互

1. 在 `app.rs` 的 `impl DiscordApp` 中新增业务方法，负责改状态 + `cx.notify()`。
2. 在对应视图元素上挂 `.id(唯一ID)` + `.on_click(cx.listener(...))`，闭包里只调用该方法。
3. 若交互涉及输入框，按 3.5 处理事件订阅。
4. 若需要右键：`.on_mouse_down(MouseButton::Right, cx.listener(|this, event: &MouseDownEvent, _, cx| ...))`，光标坐标用 `let x: f32 = event.position.x.into();` 取出（`Pixels` 实现了 `Into<f32>`）。

### 新增一个弹层菜单

1. 菜单骨架直接照 `ui/channel_menu.rs`：外层透明遮罩 `.absolute().top_0().left_0().size_full()` + `.id("xxx-backdrop").on_click(关闭)`，内层 `v_flex().absolute().w(px(210.))...` 卡片。
2. 菜单项统一 `.h(px(30.)).px_2().flex().items_center().gap_2().rounded(px(4.))` + hover 高亮，危险操作用 `palette::DANGER`。
3. 需要输入时复用 `create_dialog::dialog()` 的模态骨架，不要复制一份遮罩/按钮代码。

### 新增一个视图区域

1. 在 `ui/` 下新建模块文件，导出 `pub fn render(app: &DiscordApp, cx: &mut Context<DiscordApp>) -> impl IntoElement`。
2. 在 `ui/mod.rs` 声明模块。
3. 在 `app.rs` 的 `Render` 实现顶层骨架里 `.child(...)` 接入。

---

## 6. 验证流程（每次改动必做）

```bash
cd /Users/worldunion/WorkBuddy/2026-11-20-14-21-58
cargo check     # 快速类型检查，必须无 error / warning
cargo build     # 完整构建
cargo run       # 启动确认窗口创建正常
```

验收标准：

1. `cargo check` 输出 `Finished`，且 warning 数为 0。
2. 窗口尺寸 1440×900，标题 `Discord UI · GPUI Kit`。
3. 手动验证涉及的交互：服务器切换、频道切换、搜索过滤、消息发送、表情回应、成员栏开关、麦克风/耳机/通知开关；资料卡（点作者名/成员）、消息菜单（置顶/复制/删除/编辑）、置顶面板（header 图钉）、语音频道连接与断开；在线状态切换（点底部用户信息）、@提及高亮点击、消息行内编辑（Enter 保存 / 取消）；用户设置（点底部齿轮打开，左侧 8 个分类切换，主题选项、开关、音量加减、恢复默认，右上角 X 关闭）；新建服务器（服务器栏 + → 输入名称 → Enter/创建，自动切换）、新建频道（分类 + → 输入名称 → Enter/创建，自动切换，同名拦截）；好友页五个标签切换、待处理请求接受/拒绝、添加好友（用户名#标签 → Enter/发送，成功后跳待处理）、好友行屏蔽与已屏蔽列表取消屏蔽；私信侧栏搜索（输入关键词过滤会话、X 清除、点「发起对话」中的好友建立新会话并切入、无结果空状态）；成员栏按角色分组（管理员/版主/在线/离线，点分组标题折叠与展开，点成员仍打开资料卡）；收件箱（header Inbox 图标带未读角标 → 打开面板，点通知跳转到对应频道/私信/好友页并标为已读、全部已读、与置顶面板互斥）；@提及补全（输入框打 @ 或 @前缀 → 上方浮层匹配成员，点击插入 @完整名）；频道操作菜单（频道行右键 → 菜单跟随光标：标记为已读清角标、编辑频道主题 → 弹层 Enter/保存后标题栏刷新、删除频道 → 回退到第一个文字频道且至少保留一个文字频道；标题栏铃铛打开同一菜单；点菜单外关闭）；服务器菜单（点侧栏服务器名/⌄ → 邀请好友弹出链接弹层并可复制、静音切换后侧栏出现静音图标、标记服务器已读清空全部角标、创建频道、离开服务器 → 切到第一个服务器且至少保留一个）；分类菜单（分类标题右键 → 在该分类中创建频道后弹窗创建并落入该分类、标记为已读清该分类角标、重命名分类 → 弹层保存后侧栏标题与展开状态保持、删除分类 → 分类内频道一并消失且当前频道回退、只剩一个文字分类时该项置灰；服务器菜单「新建分类」→ 输入名称 → Enter/创建后侧栏出现空分类且可再右键改名或删除）；主题切换（设置 → 外观 → 点「浅色」→ 整个界面立刻变浅色：服务器栏/侧栏/聊天区/菜单/弹层全部跟着变，输入框文字仍可读；点「深色」切回；点「跟随系统」按当前窗口外观决定；恢复默认设置会回到深色）；贴纸消息（点输入框右侧贴纸图标 → 弹层按「日常/猫咪/礼物」三包切换 → 点任意贴纸立刻作为一条消息发出并显示 132×132 贴纸卡；点礼物图标直达礼物包；再点同一图标收起；切频道/开设置时自动收起）；消息右键菜单（右键任意消息行 → 在光标处弹出菜单：添加回应把表情层打开、回复把「回复某人」提示条挂上、编辑（自己的消息）进入行内编辑、置顶后标题栏图钉面板能看到、复制文本进剪贴板、删除后消息消失；点菜单外或再次右键即关闭；悬停省略号入口仍可用同一组操作）；成员上下文菜单（右键成员栏任意成员 → 弹出菜单，头部显示头像/名字/角色与在线小点；「查看资料卡」打开右侧资料卡、「发送私信」跳到与他的私信并补建会话、「在输入框提及」把 @名字 填进输入框并聚焦、「复制用户名」复制 @名字、「屏蔽成员」后该名字出现在好友页「已屏蔽」分组并可在那里取消，菜单再次打开显示「取消屏蔽」；点菜单外任意位置关闭）；搜索中心（点标题栏搜索框右侧放大镜 → 主区域变成搜索页，输入关键词实时出结果、切「当前服务器/全部服务器/私信/全部」chips 重算、命中片段蓝色高亮、点任意结果跳到对应频道或私信并自动退出搜索页、无结果有空状态、X 返回聊天区）；语音房间（点语音频道连接 → 底部状态条显示「语音已连接 · 点此查看房间」→ 点击弹出房间面板，看到自己与其他 3 位参与者、切换某人的静音状态、点 UserX 移出列表、底部可静音本机或断开；断开后面板与状态条一起消失）；探索页（点服务器栏底部指南针 → 整屏探索页，搜索关键词过滤卡片、切分类 chips 过滤、无结果显示空状态、点「加入」后服务器出现在服务器栏并自动进入、点 X 回到原频道视图，再次打开已加入卡片显示「已加入」）；服务器设置（服务器名/⌄ → 服务器设置 → 切到全屏页：概览改名后侧栏与顶栏同步、消息记录与分类折叠状态保留、6 个色块点击后徽标颜色与服务器栏同步、简介保存；角色页列出各角色分组与成员；表情页可移除并恢复表情；邀请页复制链接后按钮变「已复制」、打开邀请弹层会回到主界面；右上角 X 返回主界面）；附件消息（点输入框左侧 + → 弹层列出 5 个模拟文件、点选后变为待发附件条（文件名/大小/待发送 + X 移除）、可直接点发送只发附件、发送后消息内出现图片占位卡或文件卡，切换频道后草稿清空）。；消息转发（消息右键/省略号 → 转发消息 → 弹层显示来源预览，搜索框过滤频道与私信，点目标后弹层关闭、列表顶部出现「已转发到 #xxx」提示、目标会话里多出一条带「转发自 #原频道」的消息且原消息不变；点来源标签跳回原会话；转发到当前会话自动滚到底部；提示条 X 可关闭，切频道后自动消失）
4. 测试结束后**必须关闭应用进程**，不要留后台残留：

```bash
pgrep -fl 'target/debug/discord-gpui'   # 确认无残留
```

---

## 7. 环境注意事项

- **rustfmt 未安装**：本机 `cargo fmt` 会报 `cargo-fmt is not installed`。如需统一格式化，先 `rustup component add rustfmt`；在此之前手工保持格式一致。
- **截图限制**：GPUI 窗口常处于其他 macOS Space，`screencapture` 和 `CGWindowListCreateImage` 都可能抓不到像素。验证以「窗口存在 + 进程正常 + 编译通过」为准，不要因为截图失败误判为渲染异常。
- **不要使用过时文档**：遇到 API 不确定时，直接读本地 registry 里的 `gpui-kit-0.10.0` / `gpui-component-0.10.0` / `gpui-base-0.10.0` 源码，不要搜索 0.7 时代的教程。
- **依赖锁定**：`Cargo.toml` 保持只有 `gpui-kit = "0.10.0"`。升级版本前必须完整回归编译与交互验证。

---

## 8. 禁止事项

- 禁止把业务逻辑写进 `ui/` 视图模块。
- 禁止把 widget 代码写回 `main.rs` 或 `app.rs`（顶层骨架除外）。
- 禁止在 `data/` 中引用 `crate::app` 或 `crate::ui`。
- 禁止让两个整屏覆盖层（用户设置 / 服务器设置）同时可见。
- 禁止使用重复或无语义的 Element ID。
- 禁止裸写颜色值绕过 `palette`。
- 禁止在未执行 `cargo check` 的情况下声称改动完成。
- 禁止在测试后遗留运行中的应用进程。
