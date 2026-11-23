use std::collections::HashMap;

use gpui_kit::*;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{v_flex, Theme, ThemeMode};
use gpui_kit::prelude::FluentBuilder as _;

use crate::data::{
    persist,
    seed_discover_servers, seed_dm_conversations, seed_dm_messages, seed_friend_requests,
    gif_categories, help_categories, seed_friends, seed_gifs, seed_help_articles, seed_me,
    seed_members, seed_messages, seed_servers,
    seed_stickers, seed_threads, sticker_packs, Attachment, Category, Channel, ChannelKind,
    ChatThread, DiscoverServer,     DmConversation, EditRevision, ForwardOrigin, ForwardTarget, Friend, FriendRequest,
    FriendTab, Gif, HelpArticle, Member, MyProfile,
    MemberGroup, Message, Notification, NotificationKind, NotificationTarget, NotifyLevel, Reaction,
    ReadReceipt,
    ReplyRef, SearchHit, Server, Sticker, ThreadReply, UserStatus, VoiceParticipant,
};
use crate::ui::theme::palette;
use crate::ui::{
    category_dialog, category_menu, channel_menu, channel_sidebar, chat, create_dialog, dm_call_panel,
    dm_sidebar, discover, friends, help_center, invite_dialog, member_list, profile_card,
    server_menu, server_rail,
    member_menu, search_center, server_settings, settings, status_menu, topic_dialog, voice_room,
    voice_status,
};

/// 主视图模式：服务器频道 / 私信 / 好友。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewMode {
    Server,
    DirectMessage,
    Friends,
    /// 探索页：整屏展示可加入的公开服务器。
    Discover,
}

/// 私信通话类型：语音或视频（视频默认开摄像头，语音默认关）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallMode {
    Voice,
    Video,
}

impl CallMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            CallMode::Voice => "语音通话",
            CallMode::Video => "视频通话",
        }
    }
}

/// 一条进行中的私信通话。跨会话/视图保持，直到挂断或改为连接服务器语音。
#[derive(Clone)]
pub(crate) struct DmCall {
    pub(crate) friend: String,
    pub(crate) color: u32,
    pub(crate) online: bool,
    pub(crate) mode: CallMode,
    /// false 表示仍在呼叫对方，true 表示已接通（Demo 用「模拟接通」按钮推进）。
    pub(crate) connected: bool,
    pub(crate) mic_on: bool,
    pub(crate) camera_on: bool,
    pub(crate) screen_sharing: bool,
}

/// 频道操作菜单的锚点：鼠标右键位置或标题栏铃铛入口。
#[derive(Clone, Copy)]
pub(crate) enum MenuAnchor {
    /// 窗口坐标（像素），用于右键菜单跟随光标。
    Point(f32, f32),
    /// 聊天标题栏右上角，菜单贴右边缘向下展开。
    Header,
}

/// 频道操作菜单的两页：操作项与通知设置。翻页复用同一个菜单卡片与遮罩，
/// 这样 Esc / 点外部 / 切频道等既有收口逻辑无需额外状态即可同时关掉两页。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelMenuPage {
    Actions,
    Notify,
}

/// 频道操作菜单状态：目标频道名 + 展开锚点 + 当前页。
#[derive(Clone)]
pub(crate) struct ChannelMenuState {
    pub(crate) name: String,
    pub(crate) anchor: MenuAnchor,
    pub(crate) page: ChannelMenuPage,
}

/// 成员上下文菜单状态：目标成员（展示名，可能带「 —— 角色」）+ 光标锚点。
#[derive(Clone)]
pub(crate) struct MemberMenuState {
    pub(crate) name: &'static str,
    pub(crate) anchor: MenuAnchor,
}

/// 分类操作菜单状态：目标分类名 + 展开锚点。
#[derive(Clone)]
pub(crate) struct CategoryMenuState {
    pub(crate) name: String,
    pub(crate) anchor: MenuAnchor,
}

/// 用户设置页左侧分类。仅作为选择状态，展示与文案在 ui/settings 中映射。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsSection {
    Account,
    Privacy,
    Appearance,
    VoiceVideo,
    Notifications,
    TextImage,
    Keybinds,
    Language,
    Data,
}

/// 外观主题选项。浅色会真正切换色板；「跟随系统」在 Demo 里等同深色（不读系统外观）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeChoice {
    Dark,
    Light,
    System,
}

impl ThemeChoice {
    /// 持久化用的稳定键（不要用展示文案，避免改文案破坏旧快照）。
    pub(crate) fn key(self) -> &'static str {
        match self {
            ThemeChoice::Dark => "dark",
            ThemeChoice::Light => "light",
            ThemeChoice::System => "system",
        }
    }

    /// 从快照键还原；未知值退回深色。
    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "light" => ThemeChoice::Light,
            "system" => ThemeChoice::System,
            _ => ThemeChoice::Dark,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            ThemeChoice::Dark => "深色",
            ThemeChoice::Light => "浅色",
            ThemeChoice::System => "跟随系统",
        }
    }
}

/// 服务器设置页左侧分类。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerSettingsSection {
    Overview,
    Roles,
    Emoji,
    Invites,
}

/// 搜索中心的范围筛选。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchScope {
    /// 只在当前服务器内搜。
    CurrentServer,
    /// 跨所有服务器搜。
    AllServers,
    /// 只在私信里搜。
    Dm,
    /// 服务器 + 私信全搜。
    Everything,
}

/// 应用状态：只保存数据与交互状态，具体渲染交给 ui 模块。
pub struct DiscordApp {
    pub(crate) view_mode: ViewMode,
    pub(crate) servers: Vec<Server>,
    pub(crate) active_server: usize,
    pub(crate) active_channel: String,
    pub(crate) messages: HashMap<String, Vec<Message>>,
    pub(crate) members: Vec<Member>,

    pub(crate) friends: Vec<Friend>,
    pub(crate) dm_conversations: Vec<DmConversation>,
    pub(crate) dm_messages: HashMap<String, Vec<Message>>,
    pub(crate) active_dm: Option<String>,

    pub(crate) composer: Entity<InputState>,
    pub(crate) search: Entity<InputState>,
    pub(crate) search_query: String,
    /// 私信侧栏搜索会话/好友的输入与过滤词。
    pub(crate) dm_search: Entity<InputState>,
    pub(crate) dm_query: String,
    pub(crate) scroll_handle: ScrollHandle,
    pub(crate) collapsed: Vec<String>,
    /// 成员栏已折叠的角色分组 key。
    pub(crate) collapsed_roles: Vec<String>,

    pub(crate) show_members: bool,
    /// 收件箱面板显隐与已读通知的 title 集合（纯 Demo，不持久化）。
    pub(crate) show_inbox: bool,
    pub(crate) read_notifications: Vec<String>,
    pub(crate) mic_on: bool,
    pub(crate) headphones_on: bool,
    pub(crate) notifications_on: bool,

    pub(crate) emoji_picker_open: bool,
    /// Some(index) 表示给当前会话第 index 条消息加回应，None 表示插入输入框草稿。
    pub(crate) emoji_target: Option<usize>,
    /// 回复草稿：被回复作者 + 原消息摘要（发出后写入新消息的 reply_to）。
    pub(crate) reply_to: Option<ReplyRef>,
    /// 点击回复引用后高亮的原消息下标（切会话即清除）。
    pub(crate) reply_highlight: Option<usize>,
    /// @提及自动补全：当前正在输入的 @前缀 与匹配到的候选（基础名）。
    pub(crate) mention_prefix: Option<String>,
    pub(crate) mention_candidates: Vec<&'static str>,

    pub(crate) profile_target: Option<&'static str>,
    pub(crate) menu_message: Option<usize>,
    /// 消息菜单的光标锚点：Some 表示根级浮层（右键），None 表示挂在消息行内（省略号入口）。
    pub(crate) menu_message_point: Option<(f32, f32)>,
    pub(crate) show_pins: bool,
    /// 当前语音连接：(服务器索引, 频道名)；跨会话保持，符合 Discord 行为。
    pub(crate) voice_channel: Option<(usize, String)>,

    pub(crate) my_status: UserStatus,
    pub(crate) show_status_menu: bool,

    // 当前用户资料：账户页可编辑，底部用户栏与设置页预览共用。
    pub(crate) me: MyProfile,
    pub(crate) account_name_input: Entity<InputState>,
    pub(crate) account_username_input: Entity<InputState>,
    pub(crate) account_email_input: Entity<InputState>,
    pub(crate) account_bio_input: Entity<InputState>,
    /// 账户页的保存结果提示（打开设置 / 再次保存时刷新）。
    pub(crate) account_notice: Option<&'static str>,
    /// 正在编辑的消息索引；编辑输入复用独立 InputState。
    pub(crate) editing_message: Option<usize>,
    pub(crate) editor: Entity<InputState>,

    // 用户设置页：覆盖主界面的独立视图。
    pub(crate) show_settings: bool,
    pub(crate) settings_section: SettingsSection,
    pub(crate) theme_choice: ThemeChoice,
    pub(crate) compact_messages: bool,
    pub(crate) show_timestamps: bool,
    pub(crate) notification_sound: bool,
    pub(crate) mention_pings: bool,
    pub(crate) show_emoji: bool,
    pub(crate) input_volume: u8,
    pub(crate) output_volume: u8,

    // 新建服务器 / 频道弹层：输入草稿 + 弹层显隐。
    pub(crate) show_create_server: bool,
    pub(crate) show_create_channel: bool,
    pub(crate) server_name_input: Entity<InputState>,
    pub(crate) channel_name_input: Entity<InputState>,

    // 好友页：标签 + 请求 + 屏蔽列表 + 添加好友输入。
    pub(crate) friend_tab: FriendTab,
    pub(crate) friend_requests: Vec<FriendRequest>,
    pub(crate) blocked_users: Vec<String>,
    pub(crate) add_friend_input: Entity<InputState>,

    // 频道操作菜单（右键 / 标题栏铃铛）与频道主题编辑弹层。
    pub(crate) channel_menu: Option<ChannelMenuState>,
    pub(crate) show_edit_topic: bool,
    pub(crate) edit_topic_target: String,
    pub(crate) topic_input: Entity<InputState>,

    // 服务器下拉菜单（邀请 / 通知 / 静音 / 标记已读 / 创建频道 / 离开）与邀请弹层。
    pub(crate) show_server_menu: bool,
    /// 服务器菜单是否翻到「通知设置」页；每次打开菜单都会复位成操作页。
    pub(crate) server_notify_page: bool,
    pub(crate) show_invite: bool,
    pub(crate) invite_input: Entity<InputState>,

    // 分类操作菜单（右键分类标题）、分类重命名与新建分类弹层。
    pub(crate) category_menu: Option<CategoryMenuState>,
    pub(crate) show_rename_category: bool,
    pub(crate) show_create_category: bool,
    /// 重命名前的分类名，用于定位分类并在改名后迁移折叠状态。
    pub(crate) rename_category_source: String,
    pub(crate) category_input: Entity<InputState>,
    /// 新建频道的目标分类；None 表示落到第一个分类。
    pub(crate) create_channel_category: Option<String>,

    // 附件：待发送草稿 + 附件选择弹层显隐。
    pub(crate) attachment_draft: Option<Attachment>,
    pub(crate) show_attachment_picker: bool,

    // 贴纸弹层：全部贴纸 + 包顺序 + 当前包 + 显隐。
    pub(crate) stickers: Vec<Sticker>,
    pub(crate) sticker_packs: Vec<&'static str>,
    pub(crate) sticker_pack: &'static str,
    pub(crate) show_sticker_picker: bool,

    // GIF 弹层：全部 GIF + 分类顺序 + 当前分类 + 搜索词 + 显隐。
    pub(crate) gifs: Vec<Gif>,
    pub(crate) gif_categories: Vec<&'static str>,
    pub(crate) gif_category: &'static str,
    pub(crate) gif_query: String,
    pub(crate) show_gif_picker: bool,
    pub(crate) gif_search: Entity<InputState>,

    // 服务器设置页：整屏覆盖主界面，与用户设置页互斥。
    pub(crate) show_server_settings: bool,
    pub(crate) server_settings_section: ServerSettingsSection,
    pub(crate) server_name_edit: Entity<InputState>,
    pub(crate) server_desc_edit: Entity<InputState>,
    /// 服务器自定义表情（Demo：可移除与恢复，不做上传）。
    pub(crate) server_emojis: Vec<&'static str>,
    pub(crate) removed_server_emojis: Vec<&'static str>,
    /// 设置页里复制邀请链接后的「已复制」提示。
    pub(crate) invite_copied: bool,

    // 语音房间面板：连接语音时展示参与者与本地设备状态。
    pub(crate) show_voice_room: bool,
    pub(crate) voice_participants: Vec<VoiceParticipant>,

    // 私信通话：Some 即显示悬浮通话面板，跨会话与设置页保持。
    pub(crate) dm_call: Option<DmCall>,

    // 探索页：公开服务器候选、搜索框与分类筛选。
    pub(crate) discover_servers: Vec<DiscoverServer>,
    pub(crate) discover_search: Entity<InputState>,
    pub(crate) discover_filter: String,
    /// None 表示「全部」分类。
    pub(crate) discover_category: Option<&'static str>,

    // 搜索中心：把聊天主区域替换为跨频道 / 私信的结果页。
    /// 成员上下文菜单（右键成员行）：目标成员 + 锚点。
    pub(crate) member_menu: Option<MemberMenuState>,

    pub(crate) show_search_center: bool,
    pub(crate) search_center_input: Entity<InputState>,
    pub(crate) search_center_query: String,
    pub(crate) search_scope: SearchScope,
    pub(crate) search_results: Vec<SearchHit>,
    /// 当前搜索的高级过滤条件（由 search_center_query 解析而来）。
    pub(crate) search_filters: SearchFilters,

    // 帮助中心：整屏浮层，左侧分类 + 搜索 + 文档列表 / 详情。
    pub(crate) show_help_center: bool,
    pub(crate) help_input: Entity<InputState>,
    pub(crate) help_query: String,
    /// None 表示「全部」分类。
    pub(crate) help_category: Option<&'static str>,
    /// 当前打开的文档 id；None 时显示列表。
    pub(crate) help_article: Option<&'static str>,
    pub(crate) help_articles: Vec<HelpArticle>,
    pub(crate) help_categories: Vec<&'static str>,

    // 消息转发：源消息下标 + 弹层显隐 + 目标搜索 + 成功提示。
    pub(crate) forward_source: Option<usize>,
    pub(crate) show_forward: bool,
    pub(crate) forward_search: Entity<InputState>,
    pub(crate) forward_query: String,
    /// 转发成功后的顶栏提示文案，切换会话即清除。
    pub(crate) forward_notice: Option<String>,

    // 附件预览：点击消息里的附件卡打开大图/文件详情弹层。
    pub(crate) attachment_preview: Option<Attachment>,

    // 已读回执详情浮层：(消息下标, 点击坐标 x, y)。
    pub(crate) read_receipts: Option<(usize, f32, f32)>,

    // 编辑历史浮层：(消息下标, 光标 x, 光标 y)。
    pub(crate) edit_history: Option<(usize, f32, f32)>,

    // 话题：按会话键保存，thread_root 为当前打开话题在原消息列表里的下标。
    pub(crate) threads: HashMap<String, Vec<ChatThread>>,
    pub(crate) show_thread: bool,
    pub(crate) thread_root: Option<usize>,
    pub(crate) thread_input: Entity<InputState>,

    // 快速切换器：Ctrl/⌘+K 打开，搜索频道/私信并回车跳转。
    pub(crate) show_quick_switcher: bool,
    pub(crate) quick_search: Entity<InputState>,
    pub(crate) quick_query: String,
    /// 当前高亮的目标下标（上下键移动）。
    pub(crate) quick_selected: usize,
    /// 顶层键盘监听所需的焦点句柄，启动后聚焦，使 Ctrl/⌘+K 与 Esc 可捕获。
    pub(crate) root_focus: FocusHandle,

    // 会话草稿：按会话键保存未发送文本与附件，切走再切回自动恢复。
    /// 键为 session_key（频道「服务器/频道」、私信「dm:好友」）。
    pub(crate) drafts: HashMap<String, MessageDraft>,
    /// 输入框当前归属的会话键；切换时据此把内容存回旧会话。
    pub(crate) current_session: String,

    // 本地持久化：上次落盘的快照 JSON（内容不变就不重复写盘）+ 数据页提示。
    pub(crate) saved_json: String,
    pub(crate) persist_note: Option<&'static str>,
}

/// 单个会话的未发送消息草稿（文本 + 可选附件）。
#[derive(Clone)]
pub(crate) struct MessageDraft {
    pub(crate) text: String,
    pub(crate) attachment: Option<Attachment>,
}

/// 资料卡展示用的用户信息，由 app 从 friends / members 聚合而来。
pub(crate) struct ProfileInfo {
    pub(crate) name: &'static str,
    pub(crate) color: u32,
    pub(crate) online: bool,
    pub(crate) activity: &'static str,
    pub(crate) role: &'static str,
}

/// 成员显示名形如「Nova —— 管理员」，取前缀得到基础用户名。
fn base_name(display: &'static str) -> &'static str {
    display.split(" —— ").next().unwrap_or(display)
}

/// 搜索命中判定：正文或作者名包含关键词（调用方已把 query 转小写）。
/// 搜索中心的高级过滤器：从查询串解析 `from:` / `has:` / `before:` / `after:`，剩余为普通关键词。
#[derive(Default, Clone)]
pub(crate) struct SearchFilters {
    /// 普通关键词（小写）；为空表示只按过滤条件匹配。
    pub text: String,
    /// 作者名片段，任一命中即可。
    pub from: Vec<String>,
    /// 消息属性：link / image / file / attachment / sticker / gif / reaction / edited / mention / pinned。
    pub has: Vec<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

impl SearchFilters {
    pub fn parse(raw: &str) -> Self {
        let mut filters = Self::default();
        let mut words = Vec::new();
        for token in raw.split_whitespace() {
            if let Some(value) = token.strip_prefix("from:") {
                if !value.is_empty() {
                    filters.from.push(value.to_lowercase());
                }
            } else if let Some(value) = token.strip_prefix("has:") {
                if !value.is_empty() {
                    filters.has.push(value.to_lowercase());
                }
            } else if let Some(value) = token.strip_prefix("before:") {
                if !value.is_empty() {
                    filters.before = Some(value.to_lowercase());
                }
            } else if let Some(value) = token.strip_prefix("after:") {
                if !value.is_empty() {
                    filters.after = Some(value.to_lowercase());
                }
            } else {
                words.push(token);
            }
        }
        filters.text = words.join(" ").to_lowercase();
        filters
    }

    /// 是否没有任何约束（空关键词且无过滤条件）。
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
            && self.from.is_empty()
            && self.has.is_empty()
            && self.before.is_none()
            && self.after.is_none()
    }

    /// UI 展示用的已应用条件标签。
    pub fn chips(&self) -> Vec<String> {
        let mut chips = Vec::new();
        for value in &self.from {
            chips.push(format!("from:{value}"));
        }
        for value in &self.has {
            chips.push(format!("has:{value}"));
        }
        if let Some(value) = &self.before {
            chips.push(format!("before:{value}"));
        }
        if let Some(value) = &self.after {
            chips.push(format!("after:{value}"));
        }
        chips
    }

    /// 同时满足关键词、作者、属性与时间条件才算命中。
    pub fn matches(&self, message: &Message) -> bool {
        if !self.text.is_empty() && !message.text.to_lowercase().contains(&self.text) {
            return false;
        }
        if !self.from.is_empty()
            && !self
                .from
                .iter()
                .any(|name| message.author.to_lowercase().contains(name))
        {
            return false;
        }
        // 多个 has: 是 AND 关系。
        for feature in &self.has {
            let ok = match feature.as_str() {
                "link" => message.text.contains("http") || message.text.contains("链接"),
                "image" => message
                    .attachment
                    .as_ref()
                    .is_some_and(|attachment| attachment.is_image()),
                "file" => message
                    .attachment
                    .as_ref()
                    .is_some_and(|attachment| !attachment.is_image()),
                "attachment" => message.attachment.is_some(),
                "sticker" => message.sticker.is_some(),
                "gif" => message.gif.is_some(),
                "reaction" => !message.reactions.is_empty(),
                "edited" => message.is_edited(),
                "mention" => message.text.contains('@'),
                "pinned" => message.pinned,
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        // 时间基于种子数据的前缀：今天/刚刚 > 昨天 > 更早。
        let rank = if message.time.starts_with("刚刚") || message.time.starts_with("今天") {
            2
        } else if message.time.starts_with("昨天") {
            1
        } else {
            0
        };
        if let Some(value) = &self.after {
            let min = match value.as_str() {
                "today" => 2,
                "yesterday" => 1,
                _ => i32::MIN,
            };
            if rank < min {
                return false;
            }
        }
        if let Some(value) = &self.before {
            let max = match value.as_str() {
                "today" => 2,
                "yesterday" => 1,
                _ => i32::MAX,
            };
            if rank >= max {
                return false;
            }
        }
        true
    }
}

impl DiscordApp {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| {
            InputState::new(window, cx).placeholder("发送消息到 #闲聊  (Enter 发送)")
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索"));
        let dm_search = cx.new(|cx| InputState::new(window, cx).placeholder("查找或开始对话"));
        let editor = cx.new(|cx| InputState::new(window, cx).placeholder("编辑消息"));
        let server_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入服务器名称"));
        let channel_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入频道名称"));
        let add_friend_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入用户名#标签，例如 Zoe#1234"));
        let topic_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入频道主题"));
        let invite_input = cx.new(|cx| InputState::new(window, cx).placeholder("邀请链接"));
        let category_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入分类名称"));
        let account_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("显示名称"));
        let account_username_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("用户名#0001"));
        let account_email_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("邮箱地址"));
        let account_bio_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("一句话介绍自己"));
        let server_name_edit =
            cx.new(|cx| InputState::new(window, cx).placeholder("服务器名称"));
        let server_desc_edit =
            cx.new(|cx| InputState::new(window, cx).placeholder("用一两句话介绍这个服务器"));
        let discover_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索服务器或关键词"));
        let search_center_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索消息，支持 from: / has: / after:"));
        let help_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索帮助主题，例如 话题 / 快捷键"));
        let forward_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索频道或私信"));
        let quick_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("跳转到频道或私信…"));
        let gif_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索 GIF 标签…"));
        let thread_input = cx.new(|cx| InputState::new(window, cx).placeholder("回复话题…"));
        let root_focus = cx.focus_handle();

        cx.subscribe_in(&composer, window, |this, _, event: &InputEvent, window, cx| {
            match event {
                InputEvent::PressEnter { secondary: false, .. } => this.send_message(window, cx),
                InputEvent::Change => this.refresh_mention(cx),
                _ => {}
            }
        })
        .detach();

        cx.subscribe_in(&editor, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.commit_edit(window, cx);
            }
        })
        .detach();

        cx.subscribe_in(&search, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.search_query = this.search.read(cx).value().to_string();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&dm_search, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.dm_query = this.dm_search.read(cx).value().to_string();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&server_name_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.create_server(window, cx);
            }
        })
        .detach();

        cx.subscribe_in(&channel_name_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.create_channel(window, cx);
            }
        })
        .detach();

        cx.subscribe_in(&add_friend_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.send_friend_request(window, cx);
            }
        })
        .detach();

        // 分类输入框同时服务「新建分类」与「重命名分类」，按当前弹层分发 Enter。
        cx.subscribe_in(&category_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                if this.show_create_category {
                    this.create_category(window, cx);
                } else if this.show_rename_category {
                    this.save_category_rename(window, cx);
                }
            }
        })
        .detach();

        cx.subscribe_in(&search_center_input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.search_center_query = this.search_center_input.read(cx).value().to_string();
                this.run_search();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&help_input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.help_query = this.help_input.read(cx).value().to_string();
                // 搜索时把列表拉回全分类，避免结果被当前分类挡住。
                if !this.help_query.trim().is_empty() {
                    this.help_category = None;
                }
                this.help_article = None;
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&discover_search, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.discover_filter = this.discover_search.read(cx).value().to_string();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&forward_search, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.forward_query = this.forward_search.read(cx).value().to_string();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&quick_search, window, |this, _, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Change => {
                    this.quick_query = this.quick_search.read(cx).value().to_string();
                    this.quick_selected = 0;
                    cx.notify();
                }
                InputEvent::PressEnter { secondary: false, .. } => {
                    this.activate_quick_selection(window, cx);
                }
                _ => {}
            }
        })
        .detach();

        cx.subscribe_in(&gif_search, window, |this, _, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Change => {
                    this.gif_query = this.gif_search.read(cx).value().to_string();
                    cx.notify();
                }
                InputEvent::PressEnter { secondary: false, .. } => {
                    this.send_first_gif(window, cx);
                }
                _ => {}
            }
        })
        .detach();

        cx.subscribe_in(&thread_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.send_thread_reply(window, cx);
            }
        })
        .detach();

        // 服务器设置「概览」的两个输入框都按 Enter 保存。
        cx.subscribe_in(&server_name_edit, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.save_server_overview(cx);
            }
        })
        .detach();

        cx.subscribe_in(&server_desc_edit, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.save_server_overview(cx);
            }
        })
        .detach();

        cx.subscribe_in(&topic_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.save_channel_topic(window, cx);
            }
        })
        .detach();

        let mut this = Self {
            view_mode: ViewMode::Server,
            servers: seed_servers(),
            active_server: 0,
            active_channel: "闲聊".into(),
            messages: seed_messages(),
            members: seed_members(),
            friends: seed_friends(),
            dm_conversations: seed_dm_conversations(),
            dm_messages: seed_dm_messages(),
            active_dm: None,
            composer,
            search,
            search_query: String::new(),
            dm_search,
            dm_query: String::new(),
            editor,
            scroll_handle: ScrollHandle::default(),
            collapsed: Vec::new(),
            collapsed_roles: Vec::new(),
            show_members: true,
            show_inbox: false,
            read_notifications: Vec::new(),
            mic_on: true,
            headphones_on: true,
            notifications_on: true,
            emoji_picker_open: false,
            emoji_target: None,
            reply_to: None,
            reply_highlight: None,
            mention_prefix: None,
            mention_candidates: Vec::new(),
            profile_target: None,
            menu_message: None,
            menu_message_point: None,
            show_pins: false,
            voice_channel: None,
            my_status: UserStatus::Online,
            show_status_menu: false,
            me: seed_me(),
            account_name_input,
            account_username_input,
            account_email_input,
            account_bio_input,
            account_notice: None,
            editing_message: None,
            show_settings: false,
            settings_section: SettingsSection::Account,
            theme_choice: ThemeChoice::Dark,
            compact_messages: false,
            show_timestamps: true,
            notification_sound: true,
            mention_pings: true,
            show_emoji: true,
            input_volume: 80,
            output_volume: 100,
            show_create_server: false,
            show_create_channel: false,
            server_name_input,
            channel_name_input,
            friend_tab: FriendTab::Online,
            friend_requests: seed_friend_requests(),
            blocked_users: Vec::new(),
            add_friend_input,
            channel_menu: None,
            show_edit_topic: false,
            edit_topic_target: String::new(),
            topic_input,
            show_server_menu: false,
            server_notify_page: false,
            show_invite: false,
            invite_input,
            category_menu: None,
            show_rename_category: false,
            show_create_category: false,
            rename_category_source: String::new(),
            category_input,
            create_channel_category: None,
            attachment_draft: None,
            show_attachment_picker: false,
            stickers: seed_stickers(),
            sticker_pack: sticker_packs().first().copied().unwrap_or("日常"),
            sticker_packs: sticker_packs(),
            show_sticker_picker: false,
            gifs: seed_gifs(),
            gif_categories: gif_categories(),
            gif_category: "热门",
            gif_query: String::new(),
            show_gif_picker: false,
            gif_search,
            show_server_settings: false,
            server_settings_section: ServerSettingsSection::Overview,
            server_name_edit,
            server_desc_edit,
            server_emojis: vec![
                "🎨", "🚀", "🔥", "✅", "👀", "🦀", "🎧", "🎮", "🍜", "🌙", "💡", "🐧",
            ],
            removed_server_emojis: Vec::new(),
            invite_copied: false,
            show_voice_room: false,
            voice_participants: Vec::new(),
            dm_call: None,
            discover_servers: seed_discover_servers(),
            discover_search,
            discover_filter: String::new(),
            discover_category: None,
            member_menu: None,
            show_search_center: false,
            search_center_input,
            search_center_query: String::new(),
            search_scope: SearchScope::CurrentServer,
            search_results: Vec::new(),
            search_filters: SearchFilters::default(),
            show_help_center: false,
            help_input,
            help_query: String::new(),
            help_category: None,
            help_article: None,
            help_articles: seed_help_articles(),
            help_categories: help_categories(),
            forward_source: None,
            show_forward: false,
            forward_search,
            forward_query: String::new(),
            forward_notice: None,
            attachment_preview: None,
            read_receipts: None,
            edit_history: None,
            threads: seed_threads(),
            show_thread: false,
            thread_root: None,
            thread_input,
            show_quick_switcher: false,
            quick_search,
            quick_query: String::new(),
            quick_selected: 0,
            root_focus,
            drafts: HashMap::new(),
            current_session: String::new(),
            saved_json: String::new(),
            persist_note: None,
        };
        // 让输入框在首个会话就归属正确的键，默认频道草稿也能被保存。
        this.current_session = this.session_key();
        // 有本地快照就覆盖种子数据；没有则以种子数据启动并立即记一次基线。
        this.restore_state(window, cx);
        window.focus(&this.root_focus, cx);
        this
    }

    // -----------------------------------------------------------------------
    // 会话索引与消息访问
    // -----------------------------------------------------------------------

    /// 当前服务器频道的 `服务器/频道` 键。
    pub(crate) fn active_key(&self) -> String {
        format!(
            "{}/{}",
            self.servers[self.active_server].name, self.active_channel
        )
    }

    /// 输入框内容归属的会话键：频道「服务器/频道」、私信「dm:好友」，其他视图为空（不保留草稿）。
    fn session_key(&self) -> String {
        match self.view_mode {
            ViewMode::Server => self.active_key(),
            ViewMode::DirectMessage => format!("dm:{}", self.active_dm.as_deref().unwrap_or("")),
            ViewMode::Friends | ViewMode::Discover => String::new(),
        }
    }

    /// 切换会话前调用：把输入框现有文本与待发附件存入旧会话（current_session 尚未更新）。
    /// 内容为空则删除该会话草稿，避免侧栏出现空草稿标记。
    fn save_draft(&mut self, cx: &mut Context<Self>) {
        let key = self.current_session.clone();
        if key.is_empty() {
            return;
        }
        let text = self.composer.read(cx).value().trim().to_string();
        let attachment = self.attachment_draft.take();
        if text.is_empty() && attachment.is_none() {
            self.drafts.remove(&key);
        } else {
            self.drafts
                .insert(key, MessageDraft { text, attachment });
        }
    }

    /// 切换会话后调用：更新归属键并恢复该会话草稿。
    fn load_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.session_key();
        self.current_session = key.clone();
        let draft = if key.is_empty() {
            None
        } else {
            self.drafts.get(&key).cloned()
        };
        let text = draft.as_ref().map(|d| d.text.clone()).unwrap_or_default();
        self.attachment_draft = draft.and_then(|d| d.attachment);
        // set_value 不发 Change 事件，不会触发 @提及刷新，这里静默恢复即可。
        self.composer
            .update(cx, |input, cx| input.set_value(text, window, cx));
    }

    /// 侧栏草稿标记判定：指定会话键是否存在非空草稿。
    pub(crate) fn has_draft(&self, key: &str) -> bool {
        !key.is_empty() && self.drafts.contains_key(key)
    }

    /// 当前服务器某频道是否有草稿。
    pub(crate) fn channel_has_draft(&self, name: &str) -> bool {
        self.has_draft(&format!("{}/{}", self.servers[self.active_server].name, name))
    }

    /// 某好友私信是否有草稿。
    pub(crate) fn dm_has_draft(&self, friend: &str) -> bool {
        self.has_draft(&format!("dm:{friend}"))
    }

    /// 当前服务器的借用引用，供视图读取名称/徽标。
    pub(crate) fn active_server_ref(&self) -> &crate::data::Server {
        &self.servers[self.active_server]
    }

    /// 当前会话主题（频道用 topic，私信用好友名）。
    pub(crate) fn active_topic(&self) -> String {
        if self.view_mode == ViewMode::DirectMessage {
            return format!("与 {} 的私信", self.active_dm.as_deref().unwrap_or(""));
        }
        for category in &self.servers[self.active_server].categories {
            for channel in &category.channels {
                if channel.name == self.active_channel {
                    return channel.topic.clone();
                }
            }
        }
        String::new()
    }

    /// 当前会话的消息列表（克隆，供视图只读渲染）。
    pub(crate) fn current_messages(&self) -> Vec<Message> {
        match self.view_mode {
            ViewMode::Server => self
                .messages
                .get(&self.active_key())
                .cloned()
                .unwrap_or_default(),
            ViewMode::DirectMessage => self
                .active_dm
                .as_ref()
                .and_then(|name| self.dm_messages.get(name).cloned())
                .unwrap_or_default(),
            ViewMode::Friends | ViewMode::Discover => Vec::new(),
        }
    }

    /// 当前会话消息的可变引用。
    fn current_messages_mut(&mut self) -> Option<&mut Vec<Message>> {
        match self.view_mode {
            ViewMode::Server => {
                let key = self.active_key();
                self.messages.get_mut(&key)
            }
            ViewMode::DirectMessage => self
                .active_dm
                .as_ref()
                .and_then(|name| self.dm_messages.get_mut(name)),
            ViewMode::Friends | ViewMode::Discover => None,
        }
    }

    /// 当前会话标题。
    pub(crate) fn session_title(&self) -> String {
        match self.view_mode {
            ViewMode::Server => self.active_channel.clone(),
            ViewMode::DirectMessage => self.active_dm.clone().unwrap_or_default(),
            ViewMode::Friends => "好友".into(),
            ViewMode::Discover => "探索".into(),
        }
    }

    /// 当前会话是否为私信。
    pub(crate) fn is_dm(&self) -> bool {
        self.view_mode == ViewMode::DirectMessage
    }

    /// 当前私信好友的在线状态。
    pub(crate) fn active_dm_online(&self) -> bool {
        self.active_dm
            .as_ref()
            .and_then(|name| self.friends.iter().find(|f| f.name == name.as_str()))
            .map(|f| f.online)
            .unwrap_or(false)
    }

    /// 根据显示名（可能是「Nova」或「Nova —— 管理员」）聚合资料卡信息。
    pub(crate) fn profile_info(&self, display: &'static str) -> ProfileInfo {
        let base = base_name(display);
        let friend = self.friends.iter().find(|f| f.name == base);
        let member = self.members.iter().find(|m| base_name(m.name) == base);

        let color = friend.map(|f| f.color).or_else(|| member.map(|m| m.color)).unwrap_or(palette::accent());
        let online = friend
            .map(|f| f.online)
            .or_else(|| member.map(|m| m.online))
            .unwrap_or(false);
        let activity = friend
            .map(|f| f.activity)
            .unwrap_or(if online { "正在使用 GPUI Kit" } else { "离线" });
        let role = member
            .and_then(|m| m.name.split_once(" —— ").map(|(_, role)| role))
            .unwrap_or("成员");

        ProfileInfo {
            name: base,
            color,
            online,
            activity,
            role,
        }
    }

    /// 角色名对应的分组高亮色；未知角色给默认强调色。
    fn role_group_color(role: &str) -> u32 {
        match role {
            "管理员" => 0xf23f43,
            "版主" => 0x57f287,
            "贡献者" => 0x00a8fc,
            _ => palette::accent(),
        }
    }

    /// 按角色聚合成员栏：有角色的按角色名分组（首次出现顺序），
    /// 再补「在线」「离线」兜底组。返回顺序稳定，供视图直接渲染。
    pub(crate) fn member_groups(&self) -> Vec<MemberGroup> {
        let mut groups: Vec<MemberGroup> = Vec::new();

        for (index, member) in self.members.iter().enumerate() {
            if let Some((_, role)) = member.name.split_once(" —— ") {
                let key = format!("role-{role}");
                if let Some(group) = groups.iter_mut().find(|g| g.key == key) {
                    group.member_indices.push(index);
                } else {
                    groups.push(MemberGroup {
                        title: role.to_string(),
                        color: Self::role_group_color(role),
                        key,
                        member_indices: vec![index],
                    });
                }
            }
        }

        // 无角色成员按在线状态归入兜底组。
        let online: Vec<usize> = self
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.online && !m.name.contains(" —— "))
            .map(|(i, _)| i)
            .collect();
        let offline: Vec<usize> = self
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.online && !m.name.contains(" —— "))
            .map(|(i, _)| i)
            .collect();

        if !online.is_empty() {
            groups.push(MemberGroup {
                title: "在线".into(),
                color: palette::text_muted(),
                key: "role-online".into(),
                member_indices: online,
            });
        }
        if !offline.is_empty() {
            groups.push(MemberGroup {
                title: "离线".into(),
                color: palette::text_dim(),
                key: "role-offline".into(),
                member_indices: offline,
            });
        }

        groups
    }

    /// 切换成员栏角色分组的折叠状态。
    pub(crate) fn toggle_role_group(&mut self, key: String) {
        if let Some(pos) = self.collapsed_roles.iter().position(|k| k == &key) {
            self.collapsed_roles.remove(pos);
        } else {
            self.collapsed_roles.push(key);
        }
    }

    // -----------------------------------------------------------------------
    // 收件箱 / 通知中心
    // -----------------------------------------------------------------------

    /// 通知的唯一 key（用于已读判定），由类型 + 目标标识组成。
    fn notification_key(notification: &Notification) -> String {
        match &notification.target {
            NotificationTarget::Channel { server, channel } => {
                format!("channel:{server}:{channel}")
            }
            NotificationTarget::Dm(friend) => format!("dm:{friend}"),
            NotificationTarget::Friends => "friends".into(),
        }
    }

    /// 从未读频道、未读私信、收到的好友请求与 @提及 聚合通知列表。
    pub(crate) fn notifications(&self) -> Vec<Notification> {
        let mut items: Vec<Notification> = Vec::new();

        // @提及：扫描所有会话消息中含 @已知好友 的文本。
        let scan_mentions = |key: &str, target: &NotificationTarget, fallback_color: u32| {
            let mut found: Vec<Notification> = Vec::new();
            for message in self.messages.get(key).into_iter().flatten() {
                for token in message.text.split_whitespace() {
                    if let Some(mention) = token.strip_prefix('@') {
                        if self.resolve_mention(mention).is_some() {
                            found.push(Notification {
                                kind: NotificationKind::Mention,
                                title: format!("{} 在消息中提到了你", message.author),
                                body: message.text.clone(),
                                time: message.time,
                                color: fallback_color,
                                target: target.clone(),
                            });
                            break;
                        }
                    }
                }
            }
            found
        };

        for (server_index, server) in self.servers.iter().enumerate() {
            for category in &server.categories {
                for channel in &category.channels {
                    // 静音频道不进收件箱，也不产生 @提及通知（仍可正常翻消息）。
                    // 频道选「跟随服务器」时，继承服务器的默认级别一起判断。
                    let effective = match channel.notify {
                        NotifyLevel::Inherit => server.notify,
                        level => level,
                    };
                    if effective.is_muted() {
                        continue;
                    }
                    if channel.unread > 0 && channel.kind == ChannelKind::Text {
                        items.push(Notification {
                            kind: NotificationKind::UnreadChannel,
                            title: format!("#{} 有 {} 条未读消息", channel.name, channel.unread),
                            body: channel.topic.clone(),
                            time: "最近",
                            color: server.color,
                            target: NotificationTarget::Channel {
                                server: server_index,
                                channel: channel.name.clone(),
                            },
                        });
                    }
                    let key = format!("{}/{}", server.name, channel.name);
                    let target = NotificationTarget::Channel {
                        server: server_index,
                        channel: channel.name.clone(),
                    };
                    items.extend(scan_mentions(&key, &target, server.color));
                }
            }
        }

        for conversation in &self.dm_conversations {
            if conversation.unread > 0 {
                items.push(Notification {
                    kind: NotificationKind::Dm,
                    title: format!("{} 发来 {} 条私信", conversation.friend, conversation.unread),
                    body: conversation.preview.to_string(),
                    time: "最近",
                    color: conversation.color,
                    target: NotificationTarget::Dm(conversation.friend.to_string()),
                });
            }
        }

        for request in self.friend_requests.iter().filter(|r| r.incoming) {
            items.push(Notification {
                kind: NotificationKind::FriendRequest,
                title: format!("{} 想添加你为好友", request.name),
                body: "前往好友页处理请求".into(),
                time: "最近",
                color: request.color,
                target: NotificationTarget::Friends,
            });
        }

        items
    }

    /// 未读通知数量：通知 key 不在已读集合中的条目数。
    pub(crate) fn unread_notification_count(&self) -> usize {
        self.notifications()
            .iter()
            .filter(|n| !self.read_notifications.contains(&Self::notification_key(n)))
            .count()
    }

    /// 指定索引的通知是否未读。
    pub(crate) fn is_notification_unread(&self, index: usize) -> bool {
        self.notifications()
            .get(index)
            .map(|n| !self.read_notifications.contains(&Self::notification_key(n)))
            .unwrap_or(false)
    }

    /// 打开/关闭收件箱面板。
    pub(crate) fn toggle_inbox(&mut self, cx: &mut Context<Self>) {
        self.show_inbox = !self.show_inbox;
        if self.show_inbox {
            self.show_pins = false;
            self.close_thread(); self.close_read_receipts(); self.close_edit_history();
            self.show_status_menu = false;
        }
        cx.notify();
    }

    /// 标记某条通知为已读。
    fn mark_notification_read(&mut self, notification: &Notification) {
        let key = Self::notification_key(notification);
        if !self.read_notifications.contains(&key) {
            self.read_notifications.push(key);
        }
    }

    /// 点击通知：按目标跳转会话并标记已读。
    pub(crate) fn open_notification(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(notification) = self.notifications().get(index).cloned() else {
            return;
        };
        self.mark_notification_read(&notification);
        self.show_inbox = false;
        match notification.target {
            NotificationTarget::Channel { server, channel } => {
                self.view_mode = ViewMode::Server;
                self.active_server = server;
                self.active_channel = channel;
                self.reset_session(window, cx);
            }
            NotificationTarget::Dm(friend) => self.open_dm(friend, window, cx),
            NotificationTarget::Friends => {
                self.view_mode = ViewMode::Friends;
                self.friend_tab = FriendTab::Pending;
                self.reset_session(window, cx);
            }
        }
        cx.notify();
    }

    /// 将全部当前通知标记为已读。
    pub(crate) fn mark_all_notifications_read(&mut self, cx: &mut Context<Self>) {
        for notification in self.notifications() {
            self.mark_notification_read(&notification);
        }
        cx.notify();
    }

    /// 解析 `@名字` 片段：仅当匹配已知好友/成员时返回其静态名，用于高亮与资料卡。
    pub(crate) fn resolve_mention(&self, token: &str) -> Option<&'static str> {
        if let Some(friend) = self.friends.iter().find(|f| f.name == token) {
            return Some(friend.name);
        }
        self.members
            .iter()
            .find(|m| base_name(m.name) == token)
            .map(|m| base_name(m.name))
    }

    /// 输入框内容变化时刷新 @补全：取末尾正在输入的 `@前缀` 匹配好友/成员基础名。
    fn refresh_mention(&mut self, cx: &mut Context<Self>) {
        let value = self.composer.read(cx).value().to_string();
        // 取最后一段以空白分隔的 token。
        let tail = value.rsplit(char::is_whitespace).next().unwrap_or("");

        let Some(prefix) = tail.strip_prefix('@') else {
            self.mention_prefix = None;
            self.mention_candidates.clear();
            cx.notify();
            return;
        };

        let lower = prefix.to_lowercase();
        let mut candidates: Vec<&'static str> = Vec::new();
        for friend in &self.friends {
            if !candidates.contains(&friend.name) {
                candidates.push(friend.name);
            }
        }
        for member in &self.members {
            let base = base_name(member.name);
            if !candidates.contains(&base) {
                candidates.push(base);
            }
        }
        candidates.retain(|name| name.to_lowercase().starts_with(&lower));

        self.mention_prefix = Some(prefix.to_string());
        self.mention_candidates = candidates;
        cx.notify();
    }

    /// 选中候选：把输入框末尾的 `@前缀` 替换成 `@完整名 `，并关闭补全。
    pub(crate) fn apply_mention(
        &mut self,
        name: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = self.composer.read(cx).value().to_string();
        let replaced = match value.rfind('@') {
            Some(pos) => {
                let mut next = value[..pos].to_string();
                next.push('@');
                next.push_str(name);
                next.push(' ');
                next
            }
            None => value,
        };
        self.composer
            .update(cx, |input, cx| input.set_value(replaced, window, cx));
        self.mention_prefix = None;
        self.mention_candidates.clear();
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 会话切换
    // -----------------------------------------------------------------------

    /// 清空搜索框并重置过滤条件。
    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_query.clear();
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// 清空私信侧栏的搜索框与过滤词。
    pub(crate) fn clear_dm_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dm_query.clear();
        self.dm_search
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// 切换服务器，默认进入第一个分类的第一个频道。
    pub(crate) fn select_server(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.view_mode = ViewMode::Server;
        self.active_server = index;
        self.active_channel = self.first_text_channel().unwrap_or_default();
        self.reset_session(window, cx);
    }

    /// 切换当前服务器的频道。
    pub(crate) fn select_channel(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_channel == name && self.view_mode == ViewMode::Server {
            return;
        }
        self.view_mode = ViewMode::Server;
        self.active_channel = name;
        self.reset_session(window, cx);
    }

    /// 打开与某好友的私信。
    pub(crate) fn open_dm(
        &mut self,
        friend: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.view_mode = ViewMode::DirectMessage;
        self.active_dm = Some(friend);
        self.reset_session(window, cx);
    }

    /// 从好友发起新私信：若还没有会话则补建会话条目与空消息列表，再切入。
    pub(crate) fn start_dm(
        &mut self,
        friend: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dm_conversations.iter().any(|c| c.friend == friend) {
            let (color, online) = self
                .friends
                .iter()
                .find(|f| f.name == friend)
                .map(|f| (f.color, f.online))
                .or_else(|| {
                    // 从服务器成员发起的私信：用成员自己的颜色与在线态。
                    self.members
                        .iter()
                        .find(|m| base_name(m.name) == friend)
                        .map(|m| (m.color, m.online))
                })
                .unwrap_or((palette::accent(), false));
            self.dm_conversations.push(DmConversation {
                friend,
                color,
                online,
                unread: 0,
                preview: "开始新的对话",
            });
            self.dm_messages.entry(friend.to_string()).or_default();
        }
        self.open_dm(friend.to_string(), window, cx);
    }

    /// 进入好友页面。
    pub(crate) fn show_friends(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_mode = ViewMode::Friends;
        self.reset_session(window, cx);
    }

    /// 会话切换后的公共重置逻辑。
    fn reset_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 先把输入框内容存回旧会话（current_session 仍是旧键），再重置。
        self.save_draft(cx);
        self.clear_search(window, cx);
        self.clear_dm_search(window, cx);
        self.emoji_picker_open = false;
        self.emoji_target = None;
        self.reply_to = None;
        self.reply_highlight = None;
        self.mention_prefix = None;
        self.mention_candidates.clear();
        self.profile_target = None;
        self.close_message_menu();
        self.close_thread(); self.close_read_receipts(); self.close_edit_history();
        self.show_pins = false;
        self.show_inbox = false;
        self.show_status_menu = false;
        self.show_create_server = false;
        self.show_create_channel = false;
        self.channel_menu = None;
        self.show_edit_topic = false;
        self.edit_topic_target.clear();
        self.show_server_menu = false;
        self.show_invite = false;
        self.category_menu = None;
        self.show_rename_category = false;
        self.show_create_category = false;
        self.rename_category_source.clear();
        self.create_channel_category = None;
        self.attachment_draft = None;
        self.show_attachment_picker = false;
        self.show_sticker_picker = false;
        self.show_gif_picker = false;
        self.show_search_center = false;
        self.close_forward();
        self.forward_notice = None;
        self.attachment_preview = None;
        self.member_menu = None;
        self.cancel_edit();

        let placeholder = match self.view_mode {
            ViewMode::Server => format!("发送消息到 #{}", self.active_channel),
            ViewMode::DirectMessage => format!(
                "发送消息到 @{}",
                self.active_dm.as_deref().unwrap_or("")
            ),
            ViewMode::Friends => "选择好友开始对话".into(),
            ViewMode::Discover => "在服务器中发现新社区".into(),
        };
        self.composer.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx);
        });
        // 恢复目标会话草稿（含待发附件），并更新 current_session 为新键。
        self.load_draft(window, cx);

        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 新建服务器 / 频道
    // -----------------------------------------------------------------------

    /// 打开新建服务器弹层，清空输入草稿。
    pub(crate) fn open_create_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.server_name_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.show_create_server = true;
        self.show_create_channel = false;
        self.channel_menu = None;
        self.show_server_menu = false;
        cx.notify();
    }

    /// 打开新建频道弹层，清空输入草稿。仅服务器视图可用。
    pub(crate) fn open_create_channel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_mode != ViewMode::Server {
            return;
        }
        self.channel_name_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.create_channel_category = None;
        self.show_create_channel = true;
        self.show_create_server = false;
        self.channel_menu = None;
        self.show_server_menu = false;
        cx.notify();
    }

    pub(crate) fn close_create_server(&mut self) {
        self.show_create_server = false;
    }

    pub(crate) fn close_create_channel(&mut self) {
        self.show_create_channel = false;
    }

    /// 按当前服务器数量轮换配色，让新建服务器有区分度。
    fn next_server_color(&self) -> u32 {
        // 品牌色要读当前主题，所以不能写成 const 数组。
        let colors: [u32; 6] = [
            palette::accent(),
            0xeb459e,
            0x57f287,
            0xfee75c,
            0x00a8fc,
            0x9b59b6,
        ];
        colors[self.servers.len() % colors.len()]
    }

    /// 创建服务器：写入名称、默认频道，并切换过去。
    pub(crate) fn create_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.server_name_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        let color = self.next_server_color();
        self.servers.push(Server::new(name, color));
        let index = self.servers.len() - 1;
        self.show_create_server = false;
        self.select_server(index, window, cx);
    }

    /// 创建频道：加入目标分类（默认第一个分类），并切换过去。
    pub(crate) fn create_channel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.channel_name_input.read(cx).value().trim().to_string();
        let target = self
            .create_channel_category
            .take()
            .and_then(|category| self.category_position(&category));
        if name.is_empty() || self.view_mode != ViewMode::Server {
            return;
        }
        let server = &mut self.servers[self.active_server];
        if server.categories.is_empty() {
            return;
        }
        let ci = target.unwrap_or(0).min(server.categories.len() - 1);
        // 同名频道不重复创建（跨分类检查，避免侧栏出现两个同名频道）。
        if server.categories.iter().any(|category| {
            category
                .channels
                .iter()
                .any(|channel| channel.name == name)
        }) {
            return;
        }
        server.categories[ci]
            .channels
            .push(Channel::text(&name, &format!("欢迎来到 #{name}")));
        self.show_create_channel = false;
        self.select_channel(name, window, cx);
    }

    // -----------------------------------------------------------------------
    // 频道管理（右键菜单 / 主题 / 删除）
    // -----------------------------------------------------------------------

    /// 当前服务器第一个文字频道名；删除当前频道后用它做回退。
    fn first_text_channel(&self) -> Option<String> {
        self.servers[self.active_server]
            .categories
            .iter()
            .flat_map(|category| category.channels.iter())
            .find(|channel| channel.kind == ChannelKind::Text)
            .map(|channel| channel.name.clone())
    }

    /// 在当前服务器的所有分类里定位频道，返回 (分类下标, 频道下标)。
    fn channel_position(&self, name: &str) -> Option<(usize, usize)> {
        self.servers[self.active_server]
            .categories
            .iter()
            .enumerate()
            .find_map(|(ci, category)| {
                category
                    .channels
                    .iter()
                    .position(|channel| channel.name == name)
                    .map(|chi| (ci, chi))
            })
    }

    /// 频道类型；频道不存在时为 None。
    pub(crate) fn channel_kind(&self, name: &str) -> Option<ChannelKind> {
        let (ci, chi) = self.channel_position(name)?;
        Some(self.servers[self.active_server].categories[ci].channels[chi].kind)
    }

    /// 频道主题，供菜单与编辑弹层展示。
    pub(crate) fn channel_topic_of(&self, name: &str) -> String {
        self.channel_position(name)
            .map(|(ci, chi)| {
                self.servers[self.active_server].categories[ci].channels[chi]
                    .topic
                    .clone()
            })
            .unwrap_or_default()
    }

    /// 频道未读数，决定菜单里「标记为已读」是否高亮可用。
    pub(crate) fn channel_unread_of(&self, name: &str) -> usize {
        self.channel_position(name)
            .map(|(ci, chi)| {
                self.servers[self.active_server].categories[ci].channels[chi].unread
            })
            .unwrap_or(0)
    }

    /// 当前会话的未读数（频道 / 私信通用），用于未读分隔线与跳转浮标。
    pub(crate) fn current_unread_count(&self) -> usize {
        match self.view_mode {
            ViewMode::Server => self.channel_unread_of(&self.active_channel),
            ViewMode::DirectMessage => self
                .active_dm
                .as_ref()
                .and_then(|name| {
                    self.dm_conversations
                        .iter()
                        .find(|c| &c.friend == name)
                        .map(|c| c.unread)
                })
                .unwrap_or(0),
            ViewMode::Friends | ViewMode::Discover => 0,
        }
    }

    /// 未读分隔线插入位置：`Some((消息下标, 未读数))`，未读数超过消息条数时不插。
    pub(crate) fn unread_divider(&self) -> Option<(usize, usize)> {
        let unread = self.current_unread_count();
        let total = self.current_messages().len();
        if unread == 0 || unread >= total {
            return None;
        }
        Some((total - unread, unread))
    }

    /// 跳到最新：滚到底部并把当前会话标记为已读（分隔线与浮标随之消失）。
    pub(crate) fn jump_to_latest(&mut self, cx: &mut Context<Self>) {
        self.scroll_handle.scroll_to_bottom();
        self.mark_current_read();
        cx.notify();
    }

    /// 清除当前会话的未读计数，不依赖右键菜单状态。
    fn mark_current_read(&mut self) {
        match self.view_mode {
            ViewMode::Server => {
                if let Some((ci, chi)) = self.channel_position(&self.active_channel) {
                    self.servers[self.active_server].categories[ci].channels[chi].unread = 0;
                }
            }
            ViewMode::DirectMessage => {
                if let Some(name) = self.active_dm.as_ref() {
                    if let Some(conversation) = self
                        .dm_conversations
                        .iter_mut()
                        .find(|c| &c.friend == name)
                    {
                        conversation.unread = 0;
                    }
                }
            }
            ViewMode::Friends | ViewMode::Discover => {}
        }
    }

    /// 打开频道操作菜单，并收起其它浮层，避免叠层。
    pub(crate) fn open_channel_menu(&mut self, name: String, anchor: MenuAnchor) {
        self.channel_menu = Some(ChannelMenuState {
            name,
            anchor,
            page: ChannelMenuPage::Actions,
        });
        self.category_menu = None;
        self.show_server_menu = false;
        self.profile_target = None;
        self.close_message_menu();
        self.show_pins = false;
        self.show_inbox = false;
        self.show_status_menu = false;
        self.emoji_picker_open = false;
    }

    /// 关闭频道操作菜单。
    pub(crate) fn close_channel_menu(&mut self) {
        self.channel_menu = None;
    }

    /// 频道通知级别；没单独设过就是「跟随服务器」。
    pub(crate) fn channel_notify_of(&self, name: &str) -> NotifyLevel {
        self.channel_position(name)
            .map(|(ci, chi)| self.servers[self.active_server].categories[ci].channels[chi].notify)
            .unwrap_or(NotifyLevel::Inherit)
    }

    /// 频道是否静音（按生效级别判断，含「跟随服务器」继承来的静音）。
    pub(crate) fn channel_muted(&self, name: &str) -> bool {
        self.effective_notify_of(name).is_muted()
    }

    /// 把菜单翻到通知设置页（复用同一个卡片与遮罩）。
    pub(crate) fn open_channel_notify_page(&mut self) {
        if let Some(menu) = self.channel_menu.as_mut() {
            menu.page = ChannelMenuPage::Notify;
        }
    }

    /// 通知设置页返回操作页。
    pub(crate) fn close_channel_notify_page(&mut self) {
        if let Some(menu) = self.channel_menu.as_mut() {
            menu.page = ChannelMenuPage::Actions;
        }
    }

    /// 设置频道通知级别。写入后面板保持打开，方便逐项对比。
    pub(crate) fn set_channel_notify(&mut self, level: NotifyLevel, cx: &mut Context<Self>) {
        let Some(name) = self.channel_menu.as_ref().map(|menu| menu.name.clone()) else {
            return;
        };
        if let Some((ci, chi)) = self.channel_position(&name) {
            self.servers[self.active_server].categories[ci].channels[chi].notify = level;
        }
        cx.notify();
    }

    /// 标题栏铃铛：直接打开当前频道的通知设置页。
    pub(crate) fn open_active_channel_notify(&mut self, cx: &mut Context<Self>) {
        if self.view_mode != ViewMode::Server || self.active_channel.is_empty() {
            return;
        }
        let name = self.active_channel.clone();
        self.open_channel_menu(name, MenuAnchor::Header);
        self.open_channel_notify_page();
        cx.notify();
    }

    /// 把菜单指向的频道标记为已读（清除未读角标）。
    pub(crate) fn mark_channel_read(&mut self, cx: &mut Context<Self>) {
        let Some(name) = self.channel_menu.take().map(|menu| menu.name) else {
            return;
        };
        if let Some((ci, chi)) = self.channel_position(&name) {
            self.servers[self.active_server].categories[ci].channels[chi].unread = 0;
        }
        cx.notify();
    }

    /// 打开频道主题编辑弹层，输入框预填当前主题。
    pub(crate) fn open_topic_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.channel_menu.take().map(|menu| menu.name) else {
            return;
        };
        if self.channel_kind(&name) != Some(ChannelKind::Text) {
            cx.notify();
            return;
        }
        let topic = self.channel_topic_of(&name);
        self.topic_input
            .update(cx, |input, cx| input.set_value(topic, window, cx));
        self.edit_topic_target = name;
        self.show_edit_topic = true;
        cx.notify();
    }

    /// 保存频道主题：写回频道，标题栏立即刷新。
    pub(crate) fn save_channel_topic(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let topic = self.topic_input.read(cx).value().trim().to_string();
        if topic.is_empty() || self.edit_topic_target.is_empty() {
            return;
        }
        let name = self.edit_topic_target.clone();
        if let Some((ci, chi)) = self.channel_position(&name) {
            self.servers[self.active_server].categories[ci].channels[chi].topic = topic;
        }
        self.topic_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.edit_topic_target.clear();
        self.show_edit_topic = false;
        cx.notify();
    }

    /// 关闭主题编辑弹层（取消）。
    pub(crate) fn close_topic_editor(&mut self) {
        self.show_edit_topic = false;
        self.edit_topic_target.clear();
    }

    /// 删除菜单指向的频道：至少保留一个文字频道，删掉当前频道后回退到第一个文字频道。
    pub(crate) fn delete_channel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.channel_menu.take() else {
            return;
        };
        let Some((ci, chi)) = self.channel_position(&menu.name) else {
            return;
        };

        let (kind, text_count) = {
            let server = &self.servers[self.active_server];
            let kind = server.categories[ci].channels[chi].kind;
            let text_count = server
                .categories
                .iter()
                .flat_map(|category| category.channels.iter())
                .filter(|channel| channel.kind == ChannelKind::Text)
                .count();
            (kind, text_count)
        };
        // 服务器至少要留一个文字频道，否则没有可发送消息的地方。
        if kind == ChannelKind::Text && text_count <= 1 {
            cx.notify();
            return;
        }

        // 删掉的正是当前连接的语音频道时先断开。
        let in_voice = self
            .voice_channel
            .as_ref()
            .map(|(index, channel)| *index == self.active_server && channel == &menu.name)
            .unwrap_or(false);
        if in_voice {
            self.leave_voice();
        }

        let server = &mut self.servers[self.active_server];
        server.categories[ci].channels.remove(chi);
        // 空分类一并移除，避免频道栏出现空分组。
        if server.categories[ci].channels.is_empty() {
            server.categories.remove(ci);
        }

        // 该频道的历史消息一起清掉，避免重新创建同名频道时带出旧记录。
        let key = format!("{}/{}", self.servers[self.active_server].name, menu.name);
        self.messages.remove(&key);

        if self.view_mode == ViewMode::Server && self.active_channel == menu.name {
            if let Some(next) = self.first_text_channel() {
                self.active_channel = next;
            }
        }
        self.reset_session(window, cx);
    }

    // -----------------------------------------------------------------------
    // 分类管理（右键菜单 / 重命名 / 标记已读 / 删除）
    // -----------------------------------------------------------------------

    /// 在当前服务器的分类里按名称定位下标。
    fn category_position(&self, name: &str) -> Option<usize> {
        self.servers[self.active_server]
            .categories
            .iter()
            .position(|category| category.name == name)
    }

    /// 分类下的频道数量，供菜单头部展示。
    pub(crate) fn category_channel_count(&self, name: &str) -> usize {
        self.category_position(name)
            .map(|ci| self.servers[self.active_server].categories[ci].channels.len())
            .unwrap_or(0)
    }

    /// 分类内的未读总数，决定「标记为已读」是否可用。
    pub(crate) fn category_unread_count(&self, name: &str) -> usize {
        self.category_position(name)
            .map(|ci| {
                self.servers[self.active_server].categories[ci]
                    .channels
                    .iter()
                    .map(|channel| channel.unread)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// 分类是否可以删除：至少要留下一个包含文字频道的其它分类。
    pub(crate) fn category_deletable(&self, name: &str) -> bool {
        let Some(ci) = self.category_position(name) else {
            return false;
        };
        if self.servers[self.active_server].categories.len() <= 1 {
            return false;
        }
        self.servers[self.active_server]
            .categories
            .iter()
            .enumerate()
            .any(|(index, category)| {
                index != ci
                    && category
                        .channels
                        .iter()
                        .any(|channel| channel.kind == ChannelKind::Text)
            })
    }

    /// 打开分类操作菜单；同时收起其它浮层，避免叠层。
    pub(crate) fn open_category_menu(&mut self, name: String, anchor: MenuAnchor) {
        self.category_menu = Some(CategoryMenuState { name, anchor });
        self.show_server_menu = false;
        self.channel_menu = None;
        self.profile_target = None;
        self.close_message_menu();
        self.show_pins = false;
        self.show_inbox = false;
        self.show_status_menu = false;
        self.emoji_picker_open = false;
    }

    /// 关闭分类操作菜单。
    pub(crate) fn close_category_menu(&mut self) {
        self.category_menu = None;
    }

    /// 把分类内所有频道标记为已读。
    pub(crate) fn mark_category_read(&mut self, cx: &mut Context<Self>) {
        let Some(name) = self.category_menu.take().map(|menu| menu.name) else {
            return;
        };
        if let Some(ci) = self.category_position(&name) {
            for channel in &mut self.servers[self.active_server].categories[ci].channels {
                channel.unread = 0;
            }
        }
        cx.notify();
    }

    /// 在菜单指向的分类里新建频道。
    pub(crate) fn open_category_create_channel(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self.category_menu.take().map(|menu| menu.name) else {
            return;
        };
        self.open_create_channel(window, cx);
        // open_create_channel 会重置目标分类，确认弹层真的打开了再写回。
        if self.show_create_channel {
            self.create_channel_category = Some(name);
        }
    }

    /// 打开新建分类弹层：仅服务器视图可用，输入框清空。
    pub(crate) fn open_create_category(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_mode != ViewMode::Server {
            return;
        }
        self.category_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.show_server_menu = false;
        self.channel_menu = None;
        self.category_menu = None;
        self.show_rename_category = false;
        self.rename_category_source.clear();
        self.show_create_category = true;
        cx.notify();
    }

    /// 创建分类：追加到当前服务器末尾；空名或与现有分类重名不生效。
    pub(crate) fn create_category(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.category_input.read(cx).value().trim().to_string();
        if name.is_empty() || self.view_mode != ViewMode::Server {
            return;
        }
        if self.category_position(&name).is_some() {
            cx.notify();
            return;
        }
        self.servers[self.active_server]
            .categories
            .push(Category::new(name, Vec::new()));
        self.category_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.show_create_category = false;
        cx.notify();
    }

    /// 关闭新建分类弹层（取消）。
    pub(crate) fn close_create_category(&mut self) {
        self.show_create_category = false;
    }

    /// 打开分类重命名弹层，输入框预填当前名称。
    pub(crate) fn open_category_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.category_menu.take().map(|menu| menu.name) else {
            return;
        };
        self.category_input
            .update(cx, |input, cx| input.set_value(name.clone(), window, cx));
        self.show_create_category = false;
        self.rename_category_source = name;
        self.show_rename_category = true;
        cx.notify();
    }

    /// 保存分类名：写回名称并迁移折叠状态；空名或与现有分类重名则不生效。
    pub(crate) fn save_category_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.category_input.read(cx).value().trim().to_string();
        let source = std::mem::take(&mut self.rename_category_source);
        self.show_rename_category = false;
        self.category_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        if value.is_empty() || source.is_empty() || value == source {
            cx.notify();
            return;
        }
        // 同一服务器内不允许分类重名，否则按名称定位会产生歧义。
        if self.category_position(&value).is_some() {
            cx.notify();
            return;
        }
        let Some(ci) = self.category_position(&source) else {
            cx.notify();
            return;
        };
        let server_name = self.servers[self.active_server].name.clone();
        self.servers[self.active_server].categories[ci].name = value.clone();

        // 折叠状态以「服务器/分类」为 key，改名后同步迁移，保持原有展开状态。
        let old_key = format!("{server_name}/{source}");
        let new_key = format!("{server_name}/{value}");
        if let Some(pos) = self.collapsed.iter().position(|key| key == &old_key) {
            self.collapsed[pos] = new_key;
        }
        cx.notify();
    }

    /// 关闭分类重命名弹层（取消）。
    pub(crate) fn close_category_rename(&mut self) {
        self.show_rename_category = false;
        self.rename_category_source.clear();
    }

    /// 删除分类：分类内频道与历史消息一并移除，当前频道被删时回退到第一个文字频道。
    pub(crate) fn delete_category(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.category_menu.take().map(|menu| menu.name) else {
            return;
        };
        let Some(ci) = self.category_position(&name) else {
            return;
        };
        // 至少留下一个包含文字频道的分类，否则没有可发送消息的地方。
        if !self.category_deletable(&name) {
            cx.notify();
            return;
        }

        let server_name = self.servers[self.active_server].name.clone();
        let removed: Vec<String> = self.servers[self.active_server].categories[ci]
            .channels
            .iter()
            .map(|channel| channel.name.clone())
            .collect();

        // 正在连接的语音频道落在该分类里时先断开。
        let in_voice = self
            .voice_channel
            .as_ref()
            .map(|(index, channel)| *index == self.active_server && removed.contains(channel))
            .unwrap_or(false);
        if in_voice {
            self.leave_voice();
        }

        self.servers[self.active_server].categories.remove(ci);
        for channel in &removed {
            self.messages.remove(&format!("{server_name}/{channel}"));
        }
        if self.view_mode == ViewMode::Server && removed.contains(&self.active_channel) {
            self.active_channel = self.first_text_channel().unwrap_or_default();
        }
        // 折叠 key 一并清掉，避免重建同名分类后仍然处于折叠状态。
        let key = format!("{server_name}/{name}");
        self.collapsed.retain(|item| item != &key);
        self.reset_session(window, cx);
    }

    // -----------------------------------------------------------------------
    // 服务器菜单（邀请 / 静音 / 标记已读 / 离开）
    // -----------------------------------------------------------------------

    /// 打开/关闭服务器下拉菜单；打开时收起其它浮层。
    pub(crate) fn toggle_server_menu(&mut self, cx: &mut Context<Self>) {
        self.show_server_menu = !self.show_server_menu;
        // 每次展开都回到操作页，避免上次停在通知页留下错位状态。
        self.server_notify_page = false;
        if self.show_server_menu {
            self.channel_menu = None;
            self.category_menu = None;
            self.profile_target = None;
            self.close_message_menu();
            self.show_pins = false;
            self.show_inbox = false;
            self.show_status_menu = false;
            self.emoji_picker_open = false;
        }
        cx.notify();
    }

    pub(crate) fn close_server_menu(&mut self) {
        self.show_server_menu = false;
        self.server_notify_page = false;
    }

    /// 服务器默认通知级别（没有「继承」这一级，兜底 All）。
    pub(crate) fn server_notify_default(&self) -> NotifyLevel {
        match self.servers[self.active_server].notify {
            NotifyLevel::Inherit => NotifyLevel::All,
            level => level,
        }
    }

    /// 频道实际生效的通知级别：自己设过就用自己，选「跟随服务器」则取服务器默认。
    pub(crate) fn effective_notify_of(&self, name: &str) -> NotifyLevel {
        match self.channel_notify_of(name) {
            NotifyLevel::Inherit => self.server_notify_default(),
            level => level,
        }
    }

    /// 当前服务器里有多少个频道仍是「跟随服务器」，用于通知页脚注。
    pub(crate) fn inherit_channel_count(&self) -> usize {
        self.servers[self.active_server]
            .categories
            .iter()
            .flat_map(|category| category.channels.iter())
            .filter(|channel| channel.notify == NotifyLevel::Inherit)
            .count()
    }

    /// 打开服务器通知设置页（在同一个服务器菜单卡片里翻页）。
    pub(crate) fn open_server_notify_page(&mut self, cx: &mut Context<Self>) {
        self.server_notify_page = true;
        cx.notify();
    }

    /// 通知设置页返回服务器操作页。
    pub(crate) fn close_server_notify_page(&mut self, cx: &mut Context<Self>) {
        self.server_notify_page = false;
        cx.notify();
    }

    /// 设置服务器默认通知级别；选「关闭通知」即等同于静音整个服务器。
    pub(crate) fn set_server_notify(&mut self, level: NotifyLevel, cx: &mut Context<Self>) {
        self.servers[self.active_server].notify = level;
        cx.notify();
    }

    /// 静音 / 取消静音当前服务器：静音写 Nothing，取消静音回到 All。
    pub(crate) fn toggle_server_mute(&mut self, cx: &mut Context<Self>) {
        let current = self.server_notify_default();
        self.servers[self.active_server].notify = if current.is_muted() {
            NotifyLevel::All
        } else {
            NotifyLevel::Nothing
        };
        self.show_server_menu = false;
        self.server_notify_page = false;
        cx.notify();
    }

    /// 把当前服务器所有频道标记为已读。
    pub(crate) fn mark_server_read(&mut self, cx: &mut Context<Self>) {
        for category in &mut self.servers[self.active_server].categories {
            for channel in &mut category.channels {
                channel.unread = 0;
            }
        }
        self.show_server_menu = false;
        cx.notify();
    }

    /// 邀请码：ASCII 名转小写 slug，纯中文名用序号兜底。
    pub(crate) fn invite_code(&self) -> String {
        let slug: String = self.servers[self.active_server]
            .name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if slug.is_empty() {
            format!("gpui-{}", self.active_server + 1)
        } else {
            slug
        }
    }

    /// 打开邀请弹层，链接按当前服务器预填。
    pub(crate) fn open_invite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let link = format!("discord.gg/{}", self.invite_code());
        self.invite_input
            .update(cx, |input, cx| input.set_value(link, window, cx));
        self.show_server_menu = false;
        self.show_server_settings = false;
        self.show_invite = true;
        cx.notify();
    }

    pub(crate) fn close_invite(&mut self) {
        self.show_invite = false;
    }

    /// 复制邀请链接到剪贴板并关闭弹层。
    pub(crate) fn copy_invite(&mut self, cx: &mut Context<Self>) {
        let link = self.invite_input.read(cx).value().to_string();
        if !link.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(link));
        }
        self.show_invite = false;
        cx.notify();
    }

    /// 离开当前服务器：至少保留一个，离开后切到第一个服务器。
    pub(crate) fn leave_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_server_menu = false;
        if self.servers.len() <= 1 {
            cx.notify();
            return;
        }

        let name = self.servers[self.active_server].name.clone();
        let leaving_voice = self
            .voice_channel
            .as_ref()
            .map(|(index, _)| *index == self.active_server)
            .unwrap_or(false);
        self.servers.remove(self.active_server);
        // 该服务器的历史消息一并清掉。
        let prefix = format!("{name}/");
        self.messages.retain(|key, _| !key.starts_with(&prefix));

        if leaving_voice {
            self.leave_voice();
        } else if let Some((index, channel)) = self.voice_channel.clone() {
            // 被删服务器之前的连接位置不受影响，之后的整体前移一位。
            if index > self.active_server {
                self.voice_channel = Some((index - 1, channel));
            }
        }

        self.active_server = self.active_server.min(self.servers.len() - 1);
        self.active_channel = self.first_text_channel().unwrap_or_default();
        self.view_mode = ViewMode::Server;
        self.reset_session(window, cx);
    }

    // -----------------------------------------------------------------------
    // 消息与回应
    // -----------------------------------------------------------------------

    /// 发送当前草稿到当前会话。
    pub(crate) fn send_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().trim().to_string();
        // 允许「只发附件不写字」，与 Discord 行为一致。
        if (text.is_empty() && self.attachment_draft.is_none())
            || self.view_mode == ViewMode::Friends
        {
            return;
        }

        self.composer
            .update(cx, |input, cx| input.set_value("", window, cx));
        // 已发送的文本不再算草稿，清掉侧栏可能的草稿标记。
        let sent_session = self.current_session.clone();
        self.drafts.remove(&sent_session);
        let mut message = Message::new("你", "刚刚", text, palette::accent());
        if let Some(attachment) = self.attachment_draft.take() {
            message = message.attachment(attachment);
        }
        // 把回复引用写进新消息，再清空草稿状态。
        if let Some(reply) = self.reply_to.take() {
            message = message.replying_to(reply.author, reply.excerpt);
        }
        if let Some(messages) = self.current_messages_mut() {
            messages.push(message);
        }
        self.mention_prefix = None;
        self.mention_candidates.clear();
        self.show_attachment_picker = false;
        self.clear_search(window, cx);
        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }

    /// 附件选择弹层显隐：打开时收起表情层与其他浮层，避免叠层。
    // ------------------------------- 贴纸弹层 -------------------------------

    /// 打开/关闭贴纸弹层；传包名则先切到该包（礼物按钮直达「礼物」包）。
    pub(crate) fn toggle_sticker_picker(
        &mut self,
        pack: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        let same = self.show_sticker_picker && pack.is_none_or(|name| name == self.sticker_pack);
        if same {
            self.show_sticker_picker = false;
            self.show_gif_picker = false;
        } else {
            self.show_sticker_picker = true;
            self.show_gif_picker = false;
            if let Some(name) = pack {
                self.sticker_pack = name;
            }
            self.emoji_picker_open = false;
            self.emoji_target = None;
            self.show_attachment_picker = false;
            self.show_status_menu = false;
            self.channel_menu = None;
            self.category_menu = None;
            self.member_menu = None;
            self.show_server_menu = false;
            self.profile_target = None;
            self.close_message_menu();
            self.show_pins = false;
            self.show_inbox = false;
        }
        cx.notify();
    }

    /// 关闭贴纸弹层（同时收起 GIF 弹层）。
    pub(crate) fn close_sticker_picker(&mut self) {
        self.show_sticker_picker = false;
        self.show_gif_picker = false;
    }

    /// 切换贴纸包。
    pub(crate) fn set_sticker_pack(&mut self, pack: &'static str, cx: &mut Context<Self>) {
        self.sticker_pack = pack;
        cx.notify();
    }

    // ------------------------------- GIF 弹层 -------------------------------

    /// 打开 / 关闭 GIF 搜索弹层；打开时收起其他浮层，避免叠层。
    pub(crate) fn toggle_gif_picker(&mut self, cx: &mut Context<Self>) {
        self.show_gif_picker = !self.show_gif_picker;
        if self.show_gif_picker {
            self.show_sticker_picker = false;
            self.emoji_picker_open = false;
            self.emoji_target = None;
            self.show_attachment_picker = false;
            self.show_status_menu = false;
            self.channel_menu = None;
            self.category_menu = None;
            self.member_menu = None;
            self.show_server_menu = false;
            self.profile_target = None;
            self.close_message_menu();
            self.show_pins = false;
            self.show_inbox = false;
        }
        cx.notify();
    }

    /// 关闭 GIF 弹层（保留搜索词，下次打开可继续用）。
    pub(crate) fn close_gif_picker(&mut self) {
        self.show_gif_picker = false;
    }

    /// 切换 GIF 分类：清空搜索词，回到该分类的完整列表。
    pub(crate) fn set_gif_category(
        &mut self,
        category: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.gif_category = category;
        self.gif_query.clear();
        self.gif_search
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    /// 当前可见 GIF：有搜索词时跨分类匹配标题 / 标签 / 分类，否则按当前分类过滤。
    pub(crate) fn visible_gifs(&self) -> Vec<Gif> {
        let query = self.gif_query.trim().to_lowercase();
        self.gifs
            .iter()
            .copied()
            .filter(|gif| {
                if query.is_empty() {
                    gif.category == self.gif_category
                } else {
                    gif.title.to_lowercase().contains(&query)
                        || gif.tags.to_lowercase().contains(&query)
                        || gif.category.to_lowercase().contains(&query)
                }
            })
            .collect()
    }

    /// 搜索框 Enter：发送第一个匹配结果。
    pub(crate) fn send_first_gif(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(gif) = self.visible_gifs().first().copied() {
            self.send_gif(gif, window, cx);
        }
    }

    /// 点击 GIF 即发送：生成一条只有 GIF 的消息并滚到底部。
    pub(crate) fn send_gif(
        &mut self,
        gif: Gif,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.view_mode == ViewMode::Friends {
            return;
        }
        self.show_gif_picker = false;
        self.gif_query.clear();
        self.gif_search
            .update(cx, |input, cx| input.set_value("", window, cx));
        let mut message = Message::new("你", "刚刚", "", palette::accent()).gif(gif);
        if let Some(reply) = self.reply_to.take() {
            message = message.replying_to(reply.author, reply.excerpt);
        }
        if let Some(messages) = self.current_messages_mut() {
            messages.push(message);
        }
        self.mention_prefix = None;
        self.mention_candidates.clear();
        self.scroll_handle.scroll_to_bottom();
        self.composer.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// 点击贴纸即发送：生成一条只有贴纸的消息并滚到底部。
    pub(crate) fn send_sticker(
        &mut self,
        sticker: Sticker,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.view_mode == ViewMode::Friends {
            return;
        }
        self.show_sticker_picker = false;
        self.show_gif_picker = false;
        let mut message = Message::new("你", "刚刚", "", palette::accent()).sticker(sticker);
        if let Some(reply) = self.reply_to.take() {
            message = message.replying_to(reply.author, reply.excerpt);
        }
        if let Some(messages) = self.current_messages_mut() {
            messages.push(message);
        }
        self.mention_prefix = None;
        self.mention_candidates.clear();
        self.scroll_handle.scroll_to_bottom();
        self.composer.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    pub(crate) fn toggle_attachment_picker(&mut self, cx: &mut Context<Self>) {
        self.show_attachment_picker = !self.show_attachment_picker;
        if self.show_attachment_picker {
            self.emoji_picker_open = false;
            self.emoji_target = None;
            self.show_sticker_picker = false;
            self.show_gif_picker = false;
            self.show_status_menu = false;
            self.channel_menu = None;
            self.category_menu = None;
            self.show_server_menu = false;
            self.profile_target = None;
            self.close_message_menu();
            self.show_pins = false;
            self.show_inbox = false;
        }
        cx.notify();
    }

    /// 选中一个模拟文件作为待发送附件：关闭弹层并聚焦输入框。
    pub(crate) fn pick_attachment(
        &mut self,
        attachment: Attachment,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.attachment_draft = Some(attachment);
        self.show_attachment_picker = false;
        self.composer.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// 取消待发送附件。
    pub(crate) fn clear_attachment(&mut self, cx: &mut Context<Self>) {
        self.attachment_draft = None;
        cx.notify();
    }

    /// 打开附件预览弹层；同时收起其他浮层，避免叠层。
    pub(crate) fn open_attachment_preview(
        &mut self,
        attachment: Attachment,
        cx: &mut Context<Self>,
    ) {
        self.attachment_preview = Some(attachment);
        self.show_forward = false;
        self.show_pins = false;
        self.show_inbox = false;
        self.profile_target = None;
        self.show_status_menu = false;
        self.show_server_menu = false;
        self.close_message_menu();
        cx.notify();
    }

    /// 关闭附件预览弹层。
    pub(crate) fn close_attachment_preview(&mut self, cx: &mut Context<Self>) {
        self.attachment_preview = None;
        cx.notify();
    }

    /// 演示「下载」：把文件名复制到剪贴板，弹层关闭。
    pub(crate) fn copy_attachment_name(&mut self, cx: &mut Context<Self>) {
        if let Some(attachment) = &self.attachment_preview {
            cx.write_to_clipboard(ClipboardItem::new_string(attachment.name.clone()));
        }
        self.attachment_preview = None;
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 快捷键与快速切换器
    // -----------------------------------------------------------------------

    /// 所有键盘快捷键的唯一入口，由顶层 Render 注册的全局 key 监听器调用。
    pub(crate) fn handle_shortcut(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;

        if key == "k" && (modifiers.control || modifiers.platform) {
            if self.show_quick_switcher {
                self.close_quick_switcher();
            } else {
                self.open_quick_switcher(window, cx);
            }
            cx.notify();
            return;
        }

        if key == "escape" {
            self.close_top_overlay(window, cx);
            return;
        }

        if self.show_quick_switcher {
            match key {
                "down" => self.move_quick_selection(1, cx),
                "up" => self.move_quick_selection(-1, cx),
                _ => {}
            }
        }
    }

    /// 打开快速切换器：清空搜索、收起所有浮层与整屏页，聚焦搜索框。
    pub(crate) fn open_quick_switcher(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_quick_switcher = true;
        self.quick_query.clear();
        self.quick_selected = 0;
        self.quick_search.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        self.show_forward = false;
        self.attachment_preview = None;
        self.show_pins = false;
        self.show_inbox = false;
        self.show_settings = false;
        self.show_server_settings = false;
        self.show_search_center = false;
        self.show_help_center = false;
        self.show_sticker_picker = false;
        self.show_gif_picker = false;
        self.show_attachment_picker = false;
        self.emoji_picker_open = false;
        self.show_status_menu = false;
        self.show_server_menu = false;
        self.show_create_server = false;
        self.show_create_channel = false;
        self.show_invite = false;
        self.show_edit_topic = false;
        self.show_rename_category = false;
        self.show_create_category = false;
        self.show_voice_room = false;
        self.channel_menu = None;
        self.category_menu = None;
        self.member_menu = None;
        self.profile_target = None;
        self.close_message_menu();
        self.cancel_edit();
        cx.notify();
    }

    /// 关闭快速切换器。
    pub(crate) fn close_quick_switcher(&mut self) {
        self.show_quick_switcher = false;
        self.quick_query.clear();
        self.quick_selected = 0;
    }

    /// 快速切换器候选：与转发弹层共用目标集合，按自己的搜索词过滤。
    pub(crate) fn quick_targets(&self) -> Vec<ForwardTarget> {
        let query = self.quick_query.trim().to_lowercase();
        self.forward_targets()
            .into_iter()
            .filter(|target| {
                query.is_empty()
                    || target.channel.to_lowercase().contains(&query)
                    || target.server.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// 上下键移动高亮项，首尾循环。
    pub(crate) fn move_quick_selection(&mut self, delta: i32, cx: &mut Context<Self>) {
        let len = self.quick_targets().len();
        self.quick_selected = if len == 0 {
            0
        } else {
            (self.quick_selected as i32 + delta).rem_euclid(len as i32) as usize
        };
        cx.notify();
    }

    /// 点击某个候选行：先设为高亮项，再走与回车相同的跳转逻辑。
    pub(crate) fn activate_quick_at(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_selected = index;
        self.activate_quick_selection(window, cx);
    }

    /// 回车跳转到高亮目标：私信走 open_dm，频道直接写 active_* 后重置会话。
    pub(crate) fn activate_quick_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.quick_targets().get(self.quick_selected).cloned();
        self.close_quick_switcher();
        let Some(target) = target else {
            cx.notify();
            return;
        };
        if target.is_dm {
            self.open_dm(target.channel, window, cx);
            return;
        }
        if let Some(index) = self
            .servers
            .iter()
            .position(|server| server.name == target.server)
        {
            self.view_mode = ViewMode::Server;
            self.active_server = index;
            self.active_channel = target.channel;
            self.reset_session(window, cx);
        } else {
            cx.notify();
        }
    }

    /// Esc：按「最上层优先」顺序关闭当前浮层 / 整屏页，不误伤底层状态。
    fn close_top_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_message.is_some() {
            self.cancel_edit();
        } else if self.show_quick_switcher {
            self.close_quick_switcher();
        } else if self.show_help_center {
            // 帮助中心先在详情 → 列表退一层，再整体关闭。
            if self.help_article.is_some() {
                self.close_help_article(cx);
            } else {
                self.close_help_center(cx);
            }
        } else if self.attachment_preview.is_some() {
            self.attachment_preview = None;
        } else if self.show_forward {
            self.close_forward();
        } else if self.show_search_center {
            self.show_search_center = false;
        } else if self.show_thread {
            self.close_thread(); self.close_read_receipts(); self.close_edit_history();
        } else if self.show_server_settings {
            self.close_server_settings(cx);
        } else if self.show_settings {
            self.close_settings();
        } else if self.show_server_menu {
            self.close_server_menu();
        } else if self.show_invite {
            self.show_invite = false;
        } else if self.show_create_server {
            self.show_create_server = false;
        } else if self.show_create_channel {
            self.show_create_channel = false;
        } else if self.show_edit_topic {
            self.close_topic_editor();
        } else if self.show_rename_category || self.show_create_category {
            self.show_rename_category = false;
            self.show_create_category = false;
        } else if self.channel_menu.is_some() {
            self.channel_menu = None;
        } else if self.category_menu.is_some() {
            self.category_menu = None;
        } else if self.member_menu.is_some() {
            self.member_menu = None;
        } else if self.menu_message.is_some() {
            self.close_message_menu();
        } else if self.edit_history.is_some() {
            self.close_edit_history();
        } else if self.read_receipts.is_some() {
            self.close_read_receipts();
        } else if self.show_pins {
            self.show_pins = false;
        } else if self.show_inbox {
            self.show_inbox = false;
        } else if self.show_status_menu {
            self.show_status_menu = false;
        } else if self.show_gif_picker {
            self.close_gif_picker();
        } else if self.show_sticker_picker {
            self.close_sticker_picker();
        } else if self.emoji_picker_open {
            self.emoji_picker_open = false;
            self.emoji_target = None;
        } else if self.show_attachment_picker {
            self.show_attachment_picker = false;
        } else if self.profile_target.is_some() {
            self.profile_target = None;
        } else if self.show_voice_room {
            self.show_voice_room = false;
        } else if self.view_mode == ViewMode::Discover {
            self.close_discover(window, cx);
            return;
        }
        cx.notify();
    }

    /// 切换表情回应（点击已有回应）。
    pub(crate) fn toggle_reaction(
        &mut self,
        index: usize,
        reaction_index: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(messages) = self.current_messages_mut() {
            if let Some(message) = messages.get_mut(index) {
                if let Some(reaction) = message.reactions.get_mut(reaction_index) {
                    reaction.mine = !reaction.mine;
                    reaction.count = if reaction.mine {
                        reaction.count + 1
                    } else {
                        reaction.count.saturating_sub(1)
                    };
                }
            }
        }
        cx.notify();
    }

    /// 打开/关闭表情选择器。target 为 Some 时作用于指定消息，否则作用于输入框。
    pub(crate) fn toggle_emoji_picker(&mut self, target: Option<usize>) {
        if self.emoji_picker_open && self.emoji_target == target {
            self.emoji_picker_open = false;
            self.emoji_target = None;
        } else {
            self.emoji_picker_open = true;
            self.emoji_target = target;
            self.show_attachment_picker = false;
            self.show_sticker_picker = false;
            self.show_gif_picker = false;
        }
    }

    /// 关闭表情选择器。
    pub(crate) fn close_emoji_picker(&mut self) {
        self.emoji_picker_open = false;
        self.emoji_target = None;
    }

    /// 选中表情：加到目标消息或插入输入框草稿。
    pub(crate) fn pick_emoji(
        &mut self,
        emoji: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.emoji_target {
            if let Some(messages) = self.current_messages_mut() {
                if let Some(message) = messages.get_mut(index) {
                    if let Some(reaction) = message.reactions.iter_mut().find(|r| r.emoji == emoji) {
                        if !reaction.mine {
                            reaction.mine = true;
                            reaction.count += 1;
                        }
                    } else {
                        message.reactions.push(Reaction {
                            emoji,
                            count: 1,
                            mine: true,
                        });
                    }
                }
            }
        } else {
            let current = self.composer.read(cx).value().to_string();
            let next = format!("{current}{emoji}");
            self.composer
                .update(cx, |input, cx| input.set_value(&next, window, cx));
        }
        self.close_emoji_picker();
        cx.notify();
    }

    /// 设置/取消对某条消息的回复：摘要取该消息正文前 60 字（贴纸/附件退化为文案）。
    pub(crate) fn toggle_reply_at(&mut self, index: usize, cx: &mut Context<Self>) {
        let target = self.current_messages().get(index).map(|m| {
            let excerpt = if !m.text.trim().is_empty() {
                let text = m.text.trim();
                let short: String = text.chars().take(60).collect();
                if text.chars().count() > 60 { format!("{short}…") } else { short }
            } else if m.sticker.is_some() {
                "[贴纸]".to_string()
            } else if m.attachment.is_some() {
                "[附件]".to_string()
            } else {
                "[空消息]".to_string()
            };
            ReplyRef { author: m.author, excerpt }
        });
        let is_same = target.as_ref().is_some_and(|t| {
            self.reply_to.as_ref().is_some_and(|cur| cur.author == t.author && cur.excerpt == t.excerpt)
        });
        self.reply_to = if is_same { None } else { target };
        cx.notify();
    }

    /// 取消回复（输入条「取消」按钮）。
    pub(crate) fn cancel_reply(&mut self) {
        self.reply_to = None;
    }

    /// 点击消息里的回复引用：按摘要反查原消息，滚动过去并高亮。
    pub(crate) fn jump_to_reply(&mut self, excerpt: &str, cx: &mut Context<Self>) {
        let target = self.current_messages().iter().position(|m| {
            let text = m.text.trim();
            text.starts_with(excerpt.trim_end_matches('…'))
        });
        if let Some(index) = target {
            // 滚动容器的 item 下标要算上顶栏提示与未读分隔线。
            let mut item = index;
            if self.forward_notice.is_some() {
                item += 1;
            }
            if let Some((at, _)) = self.unread_divider() {
                if at <= index {
                    item += 1;
                }
            }
            self.scroll_handle.scroll_to_item(item);
            self.reply_highlight = Some(index);
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 资料卡与消息操作
    // -----------------------------------------------------------------------

    /// 打开某个用户的资料卡；重复点击同一人则关闭。
    pub(crate) fn toggle_profile(&mut self, name: &'static str) {
        if self.profile_target == Some(name) {
            self.profile_target = None;
        } else {
            self.profile_target = Some(name);
            self.close_message_menu();
            self.show_pins = false;
        }
    }

    pub(crate) fn close_profile(&mut self) {
        self.profile_target = None;
    }

    /// 打开/关闭某条消息的操作菜单。
    pub(crate) fn toggle_message_menu(&mut self, index: usize) {
        // 省略号入口：菜单挂在消息行内，不跟随光标。
        if self.menu_message == Some(index) && self.menu_message_point.is_none() {
            self.close_message_menu();
        } else {
            self.menu_message = Some(index);
            self.menu_message_point = None;
            self.show_pins = false;
        }
    }

    /// 右键消息：在光标处展开操作菜单（根级浮层），同时收起其他浮层。
    pub(crate) fn open_message_menu_at(
        &mut self,
        index: usize,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        self.menu_message = Some(index);
        self.menu_message_point = Some((x, y));
        self.show_pins = false;
        self.profile_target = None;
        self.member_menu = None;
        self.channel_menu = None;
        self.category_menu = None;
        self.show_server_menu = false;
        self.show_inbox = false;
        self.show_status_menu = false;
        self.show_attachment_picker = false;
        self.close_sticker_picker();
        self.emoji_picker_open = false;
        self.emoji_target = None;
        cx.notify();
    }

    /// 统一关闭消息菜单：行内菜单与光标浮层一起收掉。
    pub(crate) fn close_message_menu(&mut self) {
        self.menu_message = None;
        self.menu_message_point = None;
    }

    /// 置顶/取消置顶当前会话的第 index 条消息。
    pub(crate) fn toggle_pin(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(message) = self.current_messages_mut().and_then(|m| m.get_mut(index)) {
            message.pinned = !message.pinned;
        }
        self.close_message_menu();
        cx.notify();
    }

    /// 删除当前会话的第 index 条消息；同时迁移该会话里话题的根消息下标。
    pub(crate) fn delete_message(&mut self, index: usize, cx: &mut Context<Self>) {
        let key = self.session_key();
        let removed = self
            .current_messages_mut()
            .is_some_and(|messages| index < messages.len());
        if removed {
            if let Some(messages) = self.current_messages_mut() {
                messages.remove(index);
            }
            if let Some(list) = self.threads.get_mut(&key) {
                list.retain(|thread| thread.root_index != index);
                for thread in list.iter_mut() {
                    if thread.root_index > index {
                        thread.root_index -= 1;
                    }
                }
            }
            match self.thread_root {
                Some(root) if root == index => self.close_thread(),
                Some(root) if root > index => self.thread_root = Some(root - 1),
                _ => {}
            }
            // 回执与编辑历史浮层记录的都是消息下标，删除后同样要迁移或关闭。
            if let Some((at, x, y)) = self.read_receipts {
                if at == index {
                    self.close_read_receipts();
                } else if at > index {
                    self.read_receipts = Some((at - 1, x, y));
                }
            }
            if let Some((at, x, y)) = self.edit_history {
                if at == index {
                    self.close_edit_history();
                } else if at > index {
                    self.edit_history = Some((at - 1, x, y));
                }
            }
        }
        self.close_message_menu();
        cx.notify();
    }

    /// 复制消息文本到系统剪贴板。
    pub(crate) fn copy_message(&self, index: usize, cx: &mut Context<Self>) {
        if let Some(text) = self
            .current_messages()
            .get(index)
            .map(|m| m.text.clone())
        {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 话题（Thread）
    // -----------------------------------------------------------------------

    /// 某条主消息上的话题回复数，没有话题时为 0。
    pub(crate) fn thread_count(&self, index: usize) -> usize {
        self.threads
            .get(&self.session_key())
            .and_then(|list| list.iter().find(|thread| thread.root_index == index))
            .map(|thread| thread.replies.len())
            .unwrap_or(0)
    }

    /// 当前打开话题的克隆数据（面板只读渲染用）。
    pub(crate) fn current_thread(&self) -> Option<ChatThread> {
        if !self.show_thread {
            return None;
        }
        let root = self.thread_root?;
        self.threads
            .get(&self.session_key())?
            .iter()
            .find(|thread| thread.root_index == root)
            .cloned()
    }

    /// 当前话题所依附的原消息（面板顶部展示上下文）。
    pub(crate) fn thread_root_message(&self) -> Option<Message> {
        self.thread_root
            .and_then(|index| self.current_messages().get(index).cloned())
    }

    /// 打开或查看某条消息的话题；同消息再次点击即关闭。已有话题不会重建。
    pub(crate) fn toggle_thread(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index >= self.current_messages().len() {
            return;
        }
        if self.show_thread && self.thread_root == Some(index) {
            self.close_thread(); self.close_read_receipts(); self.close_edit_history();
            cx.notify();
            return;
        }

        let key = self.session_key();
        let title = self
            .current_messages()
            .get(index)
            .map(|message| {
                let text = message.text.trim();
                if text.is_empty() {
                    "新话题".to_string()
                } else {
                    let short: String = text.chars().take(16).collect();
                    if text.chars().count() > 16 {
                        format!("{short}…")
                    } else {
                        short
                    }
                }
            })
            .unwrap_or_else(|| "新话题".to_string());
        let list = self.threads.entry(key).or_default();
        if !list.iter().any(|thread| thread.root_index == index) {
            list.push(ChatThread::new(index, title, Vec::new()));
        }

        self.show_thread = true;
        self.thread_root = Some(index);
        self.show_pins = false;
        self.show_inbox = false;
        self.profile_target = None;
        self.close_message_menu();
        self.thread_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    /// 关闭话题面板（不清空已保存的话题内容）。
    pub(crate) fn close_thread(&mut self) {
        self.show_thread = false;
        self.thread_root = None;
    }

    // -----------------------------------------------------------------------
    // 已读回执详情
    // -----------------------------------------------------------------------

    /// 打开某条消息的已读详情；坐标来自消息状态行的鼠标按下事件。
    pub(crate) fn open_read_receipts(
        &mut self,
        index: usize,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        self.read_receipts = Some((index, x, y));
        self.close_message_menu();
        self.close_edit_history();
        self.profile_target = None;
        cx.notify();
    }

    /// 关闭已读详情浮层（不改消息数据）。
    pub(crate) fn close_read_receipts(&mut self) {
        self.read_receipts = None;
    }

    /// 打开编辑历史浮层（光标锚点），与回执 / 消息菜单互斥。
    pub(crate) fn open_edit_history(
        &mut self,
        index: usize,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        if !self
            .current_messages()
            .get(index)
            .is_some_and(|message| message.is_edited())
        {
            return;
        }
        self.close_message_menu();
        self.close_read_receipts();
        self.edit_history = Some((index, x, y));
        cx.notify();
    }

    /// 从消息菜单打开编辑历史：右键用光标位置，行内菜单用默认位置。
    pub(crate) fn open_edit_history_from_menu(&mut self, index: usize, cx: &mut Context<Self>) {
        let (x, y) = self.menu_message_point.unwrap_or((620., 360.));
        self.close_message_menu();
        self.open_edit_history(index, x, y, cx);
    }

    /// 关闭编辑历史浮层。
    pub(crate) fn close_edit_history(&mut self) {
        self.edit_history = None;
    }

    /// 当前编辑历史：返回消息下标与历史版本列表（旧 → 新）。
    pub(crate) fn edit_history_detail(&self) -> Option<(usize, Vec<EditRevision>)> {
        let (index, _, _) = self.edit_history?;
        let revisions = self.current_messages().get(index)?.edit_history.clone();
        Some((index, revisions))
    }

    /// 当前回执详情：返回消息下标与读者列表，读者为空时不展示浮层。
    pub(crate) fn read_receipts_detail(&self) -> Option<(usize, Vec<ReadReceipt>)> {
        let (index, _, _) = self.read_receipts?;
        let message = self.current_messages().get(index).cloned()?;
        if message.readers.is_empty() {
            return None;
        }
        Some((index, message.readers))
    }

    /// 在当前话题里发送一条回复（Enter 触发，纯前端内存数据）。
    pub(crate) fn send_thread_reply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.thread_input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        let Some(root) = self.thread_root else {
            return;
        };
        let key = self.session_key();
        let Some(thread) = self
            .threads
            .get_mut(&key)
            .and_then(|list| list.iter_mut().find(|thread| thread.root_index == root))
        else {
            return;
        };
        thread
            .replies
            .push(ThreadReply::new("你", "刚刚", text, palette::accent()));
        self.thread_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 消息转发
    // -----------------------------------------------------------------------

    /// 打开转发弹层：记住源消息下标、清空搜索并聚焦搜索框。
    pub(crate) fn open_forward(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.current_messages().get(index).is_none() {
            return;
        }
        self.forward_source = Some(index);
        self.show_forward = true;
        self.forward_query.clear();
        self.forward_search.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        self.close_message_menu();
        self.show_pins = false;
        self.show_inbox = false;
        self.profile_target = None;
        self.emoji_picker_open = false;
        self.emoji_target = None;
        self.close_sticker_picker();
        self.show_attachment_picker = false;
        cx.notify();
    }

    /// 关闭转发弹层（Esc、遮罩、取消共用）。
    pub(crate) fn close_forward(&mut self) {
        self.show_forward = false;
        self.forward_source = None;
        self.forward_query.clear();
    }

    /// 转发弹层里展示的来源消息摘要：(作者, 摘要)。
    pub(crate) fn forward_summary(&self) -> Option<(&'static str, String)> {
        let index = self.forward_source?;
        let message = self.current_messages().get(index)?.clone();
        let excerpt = if !message.text.is_empty() {
            message.text.chars().take(80).collect::<String>()
        } else if let Some(sticker) = message.sticker {
            format!("[贴纸] {}", sticker.label)
        } else if let Some(gif) = message.gif {
            format!("[GIF] {}", gif.title)
        } else if let Some(attachment) = &message.attachment {
            format!("[附件] {}", attachment.name)
        } else {
            "（空消息）".to_string()
        };
        Some((message.author, excerpt))
    }

    /// 当前会话的展示标签：「#频道」或「@好友」，用于弹层副标题。
    pub(crate) fn current_origin_label(&self) -> String {
        if self.is_dm() {
            format!("@{}", self.active_dm.as_deref().unwrap_or(""))
        } else {
            format!("#{}", self.active_channel)
        }
    }

    /// 全部可转发目标：所有服务器的文字频道 + 现有私信会话。
    pub(crate) fn forward_targets(&self) -> Vec<ForwardTarget> {
        let mut targets = Vec::new();
        for server in &self.servers {
            for category in &server.categories {
                for channel in &category.channels {
                    if channel.kind == ChannelKind::Text {
                        targets.push(ForwardTarget::channel(
                            server.name.clone(),
                            channel.name.clone(),
                            server.color,
                        ));
                    }
                }
            }
        }
        for conversation in &self.dm_conversations {
            targets.push(ForwardTarget::dm(conversation.friend, conversation.color));
        }
        targets
    }

    /// 按搜索词过滤后的转发目标（匹配频道名/好友名与服务器名）。
    pub(crate) fn filtered_forward_targets(&self) -> Vec<ForwardTarget> {
        let query = self.forward_query.trim().to_lowercase();
        self.forward_targets()
            .into_iter()
            .filter(|target| {
                query.is_empty()
                    || target.channel.to_lowercase().contains(&query)
                    || target.server.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// 把源消息转发到目标会话：保留作者/正文/附件/贴纸，附上来源标记。
    pub(crate) fn forward_message(&mut self, target: ForwardTarget, cx: &mut Context<Self>) {
        let Some(index) = self.forward_source else {
            return;
        };
        let Some(source) = self.current_messages().get(index).cloned() else {
            self.close_forward();
            return;
        };

        let origin = if self.is_dm() {
            ForwardOrigin::dm(self.active_dm.clone().unwrap_or_default())
        } else {
            ForwardOrigin::channel(
                self.active_server_ref().name.clone(),
                self.active_channel.clone(),
            )
        };

        let mut message =
            Message::new(source.author, "刚刚", source.text.clone(), palette::accent())
                .forwarded(origin);
        message.color = source.color;
        message.attachment = source.attachment.clone();
        message.sticker = source.sticker;
        message.gif = source.gif;
        message.reply_to = source.reply_to;

        let same_session = match self.view_mode {
            ViewMode::DirectMessage => {
                target.is_dm && self.active_dm.as_deref() == Some(target.channel.as_str())
            }
            ViewMode::Server => {
                !target.is_dm
                    && self.active_server_ref().name == target.server
                    && self.active_channel == target.channel
            }
            ViewMode::Friends | ViewMode::Discover => false,
        };
        let label = target.label();

        if target.is_dm {
            self.dm_messages
                .entry(target.channel.clone())
                .or_default()
                .push(message);
            if let Some(conversation) = self
                .dm_conversations
                .iter_mut()
                .find(|c| c.friend == target.channel)
            {
                conversation.preview = "转发了一条消息";
            }
        } else {
            let key = format!("{}/{}", target.server, target.channel);
            self.messages.entry(key).or_default().push(message);
        }

        self.forward_notice = Some(format!("已转发到 {label}"));
        self.close_forward();
        if same_session {
            self.scroll_handle.scroll_to_bottom();
        }
        cx.notify();
    }

    /// 点击转发消息的来源标签：跳回原频道/私信。
    pub(crate) fn jump_to_origin(
        &mut self,
        origin: ForwardOrigin,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.forward_notice = None;
        if origin.is_dm {
            self.open_dm(origin.channel, window, cx);
            return;
        }
        let index = origin
            .server
            .as_deref()
            .and_then(|name| self.servers.iter().position(|s| s.name == name));
        if let Some(index) = index {
            self.view_mode = ViewMode::Server;
            self.active_server = index;
            self.active_channel = origin.channel;
            self.reset_session(window, cx);
        }
    }

    /// 关闭转发成功提示条。
    pub(crate) fn dismiss_forward_notice(&mut self, cx: &mut Context<Self>) {
        self.forward_notice = None;
        cx.notify();
    }

    /// 打开/关闭置顶消息面板。
    pub(crate) fn toggle_pins(&mut self) {
        self.show_pins = !self.show_pins;
        if self.show_pins {
            self.show_inbox = false;
            self.close_thread(); self.close_read_receipts(); self.close_edit_history();
        }
        self.profile_target = None;
        self.close_message_menu();
    }

    /// 当前会话中所有置顶消息。
    pub(crate) fn pinned_messages(&self) -> Vec<Message> {
        self.current_messages()
            .into_iter()
            .filter(|m| m.pinned)
            .collect()
    }

    /// 取消第 nth 个置顶消息（面板里展示的序号）。
    pub(crate) fn unpin_at(&mut self, nth: usize, cx: &mut Context<Self>) {
        if let Some(messages) = self.current_messages_mut() {
            let mut seen = 0;
            for message in messages.iter_mut() {
                if message.pinned {
                    if seen == nth {
                        message.pinned = false;
                        break;
                    }
                    seen += 1;
                }
            }
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 在线状态
    // -----------------------------------------------------------------------

    /// 打开/关闭本人在线状态菜单。
    pub(crate) fn toggle_status_menu(&mut self) {
        self.show_status_menu = !self.show_status_menu;
    }

    /// 切换本人在线状态。
    pub(crate) fn set_my_status(&mut self, status: UserStatus) {
        self.my_status = status;
        self.show_status_menu = false;
    }

    // -----------------------------------------------------------------------
    // 消息编辑
    // -----------------------------------------------------------------------

    /// 进入编辑：把消息原文载入编辑器并聚焦。
    pub(crate) fn start_edit(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self
            .current_messages()
            .get(index)
            .map(|m| m.text.clone())
            .unwrap_or_default();
        self.editor.update(cx, |input, cx| {
            input.set_value(&text, window, cx);
            input.focus(window, cx);
        });
        self.editing_message = Some(index);
        self.close_message_menu();
        cx.notify();
    }

    /// 保存编辑：用编辑器内容覆盖消息原文。
    pub(crate) fn commit_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = match self.editing_message {
            Some(index) => index,
            None => return,
        };
        let text = self.editor.read(cx).value().trim().to_string();
        if !text.is_empty() {
            if let Some(message) = self.current_messages_mut().and_then(|m| m.get_mut(index)) {
                if message.text != text {
                    // 编辑前先留存旧正文，供消息旁的「已编辑」历史浮层查看。
                    let previous = message.text.clone();
                    message.edit_history.push(EditRevision {
                        text: previous,
                        time: "刚刚",
                    });
                    message.text = text;
                }
            }
        }
        self.cancel_edit();
        self.clear_search(window, cx);
        cx.notify();
    }

    /// 取消编辑并清空编辑器。
    pub(crate) fn cancel_edit(&mut self) {
        self.editing_message = None;
    }

    /// 判断某条消息是否为本人发出（可编辑）。
    pub(crate) fn is_mine(&self, index: usize) -> bool {
        self.current_messages()
            .get(index)
            .map(|m| m.author == "你")
            .unwrap_or(false)
    }

    // -----------------------------------------------------------------------
    // 好友页：标签、请求与屏蔽
    // -----------------------------------------------------------------------

    /// 切换好友页标签。
    pub(crate) fn set_friend_tab(&mut self, tab: FriendTab) {
        self.friend_tab = tab;
    }

    /// 收到的请求数量，用于标签角标。
    pub(crate) fn incoming_request_count(&self) -> usize {
        self.friend_requests.iter().filter(|r| r.incoming).count()
    }

    /// 发送好友请求：用户名#标签，校验格式与重复。
    pub(crate) fn send_friend_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.add_friend_input.read(cx).value().trim().to_string();
        if raw.is_empty() {
            return;
        }
        let name = raw.split('#').next().unwrap_or(&raw).trim().to_string();
        if name.is_empty() {
            return;
        }
        let duplicate = self.friends.iter().any(|f| f.name == name)
            || self
                .friend_requests
                .iter()
                .any(|r| r.name == raw || r.name == name);
        if duplicate {
            return;
        }
        self.friend_requests.push(FriendRequest {
            name: raw,
            color: palette::accent(),
            incoming: false,
        });
        self.add_friend_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.friend_tab = FriendTab::Pending;
        cx.notify();
    }

    /// 接受收到的请求：从请求列表移除并加入好友。
    pub(crate) fn accept_request(&mut self, index: usize, cx: &mut Context<Self>) {
        let request = self.friend_requests.get(index).cloned();
        if let Some(request) = request {
            if request.incoming {
                self.friend_requests.remove(index);
                // Demo 为纯前端静态数据模型，用泄漏字符串把动态名转为 &'static str。
                let name: &'static str = Box::leak(request.name.into_boxed_str());
                self.friends.push(Friend {
                    name,
                    color: request.color,
                    online: false,
                    activity: "离线",
                });
            }
        }
        cx.notify();
    }

    /// 拒绝请求：从列表移除。
    pub(crate) fn reject_request(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.friend_requests.len() {
            self.friend_requests.remove(index);
        }
        cx.notify();
    }

    /// 屏蔽好友：从好友列表移除并记入屏蔽名单。
    pub(crate) fn block_friend(&mut self, name: &'static str) {
        if let Some(pos) = self.friends.iter().position(|f| f.name == name) {
            self.friends.remove(pos);
            if !self.blocked_users.iter().any(|n| n == name) {
                self.blocked_users.push(name.to_string());
            }
        }
    }

    /// 取消屏蔽：从屏蔽名单移除。
    pub(crate) fn unblock_friend(&mut self, name: String) {
        if let Some(pos) = self.blocked_users.iter().position(|n| n == &name) {
            self.blocked_users.remove(pos);
        }
    }

    // -----------------------------------------------------------------------
    // 语音频道连接
    // -----------------------------------------------------------------------

    /// 加入/离开语音频道；点击已连接的频道则断开。
    pub(crate) fn toggle_voice(&mut self, server_index: usize, channel: String) {
        let same = self
            .voice_channel
            .as_ref()
            .map(|(s, c)| *s == server_index && c == &channel)
            .unwrap_or(false);
        if same {
            self.voice_channel = None;
            self.voice_participants.clear();
            self.show_voice_room = false;
        } else {
            self.dm_call = None;
            self.voice_channel = Some((server_index, channel));
            self.voice_participants = self.seed_voice_participants();
        }
    }

    pub(crate) fn leave_voice(&mut self) {
        self.voice_channel = None;
        self.voice_participants.clear();
        self.show_voice_room = false;
    }

    /// 从在线成员里抽三位作为语音房间的其他参与者。
    fn seed_voice_participants(&self) -> Vec<VoiceParticipant> {
        self.members
            .iter()
            .filter(|member| member.online && base_name(member.name) != "你")
            .take(3)
            .map(|member| VoiceParticipant {
                name: base_name(member.name),
                color: member.color,
                muted: false,
            })
            .collect()
    }

    /// 语音房间面板显隐：仅连接语音时可用。
    pub(crate) fn toggle_voice_room(&mut self, cx: &mut Context<Self>) {
        if self.voice_channel.is_none() {
            return;
        }
        self.show_voice_room = !self.show_voice_room;
        cx.notify();
    }

    /// 关闭语音房间面板。
    pub(crate) fn close_voice_room(&mut self, cx: &mut Context<Self>) {
        self.show_voice_room = false;
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 私信语音 / 视频通话
    // -----------------------------------------------------------------------

    /// 发起私信通话；与服务器语音频道互斥（同一时间只能在一个通话里）。
    pub(crate) fn start_dm_call(&mut self, friend: String, mode: CallMode, cx: &mut Context<Self>) {
        let base = friend.trim();
        if base.is_empty() {
            return;
        }
        let base = base.to_string();
        let (color, online) = self
            .friends
            .iter()
            .find(|f| f.name == base)
            .map(|f| (f.color, f.online))
            .or_else(|| {
                self.members
                    .iter()
                    .find(|m| base_name(m.name) == base)
                    .map(|m| (m.color, m.online))
            })
            .unwrap_or((palette::accent(), false));

        self.leave_voice();
        self.dm_call = Some(DmCall {
            friend: base,
            color,
            online,
            mode,
            connected: false,
            mic_on: self.mic_on,
            camera_on: mode == CallMode::Video,
            screen_sharing: false,
        });
        self.close_profile();
        self.close_quick_switcher();
        self.show_inbox = false;
        self.show_pins = false;
        self.close_message_menu();
        cx.notify();
    }

    /// Demo：把「正在呼叫」推进为「已接通」。
    pub(crate) fn connect_dm_call(&mut self, cx: &mut Context<Self>) {
        if let Some(call) = self.dm_call.as_mut() {
            call.connected = true;
        }
        cx.notify();
    }

    /// 挂断私信通话（面板由 `dm_call.is_some()` 决定是否渲染）。
    pub(crate) fn end_dm_call(&mut self, cx: &mut Context<Self>) {
        self.dm_call = None;
        cx.notify();
    }

    /// 通话内麦克风开关，与底部用户栏的 `mic_on` 互不覆盖。
    pub(crate) fn toggle_dm_call_mic(&mut self, cx: &mut Context<Self>) {
        if let Some(call) = self.dm_call.as_mut() {
            call.mic_on = !call.mic_on;
        }
        cx.notify();
    }

    /// 通话内摄像头开关；关摄像头时同时停止屏幕分享，避免两个画面同时开着。
    pub(crate) fn toggle_dm_call_camera(&mut self, cx: &mut Context<Self>) {
        if let Some(call) = self.dm_call.as_mut() {
            call.camera_on = !call.camera_on;
            if !call.camera_on {
                call.screen_sharing = false;
            }
        }
        cx.notify();
    }

    /// 通话内屏幕分享开关；开分享时自动开摄像头，让预览区有画面。
    pub(crate) fn toggle_dm_call_screen(&mut self, cx: &mut Context<Self>) {
        if let Some(call) = self.dm_call.as_mut() {
            call.screen_sharing = !call.screen_sharing;
            if call.screen_sharing {
                call.camera_on = true;
            }
        }
        cx.notify();
    }

    /// 把某位参与者移出语音房间（Demo：仅从列表移除）。
    pub(crate) fn remove_voice_participant(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.voice_participants.len() {
            self.voice_participants.remove(index);
        }
        cx.notify();
    }

    /// 切换某位参与者的服务器静音状态。
    pub(crate) fn toggle_participant_mute(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(participant) = self.voice_participants.get_mut(index) {
            participant.muted = !participant.muted;
        }
        cx.notify();
    }

    pub(crate) fn voice_label(&self) -> Option<(String, String)> {
        self.voice_channel
            .as_ref()
            .map(|(index, channel)| (self.servers[*index].name.clone(), channel.clone()))
    }

    // -----------------------------------------------------------------------
    // 用户设置
    // -----------------------------------------------------------------------

    // ---------------------------- 成员上下文菜单 ----------------------------

    /// 打开成员上下文菜单（右键成员行触发），同时收起其他浮层。
    pub(crate) fn open_member_menu(
        &mut self,
        name: &'static str,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        self.member_menu = Some(MemberMenuState {
            name,
            anchor: MenuAnchor::Point(x, y),
        });
        self.profile_target = None;
        self.close_message_menu();
        self.channel_menu = None;
        self.category_menu = None;
        self.show_pins = false;
        self.show_inbox = false;
        self.show_server_menu = false;
        self.show_status_menu = false;
        self.emoji_picker_open = false;
        self.emoji_target = None;
        cx.notify();
    }

    /// 关闭成员上下文菜单。
    pub(crate) fn close_member_menu(&mut self, cx: &mut Context<Self>) {
        self.member_menu = None;
        cx.notify();
    }

    /// 菜单目标成员的角色名（取「 —— 」后的部分），视图只读这一份解析结果。
    pub(crate) fn member_menu_role(&self) -> Option<&'static str> {
        let name = self.member_menu.as_ref()?.name;
        name.split(" —— ").nth(1)
    }

    /// 是否已被屏蔽（屏蔽名单按基础名存，成员展示名要先去后缀）。
    pub(crate) fn member_menu_blocked(&self) -> bool {
        let Some(menu) = self.member_menu.as_ref() else {
            return false;
        };
        let short = base_name(menu.name);
        self.blocked_users.iter().any(|name| name == short)
    }

    /// 从成员菜单发起私信：复用 start_dm（自动补建会话条目）。
    pub(crate) fn member_send_dm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.member_menu.clone() else {
            return;
        };
        self.member_menu = None;
        self.start_dm(base_name(menu.name), window, cx);
    }

    /// 从成员菜单查看资料卡。
    pub(crate) fn member_open_profile(&mut self, cx: &mut Context<Self>) {
        let Some(menu) = self.member_menu.clone() else {
            return;
        };
        self.member_menu = None;
        self.toggle_profile(menu.name);
        cx.notify();
    }

    /// 从成员菜单提及：把「@基础名 」追加到输入框并聚焦。
    pub(crate) fn member_mention(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.member_menu.clone() else {
            return;
        };
        let short = base_name(menu.name);
        let current = self.composer.read(cx).value().to_string();
        let next = if current.trim().is_empty() {
            format!("@{short} ")
        } else {
            format!("{current}@{short} ")
        };
        self.composer.update(cx, |input, cx| {
            input.set_value(&next, window, cx);
            input.focus(window, cx);
        });
        self.member_menu = None;
        cx.notify();
    }

    /// 复制成员用户名到剪贴板。
    pub(crate) fn member_copy_name(&mut self, cx: &mut Context<Self>) {
        let Some(menu) = self.member_menu.clone() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(format!(
            "@{}",
            base_name(menu.name)
        )));
        self.member_menu = None;
        cx.notify();
    }

    /// 切换成员屏蔽状态（只动屏蔽名单，不改好友列表——成员未必是好友）。
    pub(crate) fn member_toggle_block(&mut self, cx: &mut Context<Self>) {
        let Some(menu) = self.member_menu.clone() else {
            return;
        };
        let short = base_name(menu.name);
        if let Some(position) = self.blocked_users.iter().position(|name| name == short) {
            self.blocked_users.remove(position);
        } else {
            self.blocked_users.push(short.to_string());
        }
        self.member_menu = None;
        cx.notify();
    }

    // ----------------------------------------------------------------------

    // ------------------------------- 搜索中心 -------------------------------

    /// 打开搜索中心：继承顶栏搜索框的关键词，按当前视图选默认范围，并收起浮层。
    pub(crate) fn open_search_center(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let seed = self.search_query.trim().to_string();
        self.search_center_input
            .update(cx, |input, cx| input.set_value(seed.clone(), window, cx));
        self.search_center_query = seed;
        // 私信视图默认在私信里搜，其余默认搜当前服务器。
        self.search_scope = if self.view_mode == ViewMode::DirectMessage {
            SearchScope::Dm
        } else {
            SearchScope::CurrentServer
        };

        self.show_search_center = true;
        self.show_pins = false;
        self.show_inbox = false;
        self.close_thread(); self.close_read_receipts(); self.close_edit_history();
        self.show_status_menu = false;
        self.show_attachment_picker = false;
        self.close_sticker_picker();
        self.emoji_picker_open = false;
        self.emoji_target = None;
        self.profile_target = None;
        self.close_message_menu();
        self.channel_menu = None;
        self.category_menu = None;
        self.member_menu = None;
        self.show_server_menu = false;
        self.show_voice_room = false;
        self.run_search();
        cx.notify();
    }

    /// 关闭搜索中心，回到聊天区（关键词保留，便于再次打开）。
    pub(crate) fn close_search_center(&mut self, cx: &mut Context<Self>) {
        self.show_search_center = false;
        cx.notify();
    }

    /// 打开帮助中心：清空搜索、回到「全部」分类列表。
    pub(crate) fn open_help_center(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.help_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.help_query.clear();
        self.help_category = None;
        self.help_article = None;
        self.show_help_center = true;
        self.show_quick_switcher = false;
        self.show_pins = false;
        self.show_inbox = false;
        self.show_status_menu = false;
        self.profile_target = None;
        self.close_message_menu();
        cx.notify();
    }

    /// 关闭帮助中心（详情页一并复位）。
    pub(crate) fn close_help_center(&mut self, cx: &mut Context<Self>) {
        self.show_help_center = false;
        self.help_article = None;
        cx.notify();
    }

    /// 标题栏 ? 按钮：开着就关，关着就开。
    pub(crate) fn toggle_help_center(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_help_center {
            self.close_help_center(cx);
        } else {
            self.open_help_center(window, cx);
        }
    }

    /// 切换帮助分类（None = 全部），并回到列表。
    pub(crate) fn set_help_category(
        &mut self,
        category: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        self.help_category = category;
        self.help_article = None;
        cx.notify();
    }

    /// 打开某篇帮助文档。
    pub(crate) fn open_help_article(&mut self, id: &'static str, cx: &mut Context<Self>) {
        self.help_article = Some(id);
        cx.notify();
    }

    /// 从文档详情退回列表。
    pub(crate) fn close_help_article(&mut self, cx: &mut Context<Self>) {
        self.help_article = None;
        cx.notify();
    }

    /// 当前分类 + 关键词过滤后的帮助文档。
    pub(crate) fn help_matches(&self) -> Vec<HelpArticle> {
        let query = self.help_query.trim().to_lowercase();
        self.help_articles
            .iter()
            .copied()
            .filter(|article| self.help_category.is_none_or(|c| article.category == c))
            .filter(|article| article.matches(&query))
            .collect()
    }

    /// 按 id 取一篇帮助文档（详情页渲染用）。
    pub(crate) fn help_article_ref(&self, id: &str) -> Option<HelpArticle> {
        self.help_articles.iter().copied().find(|a| a.id == id)
    }

    /// 某分类下的文档数量（分类导航右侧计数，None 为全部）。
    pub(crate) fn help_category_count(&self, category: Option<&'static str>) -> usize {
        self.help_articles
            .iter()
            .filter(|a| category.is_none_or(|c| a.category == c))
            .count()
    }

    /// 从搜索词里移除一个高级过滤 token（搜索栏「已应用」标签点击）。
    pub(crate) fn remove_search_filter(
        &mut self,
        token: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let lowered = token.to_lowercase();
        let next = self
            .search_center_query
            .split_whitespace()
            .filter(|part| part.to_lowercase() != lowered)
            .collect::<Vec<_>>()
            .join(" ");
        self.search_center_query = next.clone();
        self.search_center_input
            .update(cx, |input, cx| input.set_value(&next, window, cx));
        self.run_search();
        cx.notify();
    }

    /// 切换搜索范围并立即重跑。
    pub(crate) fn set_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        if self.search_scope == scope {
            return;
        }
        self.search_scope = scope;
        self.run_search();
        cx.notify();
    }

    /// 按当前关键词、高级过滤条件与范围重算结果；无任何约束时结果直接清空。
    fn run_search(&mut self) {
        let filters = SearchFilters::parse(&self.search_center_query);
        self.search_filters = filters.clone();
        self.search_results.clear();
        if filters.is_empty() {
            return;
        }

        let want_servers = matches!(
            self.search_scope,
            SearchScope::CurrentServer | SearchScope::AllServers | SearchScope::Everything
        );
        let want_dm = matches!(self.search_scope, SearchScope::Dm | SearchScope::Everything);
        let only_current = self.search_scope == SearchScope::CurrentServer;
        let mut hits: Vec<SearchHit> = Vec::new();

        if want_servers {
            for (index, server) in self.servers.iter().enumerate() {
                if only_current && index != self.active_server {
                    continue;
                }
                for category in &server.categories {
                    for channel in &category.channels {
                        // 语音频道不承载消息，跳过。
                        if channel.kind != ChannelKind::Text {
                            continue;
                        }
                        let key = format!("{}/{}", server.name, channel.name);
                        let Some(messages) = self.messages.get(&key) else {
                            continue;
                        };
                        for message in messages {
                            if !filters.matches(message) {
                                continue;
                            }
                            hits.push(SearchHit {
                                server: server.name.clone(),
                                channel: channel.name.clone(),
                                is_dm: false,
                                author: message.author,
                                time: message.time,
                                text: message.text.clone(),
                                color: message.color,
                            });
                        }
                    }
                }
            }
        }

        if want_dm {
            for conversation in &self.dm_conversations {
                let Some(messages) = self.dm_messages.get(conversation.friend) else {
                    continue;
                };
                for message in messages {
                    if !filters.matches(message) {
                        continue;
                    }
                    hits.push(SearchHit {
                        server: "私信".into(),
                        channel: conversation.friend.to_string(),
                        is_dm: true,
                        author: message.author,
                        time: message.time,
                        text: message.text.clone(),
                        color: message.color,
                    });
                }
            }
        }

        hits.truncate(60);
        self.search_results = hits;
    }

    /// 跳到某条结果：私信走 open_dm，频道先切服务器再切频道；搜索中心随之关闭。
    pub(crate) fn open_search_hit(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(hit) = self.search_results.get(index).cloned() else {
            return;
        };
        self.show_search_center = false;
        if hit.is_dm {
            self.open_dm(hit.channel, window, cx);
        } else if let Some(server_index) = self
            .servers
            .iter()
            .position(|server| server.name == hit.server)
        {
            self.active_server = server_index;
            self.select_channel(hit.channel, window, cx);
        }
        cx.notify();
    }

    // ----------------------------------------------------------------------

    // ------------------------------- 探索页 -------------------------------

    /// 打开探索页：切换到独立整屏页面，保留当前频道选择以便返回。
    pub(crate) fn open_discover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_mode = ViewMode::Discover;
        self.show_voice_room = false;
        self.show_search_center = false;
        self.show_help_center = false;
        self.close_thread(); self.close_read_receipts(); self.close_edit_history();
        self.member_menu = None;
        self.discover_category = None;
        self.discover_filter.clear();
        self.discover_search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.profile_target = None;
        self.close_message_menu();
        self.show_pins = false;
        self.show_inbox = false;
        self.show_status_menu = false;
        self.show_attachment_picker = false;
        self.close_sticker_picker();
        self.emoji_picker_open = false;
        self.emoji_target = None;
        self.channel_menu = None;
        self.category_menu = None;
        self.show_server_menu = false;
        cx.notify();
    }

    /// 关闭探索页，回到频道视图。
    pub(crate) fn close_discover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_mode = ViewMode::Server;
        self.reset_session(window, cx);
    }

    /// 切换探索页分类筛选；None 表示「全部」。
    pub(crate) fn set_discover_category(
        &mut self,
        category: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        self.discover_category = category;
        cx.notify();
    }

    /// 加入探索服务器：已加入则直接切换，否则按卡片信息建一个服务器再进入。
    pub(crate) fn join_discover_server(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.discover_servers.get(index).cloned() else {
            return;
        };

        if let Some(existing) = self.servers.iter().position(|server| server.name == entry.name) {
            self.select_server(existing, window, cx);
            return;
        }

        let mut server = Server::new(entry.name.clone(), entry.color);
        server.description = entry.description.clone();
        self.servers.push(server);
        let position = self.servers.len() - 1;
        if let Some(target) = self.discover_servers.get_mut(index) {
            target.joined = true;
        }
        self.select_server(position, window, cx);
    }

    // -----------------------------------------------------------------------

    // ---------------------------- 服务器设置页 ----------------------------

    /// 打开服务器设置页：预填当前服务器名称与简介，并收起所有浮层。
    pub(crate) fn open_server_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let server = &self.servers[self.active_server];
        let name = server.name.clone();
        let description = server.description.clone();
        self.server_name_edit
            .update(cx, |input, cx| input.set_value(name, window, cx));
        self.server_desc_edit
            .update(cx, |input, cx| input.set_value(description, window, cx));

        self.show_server_settings = true;
        self.show_voice_room = false;
        self.show_search_center = false;
        self.show_help_center = false;
        self.close_thread(); self.close_read_receipts(); self.close_edit_history();
        self.member_menu = None;
        self.server_settings_section = ServerSettingsSection::Overview;
        self.invite_copied = false;
        self.show_settings = false;
        self.show_server_menu = false;
        self.channel_menu = None;
        self.category_menu = None;
        self.member_menu = None;
        self.profile_target = None;
        self.close_message_menu();
        self.show_pins = false;
        self.show_inbox = false;
        self.show_status_menu = false;
        self.show_attachment_picker = false;
        self.close_sticker_picker();
        self.emoji_picker_open = false;
        self.emoji_target = None;
        self.show_invite = false;
        self.close_topic_editor();
        cx.notify();
    }

    /// 关闭服务器设置页，回到主界面。
    pub(crate) fn close_server_settings(&mut self, cx: &mut Context<Self>) {
        self.show_server_settings = false;
        self.invite_copied = false;
        cx.notify();
    }

    /// 切换服务器设置分类。
    pub(crate) fn select_server_settings_section(
        &mut self,
        section: ServerSettingsSection,
        cx: &mut Context<Self>,
    ) {
        self.server_settings_section = section;
        self.invite_copied = false;
        cx.notify();
    }

    /// 保存「概览」：改名时同步迁移消息键与折叠键，保留分类展开状态。
    pub(crate) fn save_server_overview(&mut self, cx: &mut Context<Self>) {
        let index = self.active_server;
        let new_name = self.server_name_edit.read(cx).value().trim().to_string();
        let new_desc = self.server_desc_edit.read(cx).value().trim().to_string();
        let current = self.servers[index].name.clone();

        if !new_name.is_empty() && new_name != current {
            let old_prefix = format!("{current}/");
            let new_prefix = format!("{new_name}/");

            // 消息按「服务器/频道」存键，改名前先搬走。
            let keys: Vec<String> = self
                .messages
                .keys()
                .filter(|key| key.starts_with(&old_prefix))
                .cloned()
                .collect();
            for key in keys {
                if let Some(messages) = self.messages.remove(&key) {
                    let renamed = format!("{new_prefix}{}", &key[old_prefix.len()..]);
                    self.messages.insert(renamed, messages);
                }
            }

            // 分类折叠键同样带服务器名前缀。
            for key in self.collapsed.iter_mut() {
                if let Some(rest) = key.strip_prefix(&old_prefix) {
                    *key = format!("{new_prefix}{rest}");
                }
            }

            let badge = new_name.chars().next().unwrap_or('?').to_string();
            self.servers[index].name = new_name;
            self.servers[index].badge = badge;
        }

        if !new_desc.is_empty() {
            self.servers[index].description = new_desc;
        }
        cx.notify();
    }

    /// 切换服务器图标颜色（点击即生效）。
    pub(crate) fn set_server_color(&mut self, color: u32, cx: &mut Context<Self>) {
        if let Some(server) = self.servers.get_mut(self.active_server) {
            server.color = color;
        }
        cx.notify();
    }

    /// 移除一个服务器表情，暂存到「已移除」区以便恢复。
    pub(crate) fn remove_server_emoji(&mut self, emoji: &'static str, cx: &mut Context<Self>) {
        if let Some(position) = self.server_emojis.iter().position(|item| *item == emoji) {
            self.server_emojis.remove(position);
            self.removed_server_emojis.push(emoji);
        }
        cx.notify();
    }

    /// 从「已移除」区恢复一个表情。
    pub(crate) fn restore_server_emoji(&mut self, emoji: &'static str, cx: &mut Context<Self>) {
        if let Some(position) = self
            .removed_server_emojis
            .iter()
            .position(|item| *item == emoji)
        {
            self.removed_server_emojis.remove(position);
            self.server_emojis.push(emoji);
        }
        cx.notify();
    }

    /// 复制当前服务器的邀请链接（设置页内使用，不关闭页面）。
    pub(crate) fn copy_server_invite(&mut self, cx: &mut Context<Self>) {
        let link = format!("discord.gg/{}", self.invite_code());
        cx.write_to_clipboard(ClipboardItem::new_string(link));
        self.invite_copied = true;
        cx.notify();
    }

    // -----------------------------------------------------------------------

    /// 打开设置页（覆盖主界面），同时关闭其他浮层并把资料写回账户表单。
    pub(crate) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_account_form(window, cx);
        self.show_settings = true;
        self.show_server_settings = false;
        self.show_voice_room = false;
        self.show_search_center = false;
        self.show_help_center = false;
        self.close_thread(); self.close_read_receipts(); self.close_edit_history();
        self.show_status_menu = false;
        self.profile_target = None;
        self.close_message_menu();
        self.emoji_picker_open = false;
        self.emoji_target = None;
        self.show_pins = false;
        self.show_create_server = false;
        self.show_create_channel = false;
        self.channel_menu = None;
        self.close_topic_editor();
        self.show_server_menu = false;
        self.show_invite = false;
        self.close_category_menu();
        self.close_category_rename();
        self.close_create_category();
        self.show_attachment_picker = false;
        self.close_sticker_picker();
    }

    /// 关闭设置页返回主界面。
    pub(crate) fn close_settings(&mut self) {
        self.show_settings = false;
    }

    /// 把 `me` 写回账户表单（打开设置时调用，丢弃上次未保存的编辑）。
    pub(crate) fn sync_account_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let me = self.me.clone();
        self.account_name_input
            .update(cx, |input, cx| input.set_value(me.display_name.clone(), window, cx));
        self.account_username_input
            .update(cx, |input, cx| input.set_value(me.username.clone(), window, cx));
        self.account_email_input
            .update(cx, |input, cx| input.set_value(me.email.clone(), window, cx));
        self.account_bio_input
            .update(cx, |input, cx| input.set_value(me.bio.clone(), window, cx));
        self.account_notice = None;
    }

    /// 保存账户改动：显示名与用户名为必填，邮箱、简介允许留空。
    pub(crate) fn save_account(&mut self, cx: &mut Context<Self>) {
        let name = self.account_name_input.read(cx).value().trim().to_string();
        let username = self
            .account_username_input
            .read(cx)
            .value()
            .trim()
            .to_string();
        let email = self.account_email_input.read(cx).value().trim().to_string();
        let bio = self.account_bio_input.read(cx).value().trim().to_string();

        if name.is_empty() || username.is_empty() {
            self.account_notice = Some("显示名称与用户名不能为空");
            cx.notify();
            return;
        }

        self.me.display_name = name;
        self.me.username = username;
        self.me.email = email;
        self.me.bio = bio;
        self.account_notice = Some("已保存");
        cx.notify();
    }

    /// 放弃未保存的编辑，把表单恢复成当前资料。
    pub(crate) fn reset_account_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_account_form(window, cx);
        cx.notify();
    }

    /// 换头像颜色：立即生效，不需要点保存。
    pub(crate) fn set_avatar_color(&mut self, color: u32, cx: &mut Context<Self>) {
        self.me.color = color;
        self.account_notice = Some("头像颜色已更新");
        cx.notify();
    }

    /// 切换设置页左侧分类。
    pub(crate) fn select_settings_section(&mut self, section: SettingsSection) {
        self.settings_section = section;
    }

    /// 切换主题：同时改自定义色板与 gpui-component 的组件主题（输入框等控件读后者）。
    /// 色板是原子量，`cx.notify()` 后整棵树会用新颜色重建。
    pub(crate) fn set_theme(
        &mut self,
        theme: ThemeChoice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.theme_choice = theme;
        let light = match theme {
            ThemeChoice::Light => true,
            ThemeChoice::Dark => false,
            ThemeChoice::System => Self::window_is_light(window),
        };
        palette::set_light(light);
        Theme::change(
            if light { ThemeMode::Light } else { ThemeMode::Dark },
            Some(window),
            cx,
        );
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 本地持久化（快照 / 自动保存 / 恢复示例数据）
    // -----------------------------------------------------------------------

    /// 把当前内容状态打包成快照。只包含数据，不含浮层显隐与输入焦点。
    fn snapshot(&self) -> persist::Snapshot {
        persist::Snapshot {
            version: persist::VERSION,
            servers: self.servers.clone(),
            messages: self.messages.clone(),
            dm_conversations: self.dm_conversations.clone(),
            dm_messages: self.dm_messages.clone(),
            threads: self.threads.clone(),
            friends: self.friends.clone(),
            friend_requests: self.friend_requests.clone(),
            blocked_users: self.blocked_users.clone(),
            me: self.me.clone(),
            active_server: self.active_server,
            active_channel: self.active_channel.clone(),
            active_dm: self.active_dm.clone(),
            notifications_on: self.notifications_on,
            notification_sound: self.notification_sound,
            my_status: self.my_status,
            theme: self.theme_choice.key().to_string(),
            show_members: self.show_members,
            collapsed: self.collapsed.clone(),
            collapsed_roles: self.collapsed_roles.clone(),
            read_notifications: self.read_notifications.clone(),
        }
    }

    /// 启动时尝试读回快照；任何字段与实际数据对不上都退回种子值。
    fn restore_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(saved) = persist::load() else {
            return;
        };
        if saved.servers.is_empty() {
            return;
        }

        self.servers = saved.servers;
        self.active_server = saved.active_server.min(self.servers.len() - 1);
        let channel_ok = self.servers[self.active_server]
            .categories
            .iter()
            .flat_map(|category| category.channels.iter())
            .any(|channel| channel.name == saved.active_channel && channel.kind == ChannelKind::Text);
        self.active_channel = if channel_ok {
            saved.active_channel
        } else {
            self.first_text_channel().unwrap_or_default()
        };

        self.messages = saved.messages;
        self.dm_conversations = saved.dm_conversations;
        self.dm_messages = saved.dm_messages;
        self.active_dm = saved
            .active_dm
            .filter(|name| self.dm_conversations.iter().any(|c| c.friend == name.as_str()))
            .or_else(|| self.dm_conversations.first().map(|c| c.friend.to_string()));
        self.threads = saved.threads;

        if !saved.friends.is_empty() {
            self.friends = saved.friends;
        }
        self.friend_requests = saved.friend_requests;
        self.blocked_users = saved.blocked_users;
        if !saved.me.display_name.is_empty() {
            self.me = saved.me;
        }

        self.notifications_on = saved.notifications_on;
        self.notification_sound = saved.notification_sound;
        self.my_status = saved.my_status;
        self.show_members = saved.show_members;
        self.collapsed = saved.collapsed;
        self.collapsed_roles = saved.collapsed_roles;
        self.read_notifications = saved.read_notifications;

        // 主题：只在和当前不同的时候切，避免启动时多余的 Theme::change。
        let theme = ThemeChoice::from_key(&saved.theme);
        if theme != self.theme_choice {
            self.set_theme(theme, window, cx);
        }

        // 记录基线，避免刚启动就把同一份内容再写一遍。
        self.saved_json = serde_json::to_string(&self.snapshot()).unwrap_or_default();
    }

    /// 每帧调用：快照内容与上次落盘不同才写，避免无意义的磁盘 IO。
    fn autosave(&mut self) {
        let snapshot = self.snapshot();
        let Ok(json) = serde_json::to_string(&snapshot) else {
            return;
        };
        if json == self.saved_json {
            return;
        }
        if persist::save(&snapshot).is_ok() {
            self.saved_json = json;
        }
    }

    /// 数据页「立即保存」：立刻写盘并给出可见提示。
    pub(crate) fn persist_now(&mut self, cx: &mut Context<Self>) {
        let snapshot = self.snapshot();
        match persist::save(&snapshot) {
            Ok(_) => {
                self.saved_json = serde_json::to_string(&snapshot).unwrap_or_default();
                self.persist_note = Some("已写入本地快照");
            }
            Err(_) => {
                self.persist_note = Some("写入失败：请检查磁盘权限或剩余空间");
            }
        }
        cx.notify();
    }

    /// 数据页「恢复示例数据」：删掉快照并把内存状态重置回种子。
    pub(crate) fn reset_local_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = persist::clear();
        self.servers = seed_servers();
        self.active_server = 0;
        self.active_channel = self.first_text_channel().unwrap_or_default();
        self.view_mode = ViewMode::Server;
        self.messages = seed_messages();
        self.dm_conversations = seed_dm_conversations();
        self.dm_messages = seed_dm_messages();
        self.active_dm = self.dm_conversations.first().map(|c| c.friend.to_string());
        self.friends = seed_friends();
        self.friend_requests = seed_friend_requests();
        self.blocked_users.clear();
        self.me = seed_me();
        self.threads = seed_threads();
        self.read_notifications.clear();
        self.drafts.clear();
        self.saved_json.clear();
        self.persist_note = Some("已恢复示例数据，下次改动会重新写入快照");
        self.reset_session(window, cx);
        cx.notify();
    }

    /// 数据页展示用：快照文件路径（找不到 HOME 时退化为相对路径）。
    pub(crate) fn snapshot_path(&self) -> String {
        persist::state_path().display().to_string()
    }

    /// 数据页展示用：当前快照包含的消息条数与频道数。
    pub(crate) fn snapshot_size(&self) -> (usize, usize) {
        let messages: usize = self.messages.values().map(Vec::len).sum::<usize>()
            + self.dm_messages.values().map(Vec::len).sum::<usize>();
        let channels: usize = self
            .servers
            .iter()
            .flat_map(|server| server.categories.iter())
            .map(|category| category.channels.len())
            .sum();
        (messages, channels)
    }

    /// 读窗口当前外观：浅色 / 浅色增强都按浅色处理。
    fn window_is_light(window: &Window) -> bool {
        matches!(
            window.appearance(),
            WindowAppearance::Light | WindowAppearance::VibrantLight
        )
    }

    pub(crate) fn toggle_compact_messages(&mut self) {
        self.compact_messages = !self.compact_messages;
    }

    pub(crate) fn toggle_show_timestamps(&mut self) {
        self.show_timestamps = !self.show_timestamps;
    }

    pub(crate) fn toggle_notification_sound(&mut self) {
        self.notification_sound = !self.notification_sound;
    }

    pub(crate) fn toggle_mention_pings(&mut self) {
        self.mention_pings = !self.mention_pings;
    }

    pub(crate) fn toggle_show_emoji(&mut self) {
        self.show_emoji = !self.show_emoji;
    }

    pub(crate) fn adjust_input_volume(&mut self, delta: i8) {
        self.input_volume = Self::clamp_volume(self.input_volume as i16 + delta as i16);
    }

    pub(crate) fn adjust_output_volume(&mut self, delta: i8) {
        self.output_volume = Self::clamp_volume(self.output_volume as i16 + delta as i16);
    }

    /// 把设置项恢复为初始值。
    pub(crate) fn reset_settings(&mut self) {
        self.theme_choice = ThemeChoice::Dark;
        palette::set_light(false);
        self.compact_messages = false;
        self.show_timestamps = true;
        self.notification_sound = true;
        self.mention_pings = true;
        self.show_emoji = true;
        self.input_volume = 80;
        self.output_volume = 100;
        self.notifications_on = true;
    }

    fn clamp_volume(value: i16) -> u8 {
        value.clamp(0, 100) as u8
    }

    // -----------------------------------------------------------------------
    // 开关
    // -----------------------------------------------------------------------

    /// 展开/折叠频道分类。
    pub(crate) fn toggle_category(&mut self, key: String) {
        if let Some(pos) = self.collapsed.iter().position(|k| k == &key) {
            self.collapsed.remove(pos);
        } else {
            self.collapsed.push(key);
        }
    }

    pub(crate) fn toggle_members(&mut self) {
        self.show_members = !self.show_members;
    }

    pub(crate) fn toggle_mic(&mut self) {
        self.mic_on = !self.mic_on;
    }

    pub(crate) fn toggle_headphones(&mut self) {
        self.headphones_on = !self.headphones_on;
    }

    pub(crate) fn toggle_notifications(&mut self) {
        self.notifications_on = !self.notifications_on;
    }
}

impl DiscordApp {
    /// 主界面骨架（设置页打开时不渲染）。
    fn render_main(&mut self, cx: &mut Context<Self>) -> Div {
        let view_mode = self.view_mode;

        // 话题面板只在服务器 / 私信聊天区有意义，探索页不渲染。
        let show_thread = self.show_thread
            && self.thread_root.is_some()
            && matches!(view_mode, ViewMode::Server | ViewMode::DirectMessage);

        // 探索页是独立整屏页面：只保留服务器栏，其余区域由 discover 自行组织。
        if view_mode == ViewMode::Discover {
            return div()
                .size_full()
                .relative()
                .flex()
                .bg(rgb(palette::bg_chat()))
                .text_color(rgb(palette::text_message()))
                .text_size(px(14.))
                .child(server_rail::render(self, cx))
                .child(discover::render(self, cx))
                .when(self.show_voice_room && self.voice_channel.is_some(), |el| {
                    el.child(voice_room::render(self, cx))
                })
                .when(self.show_quick_switcher, |el| {
                    el.child(chat::quick_switcher::render(self, cx))
                })
                .child(voice_status::render(self, cx));
        }

        let show_members = self.show_members && view_mode == ViewMode::Server;

        let side = match view_mode {
            ViewMode::Server => channel_sidebar::render(self, cx).into_any_element(),
            ViewMode::DirectMessage | ViewMode::Friends | ViewMode::Discover => {
                dm_sidebar::render(self, cx).into_any_element()
            }
        };

        // 搜索中心接管聊天主区域（服务器栏与侧栏保留，便于边搜边切）。
        let search_open = self.show_search_center
            && matches!(view_mode, ViewMode::Server | ViewMode::DirectMessage);

        let main = if search_open {
            search_center::render(self, cx).into_any_element()
        } else {
            match view_mode {
                ViewMode::Friends => friends::render(self, cx).into_any_element(),
                ViewMode::Server | ViewMode::DirectMessage | ViewMode::Discover => v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .bg(rgb(palette::bg_chat()))
                    .child(chat::header::render(self, cx))
                    .child(chat::messages::render(self, cx))
                    .child(chat::composer::render(self, cx))
                    .into_any_element(),
            }
        };

        div()
            .size_full()
            .relative()
            .flex()
            .bg(rgb(palette::bg_chat()))
            .text_color(rgb(palette::text_message()))
            .text_size(px(14.))
            .child(server_rail::render(self, cx))
            .child(side)
            .child(main)
            .when(show_members, |el| el.child(member_list::render(self, cx)))
            .when(self.profile_target.is_some(), |el| {
                el.child(profile_card::render(self, cx))
            })
            .when(self.show_pins && view_mode != ViewMode::Friends, |el| {
                el.child(chat::pinned_panel::render(self, cx))
            })
            .when(show_thread, |el| {
                el.child(chat::thread_panel::render(self, cx))
            })
            .when(self.show_inbox, |el| {
                el.child(chat::inbox_panel::render(self, cx))
            })
            .when(self.member_menu.is_some(), |el| {
                el.child(member_menu::render(self, cx))
            })
            .when(self.menu_message_point.is_some(), |el| {
                el.child(chat::message_menu::render_floating(self, cx))
            })
            .when(self.read_receipts.is_some(), |el| {
                el.child(chat::read_receipts::render_floating(self, cx))
            })
            .when(self.edit_history.is_some(), |el| {
                el.child(chat::edit_history::render_floating(self, cx))
            })
            .when(self.channel_menu.is_some(), |el| {
                el.child(channel_menu::render(self, cx))
            })
            .when(self.category_menu.is_some(), |el| {
                el.child(category_menu::render(self, cx))
            })
            .when(self.show_rename_category || self.show_create_category, |el| {
                el.child(category_dialog::render(self, cx))
            })
            .when(self.show_edit_topic, |el| {
                el.child(topic_dialog::render(self, cx))
            })
            .when(self.show_server_menu, |el| {
                el.child(server_menu::render(self, cx))
            })
            .when(self.show_invite, |el| {
                el.child(invite_dialog::render(self, cx))
            })
            .when(self.show_status_menu, |el| {
                el.child(status_menu::render(self, cx))
            })
            .when(self.show_create_server, |el| {
                el.child(create_dialog::render_server(self, cx))
            })
            .when(self.show_create_channel, |el| {
                el.child(create_dialog::render_channel(self, cx))
            })
            .when(self.show_forward, |el| {
                el.child(chat::forward_dialog::render(self, cx))
            })
            .when(self.attachment_preview.is_some(), |el| {
                el.child(chat::attachment_preview::render(self, cx))
            })
            .when(self.show_voice_room && self.voice_channel.is_some(), |el| {
                el.child(voice_room::render(self, cx))
            })
            .when(self.show_help_center, |el| {
                el.child(help_center::render(self, cx))
            })
            .when(self.show_quick_switcher, |el| {
                el.child(chat::quick_switcher::render(self, cx))
            })
            .child(voice_status::render(self, cx))
    }
}

impl Render for DiscordApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 内容有变化就顺手落盘；没有变化时这里只是序列化一次做比较。
        self.autosave();
        // 键盘入口挂在元素树的根上（window.on_key_event 只能在 paint 阶段调用，Render 里会 panic）。
        // 根元素持有焦点，输入框聚焦时按键也会从焦点元素冒泡到这里。
        let content: AnyElement = if self.show_server_settings {
            server_settings::render(self, cx).into_any_element()
        } else if self.show_settings {
            settings::render(self, cx).into_any_element()
        } else {
            self.render_main(cx).into_any_element()
        };

        let root_focus = self.root_focus.clone();
        div()
            .size_full()
            .relative()
            .id("app-key-root")
            .track_focus(&root_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_shortcut(event, window, cx);
            }))
            .child(content)
            // 通话浮窗挂在根层：切频道、开设置/探索页时仍保持可见。
            .when(self.dm_call.is_some(), |el| {
                el.child(dm_call_panel::render(self, cx))
            })
    }
}
