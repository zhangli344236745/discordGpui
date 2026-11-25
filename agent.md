# agent.md — discord-gpui 开发规范

本文件是本项目的**开发契约**。后续任何人（包括 AI Agent）在此项目内新增功能、修改代码或重构时，必须先遵守本规范。规范的目标是：单一职责、依赖清晰、可编译验证、不重复踩 gpui-kit 的 API 坑。

---

## 1. 项目定位

- **产品**：一个 Discord 风格的桌面 IM 界面 Demo，纯前端模拟数据，无后端；内容状态通过本地 JSON 快照持久化。
- **优先级**：界面与交互功能优先；数据层保持可替换，方便未来接入真实 API。
- **技术栈**：Rust 2024 edition + gpui-kit `0.10.0` + serde / serde_json（仅数据层与快照使用）。
- **配色**：Discord 深色主题；涨红跌绿等金融约定与本项目无关。
- **后端（已落地骨架）**：`docs/backend-api.md` 是唯一后端契约（167 端点 / 30 事件 / 51 表），`server/` 是它的 FastAPI 实现——166 个端点可运行，`/servers/{id}/boost/subscribe` 按 Demo 约定返回 501。**改界面行为后必须回看文档是否需要同步；改接口后必须同时改 `server/` 与文档**。
- `server/` 与 Rust 工程完全解耦：不参与 `cargo` 构建，`server/.venv`、`server/data/*.db`、`server/data/uploads/*` 都在 `.gitignore` 里。

---

## 2. 目录结构与职责边界

```text
src/
├── main.rs                   # 仅：应用初始化、主题、窗口配置、打开窗口
├── app.rs                    # DiscordApp 状态 + ViewMode（Server/DirectMessage/Friends/Discover）+ 业务方法 + 顶层 Render 分发
├── data/
│   ├── mod.rs                # re-export models::* 与 seed::*；persist 单独 pub mod
│   ├── models.rs             # 纯数据结构与构造函数（含 Friend / DmConversation / UserStatus / Attachment）
│   ├── seed.rs               # 模拟数据工厂（服务器/频道/消息/成员/好友/私信/可选附件清单）
│   └── persist.rs            # 本地快照：Snapshot 结构、读写路径、反序列化镜像类型（*Owned）
└── ui/
    ├── mod.rs                # 声明子模块，re-export ico / avatar / unread_badge
    ├── theme.rs              # 色板常量 palette + ico() 图标工具
    ├── widgets.rs            # 公共组件库：avatar / 徽标 / 分组标题 / 图标按钮 / 开关行 / 筛选 chip
    ├── server_rail.rs        # 服务器栏视图（含私信主页入口、探索入口）
    ├── discover.rs           # 探索页（整屏：搜索 + 分类筛选 + 公开服务器卡片 + 加入）
    ├── settings/             # 用户设置页（全屏覆盖主界面）
    │   ├── mod.rs            # 分类数据源 SECTIONS + 全屏骨架 + 顶栏关闭
    │   ├── sidebar.rs        # 左侧设置分类栏
    │   └── pages.rs          # 各分类内容 + 公共片段（switch_row/volume_row 等）
    ├── server_settings/      # 服务器设置页（全屏覆盖主界面，与用户设置页互斥）
    │   ├── mod.rs            # 分类数据源 SECTIONS + 顶栏（服务器徽标/名称）+ 关闭
    │   ├── sidebar.rs        # 左侧分类栏（概览/角色/表情/邀请）
    │   └── pages.rs          # 四个页面：概览改名换色、角色管理、表情增删、邀请管理
    ├── status_menu.rs        # 在线状态选择弹层（绝对定位）
    ├── channel_sidebar.rs    # 频道侧栏视图（服务器菜单入口 + 分类 + 新建频道入口 + 频道/分类右键菜单）
    ├── channel_menu.rs       # 频道操作菜单浮层（标记已读/编辑主题/删除频道，含透明遮罩）
    ├── category_menu.rs      # 分类操作菜单浮层（新建频道/标记已读/重命名/删除，含透明遮罩）
    ├── category_dialog.rs    # 分类弹层（新建 / 重命名，复用 create_dialog::dialog）
    ├── server_menu.rs        # 服务器下拉菜单（邀请/静音/标记已读/创建频道/离开，贴侧栏头部下方）
    ├── invite_dialog.rs      # 邀请好友弹层（展示邀请链接，提交即复制到剪贴板）
    ├── create_dialog.rs      # 新建服务器（名称 + 模板选择，自定义三层模态）/ 新建频道（通用 dialog 骨架）
    ├── topic_dialog.rs       # 频道主题编辑弹层（复用 create_dialog::dialog）
    ├── dm_call_panel.rs      # 私信语音/视频通话浮窗（呼叫中/已接通、设备控制、挂断，挂根层跨视图保持）
    ├── dm_sidebar.rs         # 私信侧栏（真实搜索框 + 好友入口 + 会话列表 + 发起对话）
    ├── friends.rs            # 好友页（在线/待处理/全部/已屏蔽/添加好友 五个标签，点击发起私信）
    ├── help_center.rs        # 帮助中心浮层（标题栏 ? 打开：左侧分类导航 + 关键词搜索 + 文档列表 / 详情）
    ├── forum.rs              # 论坛频道页（帖子列表/搜索排序 + 帖子详情回帖 + 发帖弹层，接管聊天主区域）
    ├── welcome.rs            # 服务器欢迎屏幕（顶层模态：介绍 + 规则同意 + 推荐频道多选）
    ├── user_footer.rs        # 侧栏底部用户控制条（频道栏/私信栏共用）
    ├── member_list.rs        # 成员栏视图（按角色分组 + 折叠 + 点击成员弹资料卡 + 右键上下文菜单）
    ├── member_menu.rs        # 成员上下文菜单（资料卡/私信/提及/复制名/屏蔽，光标锚点 + 透明遮罩）
    ├── profile_card.rs       # 用户资料卡弹层（绝对定位）
    ├── voice_status.rs       # 语音连接状态条（绝对定位，跨视图保持，点击打开语音房间）
    ├── voice_room.rs         # 语音房间面板（参与者/静音/移出/本机麦克风/断开）
    ├── search_center.rs      # 搜索中心（接管聊天主区域：范围 chips + 跨频道/私信结果 + 命中高亮 + 跳转）
    └── chat/
        ├── mod.rs            # 声明 attachment_picker / attachment_preview / composer / edit_history / emoji_picker / forward_dialog / gif_picker / header / inbox_panel / messages / message_menu / pinned_panel / quick_switcher / read_receipts / sticker_picker / thread_panel
        ├── header.rs         # 聊天标题栏（频道/私信自适应）、就地过滤输入框 + 放大镜进搜索中心、置顶/收件箱开关（带未读角标）
        ├── messages.rs       # 消息列表、搜索过滤、表情回应、附件卡片、消息工具栏、作者点击
        ├── composer.rs       # 输入框、待发附件条、回复提示条、发送按钮、@提及补全浮层
        ├── attachment_picker.rs # 附件选择弹层（模拟文件清单，选中后成为待发附件）
        ├── attachment_preview.rs # 附件预览弹层（点击消息附件卡打开放大详情）
        ├── emoji_picker.rs   # 表情选择器弹层
        ├── message_menu.rs   # 消息操作菜单（行内省略号入口 + 右键光标浮层，共用一份操作项）
        ├── forward_dialog.rs # 转发消息弹层（搜索目标频道/私信，点击即转发）
        ├── edit_history.rs   # 编辑历史浮层（光标锚点卡片，倒序列出编辑前的正文与版本时间）
        ├── gif_picker.rs     # GIF 搜索弹层（搜索框 + 分类标签 + GIF 网格，点击即发送）
        ├── sticker_picker.rs # 贴纸弹层（包标签 + 贴纸网格，点击即发送）
        ├── thread_panel.rs   # 话题面板（原消息上下文 + 回复列表 + 独立输入条，覆盖聊天区右侧）
        ├── pinned_panel.rs   # 置顶消息面板（绝对定位，覆盖聊天区右侧）
        ├── quick_switcher.rs # 快速切换器（Ctrl/⌘+K 搜索频道/私信并跳转）
        ├── read_receipts.rs  # 已读回执详情浮层（光标锚点卡片，列出读者与时间）
        ├── bookmarks_panel.rs # 消息书签面板（跨频道/私信汇总 + 搜索 + 跳转高亮 + 移除）
        └── inbox_panel.rs    # 收件箱/通知中心（绝对定位，聚合提及/未读/私信/好友请求）
```

### 视图模式（ViewMode）

`app.rs` 中的 `ViewMode` 决定主布局：

- `Server`：服务器栏 + 频道侧栏 + 聊天区 + 成员栏。当前频道为 `ChannelKind::Forum` 时，聊天区由 `ui/forum.rs` 接管（帖子列表 / 详情，成员栏仍保留）。
- `DirectMessage`：服务器栏 + 私信侧栏 + 聊天区（无成员栏），标题用 `@好友名`。
- `Friends`：服务器栏 + 私信侧栏 + 好友页。
- `Discover`：服务器栏 + 探索页（独立整屏页面，不渲染频道/私信侧栏与成员栏；`render_main` 在最前面单独分支返回）。

论坛频道是第三种频道类型，不承载聊天消息（`messages` 表无对应键），帖子单独存在 `forum_posts` 表里，归属用 `server` + `channel` 两个字符串标记。渲染分发在 `render_main` 的 `main` 分支里通过 `is_forum_view()` 判断。

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

> **实践结论**：视图模块统一用 `use gpui_kit::*;` 打底，再按需从 `component` 根显式导入 `h_flex` / `v_flex` / `Size` / `Sizable` / `Input` 等。公共组件库 `widgets.rs` 同样用 `use gpui_kit::*;` 打底，再显式导入 `assets::IconName` / `component::{h_flex,v_flex,Size}` / `prelude::FluentBuilder`，不要手写零散 trait 导入（会漏 `hover` 等方法）。

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
- **公共组件**：头像、徽标、分组标题、图标按钮、开关行、筛选 chip 等可复用片段放 `ui/widgets.rs` 并由 `ui/mod.rs` 导出；只服务单一视图、或形态异构（如好友页图标 tab、成员行、模态标题栏）的片段留在该视图模块内作为私有辅助函数，不要为复用而过度泛化。
- **模型构造**：用 `Message::new(...).grouped().reaction(...)` 这类 builder 风格；新增字段时保持构造函数向后兼容。
- **状态字段**：`DiscordApp` 的字段一律 `pub(crate)`；外部读取通过字段，修改通过方法。
- **消息索引键**：统一为 `format!("{}/{}", server_name, channel_name)`，只在 `active_key()` 里生成，不要在别处拼字符串。
- **注释**：模块顶部用一行中文注释说明职责；复杂 API 约定可就近写简短注释，但不要逐行解释代码。
- **格式化**：保持 rustfmt 风格（4 空格、链式调用每行一个方法）。

### 公共组件库约定（`ui/widgets.rs`）

- 公共组件只做渲染与事件转发，不持有业务状态；交互回调统一是 `ClickHandler = dyn Fn(&mut DiscordApp, &mut Window, &mut Context<DiscordApp>) + 'static`，调用方传 `Box::new(|this, window, cx| ...)`，组件内部用 `cx.listener(move |this, _, window, cx| { handler(...); cx.notify(); })` 包装。
- 交互组件：`icon_button(icon, icon_size, side, id, active, handler, cx)`（幽灵按钮，`active` 只切主要/次要文字色，返回 `Stateful<Div>` 可继续 `.when()` 挂徽标）、`danger_icon_button`（危险操作，hover 变 danger + 白字）、`close_button`（弹层右上角 X，28×28）、`toggle_row`（设置页开关行）、`filter_chip`（搜索/探索页文字胶囊）、`menu_item(icon, label, trailing, id, handler, cx)`（浮层菜单行：图标 + 文案 + 可选右侧补充，返回 `Stateful<Div>` 可再 `.text_color()` 覆写）、`radio_item(title, hint, selected, id, handler, cx)`（互斥单选行：左侧圆环 + 标题 + 说明两行，48px）、`point_icon_button(icon, id, handler, cx)`（28px 行内图标按钮，回调额外拿到点击坐标 `(x, y)`，用于把浮层锚到光标处；handler 类型是 `Box<dyn Fn(&mut DiscordApp, f32, f32, &mut Context<DiscordApp>)>`）。纯展示组件：`section_label`（分组小标题，11px 加粗 muted）、`menu_divider`（菜单分隔线）、`avatar` / `avatar_status` / `unread_badge` / `unread_badge_muted`（灰底角标，静音频道用）/ `draft_badge`。
- `icon_button` 有 `side: Pixels` 参数覆盖 18/24/28 等尺寸；**不能**表达的颜色语义（如通知按钮的 `text_message`/`danger` 双色）保留视图内联实现，别硬塞进公共组件。
- 接入时若回调闭包不使用 window/cx，用 `|this, _, _|` 避免 unused 警告；`impl Into<ElementId>` 参数上调用方**直接传字符串字面量**（不要写 `"xxx".into()`，多 impl Trait 参数会导致 E0283 类型歧义）。
- 抽取边界：只抽形态同构、≥2 处复用的片段。好友页图标 tab（带 badge 与方块圆角）、通用标题栏（左侧内容异构）、成员/参与者行、server_settings 表单卡属于高风险过度泛化，保持各自私有实现。

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

- 角色是**一等数据**：`app.roles: Vec<Role>`（`Role { name: String, color: u32 }`，顺序即层级），成员的归属写在 `Member.role: Option<&'static str>`（存角色名，None = 无角色）。
- `app::member_groups() -> Vec<MemberGroup>` 按角色表顺序成组，空角色不出现在成员栏；无角色成员再按在线状态归入「在线」「离线」兜底组。组内含 `member_indices`（指向 app.members 的索引）。
- 颜色取 `app::role_color(name)`（角色表查不到退回 ACCENT），兜底组用 TEXT_MUTED/TEXT_DIM；视图不要自己写死颜色。
- 折叠状态存 `collapsed_roles: Vec<String>`，key 形如 `role-{角色名}`，点击走 `app::toggle_role_group(key)`；角色改名时 `save_role_name` 会把 key 一起迁移。
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
- 屏蔽入口在好友行右侧的 Ban 图标。**好友行必须是「主区域 + 按钮」的兄弟结构**：可点开私信的部分（头像 + 文案）是一个带 `on_click` 的子元素，Ban / 移动分组两个按钮是它的兄弟节点。早期把它们写成整行 `on_click` 的子节点，点子按钮会连带触发整行的私信跳转（GPUI 的 click 会沿树冒泡，独立 id 并不能阻断）。

### 好友分组约定

- 数据分两半：`Friend.group: String`（空字符串 = 未分组）记录成员归属；`app.friend_groups: Vec<String>` 只记录**分组顺序**。两者都进快照；`collapsed_groups`（折叠状态）只在内存里，重启后全部展开。
- `friend_group_names()` 是唯一的展示顺序入口：顺序表在前 → 好友身上多出来但表里没有的分组按首次出现补齐 → 空名（未分组）永远最后。顺序表里没有成员的空分组**仍然返回**，用户才有机会进去改名或删掉。
- `friends_in_group(name)` / `group_online_count(name)` / `friend_group_of(friend)` 是读取三件套，视图不再自己 filter。
- 写操作：`create_group`（空名 / 「未分组」/ 重名静默忽略）、`save_group_rename`（同步顺序表 + 成员 `group` + 折叠列表）、`delete_group`（成员回到未分组，顺序表与折叠状态一起清）、`move_group(name, ±1)`（先 `normalize_groups()` 把散落分组补进表再 swap，到边界不动）、`set_friend_group(friend, group)`（目标分组不在表里就追加）。
- 浮层两个，都在 `ui/friend_group_menu.rs`，都用「遮罩 + 卡片」兄弟节点：`render` 是分组操作（重命名 / 上移 / 下移 / 删除），右键分组标题或点行尾省略号打开 → `open_group_menu`；`render_picker` 是「移动到分组」（列全部分组、当前项打勾、底部新建分组），点好友行文件夹按钮打开 → `open_group_picker`。两个入口都会先收起资料卡 / 消息菜单 / 各类菜单，避免叠层。
- UI 位置：只改**全部**标签（`FriendTab::All`）为分组视图，在线标签保持「在线 / 离线」两段。分组标题 = 折叠箭头（`ChevronRight` / `ChevronDown`）+ 11px 粗体名 + 人数 + 「N 在线」，整行点击走 `toggle_group_collapsed`；「未分组」不给菜单按钮也不开菜单。
- 新建 / 重命名复用 `create_dialog::dialog()` 骨架（`ui/friend_group_dialog.rs`），共用一个 `group_input`，按 `show_create_group` 分流；Enter 由 `subscribe_in(&group_input)` 分发。
- Esc 收口顺序排在最前：分组弹层 → 移动菜单 → 分组菜单 → 其余浮层。

### 私信侧栏搜索约定

- 顶部搜索为真实 `InputState`：`dm_search: Entity<InputState>` + `dm_query: String`，订阅 `InputEvent::Change` 更新 `dm_query`。有内容时显示 X 清除按钮（`clear_dm_search`）。
- 搜索按好友名或消息预览（不区分大小写）过滤会话；名字匹配但**尚无会话**的好友显示在「发起对话」分组，点击走 `app::start_dm`。
- `start_dm(friend: &'static str)`：若会话不存在则补建 `DmConversation` 条目（颜色/在线状态取好友）并初始化空 `dm_messages`，再走 `open_dm` → `reset_session`（会清空 dm 搜索）。`reset_session` 同时调用 `clear_search` 与 `clear_dm_search`。
- 搜索无结果时显示空状态；搜索模式隐藏好友入口与常规标题。会话行抽成 `conversation_row()` 复用，不依赖 `app`（只取渲染所需参数）。

### 新建服务器 / 频道约定

- `Server.name` / `Server.badge` 为 `String`（支持运行时新建），`Server::new(name, color)` 生成带默认「文字频道/闲聊」的服务器，徽标取名称首字符。
- 服务器栏 `+`（`rail-create-server`）→ `app::open_create_server`；频道分类 `+` → `app::open_create_channel`（仅服务器视图可用）。由 `show_create_server` / `show_create_channel` 控制显隐，互斥。**新建服务器**用 `create_dialog.rs` 里自定义的三层模态（名称输入 + 模板卡，卡片不能嵌进可点击遮罩，否则点模板会冒泡关窗）；**新建频道**仍走通用 `dialog()` 骨架。
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
- 分类：`ServerSettingsSection`（Overview/Roles/Permissions/AutoMod/Emoji/Invites/Members/Audit/Boost），文案与图标唯一数据源是 `ui/server_settings/mod.rs::SECTIONS`，切换走 `select_server_settings_section`。
- 概览：名称与简介各一个 Input，Enter（订阅在 `new` 里）或「保存」按钮都调用 `save_server_overview`。改名是**带迁移**的：同步搬走 `messages` 里 `旧名/` 前缀的键、重写 `collapsed` 里的折叠键，并按新名首字更新 `badge`；空名或未改名不生效。图标颜色由 6 个色块 `set_server_color` 点击即生效。
- 角色：主从布局（左列表 / 右详情），数据来自 `app.roles` 与 `app.members`，见「角色管理约定」。
- 权限：见下方「权限管理约定」；纯配置演示，不校验真实操作。
- 自动审核：见下方「自动审核约定」；唯一一个会拦截真实操作的设置页。
- 表情：`server_emojis` / `removed_server_emojis` 两个向量，`remove_server_emoji` 与 `restore_server_emoji` 互转；Demo 不做真实上传。
- 邀请：见下方「邀请管理约定」；入口按钮 `open_invite` 会关掉服务器设置页（弹层挂在主界面）。
- 成员：见下方「成员管理约定」；搜索框、筛选与昵称弹层都在 app 层。
- 审核日志：见下方「审核日志约定」；纯列表页，不持有表单状态。
- 打开用户设置页（`open_settings`）会关闭服务器设置页；切换分类时清掉 `invite_copied_code`（列表里的「已复制」标记）。

### 角色管理约定

- 数据：`app.roles` + `Member.role`（角色名做归属键，改名时同步成员、折叠 key、`selected_role`）。种子在 `seed_roles()`：管理员红 / 版主绿 / 贡献者蓝。
- 页面状态：`selected_role: Option<String>`、`role_name_edit`（名称输入，Enter 与「保存」都走 `save_role_name`）、`role_notice`（保存提示）。进入「角色」分类时若未选中会自动选中第一个角色。
- 操作：`create_role`（名称 `新角色 N` 去重 + 颜色按 `ROLE_COLORS` 轮换，建完即选中）、`save_role_name`（空名 / 重名拦下）、`set_role_color`（点色块即生效）、`delete_role`（成员回到无角色、清折叠 key）、`toggle_member_role`（点成员行在「当前角色 / 无角色」间切换）。
- 布局：左列表（色点 + 角色名 + 人数 + 选中底色）与右详情（名称输入、6 色块、成员勾选行、删除按钮）都是纯视图，逻辑只在 app 层。
- `ROLE_COLORS` 是 `pub(crate)`，UI 直接复用同一份色值，避免两处各写一套。
- 持久化：`Snapshot.members` / `Snapshot.roles` 都是 `Option`，旧快照没有这两个字段时保留种子而不是清空。

### 权限管理约定

- 数据：`PermissionState` 三态枚举（`Unset` 继承 / `Allow` 允许 / `Deny` 拒绝，`next()` 循环、`label()` 文案），`PermissionGrant { role: String, permission: String, state }`（全自有类型，serde 直接派生，无 `*Owned` 镜像）。
- 权限清单是 data 层常量 `PERMISSIONS: &[PermissionDef]`（`key` 是快照稳定键，13 项，分 `PermissionGroup::{General,Text,Voice,Advanced}` 四组），视图遍历它按组渲染；`key` 一旦写入快照就不要改。种子 `seed_permissions()` 给三个角色预置 35 条非默认项。
- 只存非默认项：`app.role_permissions` 里没有对应记录即视为 `Unset`。`app::permission_state(role, key)` 是唯一查询入口；`permission_role()` 取当前选中角色，没选中回落到第一个。
- 写操作 `app::cycle_permission(role, key, label, cx)`：读旧态 → `next()` → 有记录则改态（`Unset` 时删除记录）/无记录且非 `Unset` 则新增 → 写 `permission_notice` → 记 `AuditKind::Role` 日志。视图行按钮用 `ui/server_settings/pages.rs::permission_state_button`，颜色走 `palette::success/danger/bg_rail`，点击回调四参闭包里直接调 `cycle_permission`。
- 角色改名 `save_role_name` 要把 `role_permissions` 里的 `grant.role` 一起迁移；`delete_role` 要 `retain` 掉该角色记录；`restore_state` 也要丢弃指向已不存在角色的脏记录。
- 页面状态复用 `selected_role`（角色页共用）：顶部角色 tabs 走 `select_permission_role`（只切聚焦、不写角色名输入草稿）；进权限分类时若没选中会自动选第一个；切分类清 `permission_notice`。
- 视图布局：`shell` + 角色 tabs 卡 + 按 `PermissionGroup::ALL` 分组的权限行卡（每行：标签 + hint + 76px 三态按钮，id `permission-toggle-{role}-{key}`）。纯演示，**权限位不校验任何真实操作**。
- 持久化：`Snapshot.permissions: Option<Vec<PermissionGrant>>`，旧快照缺字段时保留种子；往返测试已覆盖。

### 自动审核约定

- 数据：`AutoModAction { Block, Warn }`（`label()` 文案），`AutoModRule { name, description, keywords: Vec<String>, action, enabled }`（全自有类型，serde 直接派生；`enabled` 带 default=true）。`matched_keyword(text)` 大小写不敏感、忽略空关键词、返回首个命中的原始关键词。种子 `seed_automod_rules()` 三条（广告链接拦截=Block 启用 / 刷屏提醒=Warn 启用 / 不友善用语=Block 停用）。
- app 状态：`automod_rules` + `selected_automod` + `automod_name_edit` / `automod_keyword_edit`（两个 InputState，Enter 订阅分别走 create / add）+ `automod_notice: Option<String>`（聊天区提示条）。
- 查询：`automod_rule_focus()` 取选中规则、无则回落第一条；`automod_check(text) -> Option<(AutoModAction, 规则名, 关键词)>` 只遍历启用规则，按规则顺序取首个命中。
- 写操作：`create_automod_rule`（名非空不重名、默认 Block 启用）、`delete_automod_rule`、`toggle_automod_rule`、`set_automod_action`、`add_automod_keyword`（去重忽略空）、`remove_automod_keyword`、`select_automod_rule`（清空关键词草稿）、`dismiss_automod_notice`。凡是调用 `set_value` 的方法都要带 `window: &mut Window` 参数。所有变更写 `AuditKind::Server` 日志。
- **拦截落点**：`send_message` 里在清空 composer **之前**检查——仅 `ViewMode::Server` 且文本非空时调 `automod_check`；`Block` 则设 `automod_notice`、不清草稿、`return` 中止；`Warn` 则设提示后照常发送。`reset_session` 与切服务器设置分类都会清 `automod_notice`。
- 视图：`ui/server_settings/pages.rs::automod` 用「左规则列表+新建输入 / 右详情（启用开关 / 动作二选一 `action_button` / 关键词 chips 带删除 / 关键词输入 / 删除规则）」；`set_value` 走 `<Input as Sizable>::with_size`。拦截提示条 `ui/chat/messages.rs::automod_notice_bar`（danger 描边 + ShieldAlert，点 X 走 `dismiss_automod_notice`），挂在消息列表顶部、与 `forward_notice` 并存。动态 notice 文案要先 `clone()` 成 String 再渲染，不能把 `app` 的借用直接传给 `hint(&'static str)`（会 E0521）。
- 持久化：`Snapshot.automod_rules: Option<Vec<AutoModRule>>`，旧快照缺字段时保留种子；restore 时选中规则不存在则清空；往返测试已覆盖。

### 成员管理约定

- 数据：`Member` 除 `name`（身份键）/ `role` 外，新增 `nick: Option<&'static str>`（服务器昵称，只影响展示）与 `joined_minutes`；`display()` 返回昵称或原名，`joined_label()` 给「3 天前加入」这类文案。昵称用 `opt_leaked_str` 持久化，`Member` 仍保持 `Copy`。
- 封禁：`data::BannedUser { name, reason, minutes, by }` 全自有 String，serde 直接派生；`reason_label()` 对空原因回退「未填写原因」，`time_label()` 给「刚刚封禁 / N 分钟前封禁」。
- 页面状态：`member_search`（InputState）、`member_filter: MemberFilter`（`All / Role(String) / NoRole`）、昵称弹层的 `show_member_nick` + `member_nick_target` + `member_nick_input`。
- 查询与筛选：`member_query(cx)` 取小写关键词；`filtered_members(cx)` 返回命中成员的索引（名字与昵称都参与匹配，保持 `members` 原顺序），`member_count_of(&filter)` 只按角色筛选计数，供筛选条显示人数。
- 操作：`open_member_nick` 预填当前展示名 → `save_member_nick`（留空或填回原名都等于清昵称；只在展示名真的变化时写日志）、`kick_member`（移出列表，顺带关掉指着他的资料卡 / 右键菜单）、`ban_member`（移出 + 插入封禁列表，原因留空）、`unban_member`（只删记录，不自动加回）。三者都写 `AuditKind::Member` 日志。
- 视图：地址栏式的搜索行（图标 + Input + 条数 + 有内容时的清除按钮）、筛选 chips（全部 / 每个角色 / 无角色，都带人数）、成员行（头像 + 展示名 + `@原名` + 角色色点 badge + 加入时间 + 三个图标按钮）、封禁卡（原因 · 时间 · 操作人 + 撤销）。
- 成员栏（`ui/member_list.rs`）与成员行同源：`member_row` 的 `identity` 用于资料卡 / 菜单，`display` 只用于渲染首字与文字。
- 持久化：`Snapshot.members` 已含 `nick` / `joined_minutes`，`Snapshot.bans` 是 `Option<Vec<BannedUser>>`，旧快照缺字段时保留种子。

### 邀请管理约定

- 数据：`data::Invite { code, server, channel, inviter, uses, max_uses, expiry, created_minutes }`，`server` 按**服务器名**归属（与消息键一致），`max_uses == 0` 表示不限次数，`expiry` 是 `InviteExpiry` 枚举（30 分钟 ~ 7 天 / 永不），`minutes() == 0` 表示永不过期。
- 状态与文案全部在模型上算，视图不重复判断：`expired()` → `used_up()` → `status()`（过期优先于用完），`uses_label()`、`expiry_label()` 负责「12 / 不限」「3 小时后过期」这类展示串。
- 表单状态在 app：`invite_channel`（落地频道）、`invite_expiry`、`invite_max_uses`、`invite_copied_code`。频道失效时 `sync_invite_channel()` 回落到第一个文字频道——切换服务器、删除频道、读回快照后都要调一次。
- 创建 `create_invite`：邀请码 = `invite_code()`（ASCII slug / `gpui-N`）+ `-序号`，与已有码循环去重；写入列表头部、写剪贴板、标记 `invite_copied_code`。复制 / 撤销分别是 `copy_invite_link(code)` / `revoke_invite(code)`。
- 改名与删除的连带处理：`save_server_overview` 改名时把 `invites` 里同属旧名的记录改到新名下；`delete_channel` 会撤销指向该频道的邀请并重算落地频道。
- 持久化：`Snapshot.invites`（`InviteOwned` 镜像负责静态字符串还原），`cargo test` 里有往返断言。
- 弹层 `invite_dialog` 预填 `primary_invite_code()`（当前服务器最新一条，没有则退回 slug），因此从服务器菜单打开时看到的永远是列表里最新的链接。

### 审核日志约定

- 数据：`data::AuditEntry { seq, kind, actor, action, detail }` + `AuditKind { Server, Channel, Role, Invite, Emoji }`。全部字段都是自有 String，`Serialize` / `Deserialize` 直接派生，**不需要 `*Owned` 镜像**；`seq` 单调递增，越大越新。
- 写入只有一个入口 `app::log_audit(kind, action, detail)`：取 `audit_newest_seq() + 1` 作序号、`me.display_name` 作操作者，超过 `AUDIT_LIMIT`（200）就 `drain` 掉最旧的。业务方法里**先做变更再记录**，不要在失败分支里留日志。
- 已接入的操作：服务器改名 / 改简介 / 换图标色，建频道 / 删频道，建角色 / 角色改名 / 换色 / 删角色 / 授予与移除成员角色，创建邀请 / 撤销邀请，移除与恢复表情。新增类似操作时顺手补一条。
- 时间不存时间戳：`AuditEntry::time_label(newest)` 用「最新序号 - 本条序号」当分钟差，`0` 是「刚刚」、`<60` 分钟、`<1440` 小时、再往上按天。这样快照里没有时钟依赖，重启后也不会漂移。
- 视图：`filter_chip` 做「全部 + 五个类别」筛选（标签带条数），`app.audit_visible()` 返回倒序引用列表，`app.audit_count(kind)` 供计数；行首 30px 圆角方块用 `AuditKind::color()` 打底 + `audit_icon(kind)` 图标。清空走 `clear_audit`（不二次确认，与其它演示操作保持一致）。
- 持久化：`Snapshot.audit: Option<Vec<AuditEntry>>`，旧快照缺字段时保留种子日志（`seed_audit()` 8 条，`seq` 1~8）。

### 快捷键与快速切换器约定

- **全局按键只能挂在元素上**：`window.on_key_event` 有 `debug_assert_paint`，在 `Render::render` 里调用会启动即 panic（已踩过）。正确做法是在 `Render::render` 返回的根 `div()` 上 `.id("app-key-root").track_focus(&self.root_focus).on_key_down(cx.listener(...))`，并在 `DiscordApp::new` 末尾 `window.focus(&this.root_focus, cx)` 让根持有焦点；输入框聚焦时按键会冒泡到根，一样能收到。
- 唯一入口 `app::handle_shortcut(event, window, cx)`：`Ctrl/⌘+K`（`modifiers.control || modifiers.platform` + `key == "k"`）开/关快速切换器；`key == "escape"` 调 `close_top_overlay`；切换器打开时 `"up"/"down"` 调 `move_quick_selection(±1)`。特殊键名是小写（`escape`、`down`、`up`），不是 `Escape` / `ArrowDown`。
- `close_top_overlay` 按「最上层优先」依次关：行内编辑 → 快速切换器 → 附件预览 → 转发 → 搜索中心 → 服务器设置 → 用户设置 → 邀请/新建/主题/分类弹层 → 频道/分类/成员/消息菜单 → 置顶/收件箱 → 状态/贴纸/表情/附件/资料卡/语音房间 → 探索页；只关一层，不误伤底层状态。
- 快速切换器状态：`show_quick_switcher` / `quick_search` / `quick_query` / `quick_selected`。打开时清空搜索、聚焦输入框并收起所有浮层与整屏页；`quick_targets()` 复用 `forward_targets()` 的候选集合，按 `quick_query` 过滤。
- `InputState` 订阅里 `Change` 更新 query 并把高亮归零，`PressEnter { secondary: false }` 调 `activate_quick_selection`；上下键循环移动；点击行走 `activate_quick_at(index, ...)` 后与回车共用跳转逻辑。跳转：私信 `open_dm`，频道按服务器名定位后写 `active_server` / `active_channel` 并 `reset_session`。
- 视图 `ui/chat/quick_switcher.rs`：遮罩 `quick-switcher-backdrop`，顶部 128 居中宽 520 卡片 = 搜索框 + `max_h 360` 候选列表（44 高行、彩色圆标、名称 + 归属、高亮行右侧「↵ 跳转」）+ 底部「↑ ↓ 选择 / Enter 跳转 / Esc 关闭 / N 个会话」。标题栏搜索框右侧的 `⌘K` chip 是无键盘用户的等价入口。

### 回复引用约定

- 数据：`data::ReplyRef { author: &'static str, excerpt: String }`，`Message.reply_to: Option<ReplyRef>` + `.replying_to(author, excerpt)` builder，`new()` 默认 None。摘要取原消息正文前 60 字，超过加「…」；贴纸/附件/空消息退化为 `[贴纸]/[附件]/[空消息]`。
- 状态：`app.reply_to: Option<ReplyRef>`（回复草稿）、`reply_highlight: Option<usize>`（点击引用后高亮的原消息下标）。`toggle_reply_at(index, cx)` 从当前会话取摘要并支持再次点同一消息取消；输入条「取消」走 `cancel_reply()`；所有清 `reply_to = None` 的地方（reset/send/贴纸发送）也要清 `reply_highlight`。
- 发送文本 `send_message` 与发送贴纸 `send_sticker` 都用 `self.reply_to.take()` 把引用写入新消息后清空；转发 `forward_message` 克隆 `source.reply_to` 一并带走。
- 视图：`ui/chat/messages.rs::reply_quote_row`（正文上方、转发来源行之后）渲染「回复 @作者：摘要」，左侧竖线 + 链接色可点击，点击走 `app.jump_to_reply(excerpt, cx)`；后者按摘要前缀反查原消息 index，`scroll_handle.scroll_to_item(item)` 滚动过去（item 需算上 forward_notice 与未读分隔线的偏移），并置 `reply_highlight`。消息行 `.when(app.reply_highlight == Some(index), ...)` 用 `bg_reaction_mine()` 高亮，切会话即消失。
- 输入条 `composer.rs` 用 `app.reply_to.as_ref()` 读引用，显示「回复 @作者：摘要」。

### 话题（Thread）约定

- 数据：`data::ThreadReply { author, time, text, color }`（构造 `ThreadReply::new`）与 `data::ChatThread { root_index, title, replies }`（构造 `ChatThread::new`）。话题存在 app 的 `threads: HashMap<String, Vec<ChatThread>>`，键统一走 `session_key()`；`root_index` 是话题挂在**当前会话主消息列表**里的下标，不跨会话复用。
- 演示数据 `seed_threads()` 预置两条：频道 `开发者社区/闲聊` 的第 3 条、私信 `dm:Nova` 的第 1 条；用户可从任意消息新建话题，标题取原消息前 16 字，超出加「…」。
- 状态只有 `show_thread` / `thread_root` / `thread_input`（独立 `InputState`，Enter 订阅调用 `send_thread_reply`）。打开统一走 `toggle_thread(index, window, cx)`：同消息再次点击关闭，已有话题不重建，打开时清空回复输入并聚焦，同时关置顶/收件箱/菜单/资料卡。关闭走 `close_thread()`，不清空已保存的回复。
- 读取统一走 app 方法：`thread_count(index)`（消息徽标与菜单文案）、`current_thread()`（克隆面板数据）、`thread_root_message()`（面板顶部原消息）。视图不直接读 `threads` 字典。
- 发送回复 `send_thread_reply(window, cx)` 把文本包装成 `ThreadReply::new("你", "刚刚", text, palette::accent())` 追加到话题，清空输入。Demo 不把话题回复写回主消息列表，也不参与搜索/未读/转发。
- 面板 `ui/chat/thread_panel.rs`：绝对定位右侧 360 宽（与置顶面板同层），48 头部（使用公共 `close_button`）+ 可滚动原消息卡/`N 条回复` 分隔线/回复行 + 底部独立输入条（公共 `icon_button` 发送）。只在 `Server` / `DirectMessage` 视图挂载。
- 入口三处：消息工具栏 `msg-thread-{index}`（`thread_count > 0` 文案为「话题」否则「开话题」）、消息下方 `msg-thread-badge-{index}`（仅已有话题）、消息菜单 `msg-menu-thread-{index}`（「创建话题 / 查看话题」）。所有入口都调 `toggle_thread`，不在视图里改状态。
- 生命周期：`reset_session` / `close_top_overlay`（Esc）/ `toggle_pins` / `toggle_inbox` / `open_search_center` / `open_discover` / `open_settings` / `open_server_settings` 都会 `close_thread()`；`delete_message` 必须清理被删根消息的话题，并把同会话其余话题的 `root_index` 前移，当前打开的根被删时关闭面板。

### 送达状态与已读回执约定

- 数据：`data::DeliveryStatus { Sent, Delivered, Read }` 与 `data::ReadReceipt { name, time, color }`（`ReadReceipt::new` 为 const）。`Message` 新增 `delivery` 与 `readers`，`Message::new` 默认 `Sent`；builder `.delivered()` 标记已送达，`.read_by(readers)` 按读者是否为空自动落到 `Read` 或 `Delivered`。新发出的消息保持默认 `Sent`，Demo 不模拟后续状态推进。
- 视图只在 `message.author == "你"` 时渲染状态行：`messages.rs::delivery_row` 用 `Check`（已发送）/ `CheckCheck`（已送达、已读），已读且 `readers` 非空时显示「已读 N 人」并可点；其余状态只读。状态行挂在正文/回应/话题徽标之后、工具栏之前。
- 点击已读状态走 `on_mouse_down(MouseButton::Left)` 而不是 `on_click`，因为需要 `MouseDownEvent.position` 作为浮层锚点；调用 `open_read_receipts(index, x, y, cx)`，同时关消息菜单与资料卡。
- 浮层 `ui/chat/read_receipts.rs` 复用消息右键菜单的「全屏透明遮罩 + 绝对定位卡片」骨架：左/上取点击坐标并分别 clamp 到 1230 / 700，宽 260，头部用公共 `close_button`，主体 `max_h(240)` 可滚动，每行是头像 + 名字 + 时间。点遮罩、右键或 X 都调用 `close_read_receipts`。
- app 状态只有一个 `read_receipts: Option<(index, x, y)>`；详情读取统一走 `read_receipts_detail() -> Option<(index, Vec<ReadReceipt>)>`，读者为空则不渲染。Esc 在消息菜单之后、置顶面板之前关闭该浮层。
- 生命周期：所有关闭话题的路径（切会话 / Esc / 置顶 / 收件箱 / 搜索中心 / 探索 / 设置 / 服务器设置）同时关闭回执浮层；`delete_message` 迁移下标，删除的是被查看消息则直接关闭浮层，避免索引串位。

### 私信语音 / 视频通话约定

- 数据：`app::CallMode { Voice, Video }` 与 `app::DmCall { friend, color, online, mode, connected, mic_on, camera_on, screen_sharing }`。状态只有一个字段 `dm_call: Option<DmCall>`，Some 即渲染浮窗，None 即无通话；不额外加 `show_*` 布尔。
- 发起统一走 `start_dm_call(friend: String, mode, cx)`：自动关闭资料卡/快速切换器/收件箱/置顶/消息菜单，并先 `leave_voice()` 保证与服务器语音频道互斥；反向在 `toggle_voice` 加入频道时清空 `dm_call`。颜色与在线状态从 friends / members 聚合，未知联系人回退 `palette::accent()`。
- Demo 用两态：`connected=false` 显示「正在呼叫 @friend」+「模拟接通 / 取消」；`connect_dm_call(cx)` 推进到 `connected=true` 后显示设备控制栏。设备操作只有 `toggle_dm_call_mic/camera/screen(cx)`：关摄像头会同时停屏幕分享，开屏幕分享会自动开摄像头，避免两个画面语义打架。挂断统一 `end_dm_call(cx)`。
- 视图 `ui/dm_call_panel.rs` 挂在**根 Render 层**（`Render::render` 的 `app-key-root` 下），因此切频道、切私信、打开用户/服务器设置、进入探索页都不会消失；只有挂断或改连服务器语音才关闭。浮窗绝对定位 `right:24 / bottom:72`，宽 320，不遮挡左侧语音状态条。
- 预览区是明确标注的占位：呼叫中显示对方头像，接通后按 `screen_sharing > camera_on > 摄像头关闭` 三态切换；不采集真实麦克风/摄像头/屏幕，文案里不能写成真的音视频。
- 入口两处：DM 标题栏 `chat-dm-voice-call` / `chat-dm-video-call`（仅 `is_dm()` 时渲染，按 `dm_call_mode` 高亮），资料卡 `profile-voice-call` / `profile-video-call`（对任意资料卡目标都可发起）。全部复用公共 `icon_button`，不在回调里直接改 app 字段。
- 设置页「快捷键」列表要跟随新增交互更新：`Ctrl/⌘+K`、`↑/↓`、`Esc`、话题与通话相关入口需要在列表里可见。

### 会话草稿约定

- 数据：app 层 `drafts: HashMap<String, MessageDraft>` 按会话键存未发送内容，`MessageDraft { text: String, attachment: Option<Attachment> }`（定义在本文件，不放 data 层，因为含 UI 附件状态）。`current_session: String` 记录输入框当前归属的会话键。
- 会话键统一由 `session_key()` 生成：频道走 `active_key()`（`服务器/频道`），私信是 `dm:好友`，好友页 / 探索页返回空串（不保留草稿，输入框也被清空）。
- **切换时序很关键**：所有切会话入口都是先改 `active_*` / `view_mode` 再调 `reset_session`，所以 `reset_session` 开头调 `save_draft(cx)` 时用 `current_session`（还是旧键）存回旧会话；结尾在 placeholder 更新后调 `load_draft(window, cx)` 算出新键、更新 `current_session`、恢复新会话草稿。`set_value` 不触发 `InputEvent::Change`，不会误触发 @提及刷新。
- `new()` 末尾必须 `this.current_session = this.session_key()`，否则默认频道第一次输入的草稿会因为空键丢失。
- `send_message` 成功后 `set_value("")` 并 `drafts.remove(&current_session)`，避免已发内容残留草稿标记。
- 侧栏标记：`ui/widgets.rs::draft_badge()`（强调色小胶囊「草稿」）由 `ui/mod.rs` 导出；频道侧栏在 `name` 被 move 进 `.child(name)` **之前**先算 `has_draft = !is_voice && app.channel_has_draft(&name)`，避免 E0382；私信侧栏 `conversation_row` 多一个 `has_draft: bool` 参数，两处调用都传 `app.dm_has_draft(conversation.friend)`。判定统一走 app 的 `has_draft(key) / channel_has_draft(name) / dm_has_draft(friend)`，视图不直接读草稿字典。
- 空草稿（文本 trim 后为空且无附件）在 `save_draft` 里直接 remove 对应键，不留空标记。

### 未读分隔线与跳到最新约定

- 未读数的数据源是现有模型：`Channel.unread` 与 `DmConversation.unread`，**不**新增状态、不做持久化。切换会话时 `reset_session` 只滚到底部、不清未读，这样分隔线才可见。
- app 层三个方法：`current_unread_count()` 按 `ViewMode` 取当前频道/私信的未读数；`unread_divider() -> Option<(usize, usize)>` 给出 `(插入下标, 未读数)`，当未读为 0 或 ≥ 消息条数时不插入；`jump_to_latest(cx)` 先 `scroll_handle.scroll_to_bottom()` 再 `mark_current_read()`（私有方法，分别清 `Channel.unread` / `DmConversation.unread`）。
- 视图 `ui/chat/messages.rs`：渲染循环前一次性取 `divider`，命中下标时在消息行之前插 `unread_divider_row`（两侧红细线 + 居中 danger 胶囊「N 条新消息」，胶囊可点跳转），并把 `last_author` 清空避免跨分隔线分组；列表底部另有 `jump_latest_pill`（`absolute bottom 40 / right 20`，强调色胶囊 + `ArrowDown` + 「N 条新消息 · 跳到最新」），用 `.when(unread > 0, ...)` 挂载。
- 两个入口都只调 `app::jump_to_latest`，跳转后未读归零，分隔线与浮标同时消失；消息搜索过滤时分隔线仍按原消息下标插入，过滤掉前面的消息不会错位。

### 附件预览约定

- 入口：消息附件卡整卡点击。卡片函数签名是 `messages.rs::attachment_card(attachment, index, cx) -> impl IntoElement`——**必须**返回 `impl IntoElement`，因为 `.id()` 会把 `Div` 变成 `Stateful<Div>`，写死 `Div` 会报 E0308。
- 状态只有 `app.attachment_preview: Option<Attachment>`；`open_attachment_preview(attachment, cx)` 会顺带收起转发/置顶/收件箱/资料卡/状态/服务器菜单与消息菜单，`close_attachment_preview(cx)` 由遮罩、X、底部按钮共用，`reset_session` 也会清掉。
- 视图 `ui/chat/attachment_preview.rs` 是独立组件：遮罩 `attachment-preview-backdrop`（rgba 0x000000cc）+ 宽 560 卡片（52 头部 / 320 预览区 / 56 底部）。图片与文件共用布局，只换图标（`Image` / `FileText`）与文案；预览区是占位色块，不读磁盘。
- 图标放大用 `Size::Size(px(72.))`（`Size` 有 `Size(Pixels)` 变体，不用自己给 `Icon` 加样式）。
- 底部按钮复制文件名到剪贴板模拟「下载」：`copy_attachment_name(cx)`，复制后关闭弹层。没有真实文件与下载行为，README/规范里不要写成真的下载。

### 消息编辑历史约定
- 数据：`data::EditRevision { text: String, time: &'static str }`，`Message.edit_history: Vec<EditRevision>` + `.revised(text, time)` builder + `is_edited()`（非空即编辑过）；种子数据给两条自有消息预置历史。
- 写入：`commit_edit` 只在正文**确实变化**时把旧正文 push 到 `edit_history`（time 为「刚刚」）再覆盖；内容未变则不产生版本。
- 状态：`app.edit_history: Option<(index, x, y)>`（光标锚点）；`open_edit_history` 校验该消息有历史后打开，并与消息菜单 / 已读回执互斥；`open_edit_history_from_menu` 兼容右键（光标位置）与行内菜单（默认位置）。
- `edit_history_detail()` 返回**克隆**的 `Vec<EditRevision>`：`current_messages()` 产生临时值，不能返回引用（E0515）。切会话 / 关回执等清理路径已顺带 `close_edit_history()`；Esc 在回执之后关闭；`delete_message` 会迁移或关闭下标。
- 视图 `ui/chat/edit_history.rs` 复用回执的光标锚点骨架：倒序展示版本（最新在上）、每版显示「版本 N + 时间 + 正文」、底部注明「仅展示历史版本，不提供回滚」。
- 入口两个：`messages.rs` 作者行时间戳旁的「（已编辑）」小字（左键按下取 `event.position` 打开），以及 `message_menu.rs` 仅对有历史消息显示的「编辑历史」菜单项（`IconName::Clock`）。

### GIF 搜索面板约定
- 数据：`data::Gif { title, emoji, color, category, tags }`（Demo 用色块 + emoji 字形模拟动图），`Message.gif: Option<Gif>` + `.gif(gif)` builder；`seed_gifs()` 20 个分四类（热门 / 反应 / 搞怪 / 动物），`gif_categories()` 给出标签顺序。
- 入口：输入框右侧 GIF 按钮（`composer-gif`，`IconName::ImagePlay`）走 `toggle_gif_picker(cx)`；再点一次收起。打开时收起贴纸 / 表情 / 附件 / 资料卡 / 置顶 / 收件箱等浮层，贴纸、附件、表情、会话重置、`Esc` 也会反向收起 GIF 层（`close_sticker_picker()` 同时关 GIF）。
- 状态：`show_gif_picker / gif_query / gif_category / gif_search(InputState)`。搜索框 Change 更新 query，Enter 走 `send_first_gif`；`visible_gifs()` 有词时跨分类匹配标题 / 标签 / 分类，无词时按当前分类。切分类会清空搜索词。
- 点击 GIF 走 `send_gif`：生成 `Message::new("你", "刚刚", "", palette::accent()).gif(gif)` 直接入列，清搜索词与回复态、滚到底部、焦点还给输入框（好友视图直接返回）。转发摘要退化为 `[GIF] 标题` 且克隆 `gif` 字段。
- 视图 `ui/chat/gif_picker.rs`：底部 52 / 右侧 16、宽 348 的浮卡；搜索框 + 分类 chips（搜索时不高亮）+ 分隔线 + 3 列 GIF 格（色块 62px + emoji 30px + 标题）+ 底部「分类 / 搜索词 · 共 N 个」。⚠️ 此版本 `overflow_y_scroll` 要在 `.id()` 之后的 `Stateful<Div>` 上调用，直接挂 `Div` 会 E0599。
- 消息里的 GIF 由 `messages.rs::gif_card` 渲染：184×96 横版色块 + 40px emoji + 标题 + 「GIF · 演示占位」；正文为空时不渲染文本节点。

### 贴纸消息约定

- 数据：`data::Sticker { emoji, label, pack }`（Demo 用 emoji 字形代替贴纸图），`Message.sticker: Option<Sticker>` + `.sticker(s)` builder；种子 `seed_stickers()` 24 张分三包（日常 / 猫咪 / 礼物），`sticker_packs()` 给出标签顺序。
- 状态：`app.stickers` / `app.sticker_packs` / `app.sticker_pack`（当前包）/ `app.show_sticker_picker`。
- 入口两个，都走 `toggle_sticker_picker(Option<&'static str>, cx)`：输入框右侧**礼物按钮**（`composer-gift`）传 `Some("礼物")` 直达礼物包，**贴纸按钮**（`composer-sticker`）传 `None` 打开当前包；同一个包再点一次收起。
- 打开时与表情层 / 附件层 / 各类菜单 / 资料卡互斥（反过来打开表情层或附件层也会置 `show_sticker_picker = false`）；`reset_session` 与打开设置 / 服务器设置 / 探索页 / 搜索中心时统一 `close_sticker_picker()`。
- 点击贴纸走 `send_sticker`：生成 `Message::new("你", "刚刚", "", palette::ACCENT).sticker(s)` 直接入列、清回复态、滚到底部并把焦点还给输入框（好友视图直接返回）。
- 视图 `ui/chat/sticker_picker.rs`：底部 52 / 右侧 16、宽 348 的浮卡；顶部包标签（数据源 `app.sticker_packs`）+ 分隔线 + 4 列贴纸格（emoji 30px + 名称 10px）+ 底部「当前包 · 共 N 张」。切包走 `set_sticker_pack`。
- 消息里的贴纸由 `messages.rs::sticker_card` 渲染：132×132 圆角块 + 54px emoji + 名称；正文为空时不渲染文本节点，因此贴纸消息只有卡片。

### 消息投票约定
- 数据：`data::PollOption { label, voters: Vec<String> }`（voters 存投票者身份键，"你"是本机用户，用名单而非纯计数以支持判断"我投了哪项 / 取消"）+ `data::Poll { question, options, multiple, closed }`；全自有字段，serde 直接派生，`Message.poll: Option<Poll>` + `.poll(p)` builder。`MessageOwned` 加 `#[serde(default)] pub poll: Option<Poll>` 并在 `From<MessageOwned>` 透传，旧快照缺字段时投票为空。
- 种子：`seed_messages()` 的默认频道末尾有一条投票消息，预置了部分投票者让结果可见；快照往返测试断言 poll 的问题 / 选项 / 投票者名单完整往返。
- 创建草稿状态在 app：`show_poll_composer / poll_question_edit / poll_option_edit / poll_options_draft / poll_multiple`（草稿不进快照，已发起的投票随 Message 进快照）。入口 `composer-poll` 按钮（`IconName::ChartBar`）走 `toggle_poll_composer(cx)`；与附件 / 贴纸 / GIF / 表情 / 资料卡互斥。`add_poll_option`（去空白 / 去重 / ≤8）由选项框 Enter 触发，`send_poll` 由问题框 Enter 或按钮触发（选项 < 2 直接返回）。
- 业务：`send_poll` 生成 `Message::new("你","刚刚","",palette::accent()).poll(poll)` 入列、迁移回复引用、记 `AuditKind::Server`「发起了投票」；`vote_poll(index, option_index, cx)` 对当前会话第 index 条消息投票——单选先清掉"你"在其它选项的票，点已投项即取消，未结束才生效，记「参与了投票」；`close_poll(index, cx)` 仅作者"你"可结束，记「结束了投票」。
- 清理：`reset_session` 与 `close_top_overlay`（Esc）都走 `close_poll_composer`（清草稿 + 两个输入框 set_value）。⚠️ `toggle_poll_composer` 不碰 window，签名只有 `cx`。
- 视图 `ui/chat/poll_composer.rs`：底部 52 / 右侧 16、宽 360 的浮卡；问题输入 + 已添加选项 chips（序号 + 文本 + 删除）+ 选项输入 + 多选开关 + 发起按钮（选项 < 2 时置灰）。消息里的投票由 `messages.rs::poll_card` 渲染：选项行用绝对定位进度条按 `percent()` 铺底 + 单选圆点 + 标签 + `N% · M 票`，我的选择描 accent 边、领先项描 success 边；底部显示总票数 / 单多选 / 结束态 + 作者可「结束投票」。⚠️ 选项行 `.id()` 放到 `.hover()` 之后、直接挂 `.on_click()`（Stateful<Div> 上再调 `.hover()` 会类型错），点击安全性由 `vote_poll` 内部的 closed 判断保证。

### 服务器日程约定
- 数据：`data::ServerEvent { id, server, title, description, channel, host, attendees, starts_in_minutes, duration_minutes, category }` + `EventCategory {Game,Voice,Community,Other}`（label/color/ALL）+ `ServerEventStatus {Upcoming,Live,Ended}`，全自有字段 serde 直接派生。时间用「距现在分钟数」相对值，不依赖真实时钟：`status()` 为 starts>0 即将开始、starts+duration>0 进行中、否则已结束；`time_label()` 按状态出「N 分钟后/已开始 N 分钟/N 分钟前结束」。`Message` 无关，活动独立存 `app.events`。
- 种子 `seed_events()` 3 条跨两服务器，覆盖三种状态；`Snapshot.events: Option<Vec<ServerEvent>>`，旧快照缺字段保留种子；restore 丢弃归属已删服务器的脏记录；服务器改名 `rename_events_server`、离开 `remove_events_of_server` 迁移。
- app 字段：`events/show_events/event_filter(EventFilter::All|Upcoming|Live|Ended)/selected_event/event_notice` + 创建草稿 `show_event_create/event_title_edit/event_desc_edit/event_channel/event_category/event_starts_in/event_duration`。方法 `server_events()`（当前服务器、按开始时间升序）/ `visible_events()`（按 filter）/ `toggle_events/close_events/set_event_filter/select_event/toggle_event_create/set_event_category/set_event_channel/adjust_event_start/adjust_event_duration/create_event/delete_event/toggle_attendance`；新建与报名写 `AuditKind::Server` 日志。⚠️ `set_event_channel` 由表单「落地频道」chip 点击循环切换调用（否则 dead_code 警告）。
- 互斥与清理：`toggle_events` 关 pins/inbox；`toggle_pins`/`toggle_inbox` 反向关 events；`reset_session` 与 `close_top_overlay`（Esc）收起面板并清提示。入口是 header 的 `chat-toggle-events`（`IconName::CalendarDays`，仅非 DM）。
- 视图 `ui/events_panel.rs`：右侧 380px 全高浮层（同 pinned_panel 定位）；状态筛选 chips 用公共组件 `filter_chip`（带计数，⚠️ handler 是 3 参 `Fn(&mut DiscordApp,&mut Window,&mut Context)`，别写成 4 参）；内嵌创建表单（标题/描述 Input + 分类色块 + 开始/时长步进器 ±15 分钟 + 频道 chip + 创建按钮）；活动卡片显示分类色点、状态徽标、时间、频道、描述、报名人数与"你已报名/你发起"、报名按钮、未结束时可删除。新活动 id 用 `SystemTime` 毫秒戳。

### 消息请求约定
- 数据：`data::MessageRequest { sender, color, preview, minutes_ago, state, visible }` + `MessageRequestState {Pending,Accepted,Ignored}`（label），全自有字段 serde 直接派生（⚠️ 要带 `Debug`，快照往返测试 `assert_eq!` 需要）。`is_pending()` 同时要求 `state==Pending && visible`；`time_label()` 按分钟数出「刚刚/N 分钟前/N 小时前/N 天前」。独立存 `app.message_requests`，与 `FriendRequest`（好友申请）是两套东西。
- 种子 `seed_message_requests()` 覆盖 3 待处理 + 1 已接受 + 1 已忽略 + 1 条 `visible:false` 的已删除（验证过滤）。`Snapshot.message_requests: Option<Vec<MessageRequest>>`，旧快照缺字段保留种子。
- app 字段：`message_requests/show_message_requests/message_request_filter(MessageRequestFilter::Pending|Accepted|Ignored)/message_request_notice`。方法 `pending_request_count`（侧栏角标，只数可见待处理）/`request_count_for`/`visible_message_requests`（按 minutes_ago 升序）/ `toggle_message_requests/close_message_requests/set_message_request_filter/accept_message_request/ignore_message_request/delete_message_request/dismiss_message_request_notice`；筛选匹配用文件内私有 `state_matches_filter(state, filter)`。三种操作写 `AuditKind::Member` 日志。删除是置 `visible=false` 而非物理移除。
- ⚠️ 借用坑：`accept/ignore/delete` 里持有 `message_requests.iter_mut()` 的可变引用时不能调 `self.log_audit()`——先用作用域块/元组取出 owned 字段并释放借用，再调用其它 `&mut self` 方法。`accept_message_request` 把 sender 用 `Box::leak` 转 `&'static str` 后建 `DmConversation` + 空 `dm_messages`，再 `open_dm` 切入，形成「接受→真正开始私信」闭环。
- 互斥与清理：`toggle_message_requests` 关 pins/inbox/events；`toggle_pins`/`toggle_inbox`/`toggle_events` 反向关；`reset_session` 与 `close_top_overlay`（Esc）收起并清提示。入口在 `ui/dm_sidebar.rs` 好友入口下的 `dm-requests-entry`（`IconName::Inbox` + 待处理角标，仅常规模式显示）。
- 视图 `ui/message_requests_panel.rs`：右侧 360px 全高浮层；`filter_chip` 三态筛选带计数；卡片展示头像/名字/状态徽标/相对时间/预览，`pending` 才有「接受/忽略」，三态都有删除；卡片自身不挂 `on_click`（避免与按钮点击冒泡冲突）。

### 论坛频道约定
- 频道类型：`ChannelKind` 新增 `Forum` 变体（第三种），`Channel::forum(name, topic)` 构造。Forum 频道不进 `messages` 表（转发/搜索/邀请落地页仍只筛 `ChannelKind::Text`），侧栏/频道菜单图标均为 `IconName::MessageSquare`。
- 数据：`data::ForumPost { id, server, channel, title, author, color, body, tags, minutes_ago, pinned, locked, replies }` + `ForumReply { author, color, body, minutes_ago }`，字段全为自有 `String`/`u32`/`bool`，**serde 直接派生、无需 Owned 镜像**（和 `MessageRequest` 一样）。`ForumSort {Recent,Popular,Unanswered}`（label：最新/最热/未回应，⚠️ 带 `Debug` 供筛选 chip 的 `id` 格式化）。方法 `time_label/relative_label/reply_count/last_activity（有回帖取最小 minutes_ago）/matches(query 已转小写)`。
- 种子 `seed_forum_posts()`：开发者社区两个论坛频道 5 帖，覆盖置顶、多回帖热门、未回应、锁定态。`Snapshot.forum_posts: Option<Vec<ForumPost>>`，旧快照缺字段保留种子；`restore_state` 会 `retain` 掉归属频道已不存在或不是 Forum 的脏帖。
- app 字段：`forum_posts/forum_open(帖子 id)/forum_sort/forum_query+forum_search(Entity)/show_forum_composer/forum_title_edit+forum_body_edit+forum_tag_edit+forum_reply_edit/forum_tags_draft/forum_notice`。方法 `is_forum_view()`（Server 视图 + 当前频道 kind==Forum）/`forum_key()→(server,channel)`/`forum_visible_posts()`（按归属+搜索过滤，置顶恒定在前，再按排序）/ `forum_open_post/open_forum_post/close_forum_post/set_forum_sort/toggle_forum_composer/close_forum_composer/add_forum_tag/remove_forum_tag/create_forum_post/send_forum_reply/toggle_forum_pin/toggle_forum_lock/dismiss_forum_notice/clear_forum_search` + 迁移清理 `remove_forum_posts_of_server/rename_forum_server`。发帖/回帖/置顶/锁定写 `AuditKind::Channel` 日志。⚠️ `open_forum_post` 要传 `window` 给 `set_value`。
- 渲染：`render_main` 的 `main` 分支里 `ViewMode::Server if is_forum_view()` → `forum::render`（在 search_center 分支之后）；成员栏仍显示。搜索中心输入订阅用 `InputEvent::Change` 同步 `forum_query`；标签/回帖框订阅 `PressEnter` 回车提交。`reset_session` 与 `close_top_overlay`（Esc：先发帖弹层→再帖子详情）清理论坛状态。
- 视图 `ui/forum.rs`：需 `use gpui_kit::prelude::FluentBuilder as _`（`.when()`）。列表页固定 96px 头部（搜索框 + 排序 chips）+ 可滚动帖卡，`.id("forum-list-scroll")`；详情页绝对定位覆盖，含返回/置顶/锁定按钮、正文、回帖列表、回帖输入（锁定时换成只读提示条）；发帖弹层居中模态，遮罩与卡片是兄弟节点（`Rc` 复用取消闭包）。锁定/置顶图标用 `Shield`/`Pin`（assets 里无 Lock 图标，别用 `IconName::Lock`）。

### 消息书签约定

- 数据：`data::Bookmark { is_dm, server, channel, author, color, text, time, saved_minutes_ago }`，全自有字段，serde 直接派生。私信书签的 `server` 固定空串。方法 `location_label()`（`#频道` / `私信 · 好友`）、`source_label()`、`saved_label()`（「x 前保存」）、`preview(max)`（按字符截断）、`matches(query 已转小写)`（正文/作者/频道/服务器）、`in_session(is_dm, server, channel)`、`same_as(&other)`。
- ⚠️ Demo 消息没有 id：书签身份 = `is_dm + server + channel + text`，跳转靠正文反查原消息。所以「收藏」与「是否已收藏」必须用**同一个正文串**——统一走 `app.rs` 的自由函数 `bookmark_body(&Message)`（贴纸/GIF/附件/空消息返回 `[贴纸]` 等占位），否则空正文消息会被重复收藏。
- 种子 `seed_bookmarks()`：2 条频道 + 1 条私信，正文与 `seed_messages` / `seed_dm_messages` 保持一致，首启即可命中跳转。`Snapshot.bookmarks: Option<Vec<Bookmark>>`，旧快照缺字段保留种子；`restore_state` 会 `retain` 掉服务器/频道/好友已不存在的脏书签。
- app 字段：`bookmarks/show_bookmarks/bookmark_query+bookmark_search(Entity)/bookmark_notice`。方法 `session_scope()`（当前会话三元组）、`is_bookmarked(index)`、`toggle_bookmark(index, cx)`（收藏/取消，写提示）、`toggle_bookmarks(cx)`、`visible_bookmarks()`、`remove_bookmark(&Bookmark, cx)`、`jump_to_bookmark(&Bookmark, window, cx)`、`dismiss_bookmark_notice`、`clear_bookmark_search(window, cx)` + 迁移清理 `remove_bookmarks_of_server/rename_bookmarks_server`。
- 跳转顺序：先切会话（`open_dm` / 设 `active_*` 后 `reset_session`）→ 因为 `reset_session` 会清高亮并滚到底部，**定位必须放在切换之后**，再 `show_bookmarks = true` 重新打开面板 → `reveal_message(index)`（抽出的公共方法，`jump_to_reply` 也复用）滚动 + 高亮。找不到原消息时区分「贴纸/附件无法定位」与「原消息已删除」两种提示。
- 面板互斥：`toggle_bookmarks` 与置顶/收件箱/日程/消息请求互斥；反过来这四者打开时也要 `show_bookmarks = false`；`close_top_overlay`（Esc）把书签放在置顶之后；`reset_session` 关面板并清空搜索；面板在 Friends 视图不挂载（同置顶）。
- 视图 `ui/chat/bookmarks_panel.rs`：右侧 360px 全高浮层，头部「已保存 — N」+ 关闭，下面是带放大镜的搜索输入行，卡片含来源 chip / 服务器 / 保存时间 / 头像作者 / 正文预览 / 「跳转」按钮 + 移除图标。卡片本身不挂 `on_click`，动作只放在按钮上（避免冒泡）。

### 服务器欢迎屏幕约定

- 数据：`data::ServerWelcome { server, headline, description, rules: Vec<WelcomeRule>, recommended: Vec<String>, require_rules }` + `WelcomeRule { title, body }`，全自有字段 serde 直接派生。方法 `rule_count()/retain_recommended(&[String])`（按现存频道裁剪推荐列表，保持原顺序）。
- 只给「精心配置过」的服务器写文案（种子 `seed_welcome_screens()`：开发者社区 3 条规则、设计工坊 2 条）。其它服务器由 `app.welcome_screen_for(server)` **现场合成**：标题「欢迎来到X」+ 服务器简介 + 2 条通用规则 + 前 3 个文字频道作推荐，所以每个服务器都有欢迎页、菜单入口不会失效。
- app 字段：`welcome_screens/welcome_target(正在展示的服务器名)/welcome_rule_agreed/welcome_selected(多选频道)/welcome_joined(已完成的服务器，进快照)`。方法 `text_channels_of`（私有）/`welcome_screen_for`/`open_welcome`/`open_active_welcome`/`close_welcome`/`toggle_welcome_rule`/`toggle_welcome_channel`/`welcome_can_confirm`/`confirm_welcome`/`welcome_done_for` + 迁移 `remove_welcome_of_server/rename_welcome_server`。
- 触发：只有 `join_discover_server` 新建服务器后调用 `open_welcome`；`open_welcome` 会先把其它浮层全部收起（它是最顶层模态）。`confirm_welcome` 先写 `welcome_joined`，再按侧栏顺序取「已选中的第一个频道」`reset_session` 进去；未同意规则时直接 return。
- 持久化：`Snapshot.welcome_screens: Option<Vec<ServerWelcome>>`（缺字段保留种子）+ `welcome_joined: Vec<String>`；`restore_state` 丢弃服务器已不存在的配置与完成记录。
- 渲染：挂在根节点**最后**（`show_quick_switcher` 之后、`voice_status` 之前）保证盖住菜单与面板；`close_top_overlay` 里放**最前面**，Esc 等价「稍后再说」。视图 `ui/welcome.rs` 用「全屏容器 → 遮罩 div（兄弟）+ 居中 flex 容器（兄弟，无 click handler）→ 卡片」的三层结构：不要像 `create_dialog` 那样把卡片嵌进带 `on_click` 的遮罩里，否则点频道 chip 会顺带关掉模态。
- 服务器菜单「欢迎屏幕」项在未完成时用 `menu_item` 的 `trailing` 参数挂一个「未同意」小角标（`welcome_done_for` 唯一的调用点，别删）。

### 服务器加成约定

- 数据：`data::BOOST_TIERS = [2, 7, 14]` 是等级门槛的唯一来源（UI 的特权表 `BOOST_PERKS` 也用 `BOOST_TIERS[i]` 作门槛，别重复写常量）。`ServerBoost { server, boosters: Vec<Booster> }`，`Booster { name, color, minutes_ago }`，全自有字段 serde 直接派生。
- 等级只由人数推导：`count()/level()`（达到几档门槛就是几级）/`remaining()`（距下一级差几次，满级 `None`）/`progress()`（**当前等级区间内**的 0..=1 进度，不是总进度；满级恒 1.0）。别把 progress 实现成 `count / 14`，否则等级 1 的进度条会一直是 20% 左右的短条。
- 空记录不要存进 `Vec`：`app.boost_of(server)` 找不到时**现场合成** `ServerBoost::new(server)`（0 次），所以新建服务器、离开再回来都不会出现 `unwrap` 风险。`active_boost()` 只是对当前服务器调 `boost_of`。
- app：字段 `boosts/boost_notice`；方法 `boost_of/active_boost/toggle_boost/open_boost_page/dismiss_boost_notice` + 迁移 `remove_boost_of_server/rename_boost_server`。「你」的身份固定为 `Booster::new("你", 0x5865f2, 0)`，加成 / 撤销都先写数据再 `log_audit(AuditKind::Server, ...)`（注意先释放 `iter_mut` 借用，别在持有可变引用时调 `log_audit`）。
- 持久化：`Snapshot.boosts: Option<Vec<ServerBoost>>`（缺字段保留种子）；`restore_state` 丢弃服务器已不存在的记录。种子 `seed_boosts()`：开发者社区 8 人（等级 2、距 3 级差 6 次，含「你」）、设计工坊 1 人（等级 0、差 1 次）。
- 入口两处：`ui/server_settings/pages.rs::boost()`（设置页 9 个分类之一）+ `ui/channel_sidebar.rs` 服务器名下方的紧凑加成条（等级 + `relative(progress)` 进度条，点击 `open_boost_page`）。`open_boost_page` 必须**先** `open_server_settings`（它会把分类重置为 Overview）**再**把分类改成 Boost。
- 图标：等级用 `Sparkles`，未达成特权用 `Circle`、已达成用 `Check`（assets 里仍没有 `Lock`）。

### 服务器模板约定

- 数据：`data::ServerTemplate { key, name, tagline, badge, color, categories: &'static [TemplateCategory], welcome, rules: &'static [(&str, &str)] }` + `TemplateCategory { name, channels: &'static [TemplateChannel] }` + `TemplateChannel { name, kind, topic }`。全部 `&'static str` + `Copy`，**不实现 serde、不进快照**——只有创建出来的服务器才持久化。种子表在 `seed.rs` 的 `const TEMPLATES`，`server_templates()` 返回 `&'static [ServerTemplate]`。
- 方法：`channel_count()/text_channels()`（只取 Text，供欢迎页推荐）/`to_server(name)` / `to_welcome(name)`。`to_welcome` 取前 **2** 个文字频道作推荐，规则来自模板 `rules`。
- app：字段 `create_template: Option<String>`（存 key，不存索引，种子顺序变了也不会错位）；`select_create_template(key)` 是**切换**语义（再点一次取消）；`open_create_server` 打开时清空模板选择。
- `create_server` 顺序：读输入 → `create_template.take()` → **名称留空时用模板名兜底**（从零创建仍必须填名）→ 生成 server / 覆盖同名旧欢迎配置并插入 → push → `log_audit(AuditKind::Server, "创建了服务器", "名字（模板：X）")` → `select_server` → 用模板时再 `open_welcome`。注意 `open_welcome` 必须放在 `select_server` 之后，否则会被 `reset_session` 清掉。
- UI：`ui/create_dialog.rs::render_server` 是自定义模态（560 宽，两列 248 宽模板卡），三层兄弟结构 + `FluentBuilder` 的 `.when(active, 打勾)`；底部提交按钮文案随选择在「创建服务器 / 用模板创建」间切换；提示行随「有/无选中 + 名称是否为空」变化。

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
- 菜单状态：`app.channel_menu: Option<ChannelMenuState { name, anchor, page }>`；`MenuAnchor::Point(x, y)` 时 `.left(x).top(y)`，`Header` 时 `.right(...).top(...)`（`channel_menu.rs` 里统一换算）。
- **两页复用一张卡片**：`ChannelMenuPage::{Actions, Notify}`。`open_channel_notify_page` / `close_channel_notify_page` 只翻 `page`，不新建浮层状态，所以 Esc、点外部、切频道的既有收口逻辑天然同时关两页；通知页宽 280，操作页 212。
- 菜单项：标记为已读（`mark_channel_read`）、通知设置（右侧显示当前级别短文案，翻页）、编辑频道主题（`open_topic_editor`，仅文字频道）、删除频道（`delete_channel`，危险色）。操作页统一用公共组件 `menu_item`，不要在视图里重写行样式。
- 关闭方式：透明全屏遮罩 `.id("channel-menu-backdrop")` 点击关闭 + 每个动作执行后 `self.channel_menu.take()`/`None`。**遮罩与卡片必须是兄弟节点**（根 `div` 下两个子节点），否则卡片内的点击会冒泡到遮罩，通知页点一下单选就整层关掉。
- 菜单打开时会收起资料卡、消息菜单、置顶/收件箱/状态菜单、表情层，避免叠层。
- `delete_channel` 的硬约束：至少保留一个文字频道；删掉的若是当前频道就回退到 `first_text_channel()`；删掉的若是正在连接的语音频道先断开；空分类一并移除；同时清掉 `messages` 里 `服务器/频道` 的历史键。
- 主题编辑走 `topic_dialog.rs`，复用 `create_dialog::dialog()`（该函数已 `pub(crate)`），输入 `topic_input`，Enter/保存写回 `Channel.topic`，标题栏的 `active_topic()` 立即刷新。
- 频道定位统一用 `channel_position(name) -> Option<(分类下标, 频道下标)>`，不要在 UI 层自己遍历服务器。

### 频道通知设置约定

- 模型：`NotifyLevel { Inherit, All, Mentions, Nothing }`（`data/models.rs`，`Default = Inherit`），挂在 `Channel.notify`；`label()` 是行标题、`hint()` 是行说明、`short()` 是菜单右侧摘要、`is_muted()` 统一判断静音、`index()` 用作 Element ID。
- 状态只在 `Channel` 上，不另设 app 字段；读取走 `channel_notify_of(name)`（原始值）/ `effective_notify_of(name)`（继承解析后）/ `channel_muted(name)`，写入走 `set_channel_notify(level, cx)`（按 `channel_position` 定位，写后不关面板，方便连点对比）。
- 继承链：`Inherit` → 服务器 `Server.notify`（`server_notify_default()`，`Inherit` 兜底 `All`）。通知页「使用服务器默认」一行的说明会实时写出「当前生效：X」；服务器通知页改级别会影响所有仍选 `Inherit` 的频道（`inherit_channel_count()` 统计个数）。
- 入口两个：频道右键菜单的「通知设置」项、标题栏铃铛（`chat-toggle-notifications` → `open_active_channel_notify(cx)`，以 `MenuAnchor::Header` 打开并直接翻到通知页）。私信标题栏的同一按钮仍是全局通知开关 `toggle_notifications()`，不要混。
- 视觉反馈三处：侧栏频道行静音时加 `BellOff`（12px，dim 色）、未读角标换成 `unread_badge_muted`（灰底）；标题栏铃铛静音时变 `BellOff` + dim 色；频道菜单「通知设置」右侧显示 `short()`。侧栏服务器标题的铃铛反映的是 `Server.notify`，不是频道级别。
- 静音语义（演示范围内）：`app::notifications()` 聚合时按**生效级别** `continue` 跳过静音频道（频道 `Inherit` 时用服务器 `notify` 判断），所以它既不进收件箱、也不产生 @提及通知、不计入 `unread_notification_count()`；频道未读数本身不清零，进频道仍能看到未读分隔线。
- 种子数据刻意留了两个对照：`#闲聊` 是 `Mentions` + 3 条未读（红色角标），`#作品展示` 是 `Nothing` + 2 条未读（灰角标 + 铃铛斜线）。

### 服务器菜单约定

- 入口：频道侧栏头部（服务器名 + ⌄，`sidebar-server-menu`）→ `app::toggle_server_menu(cx)`。菜单用固定定位贴在头部下方（`left 8 / top 44`，操作页宽 232、通知页 280），不跟随光标。
- 状态：`app.show_server_menu: bool` + `app.server_notify_page: bool`。`server_menu.rs` 顶部 `if !app.show_server_menu { return div().into_any_element(); }`；`toggle_server_menu` 每次展开都把 `server_notify_page` 复位成操作页，所以别的路径只置 `show_server_menu = false` 也不会残留错页。
- 菜单项：服务器设置（`open_server_settings`）、邀请好友（`open_invite`）、通知设置（`open_server_notify_page` 翻页，右侧显示当前级别）、静音/取消静音（`toggle_server_mute`）、标记服务器已读（`mark_server_read`）、创建频道、新建分类、离开服务器（危险色 + 分隔线）。行样式统一用公共组件 `menu_item`，危险项事后 `.text_color(danger)`。
- 通知页：返回箭头 + 「服务器通知」+ 三级单选（全部消息 / 仅 @提及 / 关闭通知，复用 `radio_item`）+ 页脚「N 个频道选择跟随服务器」。
- 遮罩与卡片是**兄弟节点**（`server-menu-backdrop` 单独一层），否则点单选行会冒泡到遮罩把菜单关掉。
- Esc 收口在 `close_top_overlay` 里排在 `show_invite` 之前：`show_server_menu` → `close_server_menu()`（同时复位页状态）。
- `Server.notify: NotifyLevel` 取代了早期的 `Server.muted: bool`：服务器没有「继承」对象，`server_notify_default()` 把 `Inherit` 兜底成 `All`；「静音服务器」= 写 `Nothing`，「取消静音」= 回到 `All`（原本是 `Mentions` 的会被重置成 `All`，属演示范围内的取舍）。
- 邀请弹层 `invite_dialog.rs` 复用 `create_dialog::dialog()`：链接由 `primary_invite_code()` 取当前服务器最新一条邀请（没有时退回 `invite_code()` 的 slug，中文名用 `gpui-{序号}` 兜底），提交按钮即复制到剪贴板并关闭。
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
- 检索逻辑只在 `app::run_search` 里：服务器结果按 `servers → categories → channels` 顺序遍历、跳过 `ChannelKind::Voice`、键用 `服务器名/频道名` 去 `messages` 取；私信结果遍历 `dm_conversations` 取 `dm_messages`。命中判定走 `SearchFilters::matches`，结果 `truncate(60)`。
- 高级过滤：`app::SearchFilters::parse` 从查询串拆出 `from:` / `has:` / `before:` / `after:`，剩余词拼成小写 `text`；`is_empty()` 判空时要求四个条件与普通关键词都为空（只有 `from:` 也能搜）。`from:` 多值 OR 匹配作者名；`has:` 多值 AND，支持 `link/image/file/attachment/sticker/gif/reaction/edited/mention/pinned`；`before/after` 只接受 `today`（今天/刚刚，rank 2）与 `yesterday`（rank 1），按消息时间前缀比较，其余值不生效。
- `app.search_filters` 由 `run_search` 每次重算时刷新，UI 只读它：`search_center.rs` 顶部「已应用」标签来自 `chips()`，点标签走 `remove_search_filter(token, window, cx)`（从 query 里删 token、`set_value` 后手动 `run_search`，因为 `set_value` 不发 Change）；结果高亮用 `search_filters.text` 而不是原始 query，否则过滤 token 会干扰高亮。
- 演示种子：默认频道放了一条含 `https://` 的链接消息与一条 GIF 消息，供 `has:link` / `has:gif` 验证；`has:edited` 用编辑历史那两条种子消息。
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

### 账户资料约定

- 当前用户资料是 `app.me: MyProfile { display_name, username, email, bio, color }`，初始值来自 `data::seed_me()`；账户页、侧栏底部用户栏都只读这一份，不要再硬编码「Riven」或 `'R'`。
- 账户表单是四个独立 `InputState`（`account_name_input / account_username_input / account_email_input / account_bio_input`），不订阅 Change 事件：只有点「保存改动」才写回 `me`，避免边输边改全局状态。
- 打开设置页时 `open_settings(window, cx)` 会调 `sync_account_form`，把 `me` 重新灌进四个输入框并清空提示——所以上次没保存的编辑会被丢弃（与 Discord 的「放弃未保存改动」一致）。
- 保存走 `save_account(cx)`：显示名称与用户名必填（留空提示「显示名称与用户名不能为空」），邮箱与简介允许为空；成功提示「已保存」，用 `success` 色显示。
- 头像颜色是 `AVATAR_COLORS`（8 个固定值，与 `SERVER_COLORS` 同一套风格），点色块走 `set_avatar_color` 立即生效并提示「头像颜色已更新」，不需要保存。
- 改名不回溯历史消息：`Message.author` 是 `&'static str`，旧消息保留当时的署名，这是有意为之。
- 新增读取「我是谁」的界面一律用 `app.me`，头像用 `app.me.initial()`；不要在视图里自己取名字首字。

### 本地持久化约定

- 位置：`data/persist.rs`。`Snapshot` 只装**内容状态**（servers / messages / dm_conversations / dm_messages / threads / friends / friend_groups / invites / audit / bans / members / roles / friend_requests / blocked_users / me / 当前会话索引 / 通知与主题等偏好 / read_notifications），浮层显隐、输入焦点、语音连接、草稿、通话状态、好友分组的折叠状态一律不持久化。
- 文件：macOS 写 `~/Library/Application Support/discord-gpui/state.json`，其他平台 `~/.discord-gpui/state.json`；`state_path()` 是唯一入口，UI 只通过 `app.snapshot_path()` 展示，不自己拼路径。
- 写入时机：`DiscordApp::render` 开头调 `self.autosave()`——把当前状态序列化一次，与 `saved_json` 比较，**只有内容变化才落盘**。不要在每个业务方法里手动调 save，漏一处就会丢改动。
- 读取时机：`DiscordApp::new` 末尾 `restore_state(window, cx)`；无文件 / 版本不符 / servers 为空则保留种子数据。加载后要校验 `active_server` 越界、`active_channel` 是否仍存在（退回 `first_text_channel()`）、`active_dm` 是否在会话列表里；主题变化才调 `set_theme`，避免启动时多余的 `Theme::change`。
- 数据页：用户设置 → 「数据与存储」（`SettingsSection::Data`），展示快照路径 / 消息与频道数量 / 最近操作，按钮「立即保存」（`persist_now`）、「恢复示例数据」（`reset_local_data`：删文件 + 重置种子 + `reset_session`）。
- **serde 坑位**：模型里大量 `&'static str` 字段会让派生出来的 `Deserialize` 带上 `'de: 'static`，从普通字符串解析会编译失败；`bound` / `serde(from)` 都压不掉这个约束。现有做法是给这些类型手写 `impl Deserialize for X`，内部解析 `persist::XOwned`（String 字段）再 `Into` 转换，字符串统一走 `persist::leak()`。新增带静态字符串的模型时要补一个 `*Owned` 镜像并加进 `deserialize_via_owned!` 列表。
- 序列化方向仍然走派生 + `#[serde(with = "leaked_str" / "rgba_u32")]`（`Rgba` 存成 0xRRGGBB）。改字段名/语义时递增 `persist::VERSION`，旧快照会被整体丢弃。
- **验收可用测试**：`cargo test` 里有一条快照往返测试，覆盖静态字符串、Rgba、通知级别与私信列表。

### 帮助中心约定

- 入口只有标题栏 `?` 按钮（`chat-open-help` → `toggle_help_center`），开着时按钮显示 active 态；关闭走 X、遮罩、Esc 三处，全部落到 `close_help_center`。
- 文档数据在 `data::seed_help_articles()`（12 篇，`HelpArticle { id, category, title, summary, body, tips }`），分类顺序唯一数据源是 `help_categories()`；搜索匹配逻辑写在 `HelpArticle::matches`，正文与要点也参与，避免搜索只命中标题。
- 状态只有四件：`show_help_center` / `help_query` / `help_category: Option<&'static str>`（None = 全部）/ `help_article: Option<&'static str>`（文档 id）。列表过滤走 `app::help_matches()`，视图不要自己过滤。
- 输入变化时自动把 `help_category` 复位成 None 并清空 `help_article`，否则用户会遇到「搜到了但当前分类里不显示」。
- Esc 在帮助中心里是**两层**：先 `close_help_article` 回列表，再 `close_help_center` 整体关闭；`close_top_overlay` 里排在快速切换器之后、附件预览之前。
- 打开设置 / 服务器设置 / 探索页都会 `show_help_center = false`，防止整屏页关掉后帮助层又弹回来。
- 遮罩与卡片是**兄弟节点**而不是父子：卡片点击不会冒泡到遮罩，避免点文档列表时误关整个面板。

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

### 新增一个界面能力后同步后端契约

1. 打开 `docs/backend-api.md`，在对应 `3.x` 小节补端点行（方法 / 路径 / 说明 / 界面文件），并更新该小节标题里的计数。
2. 若引入了新实体，同步「5.1 表清单」与「5.2 索引」；若产生跨端广播，同步「4.1 下行事件」。
3. 更新第 0 节的合计（端点总数、方法分布、表数）与第 9 节分期数字——**这几个数字必须与正文一致**，可用下面的脚本自检：

```bash
python3 - <<'PY'
import re
t = open('docs/backend-api.md').read()
rows = re.findall(r'^\|\s*(GET|POST|PATCH|PUT|DELETE)\s*\|', t, re.M)
print('实际端点行数:', len(rows))
PY
```

4. 该文档只是设计，不生成服务端代码；不要在 `src/` 里引入任何 HTTP 客户端依赖。

---

## 6. 验证流程（每次改动必做）

```bash
cd /Users/worldunion/WorkBuddy/2026-11-20-14-21-58
cargo check     # 快速类型检查，必须无 error / warning
cargo build     # 完整构建
cargo test      # 快照往返等数据层测试
cargo run       # 启动确认窗口创建正常
```

验收标准：

1. `cargo check` 输出 `Finished`，且 warning 数为 0。
2. 窗口尺寸 1440×900，标题 `Discord UI · GPUI Kit`。
3. 手动验证涉及的交互：服务器切换、频道切换、搜索过滤、消息发送、表情回应、成员栏开关、麦克风/耳机/通知开关；资料卡（点作者名/成员）、消息菜单（置顶/复制/删除/编辑）、置顶面板（header 图钉）、语音频道连接与断开；在线状态切换（点底部用户信息）、@提及高亮点击、消息行内编辑（Enter 保存 / 取消）；用户设置（点底部齿轮打开，左侧 8 个分类切换，主题选项、开关、音量加减、恢复默认，右上角 X 关闭）；新建服务器（服务器栏 + → 输入名称 → Enter/创建，自动切换）、新建频道（分类 + → 输入名称 → Enter/创建，自动切换，同名拦截）；好友页五个标签切换、待处理请求接受/拒绝、添加好友（用户名#标签 → Enter/发送，成功后跳待处理）、好友行屏蔽与已屏蔽列表取消屏蔽；私信侧栏搜索（输入关键词过滤会话、X 清除、点「发起对话」中的好友建立新会话并切入、无结果空状态）；成员栏按角色分组（管理员/版主/在线/离线，点分组标题折叠与展开，点成员仍打开资料卡）；收件箱（header Inbox 图标带未读角标 → 打开面板，点通知跳转到对应频道/私信/好友页并标为已读、全部已读、与置顶面板互斥）；@提及补全（输入框打 @ 或 @前缀 → 上方浮层匹配成员，点击插入 @完整名）；频道操作菜单（频道行右键 → 菜单跟随光标：标记为已读清角标、编辑频道主题 → 弹层 Enter/保存后标题栏刷新、删除频道 → 回退到第一个文字频道且至少保留一个文字频道；标题栏铃铛打开同一菜单；点菜单外关闭）；服务器菜单（点侧栏服务器名/⌄ → 邀请好友弹出链接弹层并可复制、静音切换后侧栏出现静音图标、标记服务器已读清空全部角标、创建频道、离开服务器 → 切到第一个服务器且至少保留一个）；分类菜单（分类标题右键 → 在该分类中创建频道后弹窗创建并落入该分类、标记为已读清该分类角标、重命名分类 → 弹层保存后侧栏标题与展开状态保持、删除分类 → 分类内频道一并消失且当前频道回退、只剩一个文字分类时该项置灰；服务器菜单「新建分类」→ 输入名称 → Enter/创建后侧栏出现空分类且可再右键改名或删除）；主题切换（设置 → 外观 → 点「浅色」→ 整个界面立刻变浅色：服务器栏/侧栏/聊天区/菜单/弹层全部跟着变，输入框文字仍可读；点「深色」切回；点「跟随系统」按当前窗口外观决定；恢复默认设置会回到深色）；贴纸消息（点输入框右侧贴纸图标 → 弹层按「日常/猫咪/礼物」三包切换 → 点任意贴纸立刻作为一条消息发出并显示 132×132 贴纸卡；点礼物图标直达礼物包；再点同一图标收起；切频道/开设置时自动收起）；消息右键菜单（右键任意消息行 → 在光标处弹出菜单：添加回应把表情层打开、回复把「回复某人」提示条挂上、编辑（自己的消息）进入行内编辑、置顶后标题栏图钉面板能看到、复制文本进剪贴板、删除后消息消失；点菜单外或再次右键即关闭；悬停省略号入口仍可用同一组操作）；成员上下文菜单（右键成员栏任意成员 → 弹出菜单，头部显示头像/名字/角色与在线小点；「查看资料卡」打开右侧资料卡、「发送私信」跳到与他的私信并补建会话、「在输入框提及」把 @名字 填进输入框并聚焦、「复制用户名」复制 @名字、「屏蔽成员」后该名字出现在好友页「已屏蔽」分组并可在那里取消，菜单再次打开显示「取消屏蔽」；点菜单外任意位置关闭）；搜索中心（点标题栏搜索框右侧放大镜 → 主区域变成搜索页，输入关键词实时出结果、切「当前服务器/全部服务器/私信/全部」chips 重算、命中片段蓝色高亮、点任意结果跳到对应频道或私信并自动退出搜索页、无结果有空状态、X 返回聊天区）；语音房间（点语音频道连接 → 底部状态条显示「语音已连接 · 点此查看房间」→ 点击弹出房间面板，看到自己与其他 3 位参与者、切换某人的静音状态、点 UserX 移出列表、底部可静音本机或断开；断开后面板与状态条一起消失）；探索页（点服务器栏底部指南针 → 整屏探索页，搜索关键词过滤卡片、切分类 chips 过滤、无结果显示空状态、点「加入」后服务器出现在服务器栏并自动进入、点 X 回到原频道视图，再次打开已加入卡片显示「已加入」）；服务器设置（服务器名/⌄ → 服务器设置 → 切到全屏页：概览改名后侧栏与顶栏同步、消息记录与分类折叠状态保留、6 个色块点击后徽标颜色与服务器栏同步、简介保存；角色页默认选中「管理员」，改名 / 换色 / 给成员勾选角色后成员栏分组实时跟着变；表情页可移除并恢复表情；邀请页可选频道/有效期/次数后生成并复制链接、列表逐条复制与撤销；打开邀请弹层会回到主界面；右上角 X 返回主界面）；附件消息（点输入框左侧 + → 弹层列出 5 个模拟文件、点选后变为待发附件条（文件名/大小/待发送 + X 移除）、可直接点发送只发附件、发送后消息内出现图片占位卡或文件卡，待发附件草稿清空）。；消息转发（消息右键/省略号 → 转发消息 → 弹层显示来源预览，搜索框过滤频道与私信，点目标后弹层关闭、列表顶部出现「已转发到 #xxx」提示、目标会话里多出一条带「转发自 #原频道」的消息且原消息不变；点来源标签跳回原会话；转发到当前会话自动滚到底部；提示条 X 可关闭，切频道后自动消失）；附件预览（点消息里的图片/文件卡 → 弹出大图层：头部文件名与大小、中央放大图标占位、底部「复制文件名（演示下载）」可写入剪贴板并关闭；点遮罩或 X 也能关闭，切频道自动收起）；未读与跳到最新（默认进入「开发者社区/#闲聊」，消息列表里出现红色「3 条新消息」分隔线、右下角同文案浮标；点分隔线胶囊或浮标 → 滚到底部、频道角标清零且分隔线与浮标消失；私信会话同样生效）；快捷键与快速切换（⌘/Ctrl+K 或标题栏「⌘K」打开切换器 → 输入关键词实时过滤频道/私信、↑↓ 循环高亮、Enter 或点击跳转到目标会话、Esc 关闭；Esc 还能按最上层优先关闭消息菜单/弹层/设置页/探索页）；会话草稿（在 #闲聊 输入「草稿测试」不发送 → 切到别的频道，侧栏 #闲聊 出现「草稿」标记 → 切回 #闲聊 输入框自动恢复「草稿测试」；切到私信再切回同样保留，含待发附件一起恢复；按发送后草稿标记消失，切走再回来输入框为空；好友页/探索页不保留草稿）；公共组件库重构（回归检查：设置页 10 个开关行点击仍生效、搜索/探索页筛选 chip 高亮切换正常、标题栏铃铛/图钉/搜索/收件箱/成员按钮 active 态与点击正常且收件箱未读徽标仍在、新建弹层 X 与搜索/探索页 X 可关闭、好友请求拒绝与屏蔽按钮 hover 变红且点击生效、各侧栏分组标题显示正常）；回复引用（右键消息/悬停工具栏 → 回复，输入条显示「回复 @作者：摘要」；输入内容发送后，新消息正文上方出现引用条（竖线 + 链接色），点引用条滚回原消息并短暂高亮该行；再次回复同一条消息取消；切会话后高亮清除）；GIF 搜索（点输入框 GIF 图标 → 弹层按热门/反应/搞怪/动物四类切换，每类 5 个 → 搜索框输入「猫」跨分类匹配出猫猫 → Enter 发送首个结果、点分类清空搜索、点任意格立即作为消息发出并显示横版 GIF 卡 → 再点图标 / Esc / 切频道均收起）；编辑历史（给自己发过的消息编辑后作者行出现「（已编辑）」→ 点开浮层可看到编辑前正文与版本时间，消息菜单同样有「编辑历史」项 → 点遮罩 / X / Esc 关闭、切频道自动收起；内容未变化不产生新版本）；搜索高级过滤（点放大镜进搜索中心 → 输入 `from:Nova` 只看 Nova 发的消息、`has:image` 只看图片附件、`has:gif` 只看 GIF 消息、`has:edited` 只看编辑过的消息、`after:today` 只看今天、`has:reaction after:yesterday` 两个条件叠加 → 搜索栏出现「已应用」标签，点标签移除该条件并重跑；只有过滤条件没有关键词也能出结果，命中高亮仍按纯关键词）
4. 话题功能回归：预置话题的消息显示「N 条回复」徽标 → 点徽标 / 工具栏 / 右键菜单任一入口打开右侧面板，原消息与回复列表正确 → 在面板底部输入回复并 Enter，新回复立即出现且输入清空 → Esc / X 关闭；对无话题消息点「开话题」可新建，切会话后不串台；删除话题根消息后面板关闭、其他话题仍能正确打开。
5. 送达与回执回归：默认频道滚到底部，自己的消息显示「已读 3 人」；点击状态浮出读者卡片，能看到 Nova / 小满 / 阿哲的头像与时间，点遮罩 / X / Esc 均能关闭；私信「阿哲」里自己的消息显示「已送达」且不可点开详情；新发一条消息显示「已发送」；打开回执详情后切频道或删除更早的消息，浮层不会串到别的消息。
6. 私信通话回归：切到与 Nova 的私信 → 标题栏电话按钮打开右下浮窗显示「正在呼叫」→ 点「模拟接通」显示四组控制 → 依次点麦克风、摄像头、屏幕分享确认图标/文案切换且开分享会带开摄像头 → 点挂断浮窗消失；再从资料卡点视频按钮可发起视频通话；通话中点底部齿轮进设置页，浮窗仍可见；返回后点语音频道连接，通话浮窗自动消失。
7. 帮助中心回归：点标题栏 `?` → 弹出帮助面板，左侧列出「全部 / 入门 / 消息 / 会话 / 语音 / 设置」与各自篇数；点「消息」只剩消息类文档；在搜索框输入「话题」立刻命中并自动回到全部分类，结果计数文案跟着变；点任意文档进入详情（返回列表 / 分类标签 / 标题 / 正文段落 / 要点）→ 点「返回列表」或 Esc 回上一层，再按 Esc 整体关闭；点空白遮罩关闭，点卡片内部不会误关；打开设置或探索页后返回，帮助层不会残留。
8. 账户资料回归：打开设置 → 账户，顶部预览显示当前头像 / 名称 / 用户名 / 关于我 / 在线状态；在「显示名称」输入 `Riven Chen`、邮箱留空、点「保存改动」→ 提示「已保存」，预览与侧栏底部用户栏同步更新；把显示名称清空再保存 → 提示「显示名称与用户名不能为空」且不生效；点任意色块 → 头像立刻换色并提示「头像颜色已更新」；点「重置」→ 表单恢复成当前资料；不保存直接关设置再打开，表单回到已保存的值。
9. 频道通知回归：右键 `#作品展示` → 菜单「通知设置」右侧显示「已静音」→ 点进通知页，四行单选当前选中「关闭通知」；改选「全部消息」后菜单摘要变「全部消息」、侧栏铃铛斜线与灰角标消失；再改回「关闭通知」→ 打开收件箱确认列表里没有 `#作品展示` 的通知且 header 未读数不含它；点返回箭头回到操作页、点「标记为已读」菜单关闭；点标题栏铃铛直接进通知页且锚点在右上；Esc 与点空白处都能整层关闭；切频道后菜单不残留。
10. 服务器通知继承回归：点侧栏服务器名 → 菜单摘要「N 名成员 · 通知：全部消息」→ 点「通知设置」进三级单选页，页脚显示「5 个频道选择跟随服务器」（默认服务器）→ 选「关闭通知」：侧栏 `#公告`/`#规则` 等继承频道全部出现铃铛斜线、未读角标转灰，服务器标题右侧出现 `BellOff`，收件箱里这些频道的通知消失 → 打开 `#闲聊` 的频道通知页，「使用服务器默认」仍被选中但说明写「当前生效：仅 @提及」（它自己设过级别，不受服务器影响）→ 回到服务器页选「仅 @提及」，继承频道恢复提醒但未读角标仍是红色 → 点「返回」回操作页、点「静音服务器」后菜单关闭且标题变 `BellOff`，再打开菜单显示「取消静音服务器」→ Esc 能关闭服务器菜单。
11. 本地持久化回归：首次启动后 `~/Library/Application Support/discord-gpui/state.json` 自动出现（约 25 KB）→ 在 `#闲聊` 发一条消息、把某频道设为静音、改主题为浅色，等一帧后文件大小/内容变化 → 关闭应用再启动，消息、静音级别、浅色主题、当前频道都还在 → 设置 → 「数据与存储」显示快照路径、`N 条消息 · M 个频道`，「立即保存」提示「已写入本地快照」→ 点「恢复示例数据」后回到初始种子（消息与频道复原），提示「已恢复示例数据」，重启后仍是示例数据 → 手动把快照里的 `active_channel` 改成不存在的频道，重启后停在第一个文字频道且不崩溃。
12. 好友分组回归：好友页 → 全部标签，默认看到「一起打游戏 / 现实朋友 / 同事 / 未分组」四段，标题右侧人数与在线数对得上（未分组只剩 Kai 一位离线好友）→ 点分组标题折叠，好友行收起且箭头转右，再点展开 → 点右上「新建分组」输入「大学同学」回车，列表末尾出现空分组并显示「这个分组还没有好友」→ 点某个好友行右侧文件夹按钮 → 菜单里当前分组打勾，选「大学同学」后该好友移到新分组、人数变 1（且**没有**顺带打开私信）→ 打开分组省略号菜单：点「上移」分组顺序变化、到顶时右侧显示「已在最前」；点「重命名分组」改成「老同学」，成员仍在、顺序不变；新建一个空分组后点「删除分组」确认它消失 → 右键分组标题能打开同一个菜单，Esc 逐层关闭（菜单 → 无残留）→ 重启应用，分组顺序与成员归属都还在，折叠状态恢复为全部展开。点好友行 Ban 图标只屏蔽、不跳私信。
13. 邀请管理回归：服务器设置 → 邀请 → 顶部列出「已创建的邀请 3 条」：`dev-community` 显示「6 / 不限」「永不过期」，`dev-weekly-5` 标红「已用完」，`dev-tmp-30m` 标红「已过期」→ 选频道 `#作品展示`、有效期「1 小时」、次数「10 次」，点「生成并复制链接」：列表头部出现 `discord.gg/dev-community-4`、使用次数「0 / 10」、剩余「1 小时后过期」，同时系统剪贴板已写入该链接 → 点该行的「复制」变「已复制」，点另一行后只有新行高亮 → 点某行垃圾桶撤销，行消失、复制的标记清空，条数减一 → 切换服务器再看邀请页只剩该服务器的邀请，落地频道自动落到它的第一个文字频道 → 打开服务器菜单「邀请好友」，弹层预填的是当前服务器最新一条邀请码；重启应用后邀请列表（含新增与撤销结果）保持不变。
14. 角色管理回归：服务器设置 → 角色，左侧是「管理员（2 人）/ 版主（1 人）/ 贡献者（1 人）」三段并按角色色着色，右侧默认选中「管理员」并显示 Nova、阿哲已勾选 → 把名称改成「管理组」回车：左列表与成员栏分组同时改名、成员仍在组内；清空名称再保存提示「角色名不能为空」不生效 → 点绿色块：角色圆点与成员栏分组标题颜色同步变化，资料卡里仍显示角色名 → 点 Lin 行：Lin 加入「管理员」；再点一次退出，成员栏「贡献者」随之消失、Lin 回到「在线 / 离线」→ 新建角色（名称 `新角色 4`）后改名「实习生」，点 Kai 授予该角色，成员栏末尾出现「实习生」分组且 Kai 移出「离线」→ 删除该角色：成员栏分组消失、Kai 回到「离线」→ 重启应用，角色表顺序、颜色与各成员归属全部保留（旧的 `Nova —— 管理员` 式名字已不再出现）。

15. 审核日志回归：服务器设置 → 审核日志，顶部「共 10 条」、筛选条为「全部 10 / 服务器 2 / 频道 2 / 角色 2 / 成员 2 / 邀请 1 / 表情 1」，第一行是「将成员移出了服务器 / 旧号#0002」并显示「阿哲 · 刚刚」→ 切到「角色 2」只剩两条角色记录，条数变「2 / 8」→ 回「全部」，去概览把服务器名改成「开发社区邦」：返回日志页第一行变成「更新了服务器名称 / 开发者社区 → 开发社区邦」，第二行降级为「1 分钟前」→ 去角色页新建角色、改个颜色，日志里多出「创建了角色」「更新了角色颜色」→ 回日志页点「清空日志」：列表变成空状态「日志是空的…」，筛选条只剩「全部 0」，清空按钮消失 → 再改一次服务器简介，日志只剩这 1 条且显示「刚刚」→ 重启应用后日志内容（含清空后的新记录）保持不变。

16. 成员管理回归：服务器设置 → 成员，搜索行显示「共 6 位成员」，筛选条「全部 6 / 管理员 2 / 版主 1 / 贡献者 1 / 无角色 2」，成员栏里已设昵称的人显示昵称（`阿哲（夜猫子）`、`Lin 🌿`）→ 搜索「lin」只剩 Lin 一行，点 X 清除后恢复 → 切「管理员 2」只剩 Nova 与阿哲 → 点阿哲行的笔：弹层预填「阿哲（夜猫子）」，改成「阿哲」保存，列表与右侧成员栏同步改名、行上的 `@阿哲` 消失；再点开留空保存，昵称清空但仍显示原名 → 点 Kai 的移出按钮：Kai 从列表与成员栏消失，计数变「全部 5 / 无角色 1」，审核日志多一条「将成员移出了服务器 / Kai」→ 点 Mia 的封禁按钮：Mia 从成员列表消失，封禁卡出现「Mia · 未填写原因 · 刚刚封禁 · by Riven」，日志多一条「封禁了成员 / Mia」→ 点该行「撤销封禁」：封禁卡只剩种子那条，日志多一条「撤销了对成员的封禁 / Mia」（撤销不会把人加回服务器）→ 重启应用，昵称、移出结果与封禁列表保持不变。

17. 测试结束后**必须关闭应用进程**，不要留后台残留：

```bash
pgrep -fl 'target/debug/discord-gpui'   # 确认无残留
```

---

### 客户端网络层（src/net）

- **线程边界**：所有 HTTP/WS I/O 只在 `net::worker` / `net::ws` 的专用线程；UI 通过 `async-channel` 收发，禁止在 `cx.spawn` 或渲染路径里做同步网络调用。
- **请求构造**：URL 与 body 统一走 `net/api.rs`，UI 不拼字符串；后端字段保持 snake_case，DTO 在 `net/dto.rs`。
- **认证**：access 只在内存，`SessionStore` 提前 30 秒刷新；401 `TOKEN_EXPIRED` 只重放一次。密码不落盘，“记住我”的 refresh token 文件权限必须是 `0600`。
- **错误**：统一 `NetError`，保留后端 `code/field/request_id/retry_after`；UI 只展示 `user_message()` 并提供重试，不 panic。
- **离线回退**：`DISCORD_DEMO=1` 或 health 失败进入 `AppMode::Demo`；在线与离线互斥，不合并数据。
- **启动阶段**：`DiscordApp.auth_status` 走 `Checking → Login → Booting → Ready`（或 `Demo`）；在线模式不写快照（`autosave` 直接返回），登录后由 `net::hydrate` 填充状态。
- **id 映射**：显示名/`服务器/频道` 仍是 UI 键，但后端 id 统一存 `DiscordApp.remote`（`RemoteIndex`）；写操作必须查索引，不得按显示名反查。
- **实时**：`net::ws` 线程负责连接/心跳/退避重连与 `?since`；事件经 `NetEvent` 进 UI，`apply_gateway_event` 只做状态变更，不直接发网络请求。消息以 `Message.id` 去重，乐观消息用 `client_nonce` 对齐。
- **内容域**：论坛/投票/话题/日程/欢迎/加成/通知/书签都在 `fetch_boot` 水合，写操作走各自端点后用 `refresh_structure`（保留当前会话）回填；书签身份是 `message_id`，不再正文反查。
- **测试**：网络逻辑用 `Transport` mock + `pollster::block_on` 单测；不依赖真实后端。改 DTO 时同步补 fixture。

### 后端（server/）验证

```bash
cd server
uv run ruff check .            # 0 error
uv run pytest -q               # 21 passed
uv run alembic upgrade head    # 建表（首次或改模型后）
```

- **改了 ORM 模型必须生成迁移**：`uv run alembic revision --autogenerate -m "xxx"`，不要只靠 `create_all`（那只用于测试与开发）。
- 新增接口的流程：先在 `docs/backend-api.md` 对应小节加行 → 在 `app/api/v1/<域>.py` 实现并按需拆 `services/` → `app/api/v1/__init__.py` 注册 → 补 pytest 与 `scripts/smoke.py` 链路；写操作统一走 `realtime/outbox.queue()`。
- 端到端冒烟：`uv run uvicorn app.main:app --port 8099` 后另开一条 `uv run python scripts/smoke.py http://127.0.0.1:8099`，它会验证真实 WS 广播与 LiveKit token。
- ⚠️ 本机 `http_proxy` 常指向一个失效的本地代理：跑 `uv sync` / `pytest` / `smoke.py` 前先 `env -u http_proxy -u https_proxy -u HTTP_PROXY -u HTTPS_PROXY`，否则报 `Empty reply from server`；访问本地端口再加 `no_proxy=127.0.0.1,localhost`。
- ⚠️ SQLite 的 `PRAGMA foreign_keys=ON` 已在 `db.py` 打开，所以**没有 relationship 的批量插入不会自动排序**：先 `db.add(parent)` + `await db.flush()`，再插子表，否则报 `FOREIGN KEY constraint failed`。

---

## 7. 环境注意事项

- **rustfmt 未安装**：本机 `cargo fmt` 会报 `cargo-fmt is not installed`。如需统一格式化，先 `rustup component add rustfmt`；在此之前手工保持格式一致。
- **截图限制**：GPUI 窗口常处于其他 macOS Space，`screencapture` 和 `CGWindowListCreateImage` 都可能抓不到像素。验证以「窗口存在 + 进程正常 + 编译通过」为准，不要因为截图失败误判为渲染异常。
- **不要使用过时文档**：遇到 API 不确定时，直接读本地 registry 里的 `gpui-kit-0.10.0` / `gpui-component-0.10.0` / `gpui-base-0.10.0` 源码，不要搜索 0.7 时代的教程。
- **依赖锁定**：除 `gpui-kit = "0.10.0"` 外，网络层只允许 `ureq`（REST）、`tungstenite`（WS）、`async-channel`（线程通信）与测试用 `pollster`；不要引入 tokio/reqwest。升级版本前完整回归编译与交互验证。

---

## 8. 禁止事项

- 禁止把业务逻辑写进 `ui/` 视图模块。
- 禁止把 widget 代码写回 `main.rs` 或 `app.rs`（顶层骨架除外）。
- 禁止在 `data/` 中引用 `crate::app` 或 `crate::ui`。
- 禁止在 `data/persist.rs` 之外直接读写快照文件（路径与序列化都收口在 persist 层）。
- 禁止让两个整屏覆盖层（用户设置 / 服务器设置）同时可见。
- 禁止使用重复或无语义的 Element ID。
- 禁止裸写颜色值绕过 `palette`。
- 禁止在未执行 `cargo check` 的情况下声称改动完成。
- 禁止在测试后遗留运行中的应用进程。
