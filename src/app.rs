use std::collections::HashMap;
use std::sync::Arc;

use async_channel::Receiver;

use gpui_kit::*;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{v_flex, Theme, ThemeMode};
use gpui_kit::prelude::FluentBuilder as _;

use crate::data::{
    persist,
    seed_discover_servers, seed_dm_conversations, seed_dm_messages, seed_friend_requests,
    seed_forum_posts, seed_message_requests, gif_categories, help_categories, seed_friend_groups,
    seed_friends, seed_gifs, seed_bookmarks, seed_welcome_screens, seed_boosts,
    seed_audit, seed_events, seed_help_articles, seed_invites, seed_me, seed_roles,
    seed_members, seed_messages, seed_servers,
    seed_automod_rules, seed_bans, seed_permissions, seed_stickers, seed_threads, sticker_packs,
    Attachment, AuditEntry, AuditKind, AutoModAction, AutoModRule, BannedUser, Bookmark, Category,
    Channel, ChannelKind, DeliveryStatus,
    ChatThread, DiscoverServer,     DmConversation, EditRevision, EventCategory, ForwardOrigin, ForwardTarget, ForumPost, ForumReply, ForumSort, Friend, FriendRequest,
    FriendTab, Gif, HelpArticle, Invite, InviteExpiry, Member, MessageRequest,
    MessageRequestState, MyProfile,
    MemberGroup, Message, Notification, NotificationKind, NotificationTarget, NotifyLevel,
    PermissionGrant, PermissionState, Poll, Reaction,
    ReadReceipt,
    ReplyRef, Role, SearchHit, Server, ServerBoost, Booster, ServerEvent, ServerWelcome, Sticker, ThreadReply,
    UserStatus, VoiceParticipant, WelcomeRule,
};
use crate::net::dto::{
    DmDto, FriendDto, FriendGroupDto, MemberDto, MessageDto, PageDto, PermissionMatrixDto, RoleDto,
    ServerDto, ServerTreeDto, UnreadCountsDto, UserDto,
};
use crate::net::hydrate::{self, RemoteIndex};
use crate::net::auth;
use crate::net::ws::{NetEvent, WsClient};
use crate::net::{api, NetClient, NetConfig, NetError};
use crate::ui::theme::palette;
use crate::ui::{
    category_dialog, category_menu, channel_menu, channel_sidebar, chat, create_dialog, dm_call_panel,
    dm_sidebar, discover, events_panel, friend_group_dialog, friend_group_menu, forum, friends,
    help_center, invite_dialog, member_list, member_nick_dialog, message_requests_panel, profile_card,
    server_menu, server_rail,
    login, member_menu, search_center, server_settings, settings, status_menu, topic_dialog,
    voice_room, voice_status, welcome,
};

/// 启动 / 登录阶段：检查后端 → 登录页 → 首屏同步 → 主界面。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthStatus {
    Checking,
    Login,
    Booting,
    Ready,
    Demo,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoginMode {
    Login,
    Register,
}

/// 主视图模式：服务器频道 / 私信 / 好友。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewMode {
    Server,
    DirectMessage,
    Friends,
    /// 探索页：整屏展示可加入的公开服务器。
    Discover,
}

/// 日程面板的状态筛选。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventFilter {
    All,
    Upcoming,
    Live,
    Ended,
}

impl EventFilter {
    pub(crate) fn label(self) -> &'static str {
        match self {
            EventFilter::All => "全部",
            EventFilter::Upcoming => "即将开始",
            EventFilter::Live => "进行中",
            EventFilter::Ended => "已结束",
        }
    }
}

/// 消息请求面板的处理状态筛选。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageRequestFilter {
    Pending,
    Accepted,
    Ignored,
}

impl MessageRequestFilter {
    pub(crate) fn label(self) -> &'static str {
        match self {
            MessageRequestFilter::Pending => "待处理",
            MessageRequestFilter::Accepted => "已接受",
            MessageRequestFilter::Ignored => "已忽略",
        }
    }
}

/// 请求状态是否落入某个筛选（供计数与列表共用）。
fn iso_utc_after_minutes(minutes: i64) -> String {
    let total = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
        + minutes * 60;
    let days = total.div_euclid(86_400);
    let seconds = total.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn event_category_key(category: EventCategory) -> &'static str {
    match category {
        EventCategory::Game => "game",
        EventCategory::Voice => "voice",
        EventCategory::Community => "community",
        EventCategory::Other => "other",
    }
}

fn notify_key(level: NotifyLevel) -> &'static str {
    match level {
        NotifyLevel::Inherit => "inherit",
        NotifyLevel::All => "all",
        NotifyLevel::Mentions => "mentions",
        NotifyLevel::Nothing => "none",
    }
}

fn update_poll(message: &mut Message, data: &serde_json::Value) {
    let Some(poll) = message.poll.as_mut() else {
        return;
    };
    if let Some(closed) = data.get("closed").and_then(|value| value.as_bool()) {
        poll.closed = closed;
    }
    if let Some(options) = data.get("options").and_then(|value| value.as_array()) {
        for option in options {
            let Some(option_id) = option.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(local) = poll.options.iter_mut().find(|item| item.id == option_id) else {
                continue;
            };
            let votes = option
                .get("votes")
                .and_then(|value| value.as_u64())
                .unwrap_or_default() as usize;
            let mine = option
                .get("me")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            local.voters = if mine {
                vec!["你".to_string(); votes.max(1)]
            } else {
                vec![String::new(); votes]
            };
        }
    }
}

fn apply_reaction(message: &mut Message, emoji: &str, add: bool, by_me: bool) {
    if add {
        if let Some(reaction) = message.reactions.iter_mut().find(|item| item.emoji == emoji) {
            reaction.count += 1;
            if by_me {
                reaction.mine = true;
            }
            return;
        }
        message.reactions.push(Reaction {
            emoji: Box::leak(emoji.to_string().into_boxed_str()),
            count: 1,
            mine: by_me,
        });
    } else if let Some(reaction) = message.reactions.iter_mut().find(|item| item.emoji == emoji) {
        reaction.count = reaction.count.saturating_sub(1);
        if by_me {
            reaction.mine = false;
        }
        if reaction.count == 0 {
            message.reactions.retain(|item| item.emoji != emoji);
        }
    }
}

fn state_matches_filter(state: MessageRequestState, filter: MessageRequestFilter) -> bool {
    match filter {
        MessageRequestFilter::Pending => state == MessageRequestState::Pending,
        MessageRequestFilter::Accepted => state == MessageRequestState::Accepted,
        MessageRequestFilter::Ignored => state == MessageRequestState::Ignored,
    }
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
    pub(crate) call_id: Option<String>,
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

/// 成员上下文菜单状态：目标成员名 + 光标锚点。
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

/// 好友分组操作菜单状态：目标分组名 + 展开锚点。
#[derive(Clone)]
pub(crate) struct FriendGroupMenuState {
    pub(crate) name: String,
    pub(crate) anchor: MenuAnchor,
}

/// 「移动到分组」菜单状态：目标好友名 + 展开锚点。
#[derive(Clone)]
pub(crate) struct FriendGroupPickerState {
    pub(crate) friend: String,
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
    Permissions,
    AutoMod,
    Emoji,
    Invites,
    Members,
    Audit,
    Boost,
}

/// 成员管理页的角色筛选：全部 / 某个角色 / 无角色。
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum MemberFilter {
    All,
    Role(String),
    NoRole,
}

/// 审核日志最多保留的条数：超出后丢最旧的，避免快照无限膨胀。
pub(crate) const AUDIT_LIMIT: usize = 200;

/// 新建角色时轮换使用的颜色（与种子角色的取色一致）。
pub(crate) const ROLE_COLORS: [u32; 6] = [0xf23f43, 0x57f287, 0x00a8fc, 0xfaa61c, 0xeb459e, 0x5865f2];

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

    // 服务器日程：活动表（含归属服务器）+ 面板显隐 + 状态筛选 + 选中项 + 操作提示。
    pub(crate) events: Vec<ServerEvent>,
    pub(crate) show_events: bool,
    pub(crate) event_filter: EventFilter,
    pub(crate) selected_event: Option<String>,
    pub(crate) event_notice: Option<String>,
    // 创建活动草稿：标题/描述输入 + 落地频道/分类 + 开始偏移/时长/是否显示表单。
    pub(crate) show_event_create: bool,
    pub(crate) event_title_edit: Entity<InputState>,
    pub(crate) event_desc_edit: Entity<InputState>,
    pub(crate) event_channel: String,
    pub(crate) event_category: EventCategory,
    pub(crate) event_starts_in: i32,
    pub(crate) event_duration: u32,

    // 私信消息请求：请求表 + 面板显隐 + 状态筛选 + 操作提示。
    pub(crate) message_requests: Vec<MessageRequest>,
    pub(crate) show_message_requests: bool,
    pub(crate) message_request_filter: MessageRequestFilter,
    pub(crate) message_request_notice: Option<String>,

    // 论坛频道：帖子表 + 当前打开的帖子 + 搜索/排序 + 发帖/回帖草稿。
    pub(crate) forum_posts: Vec<ForumPost>,
    pub(crate) forum_open: Option<String>,
    pub(crate) forum_sort: ForumSort,
    pub(crate) forum_query: String,
    pub(crate) forum_search: Entity<InputState>,
    pub(crate) show_forum_composer: bool,
    pub(crate) forum_title_edit: Entity<InputState>,
    pub(crate) forum_body_edit: Entity<InputState>,
    pub(crate) forum_tag_edit: Entity<InputState>,
    pub(crate) forum_tags_draft: Vec<String>,
    pub(crate) forum_reply_edit: Entity<InputState>,
    pub(crate) forum_notice: Option<String>,

    // 消息书签：跨频道 / 私信的统一收藏表 + 右侧面板 + 搜索 + 操作提示。
    pub(crate) bookmarks: Vec<Bookmark>,
    pub(crate) show_bookmarks: bool,
    pub(crate) bookmark_query: String,
    pub(crate) bookmark_search: Entity<InputState>,
    pub(crate) bookmark_notice: Option<String>,

    // 服务器欢迎屏幕：配置表 + 当前展示的服务器 + 规则勾选 + 推荐频道多选 + 已完成的服务器。
    pub(crate) welcome_screens: Vec<ServerWelcome>,
    pub(crate) welcome_target: Option<String>,
    pub(crate) welcome_rule_agreed: bool,
    pub(crate) welcome_selected: Vec<String>,
    pub(crate) welcome_joined: Vec<String>,

    // 服务器加成：每台服务器一条记录（没人加成时按 0 次合成），提示条只显示最近一次操作结果。
    pub(crate) boosts: Vec<ServerBoost>,
    pub(crate) boost_notice: Option<String>,

    /// 当前语音连接：(服务器索引, 频道名)；跨会话保持，符合 Discord 行为。
    pub(crate) voice_channel: Option<(usize, String)>,

    pub(crate) my_status: UserStatus,
    pub(crate) show_status_menu: bool,

    // 在线会话：网络句柄、启动阶段、登录表单与首屏同步结果。
    pub(crate) auth_status: AuthStatus,
    pub(crate) login_mode: LoginMode,
    pub(crate) login_email: Entity<InputState>,
    pub(crate) login_password: Entity<InputState>,
    pub(crate) login_username: Entity<InputState>,
    pub(crate) login_remember: bool,
    pub(crate) login_busy: bool,
    pub(crate) login_error: Option<String>,
    pub(crate) boot_message: String,
    pub(crate) net: Option<Arc<NetClient>>,
    pub(crate) remote: RemoteIndex,
    pub(crate) members_by_server: HashMap<String, Vec<Member>>,
    pub(crate) roles_by_server: HashMap<String, Vec<Role>>,
    pub(crate) permissions_by_server: HashMap<String, Vec<PermissionGrant>>,
    pub(crate) gateway: Option<WsClient>,
    pub(crate) gateway_tx: async_channel::Sender<NetEvent>,
    pub(crate) gateway_connected: bool,
    pub(crate) last_event_id: Option<String>,
    pub(crate) loaded_channels: std::collections::HashSet<String>,
    pub(crate) typing_until: Option<std::time::Instant>,
    pub(crate) typing_name: Option<String>,
    pub(crate) last_typing_sent: Option<std::time::Instant>,
    pub(crate) structure_refreshing: bool,
    pub(crate) pending_server_id: Option<String>,
    pub(crate) pending_channel: Option<String>,
    pub(crate) pending_forum_post: Option<String>,
    pub(crate) backend_notifications: Vec<Notification>,
    pub(crate) automod_by_server: HashMap<String, Vec<AutoModRule>>,
    pub(crate) audit_by_server: HashMap<String, Vec<AuditEntry>>,
    pub(crate) has_more_messages: HashMap<String, bool>,
    pub(crate) loading_older: bool,
    pub(crate) loaded_dms: std::collections::HashSet<String>,

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
    /// 新建服务器时选中的模板 key（None = 从零开始）。
    pub(crate) create_template: Option<String>,
    pub(crate) server_name_input: Entity<InputState>,
    pub(crate) channel_name_input: Entity<InputState>,

    // 好友页：标签 + 请求 + 屏蔽列表 + 添加好友输入。
    pub(crate) friend_tab: FriendTab,
    pub(crate) friend_requests: Vec<FriendRequest>,
    pub(crate) friend_notice: Option<String>,
    pub(crate) blocked_users: Vec<String>,
    pub(crate) add_friend_input: Entity<InputState>,

    // 好友分组：分组顺序 + 折叠列表 + 分组菜单 / 移动菜单 / 新建重命名弹层。
    pub(crate) friend_groups: Vec<String>,
    pub(crate) collapsed_groups: Vec<String>,
    pub(crate) friend_group_menu: Option<FriendGroupMenuState>,
    pub(crate) friend_group_picker: Option<FriendGroupPickerState>,
    pub(crate) show_create_group: bool,
    pub(crate) show_rename_group: bool,
    /// 重命名前的分组名，用于定位分组并在改名后迁移折叠状态。
    pub(crate) rename_group_source: String,
    pub(crate) group_input: Entity<InputState>,

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

    // 投票创建弹层：问题 / 新选项输入 + 已添加选项草稿 + 是否多选 + 显隐。
    // 已发起的投票挂在 Message.poll 上，随消息快照持久化；这里只存"正在编辑"的草稿。
    pub(crate) show_poll_composer: bool,
    pub(crate) poll_question_edit: Entity<InputState>,
    pub(crate) poll_option_edit: Entity<InputState>,
    pub(crate) poll_options_draft: Vec<String>,
    pub(crate) poll_multiple: bool,

    // 服务器设置页：整屏覆盖主界面，与用户设置页互斥。
    pub(crate) show_server_settings: bool,
    pub(crate) server_settings_section: ServerSettingsSection,
    pub(crate) server_name_edit: Entity<InputState>,
    pub(crate) server_desc_edit: Entity<InputState>,
    /// 服务器自定义表情（Demo：可移除与恢复，不做上传）。
    pub(crate) server_emojis: Vec<&'static str>,
    pub(crate) removed_server_emojis: Vec<&'static str>,
    // 角色管理：角色表（顺序即展示顺序）+ 选中的角色 + 名称输入 + 保存提示。
    pub(crate) roles: Vec<Role>,
    pub(crate) selected_role: Option<String>,
    pub(crate) role_name_edit: Entity<InputState>,
    pub(crate) role_notice: Option<&'static str>,

    // 权限管理：角色 × 权限的三态设置（只存非默认项）+ 操作提示。
    pub(crate) role_permissions: Vec<PermissionGrant>,
    pub(crate) permission_notice: Option<&'static str>,

    // 自动审核：规则表 + 选中规则 + 新建名/关键词输入 + 拦截提示。
    pub(crate) automod_rules: Vec<AutoModRule>,
    pub(crate) selected_automod: Option<String>,
    pub(crate) automod_name_edit: Entity<InputState>,
    pub(crate) automod_keyword_edit: Entity<InputState>,
    pub(crate) automod_notice: Option<String>,

    // 邀请管理：全部邀请 + 新建表单（频道 / 有效期 / 次数上限）+ 最近复制的邀请码。
    pub(crate) invites: Vec<Invite>,
    pub(crate) invite_channel: String,
    pub(crate) invite_expiry: InviteExpiry,
    /// 新建邀请的次数上限；0 表示不限。
    pub(crate) invite_max_uses: u32,
    pub(crate) invite_copied_code: Option<String>,

    // 成员管理：搜索框 + 角色筛选 + 昵称弹层（目标成员 + 输入框）+ 封禁列表。
    pub(crate) bans: Vec<BannedUser>,
    pub(crate) member_search: Entity<InputState>,
    pub(crate) member_filter: MemberFilter,
    pub(crate) show_member_nick: bool,
    pub(crate) member_nick_target: String,
    pub(crate) member_nick_input: Entity<InputState>,

    // 审核日志：按 seq 升序的条目 + 当前筛选类别（None = 全部）。
    pub(crate) audit: Vec<AuditEntry>,
    pub(crate) audit_filter: Option<AuditKind>,

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

/// 取基础用户名：兼容早期「Nova —— 管理员」这种把角色写进名字的展示格式。
/// 现在角色独立存在 `Member.role` 上，名称本身已经不含后缀。
fn base_name(display: &'static str) -> &'static str {
    display.split(" —— ").next().unwrap_or(display)
}

/// 书签身份用的正文：贴纸 / GIF / 附件消息正文为空，用占位文案代替，
/// 保证收藏、取消收藏与列表展示三处用的是同一个字符串。
fn bookmark_body(message: &Message) -> String {
    if !message.text.trim().is_empty() {
        return message.text.clone();
    }
    if message.sticker.is_some() {
        "[贴纸]".into()
    } else if message.gif.is_some() {
        "[GIF]".into()
    } else if message.attachment.is_some() {
        "[附件]".into()
    } else {
        "[空消息]".into()
    }
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
        let group_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入分组名称"));
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
        let login_email = cx.new(|cx| InputState::new(window, cx).placeholder("you@example.com"));
        let login_password = cx.new(|cx| InputState::new(window, cx).placeholder("密码（至少 8 位）"));
        let login_username = cx.new(|cx| InputState::new(window, cx).placeholder("用户名（注册时使用）"));
        let server_name_edit =
            cx.new(|cx| InputState::new(window, cx).placeholder("服务器名称"));
        let role_name_edit = cx.new(|cx| InputState::new(window, cx).placeholder("角色名称"));
        let automod_name_edit =
            cx.new(|cx| InputState::new(window, cx).placeholder("规则名称，例如 广告拦截"));
        let automod_keyword_edit =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入关键词后回车添加"));
        let member_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索成员"));
        let member_nick_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("服务器昵称"));
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
        let poll_question_edit = cx.new(|cx| InputState::new(window, cx).placeholder("投票问题…"));
        let poll_option_edit = cx.new(|cx| InputState::new(window, cx).placeholder("添加一个选项"));
        let event_title_edit = cx.new(|cx| InputState::new(window, cx).placeholder("活动标题"));
        let event_desc_edit = cx.new(|cx| InputState::new(window, cx).placeholder("简单介绍一下这个活动…"));
        // 论坛：搜索 / 发帖标题正文标签 / 回帖输入。
        let forum_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索帖子"));
        let forum_title_edit = cx.new(|cx| InputState::new(window, cx).placeholder("帖子标题"));
        let forum_body_edit = cx.new(|cx| InputState::new(window, cx).placeholder("详细描述你的问题或想法…"));
        let forum_tag_edit = cx.new(|cx| InputState::new(window, cx).placeholder("输入标签后回车添加"));
        let forum_reply_edit = cx.new(|cx| InputState::new(window, cx).placeholder("写下你的回复…"));
        // 书签面板：跨会话搜索。
        let bookmark_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索书签"));
        let thread_input = cx.new(|cx| InputState::new(window, cx).placeholder("回复话题…"));
        let root_focus = cx.focus_handle();

        cx.subscribe_in(&composer, window, |this, _, event: &InputEvent, window, cx| {
            match event {
                InputEvent::PressEnter { secondary: false, .. } => this.send_message(window, cx),
                InputEvent::Change => {
                    this.refresh_mention(cx);
                    this.maybe_send_typing(cx);
                }
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

        for input in [&login_email, &login_username, &login_password] {
            cx.subscribe_in(input, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                    this.submit_auth(cx);
                }
            })
            .detach();
        }

        // 分组输入框同时服务「新建分组」与「重命名分组」，按当前弹层分发 Enter。
        cx.subscribe_in(&group_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                if this.show_create_group {
                    this.create_group(window, cx);
                } else if this.show_rename_group {
                    this.save_group_rename(window, cx);
                }
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
                this.run_search(cx);
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

        // 论坛：搜索框实时过滤；标签框回车加标签；回帖框回车发送。
        cx.subscribe_in(&forum_search, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.forum_query = this.forum_search.read(cx).value().trim().to_string();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&bookmark_search, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.bookmark_query = this.bookmark_search.read(cx).value().trim().to_string();
                cx.notify();
            }
        })
        .detach();
        cx.subscribe_in(&forum_tag_edit, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.add_forum_tag(window, cx);
            }
        })
        .detach();
        cx.subscribe_in(&forum_reply_edit, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.send_forum_reply(window, cx);
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

        // 昵称输入框按 Enter 保存（空值表示清除昵称）。
        cx.subscribe_in(&member_nick_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.save_member_nick(window, cx);
            }
        })
        .detach();

        // 角色名称输入框按 Enter 保存重命名。
        cx.subscribe_in(&role_name_edit, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.save_role_name(cx);
            }
        })
        .detach();

        // 自动审核：规则名回车新建规则，关键词回车添加到选中规则。
        cx.subscribe_in(
            &automod_name_edit,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                    this.create_automod_rule(window, cx);
                }
            },
        )
        .detach();
        cx.subscribe_in(
            &automod_keyword_edit,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                    this.add_automod_keyword(window, cx);
                }
            },
        )
        .detach();

        // 投票：选项框回车添加选项，问题框回车发起投票。
        cx.subscribe_in(
            &poll_option_edit,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                    this.add_poll_option(window, cx);
                }
            },
        )
        .detach();
        cx.subscribe_in(
            &poll_question_edit,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                    this.send_poll(window, cx);
                }
            },
        )
        .detach();

        // 日程：标题框回车创建活动。
        cx.subscribe_in(
            &event_title_edit,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                    this.create_event(window, cx);
                }
            },
        )
        .detach();

        cx.subscribe_in(&topic_input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false, .. }) {
                this.save_channel_topic(window, cx);
            }
        })
        .detach();

        let (net, auth_status) = match NetConfig::from_env() {
            Ok(config) if !config.demo => (Some(Arc::new(NetClient::spawn(config))), AuthStatus::Checking),
            _ => (None, AuthStatus::Demo),
        };
        let (gateway_tx, gateway_rx) = async_channel::unbounded();

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
            events: seed_events(),
            show_events: false,
            event_filter: EventFilter::All,
            selected_event: None,
            event_notice: None,
            show_event_create: false,
            event_title_edit,
            event_desc_edit,
            event_channel: "一般".into(),
            event_category: EventCategory::Community,
            event_starts_in: 60,
            event_duration: 60,
            message_requests: seed_message_requests(),
            show_message_requests: false,
            message_request_filter: MessageRequestFilter::Pending,
            message_request_notice: None,
            forum_posts: seed_forum_posts(),
            forum_open: None,
            forum_sort: ForumSort::Recent,
            forum_query: String::new(),
            forum_search,
            show_forum_composer: false,
            forum_title_edit,
            forum_body_edit,
            forum_tag_edit,
            forum_tags_draft: Vec::new(),
            forum_reply_edit,
            forum_notice: None,
            bookmarks: seed_bookmarks(),
            show_bookmarks: false,
            bookmark_query: String::new(),
            bookmark_search,
            bookmark_notice: None,
            welcome_screens: seed_welcome_screens(),
            welcome_target: None,
            welcome_rule_agreed: false,
            welcome_selected: Vec::new(),
            welcome_joined: Vec::new(),
            boosts: seed_boosts(),
            boost_notice: None,
            voice_channel: None,
            auth_status,
            login_mode: LoginMode::Login,
            login_email,
            login_password,
            login_username,
            login_remember: false,
            login_busy: false,
            login_error: None,
            boot_message: String::new(),
            net,
            remote: RemoteIndex::default(),
            members_by_server: HashMap::new(),
            roles_by_server: HashMap::new(),
            permissions_by_server: HashMap::new(),
            gateway: None,
            gateway_tx,
            gateway_connected: false,
            last_event_id: None,
            loaded_channels: std::collections::HashSet::new(),
            typing_until: None,
            typing_name: None,
            last_typing_sent: None,
            structure_refreshing: false,
            pending_server_id: None,
            pending_channel: None,
            pending_forum_post: None,
            backend_notifications: Vec::new(),
            automod_by_server: HashMap::new(),
            audit_by_server: HashMap::new(),
            has_more_messages: HashMap::new(),
            loading_older: false,
            loaded_dms: std::collections::HashSet::new(),
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
            create_template: None,
            server_name_input,
            channel_name_input,
            friend_tab: FriendTab::Online,
            friend_requests: seed_friend_requests(),
            friend_notice: None,
            blocked_users: Vec::new(),
            add_friend_input,
            friend_groups: seed_friend_groups(),
            collapsed_groups: Vec::new(),
            friend_group_menu: None,
            friend_group_picker: None,
            show_create_group: false,
            show_rename_group: false,
            rename_group_source: String::new(),
            group_input,
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
            show_poll_composer: false,
            poll_question_edit,
            poll_option_edit,
            poll_options_draft: Vec::new(),
            poll_multiple: false,
            show_server_settings: false,
            server_settings_section: ServerSettingsSection::Overview,
            server_name_edit,
            server_desc_edit,
            server_emojis: vec![
                "🎨", "🚀", "🔥", "✅", "👀", "🦀", "🎧", "🎮", "🍜", "🌙", "💡", "🐧",
            ],
            removed_server_emojis: Vec::new(),
            roles: seed_roles(),
            selected_role: None,
            role_name_edit,
            role_notice: None,
            role_permissions: seed_permissions(),
            permission_notice: None,
            automod_rules: seed_automod_rules(),
            selected_automod: None,
            automod_name_edit,
            automod_keyword_edit,
            automod_notice: None,
            invites: seed_invites(),
            invite_channel: String::new(),
            invite_expiry: InviteExpiry::OneDay,
            invite_max_uses: 0,
            invite_copied_code: None,
            bans: seed_bans(),
            member_search,
            member_filter: MemberFilter::All,
            show_member_nick: false,
            member_nick_target: String::new(),
            member_nick_input,
            audit: seed_audit(),
            audit_filter: None,
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
        this.sync_invite_channel();
        // 有本地快照就覆盖种子数据；没有则以种子数据启动并立即记一次基线。
        this.restore_state(window, cx);
        if this.auth_status == AuthStatus::Checking {
            this.check_health(cx);
        }
        this.start_event_loop(gateway_rx, cx);
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

    /// 按名字聚合资料卡信息（角色取自成员身上的 `role`）。
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
        let role = member.and_then(|m| m.role).unwrap_or("成员");

        ProfileInfo {
            name: base,
            color,
            online,
            activity,
            role,
        }
    }

    /// 角色名对应的颜色；角色表里找不到时给默认强调色。
    pub(crate) fn role_color(&self, name: &str) -> u32 {
        self.roles
            .iter()
            .find(|role| role.name == name)
            .map(|role| role.color)
            .unwrap_or(palette::accent())
    }

    /// 按角色聚合成员栏：顺序跟随角色表（角色表即层级），再补「在线」「离线」兜底组。
    /// 空角色不出现在成员栏（但仍可以在服务器设置里管理）。
    pub(crate) fn member_groups(&self) -> Vec<MemberGroup> {
        let mut groups: Vec<MemberGroup> = Vec::new();

        for role in &self.roles {
            let member_indices: Vec<usize> = self
                .members
                .iter()
                .enumerate()
                .filter(|(_, member)| member.role == Some(role.name.as_str()))
                .map(|(index, _)| index)
                .collect();
            if member_indices.is_empty() {
                continue;
            }
            groups.push(MemberGroup {
                title: role.name.clone(),
                color: role.color,
                key: format!("role-{}", role.name),
                member_indices,
            });
        }

        // 无角色成员按在线状态归入兜底组。
        let online: Vec<usize> = self
            .members
            .iter()
            .enumerate()
            .filter(|(_, member)| member.online && member.role.is_none())
            .map(|(index, _)| index)
            .collect();
        let offline: Vec<usize> = self
            .members
            .iter()
            .enumerate()
            .filter(|(_, member)| !member.online && member.role.is_none())
            .map(|(index, _)| index)
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
        if !self.backend_notifications.is_empty() {
            return self.backend_notifications.clone();
        }
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
            self.show_events = false;
            self.show_message_requests = false;
            self.show_bookmarks = false;
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
        let server_name = self.servers[index].name.clone();
        if let Some(members) = self.members_by_server.get(&server_name) {
            self.members = members.clone();
        }
        if let Some(roles) = self.roles_by_server.get(&server_name) {
            self.roles = roles.clone();
        }
        if let Some(permissions) = self.permissions_by_server.get(&server_name) {
            self.role_permissions = permissions.clone();
        }
        if let Some(rules) = self.automod_by_server.get(&server_name) {
            self.automod_rules = rules.clone();
        }
        if let Some(entries) = self.audit_by_server.get(&server_name) {
            self.audit = entries.clone();
        }
        self.active_channel = self.first_text_channel().unwrap_or_default();
        self.sync_invite_channel();
        self.reset_session(window, cx);
        self.ensure_channel_loaded(cx);
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
        self.ensure_channel_loaded(cx);
    }

    /// 打开与某好友的私信。
    pub(crate) fn open_dm(
        &mut self,
        friend: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.view_mode = ViewMode::DirectMessage;
        self.active_dm = Some(friend.clone());
        self.reset_session(window, cx);
        self.ensure_dm_loaded(friend, cx);
    }

    /// 从好友发起新私信：若还没有会话则补建会话条目与空消息列表，再切入。
    pub(crate) fn start_dm(
        &mut self,
        friend: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dm_conversations.iter().any(|c| c.friend == friend) {
            if self.is_online()
                && !self.remote.dms.contains_key(friend)
                && let Some(net) = self.net.clone()
            {
                let user_id = self
                    .remote
                    .friends
                    .get(friend)
                    .or_else(|| {
                        self.remote
                            .members
                            .get(&format!("{}/{}", self.active_server_name(), friend))
                    })
                    .cloned();
                if let Some(user_id) = user_id {
                    let friend_name = friend.to_string();
                    cx.spawn(async move |this, cx| {
                        let result = net.execute(api::open_dm(&user_id, None), true).await;
                        let _ = this.update(cx, |app, cx| {
                            match result {
                                Ok(value) => {
                                    if value.get("kind").and_then(|kind| kind.as_str()) == Some("dm") {
                                        if let Some(dm_id) = value
                                            .get("dm")
                                            .and_then(|dm| dm.get("id"))
                                            .and_then(|id| id.as_str())
                                        {
                                            app.remote
                                                .dms
                                                .insert(friend_name.clone(), dm_id.to_string());
                                        }
                                    } else {
                                        app.message_request_notice =
                                            Some("对方不是好友，已发送消息请求".into());
                                    }
                                    app.refresh_structure(cx);
                                }
                                Err(error) => {
                                    app.friend_notice = Some(error.user_message());
                                    cx.notify();
                                }
                            }
                        });
                    })
                    .detach();
                    return;
                }
            }
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
        self.show_bookmarks = false;
        self.clear_bookmark_search(window, cx);
        self.bookmark_notice = None;
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
        self.automod_notice = None;
        self.close_poll_composer(window, cx);
        self.show_events = false;
        self.show_event_create = false;
        self.event_notice = None;
        self.show_message_requests = false;
        self.message_request_notice = None;
        self.forum_open = None;
        self.clear_forum_search(window, cx);
        self.show_forum_composer = false;
        self.forum_tags_draft.clear();
        self.forum_notice = None;
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

    /// 打开新建服务器弹层，清空输入草稿与模板选择。
    pub(crate) fn open_create_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.server_name_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.create_template = None;
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

    /// 点选模板卡片：再点一次取消选择，回到「从零创建」。
    pub(crate) fn select_create_template(&mut self, key: &str, cx: &mut Context<Self>) {
        self.create_template = if self.create_template.as_deref() == Some(key) {
            None
        } else {
            Some(key.to_string())
        };
        cx.notify();
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

    /// 创建服务器：按选中模板铺好分类与频道，写入专属欢迎屏幕并切换过去。
    pub(crate) fn create_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.server_name_input.read(cx).value().trim().to_string();
        let template = self
            .create_template
            .take()
            .and_then(|key| {
                crate::data::server_templates()
                    .iter()
                    .find(|template| template.key == key)
                    .copied()
            });
        // 名称留空时用模板名兜底，从零创建时仍然必须填名字。
        let name = if typed.is_empty() {
            template.map(|template| template.name.to_string()).unwrap_or_default()
        } else {
            typed
        };
        if name.is_empty() {
            return;
        }

        if self.is_online()
            && let Some(net) = self.net.clone()
        {
            let template_key = template.map(|template| template.key.to_string());
            self.server_name_input
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.show_create_server = false;
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::create_server(&name, template_key.as_deref(), false), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(value) => {
                        app.pending_server_id = value
                            .get("id")
                            .and_then(|id| id.as_str())
                            .map(str::to_string);
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.automod_notice = Some(format!("创建服务器失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }

        let server = match template {
            Some(template) => template.to_server(&name),
            None => {
                let color = self.next_server_color();
                Server::new(name.clone(), color)
            }
        };
        if let Some(template) = template {
            // 同名服务器的旧欢迎配置会被覆盖，避免残留上一份规则。
            self.welcome_screens.retain(|welcome| welcome.server != name);
            self.welcome_screens.push(template.to_welcome(&name));
        }

        self.servers.push(server);
        let index = self.servers.len() - 1;
        self.show_create_server = false;
        self.log_audit(
            AuditKind::Server,
            "创建了服务器",
            match template {
                Some(template) => format!("{name}（模板：{}）", template.name),
                None => name.clone(),
            },
        );
        self.select_server(index, window, cx);
        // 用模板建的服务器直接走一遍欢迎屏幕，介绍与规则才有机会被看到。
        if template.is_some() {
            self.open_welcome(name, cx);
        }
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
        if self.is_online() {
            let server_name = self.active_server_name();
            let server_id = self.remote.servers.get(&server_name).cloned();
            let category_id = if let Some(category) = self.create_channel_category.clone() {
                self.remote
                    .categories
                    .get(&format!("{server_name}/{category}"))
                    .cloned()
            } else {
                None
            };
            if let (Some(server_id), Some(net)) = (server_id, self.net.clone()) {
                self.channel_name_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.show_create_channel = false;
                self.pending_channel = Some(name.clone());
                cx.spawn(async move |this, cx| {
                    let result = net
                        .execute(
                            api::create_channel(
                                &server_id,
                                &name,
                                "text",
                                category_id.as_deref(),
                                &format!("欢迎来到 #{name}"),
                            ),
                            true,
                        )
                        .await;
                    let _ = this.update(cx, |app, cx| match result {
                        Ok(_) => app.refresh_structure(cx),
                        Err(error) => {
                            app.automod_notice = Some(format!("创建频道失败：{}", error.user_message()));
                            cx.notify();
                        }
                    });
                })
                .detach();
                return;
            }
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
        self.log_audit(AuditKind::Channel, "创建了频道", format!("#{name}"));
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
        if self.is_online()
            && let (Some(channel_id), Some(net)) = (
                self.remote
                    .channels
                    .get(&format!("{}/{}", self.active_server_name(), name))
                    .cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::set_channel_notify(&channel_id, notify_key(level)), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("通知设置失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
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
        self.mark_active_channel_read(cx);
        cx.notify();
    }

    /// 打开频道主题编辑弹层，输入框预填当前主题。
    pub(crate) fn open_topic_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.channel_menu.take().map(|menu| menu.name) else {
            return;
        };
        if !matches!(
            self.channel_kind(&name),
            Some(ChannelKind::Text) | Some(ChannelKind::Forum)
        ) {
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
        if self.is_online()
            && let (Some(channel_id), Some(net)) = (
                self.remote
                    .channels
                    .get(&format!("{}/{}", self.active_server_name(), name))
                    .cloned(),
                self.net.clone(),
            )
        {
            let topic_value = topic.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_channel(&channel_id, None, Some(&topic_value), None), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("保存主题失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
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

        let channel_id = self
            .remote
            .channels
            .get(&format!("{}/{}", self.active_server_name(), menu.name))
            .cloned();
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
        if self.is_online()
            && let (Some(channel_id), Some(net)) = (channel_id, self.net.clone())
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::delete_channel(&channel_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("删除频道失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
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
        let server_name = self.servers[self.active_server].name.clone();
        let key = format!("{server_name}/{}", menu.name);
        self.messages.remove(&key);
        // 论坛频道的帖子随频道一起清掉。
        self.forum_posts
            .retain(|post| !(post.server == server_name && post.channel == menu.name));
        // 指向该频道的邀请失效：撤销掉，免得列表里留下死链接。
        self.invites
            .retain(|invite| !(invite.server == server_name && invite.channel == menu.name));
        self.sync_invite_channel();
        self.log_audit(AuditKind::Channel, "删除了频道", format!("#{}", menu.name));

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
        if self.is_online()
            && let (Some(server_id), Some(net)) =
                (self.remote.servers.get(&self.active_server_name()).cloned(), self.net.clone())
        {
            let category_name = name.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::create_category(&server_id, &category_name), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("创建分类失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            self.category_input
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.show_create_category = false;
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
        if self.is_online()
            && let (Some(category_id), Some(net)) = (
                self.remote
                    .categories
                    .get(&format!("{server_name}/{source}"))
                    .cloned(),
                self.net.clone(),
            )
        {
            let new_name = value.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::patch_category(&category_id, &new_name), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("重命名分类失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
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
        if self.is_online()
            && let (Some(server_id), Some(net)) =
                (self.remote.servers.get(&self.active_server_name()).cloned(), self.net.clone())
        {
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::set_server_notify(&server_id, notify_key(level)), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("服务器通知设置失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
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

    /// 打开邀请弹层，链接按当前服务器最新一条邀请预填。
    pub(crate) fn open_invite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let link = format!("discord.gg/{}", self.primary_invite_code());
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
        if self.is_online()
            && let (Some(server_id), Some(net)) =
                (self.remote.servers.get(&name).cloned(), self.net.clone())
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::leave_server(&server_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("离开服务器失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let leaving_voice = self
            .voice_channel
            .as_ref()
            .map(|(index, _)| *index == self.active_server)
            .unwrap_or(false);
        self.servers.remove(self.active_server);
        // 该服务器的历史消息与活动一并清掉。
        let prefix = format!("{name}/");
        self.messages.retain(|key, _| !key.starts_with(&prefix));
        self.remove_events_of_server(&name);
        self.remove_forum_posts_of_server(&name);
        self.remove_bookmarks_of_server(&name);
        self.remove_welcome_of_server(&name);
        self.remove_boost_of_server(&name);

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

        // 自动审核：仅服务器频道生效。拦截则保留草稿、提示并中止发送。
        if self.view_mode == ViewMode::Server && !text.is_empty() {
            if let Some((action, rule, keyword)) = self.automod_check(&text) {
                match action {
                    AutoModAction::Block => {
                        self.automod_notice =
                            Some(format!("消息被「{rule}」拦截（命中关键词：{keyword}）"));
                        cx.notify();
                        return;
                    }
                    AutoModAction::Warn => {
                        self.automod_notice =
                            Some(format!("提醒：「{rule}」命中关键词 {keyword}，消息仍会发送"));
                    }
                }
            }
        }

        self.composer
            .update(cx, |input, cx| input.set_value("", window, cx));
        // 已发送的文本不再算草稿，清掉侧栏可能的草稿标记。
        let sent_session = self.current_session.clone();
        self.drafts.remove(&sent_session);
        let mut message = Message::new("你", "刚刚", text.clone(), palette::accent());
        if let Some(attachment) = self.attachment_draft.take() {
            message = message.attachment(attachment);
        }
        // 把回复引用写进新消息，再清空草稿状态。
        if let Some(reply) = self.reply_to.take() {
            message = message.replying_to(reply.author, reply.excerpt);
        }

        let key = self.session_key();
        let channel_id = self.remote.channels.get(&key).cloned();
        let online_send = self.is_online() && channel_id.is_some() && !text.is_empty();
        let nonce = if online_send {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or_default();
            Some(format!("c-{stamp}-{}", self.messages.get(&key).map_or(0, Vec::len)))
        } else {
            None
        };
        if let Some(nonce) = nonce.clone() {
            message.client_nonce = Some(nonce);
            message.delivery = DeliveryStatus::Sending;
        }
        if let Some(messages) = self.current_messages_mut() {
            messages.push(message);
        }
        if let (true, Some(channel_id), Some(nonce), Some(net)) = (
            online_send,
            channel_id,
            nonce,
            self.net.clone(),
        ) {
            let key_for_send = key.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::send_message(&channel_id, &text, None, Some(&nonce)), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    app.finish_send(&key_for_send, &nonce, result);
                    cx.notify();
                });
            })
            .detach();
        } else if self.is_online()
            && self.view_mode == ViewMode::DirectMessage
            && !text.is_empty()
            && let Some(friend) = self.active_dm.clone()
            && let Some(dm_id) = self.remote.dms.get(&friend).cloned()
            && let Some(net) = self.net.clone()
        {
            let nonce = format!(
                "d-{}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_millis())
                    .unwrap_or_default(),
                self.dm_messages.get(&friend).map_or(0, Vec::len)
            );
            if let Some(message) = self
                .dm_messages
                .get_mut(&friend)
                .and_then(|messages| messages.last_mut())
            {
                message.client_nonce = Some(nonce.clone());
                message.delivery = DeliveryStatus::Sending;
            }
            let friend_for_send = friend.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::send_dm_message(&dm_id, &text, Some(&nonce)), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    app.finish_dm_send(&friend_for_send, &nonce, result);
                    cx.notify();
                });
            })
            .detach();
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
        if self.welcome_target.is_some() {
            // 欢迎屏幕是顶层模态，Esc 先关它（等价于「稍后再说」）。
            self.close_welcome(cx);
        } else if self.show_member_nick {
            self.close_member_nick(cx);
        } else if self.show_create_group || self.show_rename_group {
            self.close_group_dialog(cx);
        } else if self.friend_group_picker.is_some() {
            self.friend_group_picker = None;
        } else if self.friend_group_menu.is_some() {
            self.friend_group_menu = None;
        } else if self.editing_message.is_some() {
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
        } else if self.show_forum_composer {
            self.close_forum_composer(window, cx);
        } else if self.forum_open.is_some() {
            self.close_forum_post(cx);
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
        } else if self.show_bookmarks {
            self.show_bookmarks = false;
        } else if self.show_events {
            self.close_events(cx);
        } else if self.show_message_requests {
            self.close_message_requests(cx);
        } else if self.show_inbox {
            self.show_inbox = false;
        } else if self.show_status_menu {
            self.show_status_menu = false;
        } else if self.show_poll_composer {
            self.close_poll_composer(window, cx);
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
        let mut request = None;
        if let Some(messages) = self.current_messages_mut() {
            if let Some(message) = messages.get_mut(index) {
                if let Some(reaction) = message.reactions.get_mut(reaction_index) {
                    reaction.mine = !reaction.mine;
                    reaction.count = if reaction.mine {
                        reaction.count + 1
                    } else {
                        reaction.count.saturating_sub(1)
                    };
                    if !message.id.is_empty() {
                        request = Some((message.id.clone(), reaction.emoji, reaction.mine));
                    }
                }
            }
        }
        if let (Some((message_id, emoji, mine)), Some(net)) = (request, self.net.clone()) {
            cx.spawn(async move |this, cx| {
                let call = if mine {
                    api::add_reaction(&message_id, emoji)
                } else {
                    api::remove_reaction(&message_id, emoji)
                };
                if let Err(error) = net.execute(call, true).await {
                    let _ = this.update(cx, |app, cx| {
                        app.automod_notice = Some(format!("回应失败：{}", error.user_message()));
                        cx.notify();
                    });
                }
            })
            .detach();
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

    /// 滚动到指定消息并高亮。滚动容器的 item 下标要算上顶栏提示与未读分隔线。
    fn reveal_message(&mut self, index: usize) {
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

    /// 点击消息里的回复引用：按摘要反查原消息，滚动过去并高亮。
    pub(crate) fn jump_to_reply(&mut self, excerpt: &str, cx: &mut Context<Self>) {
        let target = self.current_messages().iter().position(|m| {
            let text = m.text.trim();
            text.starts_with(excerpt.trim_end_matches('…'))
        });
        if let Some(index) = target {
            self.reveal_message(index);
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
        let mut request = None;
        if let Some(message) = self.current_messages_mut().and_then(|m| m.get_mut(index)) {
            message.pinned = !message.pinned;
            if !message.id.is_empty() {
                request = Some((message.id.clone(), message.pinned));
            }
        }
        if let (Some((id, pinned)), Some(net)) = (request, self.net.clone()) {
            cx.spawn(async move |this, cx| {
                if let Err(error) = net.execute(api::pin_message(&id, pinned), true).await {
                    let _ = this.update(cx, |app, cx| {
                        app.automod_notice = Some(format!("置顶失败：{}", error.user_message()));
                        cx.notify();
                    });
                }
            })
            .detach();
        }
        self.close_message_menu();
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 消息投票
    // -----------------------------------------------------------------------

    /// 打开/关闭投票创建弹层；打开时互斥收起其他输入浮层。
    pub(crate) fn toggle_poll_composer(&mut self, cx: &mut Context<Self>) {
        let opening = !self.show_poll_composer;
        self.show_poll_composer = opening;
        if opening {
            self.show_attachment_picker = false;
            self.show_sticker_picker = false;
            self.show_gif_picker = false;
            self.emoji_picker_open = false;
            self.emoji_target = None;
            self.show_status_menu = false;
            self.show_server_menu = false;
            self.close_message_menu();
        }
        cx.notify();
    }

    /// 关闭投票创建弹层并清空草稿（切会话 / 发送成功后调用）。
    pub(crate) fn close_poll_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_poll_composer = false;
        self.poll_options_draft.clear();
        self.poll_multiple = false;
        self.poll_question_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.poll_option_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// 把选项输入框的内容加入草稿（去空白、去重、最多 8 项）。
    pub(crate) fn add_poll_option(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.poll_option_edit.read(cx).value().trim().to_string();
        if value.is_empty()
            || self.poll_options_draft.len() >= 8
            || self.poll_options_draft.iter().any(|item| item == &value)
        {
            return;
        }
        self.poll_options_draft.push(value);
        self.poll_option_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    /// 删除草稿中指定下标的选项。
    pub(crate) fn remove_poll_option(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.poll_options_draft.len() {
            self.poll_options_draft.remove(index);
            cx.notify();
        }
    }

    /// 切换"多选"草稿开关。
    pub(crate) fn toggle_poll_multiple(&mut self, cx: &mut Context<Self>) {
        self.poll_multiple = !self.poll_multiple;
        cx.notify();
    }

    /// 用当前草稿发起投票：至少 1 个选项时把投票挂到一条新消息并发送。
    /// 问题为空时回退到"投票"作为标题；单选/多选取自草稿开关。
    pub(crate) fn send_poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.poll_options_draft.len() < 2 {
            return;
        }
        let question = {
            let raw = self.poll_question_edit.read(cx).value().trim().to_string();
            if raw.is_empty() {
                "投票".to_string()
            } else {
                raw
            }
        };
        let labels: Vec<&str> = self
            .poll_options_draft
            .iter()
            .map(|option| option.as_str())
            .collect();
        let poll = Poll::new(question.clone(), &labels, self.poll_multiple);
        let mut message = Message::new("你", "刚刚", String::new(), palette::accent()).poll(poll);
        if let Some(reply) = self.reply_to.take() {
            message = message.replying_to(reply.author, reply.excerpt);
        }
        let sent_session = self.current_session.clone();
        self.drafts.remove(&sent_session);
        if let Some(messages) = self.current_messages_mut() {
            messages.push(message);
        }
        if self.is_online()
            && let (Some(channel_id), Some(net)) = (
                self.remote.channels.get(&self.active_key()).cloned(),
                self.net.clone(),
            )
        {
            let labels_owned = self.poll_options_draft.clone();
            let multi = self.poll_multiple;
            let question_for_poll = question.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(
                        api::create_poll(&channel_id, &question_for_poll, &labels_owned, multi),
                        true,
                    )
                    .await;
                let _ = this.update(cx, |app, cx| {
                    match result {
                        Ok(value) => {
                            if let Ok(dto) = serde_json::from_value::<MessageDto>(value)
                                && let Some(list) = app.messages.get_mut(&sent_session)
                                && let Some(index) = list
                                    .iter()
                                    .rposition(|message| message.id.is_empty() && message.poll.is_some())
                            {
                                let mut converted = hydrate::message_from(&dto);
                                if dto.author.id == app.remote.me_id {
                                    converted.author = "你";
                                }
                                list[index] = converted;
                            }
                            cx.notify();
                        }
                        Err(error) => {
                            app.automod_notice = Some(format!("发起投票失败：{}", error.user_message()));
                            cx.notify();
                        }
                    }
                });
            })
            .detach();
        }
        self.log_audit(AuditKind::Server, "发起了投票", question);
        self.close_poll_composer(window, cx);
        self.clear_search(window, cx);
        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }

    /// 给当前会话第 index 条消息的投票投一票 / 取消自己的票。
    /// 单选会先移除"你"在其它选项上的票；已投的选项再点即取消。
    pub(crate) fn vote_poll(
        &mut self,
        index: usize,
        option_index: usize,
        cx: &mut Context<Self>,
    ) {
        let me = "你";
        let mut changed = false;
        let mut question = String::new();
        let mut remote_vote = None;
        if self.is_online()
            && let Some(net) = self.net.clone()
            && let Some(Some(poll)) = self
                .current_messages()
                .get(index)
                .map(|message| message.poll.as_ref())
        {
            if let Some(option) = poll.options.get(option_index) {
                let already = option.voters.iter().any(|name| name == me);
                if !poll.closed {
                    remote_vote = Some((
                        net,
                        poll.id.clone(),
                        option.id.clone(),
                        already,
                        poll.multiple,
                    ));
                }
            }
        }
        if let Some(Some(poll)) = self
            .current_messages_mut()
            .and_then(|messages| messages.get_mut(index))
            .map(|message| message.poll.as_mut())
        {
            if !poll.closed && option_index < poll.options.len() {
                question = poll.question.clone();
                let already = poll.options[option_index]
                    .voters
                    .iter()
                    .any(|name| name == me);
                if !poll.multiple {
                    for option in poll.options.iter_mut() {
                        option.voters.retain(|name| name != me);
                    }
                }
                if already {
                    poll.options[option_index].voters.retain(|name| name != me);
                } else {
                    poll.options[option_index].voters.push(me.to_string());
                }
                changed = true;
            }
        }
        if let Some((net, poll_id, option_id, already, multiple)) = remote_vote {
            let call = if already && !multiple {
                api::vote_poll(&poll_id, &option_id)
            } else {
                api::vote_poll(&poll_id, &option_id)
            };
            cx.spawn(async move |this, cx| {
                let result = net.execute(call, true).await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("投票失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        if changed {
            self.log_audit(
                AuditKind::Server,
                "参与了投票",
                format!("{question} · 选项 {}", option_index + 1),
            );
            cx.notify();
        }
    }

    /// 结束当前会话第 index 条消息的投票（仅自己发起的投票可结束）。
    pub(crate) fn close_poll(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut question = String::new();
        let mut changed = false;
        let mut remote_close = None;
        if self.is_online()
            && let (Some(net), Some(poll_id)) = (
                self.net.clone(),
                self.current_messages()
                    .get(index)
                    .and_then(|message| message.poll.as_ref())
                    .map(|poll| poll.id.clone())
                    .filter(|id| !id.is_empty()),
            )
        {
            remote_close = Some((net, poll_id));
        }
        if let Some(message) = self.current_messages_mut().and_then(|m| m.get_mut(index))
            && let Some(poll) = message.poll.as_mut()
            && !poll.closed
            && message.author == "你"
        {
            question = poll.question.clone();
            poll.closed = true;
            changed = true;
        }
        if let Some((net, poll_id)) = remote_close {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::close_poll(&poll_id), true).await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("结束投票失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        if changed {
            self.log_audit(AuditKind::Server, "结束了投票", question);
            cx.notify();
        }
    }

    // -----------------------------------------------------------------------
    // 服务器日程 / 活动
    // -----------------------------------------------------------------------

    /// 当前服务器名（日程按此归属隔离）。
    pub(crate) fn active_server_name(&self) -> String {
        self.servers
            .get(self.active_server)
            .map(|server| server.name.clone())
            .unwrap_or_default()
    }

    /// 当前服务器的活动，按开始时间升序（越近越靠前）。
    pub(crate) fn server_events(&self) -> Vec<ServerEvent> {
        let server = self.active_server_name();
        let mut list: Vec<ServerEvent> = self
            .events
            .iter()
            .filter(|event| event.server == server)
            .cloned()
            .collect();
        list.sort_by_key(|event| (event.starts_in_minutes, event.title.clone()));
        list
    }

    /// 按状态筛选后的活动。
    pub(crate) fn visible_events(&self) -> Vec<ServerEvent> {
        let all = self.server_events();
        if self.event_filter == EventFilter::All {
            return all;
        }
        all.into_iter()
            .filter(|event| {
                let live = event.status() == crate::data::ServerEventStatus::Live;
                let ended = event.status() == crate::data::ServerEventStatus::Ended;
                match self.event_filter {
                    EventFilter::Upcoming => !live && !ended,
                    EventFilter::Live => live,
                    EventFilter::Ended => ended,
                    EventFilter::All => true,
                }
            })
            .collect()
    }

    /// 打开/关闭日程面板（与 pins/inbox 互斥），关闭时收起创建表单。
    pub(crate) fn toggle_events(&mut self, cx: &mut Context<Self>) {
        self.show_events = !self.show_events;
        if self.show_events {
            self.show_pins = false;
            self.show_inbox = false;
            self.show_message_requests = false;
            self.show_bookmarks = false;
            self.event_notice = None;
        } else {
            self.show_event_create = false;
        }
        cx.notify();
    }

    pub(crate) fn close_events(&mut self, cx: &mut Context<Self>) {
        self.show_events = false;
        self.show_event_create = false;
        self.event_notice = None;
        cx.notify();
    }

    pub(crate) fn set_event_filter(&mut self, filter: EventFilter, cx: &mut Context<Self>) {
        self.event_filter = filter;
        cx.notify();
    }

    pub(crate) fn select_event(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected_event = Some(id);
        self.event_notice = None;
        cx.notify();
    }

    /// 展开/收起创建活动表单，展开时清空上次草稿。
    pub(crate) fn toggle_event_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let opening = !self.show_event_create;
        self.show_event_create = opening;
        if opening {
            self.event_notice = None;
            self.event_channel = "一般".into();
            self.event_category = EventCategory::Community;
            self.event_starts_in = 60;
            self.event_duration = 60;
            self.event_title_edit
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.event_desc_edit
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
    }

    pub(crate) fn set_event_category(
        &mut self,
        category: EventCategory,
        cx: &mut Context<Self>,
    ) {
        self.event_category = category;
        cx.notify();
    }

    pub(crate) fn set_event_channel(&mut self, channel: String, cx: &mut Context<Self>) {
        self.event_channel = channel;
        cx.notify();
    }

    /// 调整开始偏移（步进 15 分钟，范围 -240 ~ +10080 即一周）。
    pub(crate) fn adjust_event_start(&mut self, delta: i32, cx: &mut Context<Self>) {
        self.event_starts_in = (self.event_starts_in + delta).clamp(-240, 10080);
        cx.notify();
    }

    /// 调整时长（步进 15 分钟，范围 15 ~ 480）。
    pub(crate) fn adjust_event_duration(&mut self, delta: i32, cx: &mut Context<Self>) {
        self.event_duration = (self.event_duration as i32 + delta)
            .clamp(15, 480) as u32;
        cx.notify();
    }

    /// 用草稿创建活动：标题非空即可，归属当前服务器，发起人"你"。
    pub(crate) fn create_event(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.event_title_edit.read(cx).value().trim().to_string();
        if title.is_empty() {
            self.event_notice = Some("请先填写活动标题".into());
            cx.notify();
            return;
        }
        let description = self.event_desc_edit.read(cx).value().trim().to_string();
        let server = self.active_server_name();
        if self.is_online()
            && let (Some(server_id), Some(net)) = (
                self.remote.servers.get(&server).cloned(),
                self.net.clone(),
            )
        {
            let category = event_category_key(self.event_category);
            let channel_id = self
                .remote
                .channels
                .get(&format!("{server}/{}", self.event_channel))
                .cloned();
            let starts_at = iso_utc_after_minutes(self.event_starts_in as i64);
            let duration = self.event_duration;
            let description_for_event = description.clone();
            let title_for_event = title.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(
                        api::create_event(
                            &server_id,
                            &title_for_event,
                            &description_for_event,
                            category,
                            channel_id.as_deref(),
                            &starts_at,
                            duration,
                        ),
                        true,
                    )
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.event_notice = Some(format!("活动「{title_for_event}」已创建"));
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.event_notice = Some(format!("创建活动失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            self.show_event_create = false;
            self.event_title_edit
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.event_desc_edit
                .update(cx, |input, cx| input.set_value("", window, cx));
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let id = format!("evt-{}-{stamp}", server);
        let mut event = ServerEvent::new(
            id.clone(),
            server,
            title.clone(),
            self.event_channel.clone(),
            "你",
            self.event_starts_in,
            self.event_duration,
            self.event_category,
        );
        event.description = description;
        event.attendees.push("你".into());
        self.events.push(event);
        self.log_audit(AuditKind::Server, "创建了活动", title.clone());
        self.selected_event = Some(id);
        self.event_filter = EventFilter::All;
        self.show_event_create = false;
        self.event_title_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.event_desc_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.event_notice = Some(format!("活动「{title}」已创建，你已自动报名"));
        cx.notify();
    }

    /// 删除指定 id 的活动（不限归属，按 id 精确匹配）。
    pub(crate) fn delete_event(&mut self, id: String, cx: &mut Context<Self>) {
        if self.is_online()
            && !id.starts_with("evt-")
            && let Some(net) = self.net.clone()
        {
            let event_id = id.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::delete_event(&event_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.event_notice = Some("活动已删除".into());
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.event_notice = Some(format!("删除活动失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let before = self.events.len();
        self.events.retain(|event| event.id != id);
        if self.events.len() == before {
            return;
        }
        if self.selected_event.as_deref() == Some(id.as_str()) {
            self.selected_event = None;
        }
        self.log_audit(AuditKind::Server, "删除了活动", id);
        self.event_notice = Some("活动已删除".into());
        cx.notify();
    }

    /// 报名 / 取消报名指定活动（仅本人操作本地名单）。
    pub(crate) fn toggle_attendance(&mut self, id: String, cx: &mut Context<Self>) {
        if self.is_online()
            && !id.starts_with("evt-")
            && let Some(net) = self.net.clone()
        {
            let going = self
                .events
                .iter()
                .find(|event| event.id == id)
                .map(|event| !event.is_attending("你"))
                .unwrap_or(true);
            let event_id = id.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::rsvp_event(&event_id, going), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.event_notice = Some(format!("报名失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let mut title = String::new();
        let mut joined = false;
        if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
            title = event.title.clone();
            if event.is_attending("你") {
                event.attendees.retain(|name| name != "你");
            } else {
                event.attendees.push("你".into());
                joined = true;
            }
        }
        if !title.is_empty() {
            self.log_audit(
                AuditKind::Server,
                if joined { "报名了活动" } else { "取消了活动报名" },
                title.clone(),
            );
            self.event_notice = Some(if joined {
                format!("已报名「{title}」")
            } else {
                format!("已取消报名「{title}」")
            });
            cx.notify();
        }
    }

    pub(crate) fn dismiss_event_notice(&mut self, cx: &mut Context<Self>) {
        self.event_notice = None;
        cx.notify();
    }

    // ---------------------------- 消息请求 ----------------------------

    /// 待处理请求数（侧栏角标用）。
    pub(crate) fn pending_request_count(&self) -> usize {
        self.message_requests.iter().filter(|r| r.is_pending()).count()
    }

    /// 某个筛选下的请求数（chip 计数用）。
    pub(crate) fn request_count_for(&self, filter: MessageRequestFilter) -> usize {
        self.message_requests
            .iter()
            .filter(|r| r.visible && state_matches_filter(r.state, filter))
            .count()
    }

    /// 当前筛选下可见的请求（按收到时间升序，越早越靠前）。
    pub(crate) fn visible_message_requests(&self) -> Vec<&MessageRequest> {
        let mut items: Vec<&MessageRequest> = self
            .message_requests
            .iter()
            .filter(|r| r.visible && state_matches_filter(r.state, self.message_request_filter))
            .collect();
        items.sort_by_key(|r| r.minutes_ago);
        items
    }

    /// 打开/关闭消息请求面板（与 pins/inbox/events 互斥）。
    pub(crate) fn toggle_message_requests(&mut self, cx: &mut Context<Self>) {
        self.show_message_requests = !self.show_message_requests;
        if self.show_message_requests {
            self.show_pins = false;
            self.show_inbox = false;
            self.show_events = false;
            self.show_bookmarks = false;
            self.message_request_notice = None;
        }
        cx.notify();
    }

    pub(crate) fn close_message_requests(&mut self, cx: &mut Context<Self>) {
        self.show_message_requests = false;
        self.message_request_notice = None;
        cx.notify();
    }

    pub(crate) fn set_message_request_filter(
        &mut self,
        filter: MessageRequestFilter,
        cx: &mut Context<Self>,
    ) {
        self.message_request_filter = filter;
        cx.notify();
    }

    /// 接受请求：标记已接受，真正建立私信会话并切入（复用 start_dm）。
    pub(crate) fn accept_message_request(
        &mut self,
        sender: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_online()
            && let Some(request_id) = self
                .message_requests
                .iter()
                .find(|request| request.sender == sender && request.visible)
                .map(|request| request.id.clone())
                .filter(|id| !id.is_empty())
            && let Some(net) = self.net.clone()
        {
            let notify_sender = sender.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::accept_message_request(&request_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.message_request_notice = Some(format!("已接受 {notify_sender} 的私信请求"));
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.message_request_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        // 先把请求里的字段取成 owned，释放可变借用后再调用其它 &mut self 方法。
        let (color, preview) = {
            let Some(request) = self
                .message_requests
                .iter_mut()
                .find(|r| r.sender == sender && r.visible)
            else {
                return;
            };
            request.state = MessageRequestState::Accepted;
            (request.color, request.preview.clone())
        };
        self.log_audit(AuditKind::Member, "接受了私信请求", sender.clone());
        self.message_request_notice = Some(format!("已接受 {sender} 的私信请求"));
        // 把请求里的预览与颜色带入新会话（若会话不存在）。
        if !self.dm_conversations.iter().any(|c| c.friend == sender) {
            let friend: &'static str = Box::leak(sender.clone().into_boxed_str());
            self.dm_conversations.push(DmConversation {
                friend,
                color,
                online: false,
                unread: 0,
                preview: Box::leak(preview.into_boxed_str()),
            });
            self.dm_messages.entry(sender.clone()).or_default();
        }
        let friend: &'static str = Box::leak(sender.clone().into_boxed_str());
        self.show_message_requests = false;
        self.open_dm(friend.to_string(), window, cx);
    }

    /// 忽略请求：标记已忽略，不再计入待处理角标。
    pub(crate) fn ignore_message_request(&mut self, sender: String, cx: &mut Context<Self>) {
        if self.is_online()
            && let Some(request_id) = self
                .message_requests
                .iter()
                .find(|request| request.sender == sender && request.visible)
                .map(|request| request.id.clone())
                .filter(|id| !id.is_empty())
            && let Some(net) = self.net.clone()
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::ignore_message_request(&request_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.message_request_notice = Some("已忽略消息请求".into());
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.message_request_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        {
            let Some(request) = self
                .message_requests
                .iter_mut()
                .find(|r| r.sender == sender && r.visible)
            else {
                return;
            };
            request.state = MessageRequestState::Ignored;
        }
        self.log_audit(AuditKind::Member, "忽略了私信请求", sender.clone());
        self.message_request_notice = Some(format!("已忽略 {sender} 的私信请求"));
        cx.notify();
    }

    /// 删除请求：标记不可见（不物理删除，保留审计一致性）。
    pub(crate) fn delete_message_request(&mut self, sender: String, cx: &mut Context<Self>) {
        if self.is_online()
            && let Some(request_id) = self
                .message_requests
                .iter()
                .find(|request| request.sender == sender && request.visible)
                .map(|request| request.id.clone())
                .filter(|id| !id.is_empty())
            && let Some(net) = self.net.clone()
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::delete_message_request(&request_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.message_request_notice = Some("请求已删除".into());
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.message_request_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        {
            let Some(request) = self
                .message_requests
                .iter_mut()
                .find(|r| r.sender == sender && r.visible)
            else {
                return;
            };
            request.visible = false;
        }
        self.log_audit(AuditKind::Member, "删除了私信请求", sender.clone());
        self.message_request_notice = Some("请求已删除".into());
        cx.notify();
    }

    pub(crate) fn dismiss_message_request_notice(&mut self, cx: &mut Context<Self>) {
        self.message_request_notice = None;
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 论坛频道
    // -----------------------------------------------------------------------

    /// 当前是否停留在论坛频道（服务器视图 + 当前频道为 Forum）。
    pub(crate) fn is_forum_view(&self) -> bool {
        if self.view_mode != ViewMode::Server {
            return false;
        }
        self.channel_kind(&self.active_channel) == Some(ChannelKind::Forum)
    }

    /// 当前论坛帖子的归属键：(服务器名, 频道名)。
    fn forum_key(&self) -> (String, String) {
        (self.active_server_name(), self.active_channel.clone())
    }

    /// 当前频道下、按搜索词与排序过滤后的帖子（置顶恒定在前）。
    pub(crate) fn forum_visible_posts(&self) -> Vec<ForumPost> {
        let (server, channel) = self.forum_key();
        let query = self.forum_query.to_lowercase();
        let mut posts: Vec<ForumPost> = self
            .forum_posts
            .iter()
            .filter(|post| post.server == server && post.channel == channel && post.matches(&query))
            .cloned()
            .collect();
        // 未回应筛选：只保留零回帖的帖子。
        if self.forum_sort == ForumSort::Unanswered {
            posts.retain(|post| post.reply_count() == 0);
        }
        posts.sort_by(|a, b| {
            // 置顶优先，其次按排序方式。
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| match self.forum_sort {
                    ForumSort::Recent => a.last_activity().cmp(&b.last_activity()),
                    ForumSort::Popular => b.reply_count().cmp(&a.reply_count()),
                    ForumSort::Unanswered => a.minutes_ago.cmp(&b.minutes_ago),
                })
        });
        posts
    }

    /// 当前打开的帖子详情（按 id 精确查找）。
    pub(crate) fn forum_open_post(&self) -> Option<ForumPost> {
        let id = self.forum_open.as_deref()?;
        self.forum_posts.iter().find(|post| post.id == id).cloned()
    }

    pub(crate) fn set_forum_sort(&mut self, sort: ForumSort, cx: &mut Context<Self>) {
        self.forum_sort = sort;
        cx.notify();
    }

    pub(crate) fn open_forum_post(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.forum_open = Some(id.clone());
        self.forum_notice = None;
        self.forum_reply_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        if self.is_online()
            && let Some(net) = self.net.clone()
        {
            let post_id = id.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute_as::<crate::net::dto::ForumPostDetailDto>(api::forum_post(&post_id), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Ok(detail) = result {
                        if let Some(post) =
                            app.forum_posts.iter_mut().find(|post| post.id == post_id)
                        {
                            post.replies = detail
                                .replies
                                .iter()
                                .map(|reply| ForumReply::new(
                                    reply.author.name.clone(),
                                    reply.author.color,
                                    reply.body.clone(),
                                    0,
                                ))
                                .collect();
                            post.reply_count_override = None;
                        }
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn close_forum_post(&mut self, cx: &mut Context<Self>) {
        self.forum_open = None;
        self.forum_notice = None;
        cx.notify();
    }

    pub(crate) fn toggle_forum_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_forum_view() {
            return;
        }
        let opening = !self.show_forum_composer;
        self.show_forum_composer = opening;
        self.forum_title_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.forum_body_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.forum_tag_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.forum_tags_draft.clear();
        self.forum_notice = None;
        cx.notify();
    }

    pub(crate) fn close_forum_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_forum_composer = false;
        self.forum_title_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.forum_body_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.forum_tag_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.forum_tags_draft.clear();
        cx.notify();
    }

    /// 发帖弹层里回车添加标签（去重，最多 5 个）。
    pub(crate) fn add_forum_tag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tag = self.forum_tag_edit.read(cx).value().trim().to_string();
        if tag.is_empty() {
            return;
        }
        if !self.forum_tags_draft.iter().any(|item| item == &tag) {
            if self.forum_tags_draft.len() >= 5 {
                self.forum_notice = Some("最多添加 5 个标签".into());
            } else {
                self.forum_tags_draft.push(tag);
            }
        }
        self.forum_tag_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    pub(crate) fn remove_forum_tag(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.forum_tags_draft.len() {
            self.forum_tags_draft.remove(index);
            cx.notify();
        }
    }

    /// 发帖：标题必填，写入当前服务器/频道，成功后自动打开详情。
    pub(crate) fn create_forum_post(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_forum_view() {
            return;
        }
        let title = self.forum_title_edit.read(cx).value().trim().to_string();
        if title.is_empty() {
            self.forum_notice = Some("请先填写帖子标题".into());
            cx.notify();
            return;
        }
        let body = self.forum_body_edit.read(cx).value().trim().to_string();
        let (server, channel) = self.forum_key();
        if self.is_online()
            && let (Some(channel_id), Some(net)) = (
                self.remote
                    .channels
                    .get(&format!("{server}/{channel}"))
                    .cloned(),
                self.net.clone(),
            )
        {
            let tags = self.forum_tags_draft.clone();
            let title_for_post = title.clone();
            self.close_forum_composer(window, cx);
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::create_forum_post(&channel_id, &title_for_post, &body, &tags), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(value) => {
                        if let Some(post_id) = value.get("id").and_then(|id| id.as_str()) {
                            app.pending_forum_post = Some(post_id.to_string());
                        }
                        app.forum_notice = Some(format!("帖子「{title_for_post}」已发布"));
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.forum_notice = Some(format!("发帖失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let id = format!("forum-{}-{stamp}", self.active_server);
        let mut post = ForumPost::new(
            id.clone(),
            server,
            channel,
            title.clone(),
            self.me.display_name.clone(),
            self.me.color,
            body,
            0,
        );
        post.tags = self.forum_tags_draft.clone();
        self.forum_posts.push(post);
        self.log_audit(AuditKind::Channel, "发布了论坛帖子", title.clone());
        self.close_forum_composer(window, cx);
        self.forum_open = Some(id);
        self.forum_notice = Some(format!("帖子「{title}」已发布"));
        cx.notify();
    }

    /// 回帖：锁定的帖子不可回复。
    pub(crate) fn send_forum_reply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.forum_open.clone() else {
            return;
        };
        let body = self.forum_reply_edit.read(cx).value().trim().to_string();
        if body.is_empty() {
            return;
        }
        let online_reply = self
            .is_online()
            .then(|| {
                self.net
                    .clone()
                    .map(|net| (net, id.clone(), body.clone()))
            })
            .flatten();
        let Some(post) = self.forum_posts.iter_mut().find(|post| post.id == id) else {
            return;
        };
        if post.locked {
            self.forum_notice = Some("帖子已锁定，无法回复".into());
            cx.notify();
            return;
        }
        post.replies.push(ForumReply::new(
            self.me.display_name.clone(),
            self.me.color,
            body,
            0,
        ));
        if let Some((net, post_id, reply_body)) = online_reply {
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::create_forum_reply(&post_id, &reply_body), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.forum_notice = Some(format!("回帖失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        self.forum_reply_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.forum_notice = Some("回复已发布".into());
        cx.notify();
    }

    pub(crate) fn toggle_forum_pin(&mut self, id: String, cx: &mut Context<Self>) {
        let mut pinned = false;
        let mut title = String::new();
        if let Some(post) = self.forum_posts.iter_mut().find(|post| post.id == id) {
            post.pinned = !post.pinned;
            pinned = post.pinned;
            title = post.title.clone();
        }
        if !title.is_empty() {
            if self.is_online()
                && let Some(net) = self.net.clone()
            {
                let post_id = id.clone();
                cx.spawn(async move |this, cx| {
                    let result = net
                        .execute(api::patch_forum_post(&post_id, Some(pinned), None), true)
                        .await;
                    let _ = this.update(cx, |app, cx| {
                        if let Err(error) = result {
                            app.forum_notice = Some(format!("置顶失败：{}", error.user_message()));
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            self.log_audit(
                AuditKind::Channel,
                if pinned { "置顶了论坛帖子" } else { "取消置顶论坛帖子" },
                title.clone(),
            );
            self.forum_notice = Some(if pinned {
                format!("已置顶「{title}」")
            } else {
                format!("已取消置顶「{title}」")
            });
            cx.notify();
        }
    }

    pub(crate) fn toggle_forum_lock(&mut self, id: String, cx: &mut Context<Self>) {
        let mut locked = false;
        let mut title = String::new();
        if let Some(post) = self.forum_posts.iter_mut().find(|post| post.id == id) {
            post.locked = !post.locked;
            locked = post.locked;
            title = post.title.clone();
        }
        if !title.is_empty() {
            if self.is_online()
                && let Some(net) = self.net.clone()
            {
                let post_id = id.clone();
                cx.spawn(async move |this, cx| {
                    let result = net
                        .execute(api::patch_forum_post(&post_id, None, Some(locked)), true)
                        .await;
                    let _ = this.update(cx, |app, cx| {
                        if let Err(error) = result {
                            app.forum_notice = Some(format!("锁定失败：{}", error.user_message()));
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            self.log_audit(
                AuditKind::Channel,
                if locked { "锁定了论坛帖子" } else { "解锁了论坛帖子" },
                title.clone(),
            );
            self.forum_notice = Some(if locked {
                format!("已锁定「{title}」，不再接受回复")
            } else {
                format!("已解锁「{title}」")
            });
            cx.notify();
        }
    }

    pub(crate) fn dismiss_forum_notice(&mut self, cx: &mut Context<Self>) {
        self.forum_notice = None;
        cx.notify();
    }

    pub(crate) fn clear_forum_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.forum_query.clear();
        self.forum_search
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// 删除服务器时清掉其名下所有帖子。
    pub(crate) fn remove_forum_posts_of_server(&mut self, server: &str) {
        self.forum_posts.retain(|post| post.server != server);
    }

    /// 服务器/频道改名时迁移论坛帖子的归属。
    pub(crate) fn rename_forum_server(&mut self, old: &str, new: &str) {
        for post in self.forum_posts.iter_mut() {
            if post.server == old {
                post.server = new.to_string();
            }
        }
    }

    // -----------------------------------------------------------------------
    // 消息书签
    // -----------------------------------------------------------------------

    /// 当前会话的定位三元组：(是否私信, 服务器名, 频道 / 好友名)。
    fn session_scope(&self) -> (bool, String, String) {
        if self.is_dm() {
            (true, String::new(), self.active_dm.clone().unwrap_or_default())
        } else {
            (
                false,
                self.servers[self.active_server].name.clone(),
                self.active_channel.clone(),
            )
        }
    }

    /// 当前会话里的某条消息是否已收藏。
    pub(crate) fn is_bookmarked(&self, index: usize) -> bool {
        let Some(message) = self.current_messages().get(index).cloned() else {
            return false;
        };
        let body = bookmark_body(&message);
        let (is_dm, server, channel) = self.session_scope();
        self.bookmarks
            .iter()
            .any(|b| b.in_session(is_dm, &server, &channel) && b.text == body)
    }

    /// 收藏 / 取消收藏当前会话里的某条消息（消息菜单入口）。
    pub(crate) fn toggle_bookmark(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(message) = self.current_messages().get(index).cloned() else {
            return;
        };
        let body = bookmark_body(&message);
        let (is_dm, server, channel) = self.session_scope();
        let existing = if !message.id.is_empty() {
            self.bookmarks
                .iter()
                .position(|b| b.message_id == message.id)
        } else {
            self.bookmarks
                .iter()
                .position(|b| b.in_session(is_dm, &server, &channel) && b.text == body)
        };
        if let Some(pos) = existing {
            let removed_id = self.bookmarks[pos].message_id.clone();
            if self.is_online()
                && !removed_id.is_empty()
                && let Some(net) = self.net.clone()
            {
                cx.spawn(async move |this, cx| {
                    let result = net.execute(api::remove_bookmark(&removed_id), true).await;
                    let _ = this.update(cx, |app, cx| {
                        if let Err(error) = result {
                            app.bookmark_notice = Some(format!("移除书签失败：{}", error.user_message()));
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            self.bookmarks.remove(pos);
            self.bookmark_notice = Some("已从书签移除".into());
        } else {
            self.bookmarks.insert(
                0,
                Bookmark::new(
                    is_dm,
                    server,
                    channel,
                    message.author,
                    message.color.into(),
                    body,
                    message.time,
                    0,
                ),
            );
            if let Some(first) = self.bookmarks.first_mut() {
                first.message_id = message.id.clone();
                first.target_kind = if is_dm { "dm".into() } else { "channel".into() };
            }
            if self.is_online()
                && !message.id.is_empty()
                && let Some(net) = self.net.clone()
            {
                let message_id = message.id.clone();
                let kind = if is_dm { "dm" } else { "channel" }.to_string();
                cx.spawn(async move |this, cx| {
                    let result = net
                        .execute(api::save_bookmark(&message_id, &kind), true)
                        .await;
                    let _ = this.update(cx, |app, cx| {
                        if let Err(error) = result {
                            app.bookmark_notice = Some(format!("保存书签失败：{}", error.user_message()));
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            self.bookmark_notice = Some("已保存到书签".into());
        }
        self.close_message_menu();
        cx.notify();
    }

    /// 打开 / 关闭书签面板（与置顶、收件箱、日程、消息请求互斥）。
    pub(crate) fn toggle_bookmarks(&mut self, cx: &mut Context<Self>) {
        self.show_bookmarks = !self.show_bookmarks;
        if self.show_bookmarks {
            self.show_pins = false;
            self.show_inbox = false;
            self.show_events = false;
            self.show_message_requests = false;
            self.close_thread();
            self.close_read_receipts();
            self.close_edit_history();
            self.bookmark_notice = None;
        }
        self.profile_target = None;
        self.close_message_menu();
        cx.notify();
    }

    /// 面板列表：按关键词过滤，保持「最近保存在前」的顺序。
    pub(crate) fn visible_bookmarks(&self) -> Vec<Bookmark> {
        let query = self.bookmark_query.to_lowercase();
        self.bookmarks
            .iter()
            .filter(|b| b.matches(&query))
            .cloned()
            .collect()
    }

    /// 按身份移除一条书签。
    pub(crate) fn remove_bookmark(&mut self, target: &Bookmark, cx: &mut Context<Self>) {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| !b.same_as(target));
        if self.bookmarks.len() != before {
            self.bookmark_notice = Some("已从书签移除".into());
        }
        cx.notify();
    }

    /// 跳回书签来源：切会话 → 按正文反查原消息 → 滚动并高亮。
    pub(crate) fn jump_to_bookmark(
        &mut self,
        target: &Bookmark,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if target.is_dm {
            if !self.friends.iter().any(|f| f.name == target.channel) {
                self.bookmark_notice = Some("原私信会话已不存在".into());
                cx.notify();
                return;
            }
            self.open_dm(target.channel.clone(), window, cx);
        } else {
            let Some(server_index) = self.servers.iter().position(|s| s.name == target.server)
            else {
                self.bookmark_notice = Some("原服务器已不存在".into());
                cx.notify();
                return;
            };
            self.view_mode = ViewMode::Server;
            self.active_server = server_index;
            self.active_channel = target.channel.clone();
            self.reset_session(window, cx);
        }

        // reset_session 会关掉面板，跳转后重新打开，方便连续翻书签。
        self.show_bookmarks = true;
        // 切会话后 reset_session 会清掉高亮并滚到底部，所以定位必须放在之后。
        let position = if !target.message_id.is_empty() {
            self.current_messages()
                .iter()
                .position(|m| m.id == target.message_id)
        } else {
            self.current_messages()
                .iter()
                .position(|m| m.text == target.text)
        };
        match position
        {
            Some(index) => {
                self.reveal_message(index);
                self.bookmark_notice = Some(format!("已定位到 {}", target.location_label()));
            }
            None => {
                if !target.message_id.is_empty() && self.has_more_messages() {
                    self.bookmark_notice = Some("正在加载更早的消息，稍后再点一次书签".into());
                    self.load_older_messages(cx);
                    return;
                }
                // 贴纸 / GIF / 附件消息没有正文，本来就无法按文本反查。
                let placeholder = target.text.starts_with('[') && target.text.ends_with(']');
                self.bookmark_notice = Some(if placeholder {
                    "贴纸 / 附件消息没有正文，只能查看书签副本".into()
                } else {
                    "原消息已删除，仅保留书签副本".into()
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn dismiss_bookmark_notice(&mut self, cx: &mut Context<Self>) {
        self.bookmark_notice = None;
        cx.notify();
    }

    pub(crate) fn clear_bookmark_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.bookmark_query.clear();
        self.bookmark_search
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// 离开服务器时清掉它名下的书签（私信书签保留）。
    pub(crate) fn remove_bookmarks_of_server(&mut self, server: &str) {
        self.bookmarks.retain(|b| b.is_dm || b.server != server);
    }

    /// 服务器改名时迁移书签归属。
    pub(crate) fn rename_bookmarks_server(&mut self, old: &str, new: &str) {
        for bookmark in self.bookmarks.iter_mut() {
            if !bookmark.is_dm && bookmark.server == old {
                bookmark.server = new.to_string();
            }
        }
    }

    // -----------------------------------------------------------------------
    // 服务器欢迎屏幕
    // -----------------------------------------------------------------------

    /// 某服务器下的文字频道名（顺序即侧栏顺序）。
    fn text_channels_of(&self, server: &str) -> Vec<String> {
        self.servers
            .iter()
            .find(|s| s.name == server)
            .map(|s| {
                s.categories
                    .iter()
                    .flat_map(|category| category.channels.iter())
                    .filter(|channel| channel.kind == ChannelKind::Text)
                    .map(|channel| channel.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 取某服务器的欢迎屏幕：优先用配置过的文案，没有就按简介 + 文字频道合成。
    pub(crate) fn welcome_screen_for(&self, server: &str) -> Option<ServerWelcome> {
        let info = self.servers.iter().find(|s| s.name == server)?;
        let channels = self.text_channels_of(server);
        let mut welcome = self
            .welcome_screens
            .iter()
            .find(|welcome| welcome.server == server)
            .cloned()
            .unwrap_or_else(|| {
                let defaults: Vec<&str> = channels.iter().take(3).map(String::as_str).collect();
                ServerWelcome::new(
                    info.name.clone(),
                    format!("欢迎来到{}", info.name),
                    if info.description.trim().is_empty() {
                        "先看看下面的频道，找到感兴趣的话题就开始聊吧。".to_string()
                    } else {
                        info.description.clone()
                    },
                )
                .rules(vec![
                    WelcomeRule::new("友善交流", "尊重不同观点，不人身攻击、不刷屏。"),
                    WelcomeRule::new("按频道发帖", "尽量把内容发到对应频道，方便别人找到。"),
                ])
                .recommended(&defaults)
            });
        // 推荐频道可能已被删除，渲染前按现存频道裁剪一次。
        welcome.retain_recommended(&channels);
        Some(welcome)
    }

    /// 打开某服务器的欢迎屏幕；推荐频道默认全选、规则默认未同意。
    pub(crate) fn open_welcome(&mut self, server: String, cx: &mut Context<Self>) {
        let Some(welcome) = self.welcome_screen_for(&server) else {
            return;
        };
        self.welcome_selected = welcome.recommended.clone();
        self.welcome_rule_agreed = false;
        self.welcome_target = Some(server);

        // 欢迎页是顶层模态，先把其它浮层收起来，避免叠着显示。
        self.show_settings = false;
        self.show_server_settings = false;
        self.show_search_center = false;
        self.show_help_center = false;
        self.show_server_menu = false;
        self.show_status_menu = false;
        self.show_invite = false;
        self.show_create_server = false;
        self.show_create_channel = false;
        self.show_voice_room = false;
        self.channel_menu = None;
        self.category_menu = None;
        self.member_menu = None;
        self.profile_target = None;
        self.close_message_menu();
        self.close_thread();
        self.close_read_receipts();
        self.close_edit_history();
        self.show_pins = false;
        self.show_bookmarks = false;
        self.show_inbox = false;
        self.show_events = false;
        self.show_message_requests = false;
        cx.notify();
    }

    /// 服务器菜单入口：看当前服务器的欢迎屏幕。
    pub(crate) fn open_active_welcome(&mut self, cx: &mut Context<Self>) {
        let server = self.servers[self.active_server].name.clone();
        self.open_welcome(server, cx);
    }

    pub(crate) fn close_welcome(&mut self, cx: &mut Context<Self>) {
        self.welcome_target = None;
        self.welcome_rule_agreed = false;
        self.welcome_selected.clear();
        cx.notify();
    }

    /// 勾选 / 取消「我已阅读并同意以上规则」。
    pub(crate) fn toggle_welcome_rule(&mut self, cx: &mut Context<Self>) {
        self.welcome_rule_agreed = !self.welcome_rule_agreed;
        cx.notify();
    }

    /// 多选推荐频道。
    pub(crate) fn toggle_welcome_channel(&mut self, name: String, cx: &mut Context<Self>) {
        if let Some(pos) = self.welcome_selected.iter().position(|channel| channel == &name) {
            self.welcome_selected.remove(pos);
        } else {
            self.welcome_selected.push(name);
        }
        cx.notify();
    }

    /// 当前欢迎页是否允许「开始聊天」：需要同意规则且还没勾时不允许。
    pub(crate) fn welcome_can_confirm(&self) -> bool {
        let Some(server) = &self.welcome_target else {
            return false;
        };
        match self.welcome_screen_for(server) {
            Some(welcome) => !welcome.require_rules || self.welcome_rule_agreed,
            None => false,
        }
    }

    /// 完成欢迎流程：记住已同意、按勾选的推荐频道进入，然后关闭。
    pub(crate) fn confirm_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.welcome_can_confirm() {
            return;
        }
        let Some(server) = self.welcome_target.clone() else {
            return;
        };
        if self.is_online()
            && let (Some(server_id), Some(net)) = (
                self.remote.servers.get(&server).cloned(),
                self.net.clone(),
            )
        {
            let channels: Vec<String> = self
                .welcome_selected
                .iter()
                .filter_map(|name| self.remote.channels.get(&format!("{server}/{name}")).cloned())
                .collect();
            let server_for_welcome = server.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(
                        api::accept_welcome(&server_id, true, &channels),
                        true,
                    )
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        if !app.welcome_joined.iter().any(|name| name == &server_for_welcome) {
                            app.welcome_joined.push(server_for_welcome.clone());
                        }
                        app.welcome_target = None;
                        app.welcome_rule_agreed = false;
                        app.welcome_selected.clear();
                        cx.notify();
                    }
                    Err(error) => {
                        app.automod_notice = Some(format!("同意规则失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        if !self.welcome_joined.iter().any(|name| name == &server) {
            self.welcome_joined.push(server.clone());
        }
        let channels = self.text_channels_of(&server);
        // 勾了推荐频道就进第一个（按侧栏顺序），没勾就留在当前频道。
        let target = channels
            .iter()
            .find(|channel| self.welcome_selected.contains(channel))
            .cloned();

        self.welcome_target = None;
        self.welcome_rule_agreed = false;
        self.welcome_selected.clear();
        if let Some(channel) = target {
            self.view_mode = ViewMode::Server;
            self.active_channel = channel;
            self.reset_session(window, cx);
        }
        cx.notify();
    }

    /// 某服务器是否已走完欢迎流程（同意过规则）。
    pub(crate) fn welcome_done_for(&self, server: &str) -> bool {
        self.welcome_joined.iter().any(|name| name == server)
    }

    /// 离开服务器时清掉它的欢迎配置与完成记录。
    pub(crate) fn remove_welcome_of_server(&mut self, server: &str) {
        self.welcome_screens.retain(|welcome| welcome.server != server);
        self.welcome_joined.retain(|name| name != server);
        if self.welcome_target.as_deref() == Some(server) {
            self.welcome_target = None;
            self.welcome_rule_agreed = false;
            self.welcome_selected.clear();
        }
    }

    /// 服务器改名时迁移欢迎配置与完成记录。
    pub(crate) fn rename_welcome_server(&mut self, old: &str, new: &str) {
        for welcome in self.welcome_screens.iter_mut() {
            if welcome.server == old {
                welcome.server = new.to_string();
            }
        }
        for name in self.welcome_joined.iter_mut() {
            if name == old {
                *name = new.to_string();
            }
        }
        if self.welcome_target.as_deref() == Some(old) {
            self.welcome_target = Some(new.to_string());
        }
    }

    // -----------------------------------------------------------------------
    // 服务器加成
    // -----------------------------------------------------------------------

    /// 某服务器的加成记录；没有记录时返回 0 次的空记录（新建服务器也会走这里）。
    pub(crate) fn boost_of(&self, server: &str) -> ServerBoost {
        self.boosts
            .iter()
            .find(|boost| boost.server == server)
            .cloned()
            .unwrap_or_else(|| ServerBoost::new(server))
    }

    /// 当前服务器的加成记录。
    pub(crate) fn active_boost(&self) -> ServerBoost {
        self.boost_of(&self.active_server_name())
    }

    /// 加成 / 撤销加成当前服务器（Demo 里只有「你」能真的操作）。
    pub(crate) fn toggle_boost(&mut self, cx: &mut Context<Self>) {
        let server = self.active_server_name();
        if server.is_empty() {
            return;
        }
        let boosted = self.active_boost().boosted_by("你");
        if self.is_online()
            && let (Some(server_id), Some(net)) = (
                self.remote.servers.get(&server).cloned(),
                self.net.clone(),
            )
        {
            let active = !boosted;
            let server_for_boost = server.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::toggle_boost(&server_id, active), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.boost_notice = Some(if active {
                            format!("已加成「{server_for_boost}」")
                        } else {
                            format!("已撤销对「{server_for_boost}」的加成")
                        });
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.boost_notice = Some(format!("加成失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }

        if boosted {
            if let Some(boost) = self.boosts.iter_mut().find(|boost| boost.server == server) {
                boost.remove_by("你");
            }
            self.boost_notice = Some(format!("已撤销对「{server}」的加成"));
            self.log_audit(AuditKind::Server, "撤销加成", format!("「{server}」"));
        } else {
            let me = Booster::new("你", 0x5865f2, 0);
            match self.boosts.iter_mut().find(|boost| boost.server == server) {
                Some(boost) => boost.boosters.insert(0, me),
                None => self.boosts.push(ServerBoost::new(server.clone()).with(vec![me])),
            }
            let boost = self.boost_of(&server);
            self.boost_notice = Some(match boost.remaining() {
                Some(left) => format!(
                    "已加成「{server}」，距离等级 {} 还差 {left} 次",
                    boost.level() + 1
                ),
                None => format!("已加成「{server}」，服务器已经是最高等级"),
            });
            self.log_audit(AuditKind::Server, "服务器加成", format!("「{server}」"));
        }
        // 加成会抬高头像/表情等配额，提示条不自动消失，由用户关掉或切页时清理。
        cx.notify();
    }

    /// 从侧栏加成条或菜单直达「加成」设置页。
    pub(crate) fn open_boost_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_server_settings(window, cx);
        self.server_settings_section = ServerSettingsSection::Boost;
        self.boost_notice = None;
        cx.notify();
    }

    pub(crate) fn dismiss_boost_notice(&mut self, cx: &mut Context<Self>) {
        self.boost_notice = None;
        cx.notify();
    }

    /// 离开服务器时清掉它的加成记录。
    pub(crate) fn remove_boost_of_server(&mut self, server: &str) {
        self.boosts.retain(|boost| boost.server != server);
    }

    /// 服务器改名时迁移加成记录。
    pub(crate) fn rename_boost_server(&mut self, old: &str, new: &str) {
        for boost in self.boosts.iter_mut() {
            if boost.server == old {
                boost.server = new.to_string();
            }
        }
    }

    /// 删除服务器时清掉其名下所有活动。
    pub(crate) fn remove_events_of_server(&mut self, server: &str) {
        self.events.retain(|event| event.server != server);
    }

    /// 服务器改名时迁移活动归属。
    pub(crate) fn rename_events_server(&mut self, old: &str, new: &str) {
        for event in self.events.iter_mut() {
            if event.server == old {
                event.server = new.to_string();
            }
        }
    }

    /// 删除当前会话的第 index 条消息；同时迁移该会话里话题的根消息下标。
    pub(crate) fn delete_message(&mut self, index: usize, cx: &mut Context<Self>) {
        let key = self.session_key();
        let message_id = self
            .current_messages()
            .get(index)
            .map(|message| message.id.clone())
            .filter(|id| !id.is_empty());
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
        if let (Some(id), Some(net)) = (message_id, self.net.clone()) {
            cx.spawn(async move |this, cx| {
                if let Err(error) = net.execute(api::delete_message(&id), true).await {
                    let _ = this.update(cx, |app, cx| {
                        app.automod_notice = Some(format!("删除失败：{}", error.user_message()));
                        cx.notify();
                    });
                }
            })
            .detach();
        }
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
        let message_id = self
            .current_messages()
            .get(index)
            .map(|message| message.id.clone())
            .filter(|id| !id.is_empty());
        let list = self.threads.entry(key).or_default();
        if !list.iter().any(|thread| thread.root_index == index) {
            list.push(ChatThread::new(index, title.clone(), Vec::new()));
        }

        if self.is_online()
            && let (Some(message_id), Some(net)) = (message_id, self.net.clone())
        {
            let thread_title = title.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::create_thread(&message_id, &thread_title), true)
                    .await;
                let _ = this.update(cx, |app, _cx| {
                    if let Ok(value) = result {
                        if let Some(thread_id) = value.get("id").and_then(|id| id.as_str())
                            && let Some(thread) = app
                                .threads
                                .values_mut()
                                .flat_map(|list| list.iter_mut())
                                .find(|thread| thread.id.is_empty() && thread.title == thread_title)
                        {
                            thread.id = thread_id.to_string();
                        }
                    }
                });
            })
            .detach();
        }

        self.show_thread = true;
        self.thread_root = Some(index);
        self.show_pins = false;
        self.show_inbox = false;
        self.show_bookmarks = false;
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
        let remote_thread = self
            .threads
            .get(&key)
            .and_then(|list| list.iter().find(|thread| thread.root_index == root))
            .map(|thread| thread.id.clone())
            .filter(|id| !id.is_empty());
        let Some(thread) = self
            .threads
            .get_mut(&key)
            .and_then(|list| list.iter_mut().find(|thread| thread.root_index == root))
        else {
            return;
        };
        thread
            .replies
            .push(ThreadReply::new("你", "刚刚", text.clone(), palette::accent()));
        if let (Some(thread_id), Some(net)) = (remote_thread, self.net.clone())
            && self.is_online()
        {
            let reply_text = text.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::create_thread_reply(&thread_id, &reply_text), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("话题回复失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
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
            self.show_events = false;
            self.show_message_requests = false;
            self.show_bookmarks = false;
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
        let mut target_id = None;
        if !text.is_empty() {
            if let Some(message) = self.current_messages_mut().and_then(|m| m.get_mut(index)) {
                target_id = (!message.id.is_empty()).then(|| message.id.clone());
                if message.text != text {
                    // 编辑前先留存旧正文，供消息旁的「已编辑」历史浮层查看。
                    let previous = message.text.clone();
                    message.edit_history.push(EditRevision {
                        text: previous,
                        time: "刚刚",
                    });
                    message.text = text.clone();
                }
            }
        }
        if let (Some(id), Some(net)) = (target_id, self.net.clone()) {
            let body = text.clone();
            cx.spawn(async move |this, cx| {
                if let Err(error) = net.execute(api::edit_message(&id, &body, None), true).await {
                    let _ = this.update(cx, |app, cx| {
                        app.automod_notice = Some(format!("编辑失败：{}", error.user_message()));
                        cx.notify();
                    });
                }
            })
            .detach();
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
        let Some((username, tag)) = raw.split_once('#') else {
            self.friend_notice = Some("请输入 用户名#标签，例如 Zoe#1234".into());
            cx.notify();
            return;
        };
        let username = username.trim().to_string();
        let tag = tag.trim().to_string();
        if username.is_empty() || tag.len() != 4 || !tag.chars().all(|ch| ch.is_ascii_digit()) {
            self.friend_notice = Some("标签必须是 4 位数字".into());
            cx.notify();
            return;
        }
        if self.friends.iter().any(|friend| friend.name == username)
            || self
                .friend_requests
                .iter()
                .any(|request| request.name == username || request.name == raw)
        {
            self.friend_notice = Some("已经是好友或已有待处理请求".into());
            cx.notify();
            return;
        }
        self.add_friend_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.friend_tab = FriendTab::Pending;
        self.friend_notice = None;

        let Some(net) = self.net.clone().filter(|_| self.is_online()) else {
            self.friend_requests.push(FriendRequest {
                id: String::new(),
                name: raw,
                color: palette::accent(),
                incoming: false,
            });
            cx.notify();
            return;
        };
        cx.spawn(async move |this, cx| {
            let result = net
                .execute(api::send_friend_request(&username, &tag, ""), true)
                .await;
            let _ = this.update(cx, |app, cx| {
                match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.friend_notice = Some(error.user_message());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// 接受收到的请求：从请求列表移除并加入好友。
    pub(crate) fn accept_request(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(request) = self.friend_requests.get(index).cloned() else {
            return;
        };
        if !request.incoming {
            return;
        }
        if self.is_online() && !request.id.is_empty() {
            let Some(net) = self.net.clone() else {
                return;
            };
            let request_id = request.id.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::accept_friend_request(&request_id), true).await;
                let _ = this.update(cx, |app, cx| {
                    match result {
                        Ok(_) => app.refresh_structure(cx),
                        Err(error) => {
                            app.friend_notice = Some(error.user_message());
                            cx.notify();
                        }
                    }
                });
            })
            .detach();
            return;
        }
        self.friend_requests.remove(index);
        let name: &'static str = Box::leak(request.name.into_boxed_str());
        self.friends.push(Friend {
            name,
            color: request.color,
            online: false,
            activity: "离线",
            group: String::new(),
        });
        cx.notify();
    }

    pub(crate) fn reject_request(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(request) = self.friend_requests.get(index).cloned() else {
            return;
        };
        if self.is_online() && !request.id.is_empty() {
            let Some(net) = self.net.clone() else {
                return;
            };
            let request_id = request.id.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::reject_friend_request(&request_id), true).await;
                let _ = this.update(cx, |app, cx| {
                    match result {
                        Ok(_) => app.refresh_structure(cx),
                        Err(error) => {
                            app.friend_notice = Some(error.user_message());
                            cx.notify();
                        }
                    }
                });
            })
            .detach();
            return;
        }
        self.friend_requests.remove(index);
        cx.notify();
    }

    /// 屏蔽好友：从好友列表移除并记入屏蔽名单。
    pub(crate) fn block_friend(&mut self, name: &'static str, cx: &mut Context<Self>) {
        if let Some(pos) = self.friends.iter().position(|f| f.name == name) {
            self.friends.remove(pos);
            if !self.blocked_users.iter().any(|n| n == name) {
                self.blocked_users.push(name.to_string());
            }
        }
        if self.is_online()
            && let (Some(user_id), Some(net)) = (
                self.remote
                    .friends
                    .get(name)
                    .or_else(|| self.remote.members.get(&format!("{}/{}", self.active_server_name(), name)))
                    .cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                if let Err(error) = net.execute(api::block_user(&user_id, ""), true).await {
                    let _ = this.update(cx, |app, cx| {
                        app.friend_notice = Some(error.user_message());
                        cx.notify();
                    });
                }
            })
            .detach();
        }
    }

    /// 取消屏蔽：从屏蔽名单移除。
    pub(crate) fn unblock_friend(&mut self, name: String, cx: &mut Context<Self>) {
        if let Some(pos) = self.blocked_users.iter().position(|n| n == &name) {
            self.blocked_users.remove(pos);
        }
        if self.is_online()
            && let (Some(user_id), Some(net)) =
                (self.remote.blocks.get(&name).cloned(), self.net.clone())
        {
            cx.spawn(async move |this, cx| {
                if let Err(error) = net.execute(api::unblock_user(&user_id), true).await {
                    let _ = this.update(cx, |app, cx| {
                        app.friend_notice = Some(error.user_message());
                        cx.notify();
                    });
                }
            })
            .detach();
        }
    }

    // ---------------------------- 好友分组 ----------------------------

    /// 分组显示顺序：自定义顺序在前，好友身上多出来的分组按首次出现补齐，空名（未分组）永远最后。
    /// 没有成员但仍在顺序表里的空分组也会返回，方便用户进去删掉它。
    pub(crate) fn friend_group_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .friend_groups
            .iter()
            .filter(|name| !name.is_empty())
            .cloned()
            .collect();
        for friend in &self.friends {
            if !friend.group.is_empty() && !names.iter().any(|name| name == &friend.group) {
                names.push(friend.group.clone());
            }
        }
        names.push(String::new());
        names
    }

    /// 好友当前所在的分组名；空字符串表示未分组。
    pub(crate) fn friend_group_of(&self, name: &str) -> String {
        self.friends
            .iter()
            .find(|friend| friend.name == name)
            .map(|friend| friend.group.clone())
            .unwrap_or_default()
    }

    /// 某个分组里的好友，保持 `friends` 里的原始顺序。
    pub(crate) fn friends_in_group(&self, group: &str) -> Vec<&Friend> {
        self.friends.iter().filter(|f| f.group == group).collect()
    }

    /// 分组标题右侧的「N 人在线」。
    pub(crate) fn group_online_count(&self, group: &str) -> usize {
        self.friends
            .iter()
            .filter(|f| f.group == group && f.online)
            .count()
    }

    pub(crate) fn is_group_collapsed(&self, group: &str) -> bool {
        self.collapsed_groups.iter().any(|name| name == group)
    }

    /// 折叠 / 展开分组（未分组也允许折叠）。
    pub(crate) fn toggle_group_collapsed(&mut self, group: String, cx: &mut Context<Self>) {
        if let Some(pos) = self.collapsed_groups.iter().position(|name| name == &group) {
            self.collapsed_groups.remove(pos);
        } else {
            self.collapsed_groups.push(group);
        }
        cx.notify();
    }

    /// 把好友身上出现、但顺序表里还没有的分组补进去，保证上下移动有稳定基准。
    fn normalize_groups(&mut self) {
        for name in self.friend_group_names() {
            if name.is_empty() {
                continue;
            }
            if !self.friend_groups.iter().any(|existing| existing == &name) {
                self.friend_groups.push(name);
            }
        }
    }

    /// 打开「新建分组」弹层。
    pub(crate) fn open_create_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.group_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.show_rename_group = false;
        self.rename_group_source.clear();
        self.show_create_group = true;
        self.friend_group_menu = None;
        self.friend_group_picker = None;
        cx.notify();
    }

    /// 新建分组：空名、「未分组」与重名都不生效。
    pub(crate) fn create_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.group_input.read(cx).value().trim().to_string();
        if name.is_empty() || name == "未分组" {
            return;
        }
        if self
            .friend_group_names()
            .iter()
            .any(|existing| existing == &name)
        {
            return;
        }
        self.group_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.show_create_group = false;
        if self.is_online() {
            if let Some(net) = self.net.clone() {
                let group_name = name.clone();
                cx.spawn(async move |this, cx| {
                    let result = net.execute(api::create_friend_group(&group_name), true).await;
                    let _ = this.update(cx, |app, cx| match result {
                        Ok(_) => app.refresh_structure(cx),
                        Err(error) => {
                            app.friend_notice = Some(error.user_message());
                            cx.notify();
                        }
                    });
                })
                .detach();
                cx.notify();
                return;
            }
        }
        self.friend_groups.push(name);
        cx.notify();
    }

    /// 打开重命名分组弹层，输入框预填当前分组名。
    pub(crate) fn open_group_rename(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.group_input.update(cx, |input, cx| {
            input.set_value(name.clone(), window, cx)
        });
        self.rename_group_source = name;
        self.show_create_group = false;
        self.show_rename_group = true;
        self.friend_group_menu = None;
        self.friend_group_picker = None;
        cx.notify();
    }

    /// 保存重命名：同步顺序表、成员身上的分组名与折叠状态。
    pub(crate) fn save_group_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.group_input.read(cx).value().trim().to_string();
        let source = std::mem::take(&mut self.rename_group_source);
        self.show_rename_group = false;
        self.group_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        if value.is_empty() || source.is_empty() || value == source || value == "未分组" {
            cx.notify();
            return;
        }
        if self
            .friend_group_names()
            .iter()
            .any(|existing| existing == &value)
        {
            cx.notify();
            return;
        }

        if self.is_online()
            && let (Some(group_id), Some(net)) = (
                self.remote.friend_groups.get(&source).cloned(),
                self.net.clone(),
            )
        {
            let new_name = value.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::rename_friend_group(&group_id, &new_name), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.friend_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            cx.notify();
            return;
        }
        if let Some(pos) = self.friend_groups.iter().position(|name| name == &source) {
            self.friend_groups[pos] = value.clone();
        }
        for friend in self.friends.iter_mut() {
            if friend.group == source {
                friend.group = value.clone();
            }
        }
        if let Some(pos) = self.collapsed_groups.iter().position(|name| name == &source) {
            self.collapsed_groups[pos] = value;
        }
        cx.notify();
    }

    /// 关闭分组弹层（新建与重命名共用）。
    pub(crate) fn close_group_dialog(&mut self, cx: &mut Context<Self>) {
        self.show_create_group = false;
        self.show_rename_group = false;
        self.rename_group_source.clear();
        cx.notify();
    }

    /// 删除分组：成员回到「未分组」，顺序表与折叠状态一并清掉。
    pub(crate) fn delete_group(&mut self, name: String, cx: &mut Context<Self>) {
        if name.is_empty() {
            return;
        }
        if self.is_online()
            && let (Some(group_id), Some(net)) = (
                self.remote.friend_groups.get(&name).cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::delete_friend_group(&group_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.friend_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            self.friend_group_menu = None;
            cx.notify();
            return;
        }
        self.friend_groups.retain(|existing| existing != &name);
        self.collapsed_groups.retain(|existing| existing != &name);
        for friend in self.friends.iter_mut() {
            if friend.group == name {
                friend.group.clear();
            }
        }
        self.friend_group_menu = None;
        cx.notify();
    }

    /// 分组上移 / 下移（delta = -1 / 1），到达边界时不动作。
    pub(crate) fn move_group(&mut self, name: String, delta: isize, cx: &mut Context<Self>) {
        self.normalize_groups();
        let Some(pos) = self.friend_groups.iter().position(|group| group == &name) else {
            return;
        };
        let target = pos as isize + delta;
        if target < 0 || target as usize >= self.friend_groups.len() {
            return;
        }
        self.friend_groups.swap(pos, target as usize);
        self.friend_group_menu = None;
        if self.is_online()
            && let Some(net) = self.net.clone()
        {
            let ids: Vec<String> = self
                .friend_groups
                .iter()
                .filter_map(|group| self.remote.friend_groups.get(group).cloned())
                .collect();
            if ids.len() == self.friend_groups.len() {
                cx.spawn(async move |this, cx| {
                    if let Err(error) = net.execute(api::order_friend_groups(&ids), true).await {
                        let _ = this.update(cx, |app, cx| {
                            app.friend_notice = Some(error.user_message());
                            app.refresh_structure(cx);
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
        }
        cx.notify();
    }

    /// 打开分组操作菜单（右键分组标题），同时收起其他浮层。
    pub(crate) fn open_group_menu(
        &mut self,
        name: String,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        self.friend_group_menu = Some(FriendGroupMenuState {
            name,
            anchor: MenuAnchor::Point(x, y),
        });
        self.friend_group_picker = None;
        self.profile_target = None;
        self.close_message_menu();
        self.channel_menu = None;
        self.category_menu = None;
        self.member_menu = None;
        self.show_pins = false;
        self.show_inbox = false;
        self.show_server_menu = false;
        self.show_status_menu = false;
        cx.notify();
    }

    /// 打开「移动到分组」菜单（好友行右侧文件夹按钮）。
    pub(crate) fn open_group_picker(
        &mut self,
        friend: String,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        self.friend_group_picker = Some(FriendGroupPickerState {
            friend,
            anchor: MenuAnchor::Point(x, y),
        });
        self.friend_group_menu = None;
        self.profile_target = None;
        self.close_message_menu();
        self.channel_menu = None;
        self.category_menu = None;
        self.member_menu = None;
        self.show_pins = false;
        self.show_inbox = false;
        self.show_server_menu = false;
        self.show_status_menu = false;
        cx.notify();
    }

    /// 把好友移动到指定分组；目标分组不在顺序表里时补到末尾。空名表示「未分组」。
    pub(crate) fn set_friend_group(
        &mut self,
        friend: String,
        group: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(pos) = self.friends.iter().position(|f| f.name == friend) {
            self.friends[pos].group = group.clone();
        }
        if !group.is_empty() && !self.friend_groups.iter().any(|name| name == &group) {
            self.friend_groups.push(group.clone());
        }
        if self.is_online()
            && let (Some(user_id), Some(net)) = (
                self.remote.friends.get(&friend).cloned(),
                self.net.clone(),
            )
        {
            let group_id = self.remote.friend_groups.get(&group).cloned();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::move_friend(&user_id, group_id.as_deref()), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.friend_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        self.friend_group_picker = None;
        cx.notify();
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
            friend: base.clone(),
            color,
            online,
            mode,
            connected: false,
            mic_on: self.mic_on,
            camera_on: mode == CallMode::Video,
            screen_sharing: false,
            call_id: None,
        });
        self.close_profile();
        self.close_quick_switcher();
        self.show_inbox = false;
        self.show_pins = false;
        self.close_message_menu();
        if self.is_online()
            && let (Some(dm_id), Some(net)) = (
                self.remote.dms.get(&base).cloned(),
                self.net.clone(),
            )
        {
            let kind = match mode {
                CallMode::Voice => "voice",
                CallMode::Video => "video",
            };
            let friend_for_call = base.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::start_dm_call(&dm_id, kind), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(value) => {
                        if let Some(call_id) = value.get("id").and_then(|id| id.as_str())
                            && let Some(call) = app.dm_call.as_mut()
                            && call.friend == friend_for_call
                        {
                            call.call_id = Some(call_id.to_string());
                        }
                    }
                    Err(error) => {
                        app.dm_call = None;
                        app.automod_notice = Some(format!("呼叫失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// Demo：把「正在呼叫」推进为「已接通」。
    pub(crate) fn connect_dm_call(&mut self, cx: &mut Context<Self>) {
        let Some(friend) = self.dm_call.as_ref().map(|call| call.friend.clone()) else {
            return;
        };
        if self.is_online()
            && let (Some(dm_id), Some(net)) = (
                self.remote.dms.get(&friend).cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::accept_dm_call(&dm_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        if let Some(call) = app.dm_call.as_mut() {
                            call.connected = true;
                        }
                        cx.notify();
                    }
                    Err(error) => {
                        app.automod_notice = Some(format!("接听失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        if let Some(call) = self.dm_call.as_mut() {
            call.connected = true;
        }
        cx.notify();
    }

    /// 挂断私信通话（面板由 `dm_call.is_some()` 决定是否渲染）。
    pub(crate) fn end_dm_call(&mut self, cx: &mut Context<Self>) {
        let friend = self.dm_call.as_ref().map(|call| call.friend.clone());
        self.dm_call = None;
        if self.is_online()
            && let (Some(friend), Some(net)) = (friend, self.net.clone())
            && let Some(dm_id) = self.remote.dms.get(&friend).cloned()
        {
            cx.spawn(async move |_this, _cx| {
                let _ = net.execute(api::hangup_dm_call(&dm_id), true).await;
            })
            .detach();
        }
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

    /// 菜单目标成员的角色名，视图只读这一份解析结果。
    pub(crate) fn member_menu_role(&self) -> Option<&'static str> {
        let name = self.member_menu.as_ref()?.name;
        self.members
            .iter()
            .find(|member| member.name == name)
            .and_then(|member| member.role)
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
        self.show_bookmarks = false;
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
        self.run_search(cx);
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
        self.run_search(cx);
        cx.notify();
    }

    /// 切换搜索范围并立即重跑。
    pub(crate) fn set_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        if self.search_scope == scope {
            return;
        }
        self.search_scope = scope;
        self.run_search(cx);
        cx.notify();
    }

    /// 按当前关键词、高级过滤条件与范围重算结果；无任何约束时结果直接清空。
    fn run_search(&mut self, cx: &mut Context<Self>) {
        let filters = SearchFilters::parse(&self.search_center_query);
        self.search_filters = filters.clone();
        self.search_results.clear();
        if filters.is_empty() {
            return;
        }

        if self.is_online()
            && let Some(net) = self.net.clone()
        {
            let (scope, server_id) = match self.search_scope {
                SearchScope::CurrentServer => (
                    "server",
                    self.remote.servers.get(&self.active_server_name()).cloned(),
                ),
                SearchScope::AllServers => ("all", None),
                SearchScope::Dm => ("dm", None),
                SearchScope::Everything => ("everywhere", None),
            };
            let query = self.search_center_query.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute_as::<crate::net::dto::SearchPageDto>(
                        api::search(&query, scope, server_id.as_deref(), 50),
                        true,
                    )
                    .await;
                let _ = this.update(cx, |app, cx| {
                    match result {
                        Ok(page) => {
                            app.search_results = page
                                .items
                                .into_iter()
                                .map(|hit| {
                                    let server = if hit.dm_id.is_some() {
                                        "私信".to_string()
                                    } else {
                                        app.remote
                                            .servers
                                            .iter()
                                            .find(|(_, id)| id.as_str() == hit.server_id)
                                            .map(|(name, _)| name.clone())
                                            .unwrap_or_default()
                                    };
                                    let channel = if server == "私信" {
                                        hit.dm_id
                                            .as_deref()
                                            .and_then(|dm_id| {
                                                app.remote
                                                    .dms
                                                    .iter()
                                                    .find(|(_, id)| id.as_str() == dm_id)
                                                    .map(|(name, _)| name.clone())
                                            })
                                            .unwrap_or_default()
                                    } else {
                                        hit.channel_name.clone()
                                    };
                                    SearchHit {
                                        server,
                                        channel,
                                        is_dm: hit.dm_id.is_some(),
                                        author: Box::leak(hit.author.name.clone().into_boxed_str()),
                                        time: hydrate::short_time(&hit.created_at),
                                        text: hit.content,
                                        color: rgb(hit.author.color),
                                    }
                                })
                                .collect();
                        }
                        Err(error) => {
                            app.automod_notice = Some(format!("搜索失败：{}", error.user_message()));
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
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
        // 新加入的服务器先走一遍欢迎屏幕（介绍 + 规则 + 推荐频道）。
        self.open_welcome(entry.name.clone(), cx);
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
        self.boost_notice = None;
        self.invite_copied_code = None;
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
        self.invite_copied_code = None;
        self.show_member_nick = false;
        self.member_nick_target.clear();
        cx.notify();
    }

    /// 切换服务器设置分类。
    pub(crate) fn select_server_settings_section(
        &mut self,
        section: ServerSettingsSection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.server_settings_section = section;
        self.invite_copied_code = None;
        self.role_notice = None;
        self.permission_notice = None;
        self.automod_notice = None;
        self.boost_notice = None;
        // 进角色页/权限页时默认选中第一个角色，右侧详情不至于空白。
        if (section == ServerSettingsSection::Roles || section == ServerSettingsSection::Permissions)
            && self.selected_role.is_none()
            && let Some(first) = self.roles.first().map(|role| role.name.clone())
        {
            self.select_role(first, window, cx);
        }
        // 进自动审核页时默认选中第一条规则。
        if section == ServerSettingsSection::AutoMod
            && self.selected_automod.is_none()
            && let Some(first) = self.automod_rules.first().map(|rule| rule.name.clone())
        {
            self.selected_automod = Some(first);
        }
        cx.notify();
    }

    /// 保存「概览」：改名时同步迁移消息键与折叠键，保留分类展开状态。
    pub(crate) fn save_server_overview(&mut self, cx: &mut Context<Self>) {
        let index = self.active_server;
        let new_name = self.server_name_edit.read(cx).value().trim().to_string();
        let new_desc = self.server_desc_edit.read(cx).value().trim().to_string();
        let current = self.servers[index].name.clone();

        if self.is_online()
            && let (Some(server_id), Some(net)) = (
                self.remote.servers.get(&current).cloned(),
                self.net.clone(),
            )
        {
            let name_arg = (!new_name.is_empty() && new_name != current).then_some(new_name.clone());
            let desc_arg = (!new_desc.is_empty()).then_some(new_desc.clone());
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_server(&server_id, name_arg.as_deref(), desc_arg.as_deref(), None), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("保存服务器失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }

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

            // 活动、论坛帖子与书签按服务器名归属，改名后同步迁移。
            self.rename_events_server(&current, &new_name);
            self.rename_forum_server(&current, &new_name);
            self.rename_bookmarks_server(&current, &new_name);
            self.rename_welcome_server(&current, &new_name);
            self.rename_boost_server(&current, &new_name);

            let badge = new_name.chars().next().unwrap_or('?').to_string();
            self.servers[index].name = new_name;
            self.servers[index].badge = badge;

            // 邀请按服务器名归属，改名后一并搬到新名下。
            let new_key = self.servers[index].name.clone();
            for invite in self.invites.iter_mut() {
                if invite.server == current {
                    invite.server = new_key.clone();
                }
            }
            self.log_audit(
                AuditKind::Server,
                "更新了服务器名称",
                format!("{current} → {new_key}"),
            );
        }

        if !new_desc.is_empty() && self.servers[index].description != new_desc {
            self.servers[index].description = new_desc;
            self.log_audit(AuditKind::Server, "更新了服务器简介", "新的简介已保存");
        }
        cx.notify();
    }

    /// 切换服务器图标颜色（点击即生效）。
    pub(crate) fn set_server_color(&mut self, color: u32, cx: &mut Context<Self>) {
        let index = self.active_server;
        if self.servers[index].color != color {
            self.servers[index].color = color;
            self.log_audit(
                AuditKind::Server,
                "更新了服务器图标颜色",
                format!("#{color:06x}"),
            );
        }
        cx.notify();
    }

    /// 移除一个服务器表情，暂存到「已移除」区以便恢复。
    pub(crate) fn remove_server_emoji(&mut self, emoji: &'static str, cx: &mut Context<Self>) {
        if let Some(position) = self.server_emojis.iter().position(|item| *item == emoji) {
            self.server_emojis.remove(position);
            self.removed_server_emojis.push(emoji);
            self.log_audit(AuditKind::Emoji, "移除了表情", emoji);
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
            self.log_audit(AuditKind::Emoji, "恢复了表情", emoji);
        }
        cx.notify();
    }

    // ---------------------------- 角色管理 ----------------------------

    /// 角色下的成员数。
    pub(crate) fn role_member_count(&self, role: &str) -> usize {
        self.members
            .iter()
            .filter(|member| member.role == Some(role))
            .count()
    }

    /// 选中角色并把名称预填到输入框。
    pub(crate) fn select_role(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let role_name = name.clone();
        self.role_name_edit
            .update(cx, |input, cx| input.set_value(role_name, window, cx));
        self.selected_role = Some(name);
        self.role_notice = None;
        cx.notify();
    }

    /// 新建角色：名称自动去重、颜色按角色表轮换，建完直接选中。
    pub(crate) fn create_role(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut serial = self.roles.len() + 1;
        let mut name = format!("新角色 {serial}");
        while self.roles.iter().any(|role| role.name == name) {
            serial += 1;
            name = format!("新角色 {serial}");
        }
        let color = ROLE_COLORS[self.roles.len() % ROLE_COLORS.len()];
        if self.is_online()
            && let (Some(server_id), Some(net)) =
                (self.remote.servers.get(&self.active_server_name()).cloned(), self.net.clone())
        {
            let role_name = name.clone();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::create_role(&server_id, &role_name, color), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.role_notice = Some("创建角色失败");
                        app.automod_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            self.select_role(name, window, cx);
            return;
        }
        self.roles.push(Role::new(name.clone(), color));
        self.log_audit(AuditKind::Role, "创建了角色", name.clone());
        self.select_role(name, window, cx);
        self.role_notice = Some("已新建角色，改名后回车保存");
        cx.notify();
    }

    /// 保存角色名称：非空、不与其它角色重名；成员身上的角色名与折叠键一起迁移。
    pub(crate) fn save_role_name(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.selected_role.clone() else {
            return;
        };
        let new_name = self.role_name_edit.read(cx).value().trim().to_string();
        if new_name.is_empty() {
            self.role_notice = Some("角色名不能为空");
            cx.notify();
            return;
        }
        if new_name == current {
            self.role_notice = Some("角色名未变化");
            cx.notify();
            return;
        }
        if self.roles.iter().any(|role| role.name == new_name) {
            self.role_notice = Some("已有同名角色");
            cx.notify();
            return;
        }

        let server_name = self.active_server_name();
        if self.is_online()
            && let (Some(role_id), Some(net)) = (
                self.remote
                    .roles
                    .get(&format!("{server_name}/{current}"))
                    .cloned(),
                self.net.clone(),
            )
        {
            let renamed = new_name.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_role(&role_id, Some(&renamed), None), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.role_notice = Some("角色改名失败");
                        app.automod_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        // 角色名是成员的归属键：改名要同步成员与成员栏的折叠 key。
        let leaked = persist::leak(new_name.clone());
        for role in self.roles.iter_mut() {
            if role.name == current {
                role.name = new_name.clone();
            }
        }
        for member in self.members.iter_mut() {
            if member.role == Some(current.as_str()) {
                member.role = Some(leaked);
            }
        }
        let old_key = format!("role-{current}");
        let new_key = format!("role-{new_name}");
        for key in self.collapsed_roles.iter_mut() {
            if *key == old_key {
                *key = new_key.clone();
            }
        }
        // 角色名同时是权限归属键，一起迁移。
        for grant in self.role_permissions.iter_mut() {
            if grant.role == current {
                grant.role = new_name.clone();
            }
        }

        self.log_audit(
            AuditKind::Role,
            "更新了角色名称",
            format!("{current} → {new_name}"),
        );
        self.selected_role = Some(new_name);
        self.role_notice = Some("角色名已更新");
        cx.notify();
    }

    /// 改角色颜色（点色块即生效）。
    pub(crate) fn set_role_color(&mut self, color: u32, cx: &mut Context<Self>) {
        let Some(current) = self.selected_role.clone() else {
            return;
        };
        let mut changed = false;
        if let Some(role) = self.roles.iter_mut().find(|role| role.name == current)
            && role.color != color
        {
            role.color = color;
            changed = true;
        }
        if changed {
            let server_name = self.active_server_name();
            if self.is_online()
                && let (Some(role_id), Some(net)) = (
                    self.remote
                        .roles
                        .get(&format!("{server_name}/{current}"))
                        .cloned(),
                    self.net.clone(),
                )
            {
                cx.spawn(async move |this, cx| {
                    let result = net
                        .execute(api::patch_role(&role_id, None, Some(color)), true)
                        .await;
                    let _ = this.update(cx, |app, cx| {
                        if let Err(error) = result {
                            app.automod_notice = Some(error.user_message());
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            self.log_audit(
                AuditKind::Role,
                "更新了角色颜色",
                format!("{current} · #{color:06x}"),
            );
        }
        self.role_notice = Some("颜色已更新");
        cx.notify();
    }

    /// 删除角色：成员回到「无角色」，对应的折叠 key 一并清掉。
    pub(crate) fn delete_role(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.selected_role.clone() else {
            return;
        };
        if self.is_online()
            && let (Some(role_id), Some(net)) = (
                self.remote
                    .roles
                    .get(&format!("{}/{}", self.active_server_name(), current))
                    .cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::delete_role(&role_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.selected_role = None;
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.role_notice = Some("删除角色失败");
                        app.automod_notice = Some(error.user_message());
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        self.roles.retain(|role| role.name != current);
        for member in self.members.iter_mut() {
            if member.role == Some(current.as_str()) {
                member.role = None;
            }
        }
        let key = format!("role-{current}");
        self.collapsed_roles.retain(|item| item != &key);
        self.role_permissions.retain(|grant| grant.role != current);
        self.log_audit(AuditKind::Role, "删除了角色", current.clone());
        self.selected_role = None;
        self.role_notice = Some("角色已删除，相关成员回到「无角色」");
        cx.notify();
    }

    /// 在「当前角色 / 无角色」之间切换某个成员的归属。
    ///
    /// 角色名是 `&'static str`，这里按次 leak 一份（每次点击几十字节，Demo 量级可忽略）。
    pub(crate) fn toggle_member_role(&mut self, member_name: &str, cx: &mut Context<Self>) {
        let Some(current) = self.selected_role.clone() else {
            return;
        };
        let leaked = persist::leak(current.clone());
        let Some(index) = self.members.iter().position(|m| m.name == member_name) else {
            return;
        };
        let had_role = self.members[index].role == Some(current.as_str());
        self.members[index].role = if had_role { None } else { Some(leaked) };
        if self.is_online()
            && let (Some(server_id), Some(role_id), Some(user_id), Some(net)) = (
                self.remote.servers.get(&self.active_server_name()).cloned(),
                self.remote
                    .roles
                    .get(&format!("{}/{}", self.active_server_name(), current))
                    .cloned(),
                self.remote
                    .members
                    .get(&format!("{}/{}", self.active_server_name(), member_name))
                    .cloned(),
                self.net.clone(),
            )
        {
            let assigned = !had_role;
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::assign_role(&server_id, &user_id, &role_id, assigned), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("角色更新失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        if had_role {
            self.log_audit(
                AuditKind::Role,
                "移除了成员的角色",
                format!("{member_name} ✕ {current}"),
            );
        } else {
            self.log_audit(
                AuditKind::Role,
                "给成员授予了角色",
                format!("{member_name} → {current}"),
            );
        }
        cx.notify();
    }

    // ---------------------------- 权限管理 ----------------------------

    /// 查询某角色对某权限位的当前状态；没有显式设置就是「继承」。
    pub(crate) fn permission_state(&self, role: &str, key: &str) -> PermissionState {
        self.role_permissions
            .iter()
            .find(|grant| grant.role == role && grant.permission == key)
            .map(|grant| grant.state)
            .unwrap_or(PermissionState::Unset)
    }

    /// 循环切换角色权限位：继承 → 允许 → 拒绝 → 继承；允许/拒绝落库，回到继承删除记录。
    pub(crate) fn cycle_permission(
        &mut self,
        role: String,
        key: &'static str,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        let before = self.permission_state(&role, key);
        let after = before.next();
        if let Some(index) = self
            .role_permissions
            .iter()
            .position(|grant| grant.role == role && grant.permission == key)
        {
            if after == PermissionState::Unset {
                self.role_permissions.remove(index);
            } else {
                self.role_permissions[index].state = after;
            }
        } else if after != PermissionState::Unset {
            self.role_permissions
                .push(PermissionGrant::new(role.clone(), key, after));
        }

        if self.is_online() {
            let server_name = self.active_server_name();
            let server_id = self.remote.servers.get(&server_name).cloned();
            let role_id = self
                .remote
                .roles
                .get(&format!("{server_name}/{role}"))
                .cloned();
            if let (Some(server_id), Some(role_id), Some(net)) = (server_id, role_id, self.net.clone()) {
                let items: Vec<serde_json::Value> = crate::data::PERMISSIONS
                    .iter()
                    .map(|definition| {
                        let state = match self.permission_state(&role, definition.key) {
                            PermissionState::Unset => "inherit",
                            PermissionState::Allow => "allow",
                            PermissionState::Deny => "deny",
                        };
                        serde_json::json!({
                            "role_id": role_id,
                            "permission": definition.key,
                            "state": state,
                        })
                    })
                    .collect();
                cx.spawn(async move |this, cx| {
                    let result = net
                        .execute(api::write_permissions(&server_id, serde_json::Value::Array(items)), true)
                        .await;
                    let _ = this.update(cx, |app, cx| {
                        if let Err(error) = result {
                            app.automod_notice = Some(format!("保存权限失败：{}", error.user_message()));
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
        }
        self.permission_notice = Some(match after {
            PermissionState::Unset => "已恢复为继承默认",
            PermissionState::Allow => "已设为允许",
            PermissionState::Deny => "已设为拒绝",
        });
        self.log_audit(
            AuditKind::Role,
            "更新了角色权限",
            format!("{role} · {label}：{} → {}", before.label(), after.label()),
        );
        cx.notify();
    }

    /// 当前权限页聚焦的角色：优先用角色页选中项，否则回落到第一个角色。
    pub(crate) fn permission_role(&self) -> Option<String> {
        self.selected_role
            .clone()
            .or_else(|| self.roles.first().map(|role| role.name.clone()))
    }

    /// 权限页切换聚焦角色（不写角色页的名称输入草稿）。
    pub(crate) fn select_permission_role(&mut self, name: String, cx: &mut Context<Self>) {
        self.selected_role = Some(name);
        self.permission_notice = None;
        cx.notify();
    }

    // ---------------------------- 自动审核 ----------------------------

    /// 选中一条规则（并清空关键词输入草稿）。
    pub(crate) fn select_automod_rule(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_automod = Some(name);
        self.automod_notice = None;
        self.automod_keyword_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    /// 当前聚焦的规则名：选中项优先，否则回落第一条。
    pub(crate) fn automod_rule_focus(&self) -> Option<String> {
        self.selected_automod.clone().or_else(|| {
            self.automod_rules
                .first()
                .map(|rule| rule.name.clone())
        })
    }

    /// 新建规则：名称非空且不重名，默认启用、动作为拦截、关键词为空。
    pub(crate) fn create_automod_rule(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = self
            .automod_name_edit
            .read(cx)
            .value()
            .trim()
            .to_string();
        if name.is_empty() {
            self.automod_notice = Some("规则名称不能为空".into());
            cx.notify();
            return;
        }
        if self.automod_rules.iter().any(|rule| rule.name == name) {
            self.automod_notice = Some("已有同名规则".into());
            cx.notify();
            return;
        }
        if self.is_online()
            && let (Some(server_id), Some(net)) = (
                self.remote.servers.get(&self.active_server_name()).cloned(),
                self.net.clone(),
            )
        {
            let rule_name = name.clone();
            cx.spawn(async move |this, cx| {
                let empty: Vec<String> = Vec::new();
                let result = net
                    .execute(
                        api::create_automod_rule(&server_id, &rule_name, "block", &empty),
                        true,
                    )
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("创建规则失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            self.automod_name_edit
                .update(cx, |input, cx| input.set_value("", window, cx));
            return;
        }
        let rule = AutoModRule::new(
            name.clone(),
            "新规则，添加关键词后生效。",
            &[],
            AutoModAction::Block,
            true,
        );
        self.automod_rules.push(rule);
        self.log_audit(
            AuditKind::Server,
            "创建了自动审核规则",
            name.clone(),
        );
        self.automod_name_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.select_automod_rule(name, window, cx);
        self.automod_notice = Some("规则已创建，添加关键词后生效".into());
        cx.notify();
    }

    /// 删除选中的规则。
    pub(crate) fn delete_automod_rule(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.selected_automod.clone() else {
            return;
        };
        if self.is_online()
            && let (Some(rule_id), Some(net)) = (
                self.automod_rules
                    .iter()
                    .find(|rule| rule.name == current)
                    .map(|rule| rule.id.clone())
                    .filter(|id| !id.is_empty()),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::delete_automod_rule(&rule_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.selected_automod = None;
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.automod_notice = Some(format!("删除规则失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        self.automod_rules.retain(|rule| rule.name != current);
        self.log_audit(AuditKind::Server, "删除了自动审核规则", current.clone());
        self.selected_automod = None;
        self.automod_notice = Some("规则已删除".into());
        cx.notify();
    }

    /// 切换选中规则的启用状态。
    pub(crate) fn toggle_automod_rule(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.automod_rule_focus() else {
            return;
        };
        let mut remote_toggle = None;
        let net_handle = self.is_online().then(|| self.net.clone()).flatten();
        if let Some(rule) = self.automod_rules.iter_mut().find(|rule| rule.name == current) {
            rule.enabled = !rule.enabled;
            let text = if rule.enabled { "启用" } else { "停用" };
            if let Some(net) = net_handle.clone().filter(|_| !rule.id.is_empty()) {
                remote_toggle = Some((net, rule.id.clone(), rule.enabled));
            }
            self.log_audit(
                AuditKind::Server,
                "更新了自动审核规则",
                format!("{current} · {text}"),
            );
        }
        if let Some((net, rule_id, enabled)) = remote_toggle {
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_automod_rule(&rule_id, None, None, Some(enabled), None), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("更新规则失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// 修改选中规则的处理动作。
    pub(crate) fn set_automod_action(
        &mut self,
        action: AutoModAction,
        cx: &mut Context<Self>,
    ) {
        let Some(current) = self.automod_rule_focus() else {
            return;
        };
        let mut remote_action = None;
        let net_handle = self.is_online().then(|| self.net.clone()).flatten();
        if let Some(rule) = self.automod_rules.iter_mut().find(|rule| rule.name == current)
            && rule.action != action
        {
            rule.action = action;
            if let Some(net) = net_handle.clone().filter(|_| !rule.id.is_empty()) {
                remote_action = Some((net, rule.id.clone(), action));
            }
            self.log_audit(
                AuditKind::Server,
                "更新了自动审核规则",
                format!("{current} · 动作改为{}", action.label()),
            );
        }
        if let Some((net, rule_id, action)) = remote_action {
            let action_key = match action {
                AutoModAction::Block => "block",
                AutoModAction::Warn => "warn",
            };
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_automod_rule(&rule_id, None, Some(action_key), None, None), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("更新动作失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// 给选中规则添加关键词（去重、忽略空值）。
    pub(crate) fn add_automod_keyword(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keyword = self
            .automod_keyword_edit
            .read(cx)
            .value()
            .trim()
            .to_string();
        if keyword.is_empty() {
            return;
        }
        let Some(current) = self.automod_rule_focus() else {
            return;
        };
        let mut remote_keywords = None;
        let net_handle = self.is_online().then(|| self.net.clone()).flatten();
        if let Some(rule) = self.automod_rules.iter_mut().find(|rule| rule.name == current) {
            if rule.keywords.iter().any(|key| key == &keyword) {
                self.automod_notice = Some("关键词已存在".into());
            } else {
                rule.keywords.push(keyword.clone());
                if let Some(net) = net_handle.clone().filter(|_| !rule.id.is_empty()) {
                    remote_keywords = Some((net, rule.id.clone(), rule.keywords.clone()));
                }
                self.log_audit(
                    AuditKind::Server,
                    "添加了审核关键词",
                    format!("{current} · {keyword}"),
                );
                self.automod_notice = Some("关键词已添加".into());
            }
        }
        if let Some((net, rule_id, keywords)) = remote_keywords {
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_automod_rule(&rule_id, None, None, None, Some(&keywords)), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("添加关键词失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        self.automod_keyword_edit
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    /// 删除选中规则的一个关键词。
    pub(crate) fn remove_automod_keyword(
        &mut self,
        keyword: String,
        cx: &mut Context<Self>,
    ) {
        let Some(current) = self.automod_rule_focus() else {
            return;
        };
        let mut remote_remove = None;
        let net_handle = self.is_online().then(|| self.net.clone()).flatten();
        if let Some(rule) = self.automod_rules.iter_mut().find(|rule| rule.name == current)
            && let Some(index) = rule.keywords.iter().position(|key| key == &keyword)
        {
            rule.keywords.remove(index);
            if let Some(net) = net_handle.clone().filter(|_| !rule.id.is_empty()) {
                remote_remove = Some((net, rule.id.clone(), rule.keywords.clone()));
            }
            self.log_audit(
                AuditKind::Server,
                "移除了审核关键词",
                format!("{current} · {keyword}"),
            );
        }
        if let Some((net, rule_id, keywords)) = remote_remove {
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_automod_rule(&rule_id, None, None, None, Some(&keywords)), true)
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if let Err(error) = result {
                        app.automod_notice = Some(format!("移除关键词失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// 检查文本是否命中启用的审核规则；返回（动作、规则名、命中关键词）。
    pub(crate) fn automod_check(&self, text: &str) -> Option<(AutoModAction, String, String)> {
        for rule in self.automod_rules.iter().filter(|rule| rule.enabled) {
            if let Some(keyword) = rule.matched_keyword(text) {
                return Some((rule.action, rule.name.clone(), keyword.to_string()));
            }
        }
        None
    }

    /// 关闭聊天区的审核提示条。
    pub(crate) fn dismiss_automod_notice(&mut self, cx: &mut Context<Self>) {
        self.automod_notice = None;
        cx.notify();
    }

    // ---------------------------- 邀请管理 ----------------------------

    /// 当前服务器可用于邀请的文字频道（语音频道不能做邀请落地页）。
    pub(crate) fn invite_channel_options(&self) -> Vec<String> {
        self.servers[self.active_server]
            .categories
            .iter()
            .flat_map(|category| category.channels.iter())
            .filter(|channel| channel.kind == ChannelKind::Text)
            .map(|channel| channel.name.clone())
            .collect()
    }

    /// 邀请频道失效时回落到第一个文字频道；服务器切换 / 频道删除后都要调一次。
    pub(crate) fn sync_invite_channel(&mut self) {
        let options = self.invite_channel_options();
        if !options.iter().any(|name| name == &self.invite_channel) {
            self.invite_channel = options.first().cloned().unwrap_or_default();
        }
    }

    /// 当前服务器的邀请列表（副本，视图只读）。
    pub(crate) fn server_invites(&self) -> Vec<Invite> {
        let server = self.servers[self.active_server].name.as_str();
        self.invites
            .iter()
            .filter(|invite| invite.server == server)
            .cloned()
            .collect()
    }

    /// 当前服务器最新一条邀请码；一条都没有时退回按服务器名生成的 slug。
    pub(crate) fn primary_invite_code(&self) -> String {
        self.server_invites()
            .first()
            .map(|invite| invite.code.clone())
            .unwrap_or_else(|| self.invite_code())
    }

    pub(crate) fn set_invite_channel(&mut self, name: String, cx: &mut Context<Self>) {
        self.invite_channel = name;
        cx.notify();
    }

    pub(crate) fn set_invite_expiry(&mut self, expiry: InviteExpiry, cx: &mut Context<Self>) {
        self.invite_expiry = expiry;
        cx.notify();
    }

    /// 次数上限：0 表示不限。
    pub(crate) fn set_invite_max_uses(&mut self, max: u32, cx: &mut Context<Self>) {
        self.invite_max_uses = max;
        cx.notify();
    }

    /// 生成一条邀请并立刻复制链接。邀请码形如 `dev-community-4`，与已有码去重。
    pub(crate) fn create_invite(&mut self, cx: &mut Context<Self>) {
        self.sync_invite_channel();
        let channel = self.invite_channel.clone();
        if channel.is_empty() {
            return;
        }

        let server = self.servers[self.active_server].name.clone();
        if self.is_online()
            && let (Some(channel_id), Some(net)) = (
                self.remote
                    .channels
                    .get(&format!("{server}/{channel}"))
                    .cloned(),
                self.net.clone(),
            )
        {
            let expiry = match self.invite_expiry {
                InviteExpiry::ThirtyMinutes | InviteExpiry::OneHour => "1h",
                InviteExpiry::SixHours | InviteExpiry::TwelveHours | InviteExpiry::OneDay => "1d",
                InviteExpiry::SevenDays => "7d",
                InviteExpiry::Never => "never",
            };
            let max_uses = (self.invite_max_uses > 0).then_some(self.invite_max_uses);
            let max_uses = max_uses;
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::create_invite(&channel_id, expiry, max_uses), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(value) => {
                        if let Some(code) = value.get("code").and_then(|code| code.as_str()) {
                            app.invite_copied_code = Some(code.to_string());
                            cx.write_to_clipboard(ClipboardItem::new_string(format!(
                                "discord.gg/{code}"
                            )));
                        }
                        app.refresh_structure(cx);
                    }
                    Err(error) => {
                        app.invite_copied_code = None;
                        app.automod_notice = Some(format!("创建邀请失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let base = self.invite_code();
        let mut serial = self.invites.iter().filter(|invite| invite.server == server).count() + 1;
        let mut code = format!("{base}-{serial}");
        while self.invites.iter().any(|invite| invite.code == code) {
            serial += 1;
            code = format!("{base}-{serial}");
        }

        self.log_audit(
            AuditKind::Invite,
            "创建了邀请",
            format!("discord.gg/{code} · #{channel}"),
        );
        self.invites.insert(
            0,
            Invite {
                code: code.clone(),
                server,
                channel,
                inviter: persist::leak(self.me.display_name.clone()),
                uses: 0,
                max_uses: self.invite_max_uses,
                expiry: self.invite_expiry,
                created_minutes: 0,
            },
        );
        cx.write_to_clipboard(ClipboardItem::new_string(format!("discord.gg/{code}")));
        self.invite_copied_code = Some(code);
        cx.notify();
    }

    /// 复制某条邀请的链接，并在列表行上标记「已复制」。
    pub(crate) fn copy_invite_link(&mut self, code: &str, cx: &mut Context<Self>) {
        let Some(invite) = self.invites.iter().find(|invite| invite.code == code) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(invite.link()));
        self.invite_copied_code = Some(code.to_string());
        cx.notify();
    }

    /// 撤销（删除）一条邀请。
    pub(crate) fn revoke_invite(&mut self, code: &str, cx: &mut Context<Self>) {
        if self.is_online()
            && let Some(net) = self.net.clone()
        {
            let invite_code = code.to_string();
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::revoke_invite(&invite_code), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("撤销邀请失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        self.invites.retain(|invite| invite.code != code);
        if self.invite_copied_code.as_deref() == Some(code) {
            self.invite_copied_code = None;
        }
        self.log_audit(
            AuditKind::Invite,
            "撤销了邀请",
            format!("discord.gg/{code}"),
        );
        cx.notify();
    }

    // ---------------------------- 成员管理 ----------------------------

    /// 搜索框关键词（转小写，匹配时不用再转）。
    pub(crate) fn member_query(&self, cx: &Context<Self>) -> String {
        self.member_search.read(cx).value().trim().to_lowercase()
    }

    /// 单个成员是否命中筛选与搜索：名字和昵称都参与匹配。
    fn member_matches(&self, index: usize, filter: &MemberFilter, query: &str) -> bool {
        let Some(member) = self.members.get(index) else {
            return false;
        };
        let by_filter = match filter {
            MemberFilter::All => true,
            MemberFilter::Role(role) => member.role == Some(role.as_str()),
            MemberFilter::NoRole => member.role.is_none(),
        };
        if !by_filter {
            return false;
        }
        query.is_empty()
            || member.name.to_lowercase().contains(query)
            || member.display().to_lowercase().contains(query)
    }

    /// 当前搜索 + 筛选下命中的成员索引（保持 members 原顺序）。
    pub(crate) fn filtered_members(&self, cx: &Context<Self>) -> Vec<usize> {
        let query = self.member_query(cx);
        (0..self.members.len())
            .filter(|index| self.member_matches(*index, &self.member_filter, &query))
            .collect()
    }

    /// 某个筛选下的成员数（不看搜索，用于筛选条计数）。
    pub(crate) fn member_count_of(&self, filter: &MemberFilter) -> usize {
        (0..self.members.len())
            .filter(|index| self.member_matches(*index, filter, ""))
            .count()
    }

    pub(crate) fn set_member_filter(&mut self, filter: MemberFilter, cx: &mut Context<Self>) {
        self.member_filter = filter;
        cx.notify();
    }

    pub(crate) fn clear_member_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.member_search
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.notify();
    }

    /// 打开昵称弹层：输入框预填当前昵称，没昵称就填原名。
    pub(crate) fn open_member_nick(
        &mut self,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self
            .members
            .iter()
            .find(|member| member.name == name)
            .map(|member| member.display().to_string())
            .unwrap_or_default();
        self.member_nick_input
            .update(cx, |input, cx| input.set_value(current, window, cx));
        self.member_nick_target = name;
        self.show_member_nick = true;
        cx.notify();
    }

    /// 保存昵称：留空或填回原名都等于清除昵称。
    pub(crate) fn save_member_nick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.member_nick_input.read(cx).value().trim().to_string();
        let target = std::mem::take(&mut self.member_nick_target);
        self.show_member_nick = false;
        self.member_nick_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        if target.is_empty() {
            cx.notify();
            return;
        }
        let Some(index) = self.members.iter().position(|member| member.name == target) else {
            cx.notify();
            return;
        };
        if self.is_online()
            && let (Some(server_id), Some(user_id), Some(net)) = (
                self.remote.servers.get(&self.active_server_name()).cloned(),
                self.remote
                    .members
                    .get(&format!("{}/{}", self.active_server_name(), target))
                    .cloned(),
                self.net.clone(),
            )
        {
            let nickname = value.clone();
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::patch_member(&server_id, &user_id, &nickname), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("昵称修改失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let before = self.members[index].display().to_string();
        if value.is_empty() || value == target {
            self.members[index].nick = None;
        } else {
            self.members[index].nick = Some(persist::leak(value.clone()));
        }
        let after = self.members[index].display().to_string();
        if before != after {
            self.log_audit(
                AuditKind::Member,
                "更新了成员昵称",
                format!("{before} → {after}"),
            );
        }
        cx.notify();
    }

    pub(crate) fn close_member_nick(&mut self, cx: &mut Context<Self>) {
        self.show_member_nick = false;
        self.member_nick_target.clear();
        cx.notify();
    }

    /// 把成员移出服务器（演示：只从本地列表删除）。
    pub(crate) fn kick_member(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(index) = self.members.iter().position(|member| member.name == name) else {
            return;
        };
        if self.is_online()
            && let (Some(server_id), Some(user_id), Some(net)) = (
                self.remote.servers.get(&self.active_server_name()).cloned(),
                self.remote
                    .members
                    .get(&format!("{}/{}", self.active_server_name(), name))
                    .cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::kick_member(&server_id, &user_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("移出成员失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let display = self.members[index].display().to_string();
        self.members.remove(index);
        // 被移除的人如果正被资料卡或右键菜单指着，一并关掉。
        if self.profile_target.is_some_and(|target| target == name) {
            self.profile_target = None;
        }
        if self.member_menu.as_ref().is_some_and(|menu| menu.name == name) {
            self.member_menu = None;
        }
        self.log_audit(AuditKind::Member, "将成员移出了服务器", display);
        cx.notify();
    }

    /// 封禁成员：移出列表并记入封禁列表（原因留空，等后续补）。
    pub(crate) fn ban_member(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(index) = self.members.iter().position(|member| member.name == name) else {
            return;
        };
        if self.is_online()
            && let (Some(server_id), Some(user_id), Some(net)) = (
                self.remote.servers.get(&self.active_server_name()).cloned(),
                self.remote
                    .members
                    .get(&format!("{}/{}", self.active_server_name(), name))
                    .cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net
                    .execute(api::ban_member(&server_id, &user_id, "", 0), true)
                    .await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => app.refresh_structure(cx),
                    Err(error) => {
                        app.automod_notice = Some(format!("封禁失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        let display = self.members[index].display().to_string();
        let by = self.me.display_name.clone();
        self.members.remove(index);
        self.bans.insert(
            0,
            BannedUser {
                name: display.clone(),
                reason: String::new(),
                minutes: 0,
                by,
            },
        );
        if self.profile_target.is_some_and(|target| target == name) {
            self.profile_target = None;
        }
        if self.member_menu.as_ref().is_some_and(|menu| menu.name == name) {
            self.member_menu = None;
        }
        self.log_audit(AuditKind::Member, "封禁了成员", display);
        cx.notify();
    }

    /// 撤销封禁：只清掉列表里的记录，不自动把人加回服务器。
    pub(crate) fn unban_member(&mut self, name: &str, cx: &mut Context<Self>) {
        if let Some(position) = self.bans.iter().position(|entry| entry.name == name) {
            let removed = self.bans.remove(position);
            self.log_audit(AuditKind::Member, "撤销了对成员的封禁", removed.name);
        }
        if self.is_online()
            && let (Some(server_id), Some(net)) =
                (self.remote.servers.get(&self.active_server_name()).cloned(), self.net.clone())
        {
            let user_id = self.remote.blocks.get(name).cloned().or_else(|| {
                self.bans
                    .iter()
                    .find(|entry| entry.name == name)
                    .and_then(|_| self.remote.blocks.get(name).cloned())
            });
            if let Some(user_id) = user_id {
                cx.spawn(async move |this, cx| {
                    let result = net.execute(api::unban_member(&server_id, &user_id), true).await;
                    let _ = this.update(cx, |app, cx| match result {
                        Ok(_) => app.refresh_structure(cx),
                        Err(error) => {
                            app.automod_notice = Some(format!("撤销封禁失败：{}", error.user_message()));
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    // ---------------------------- 审核日志 ----------------------------

    /// 最新一条日志的序号；日志为空时是 0，新条目自然从 1 开始。
    pub(crate) fn audit_newest_seq(&self) -> u32 {
        self.audit.last().map(|entry| entry.seq).unwrap_or(0)
    }

    /// 追加一条日志。操作者固定为当前用户，序号接在最新一条之后。
    /// 超过 `AUDIT_LIMIT` 时丢最旧的几条。
    pub(crate) fn log_audit(
        &mut self,
        kind: AuditKind,
        action: impl Into<String>,
        detail: impl Into<String>,
    ) {
        let seq = self.audit_newest_seq() + 1;
        let actor = self.me.display_name.clone();
        self.audit
            .push(AuditEntry::new(seq, kind, actor, action, detail));
        if self.audit.len() > AUDIT_LIMIT {
            let overflow = self.audit.len() - AUDIT_LIMIT;
            self.audit.drain(..overflow);
        }
    }

    /// 当前筛选下的日志，最新的排在最前面。
    pub(crate) fn audit_visible(&self) -> Vec<&AuditEntry> {
        self.audit
            .iter()
            .rev()
            .filter(|entry| self.audit_filter.is_none_or(|kind| entry.kind == kind))
            .collect()
    }

    /// 某个类别（`None` = 全部）的条数，用于筛选条上的计数。
    pub(crate) fn audit_count(&self, kind: Option<AuditKind>) -> usize {
        self.audit
            .iter()
            .filter(|entry| kind.is_none_or(|value| entry.kind == value))
            .count()
    }

    pub(crate) fn set_audit_filter(&mut self, kind: Option<AuditKind>, cx: &mut Context<Self>) {
        self.audit_filter = kind;
        cx.notify();
    }

    /// 清空日志（不可撤销）。
    pub(crate) fn clear_audit(&mut self, cx: &mut Context<Self>) {
        if self.is_online()
            && let (Some(server_id), Some(net)) = (
                self.remote.servers.get(&self.active_server_name()).cloned(),
                self.net.clone(),
            )
        {
            cx.spawn(async move |this, cx| {
                let result = net.execute(api::clear_audit(&server_id), true).await;
                let _ = this.update(cx, |app, cx| match result {
                    Ok(_) => {
                        app.audit.clear();
                        app.audit_filter = None;
                        cx.notify();
                    }
                    Err(error) => {
                        app.automod_notice = Some(format!("清空日志失败：{}", error.user_message()));
                        cx.notify();
                    }
                });
            })
            .detach();
            return;
        }
        self.audit.clear();
        self.audit_filter = None;
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
    // 在线会话：健康检查 / 登录 / 首屏同步 / 退出
    // -----------------------------------------------------------------------

    pub(crate) fn is_online(&self) -> bool {
        self.auth_status == AuthStatus::Ready
    }

    fn check_health(&mut self, cx: &mut Context<Self>) {
        let Some(net) = self.net.clone() else {
            self.auth_status = AuthStatus::Demo;
            return;
        };
        let session_file = self
            .net
            .as_ref()
            .map(|net| net.config().session_file.clone());
        let base = self.net.as_ref().map(|net| net.config().base.clone());
        cx.spawn(async move |this, cx| {
            let health = net.execute(api::health(), false).await;
            if health.is_err() {
                let _ = this.update(cx, |app, cx| {
                    app.auth_status = AuthStatus::Demo;
                    cx.notify();
                });
                return;
            }
            let saved = session_file
                .as_deref()
                .zip(base.as_deref())
                .and_then(|(path, base)| auth::load_refresh_token(path, base));
            if let Some(refresh_token) = saved {
                net.session().set_refresh_only(refresh_token);
                if net.refresh().await.is_ok() {
                    let _ = this.update(cx, |app, cx| {
                        app.auth_status = AuthStatus::Login;
                        app.login_busy = false;
                        app.begin_boot(cx);
                    });
                    return;
                }
            }
            let _ = this.update(cx, |app, cx| {
                app.auth_status = AuthStatus::Login;
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn has_more_messages(&self) -> bool {
        self.has_more_messages
            .get(&self.session_key())
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn load_older_messages(&mut self, cx: &mut Context<Self>) {
        if !self.is_online() || self.loading_older || !self.has_more_messages() {
            return;
        }
        let key = self.session_key();
        let Some(channel_id) = self.remote.channels.get(&key).cloned() else {
            return;
        };
        let Some(before) = self
            .messages
            .get(&key)
            .and_then(|messages| messages.first())
            .map(|message| message.id.clone())
            .filter(|id| !id.is_empty())
        else {
            return;
        };
        let Some(net) = self.net.clone() else {
            return;
        };
        self.loading_older = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = net
                .execute_as::<PageDto<MessageDto>>(api::messages(&channel_id, Some(&before), 50), true)
                .await;
            let _ = this.update(cx, |app, cx| {
                app.loading_older = false;
                match result {
                    Ok(page) => {
                        let mut older: Vec<Message> = page
                            .items
                            .iter()
                            .map(|dto| {
                                let mut message = hydrate::message_from(dto);
                                if dto.author.id == app.remote.me_id {
                                    message.author = "你";
                                }
                                message
                            })
                            .collect();
                        let existing = app.messages.entry(key.clone()).or_default();
                        older.retain(|message| {
                            !existing.iter().any(|item| {
                                (!message.id.is_empty() && item.id == message.id)
                                    || (message.client_nonce.is_some()
                                        && item.client_nonce == message.client_nonce)
                            })
                        });
                        older.append(existing);
                        *existing = older;
                        app.has_more_messages.insert(key, page.has_more);
                    }
                    Err(error) => {
                        app.automod_notice = Some(format!("加载更早消息失败：{}", error.user_message()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn typing_active(&self) -> bool {
        self.typing_until
            .is_some_and(|until| std::time::Instant::now() < until)
    }

    pub(crate) fn typing_label(&self) -> String {
        match &self.typing_name {
            Some(name) => format!("{name} 正在输入…"),
            None => format!("{} 正在输入…", self.session_title()),
        }
    }

    fn maybe_send_typing(&mut self, cx: &mut Context<Self>) {
        if !self.is_online() || self.view_mode != ViewMode::Server {
            return;
        }
        if self
            .last_typing_sent
            .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(3))
        {
            return;
        }
        let Some(channel_id) = self.remote.channels.get(&self.active_key()).cloned() else {
            return;
        };
        let Some(net) = self.net.clone() else {
            return;
        };
        self.last_typing_sent = Some(std::time::Instant::now());
        cx.spawn(async move |_this, _cx| {
            let _ = net.execute(api::typing(&channel_id), true).await;
        })
        .detach();
    }

    fn show_typing(&mut self, name: String, cx: &mut Context<Self>) {
        self.typing_name = Some(name);
        self.typing_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(4));
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(4))
                .await;
            let _ = this.update(cx, |app, cx| {
                if !app.typing_active() {
                    app.typing_until = None;
                    app.typing_name = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn mark_active_channel_read(&self, cx: &mut Context<Self>) {
        if !self.is_online() {
            return;
        }
        let Some(net) = self.net.clone() else {
            return;
        };
        let Some(channel_id) = self.remote.channels.get(&self.active_key()).cloned() else {
            return;
        };
        let Some(message_id) = self
            .current_messages()
            .last()
            .map(|message| message.id.clone())
            .filter(|id| !id.is_empty())
        else {
            return;
        };
        cx.spawn(async move |_this, _cx| {
            let _ = net
                .execute(api::mark_channel_read(&channel_id, &message_id), true)
                .await;
        })
        .detach();
    }

    fn ensure_channel_loaded(&mut self, cx: &mut Context<Self>) {
        if !self.is_online() || self.view_mode != ViewMode::Server {
            return;
        }
        let key = self.active_key();
        if self.loaded_channels.contains(&key) {
            return;
        }
        let Some(channel_id) = self.remote.channels.get(&key).cloned() else {
            return;
        };
        let Some(net) = self.net.clone() else {
            return;
        };
        self.loaded_channels.insert(key.clone());
        cx.spawn(async move |this, cx| {
            let result = net
                .execute_as::<PageDto<MessageDto>>(api::messages(&channel_id, None, 50), true)
                .await;
            let _ = this.update(cx, |app, cx| {
                match result {
                    Ok(page) => {
                        let mut incoming: Vec<Message> = page
                            .items
                            .iter()
                            .map(|dto| {
                                let mut message = hydrate::message_from(dto);
                                if dto.author.id == app.remote.me_id {
                                    message.author = "你";
                                }
                                message
                            })
                            .collect();
                        let mut existing = app.messages.remove(&key).unwrap_or_default();
                        for message in incoming.drain(..) {
                            if !message.id.is_empty()
                                && let Some(index) =
                                    existing.iter().position(|item| item.id == message.id)
                            {
                                existing[index] = message;
                            } else {
                                existing.push(message);
                            }
                        }
                        existing.sort_by(|a, b| a.id.cmp(&b.id));
                        app.has_more_messages.insert(key.clone(), page.has_more);
                        app.messages.insert(key, existing);
                        app.mark_active_channel_read(cx);
                    }
                    Err(error) => {
                        app.loaded_channels.remove(&key);
                        app.automod_notice = Some(format!("加载消息失败：{}", error.user_message()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn start_event_loop(&mut self, rx: Receiver<NetEvent>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                let _ = this.update(cx, |app, cx| app.handle_net_event(event, cx));
            }
        })
        .detach();
    }

    fn start_gateway(&mut self, cx: &mut Context<Self>) {
        self.stop_gateway();
        let Some(net) = self.net.clone() else {
            return;
        };
        if !net.is_logged_in() {
            return;
        }
        self.gateway = Some(WsClient::start(
            net.config().clone(),
            net.session(),
            self.gateway_tx.clone(),
            self.last_event_id.clone(),
        ));
        cx.notify();
    }

    fn stop_gateway(&mut self) {
        if let Some(mut gateway) = self.gateway.take() {
            gateway.stop();
        }
        self.gateway_connected = false;
    }

    fn refresh_for_gateway(&mut self, cx: &mut Context<Self>) {
        let Some(net) = self.net.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let result = net.refresh().await;
            let _ = this.update(cx, |app, cx| {
                if result.is_ok() {
                    app.start_gateway(cx);
                } else {
                    app.logout(cx);
                }
            });
        })
        .detach();
    }

    fn handle_net_event(&mut self, event: NetEvent, cx: &mut Context<Self>) {
        match event {
            NetEvent::Connected => {
                self.gateway_connected = true;
                cx.notify();
            }
            NetEvent::Disconnected { reason } => {
                self.gateway_connected = false;
                if reason.contains("4001") || reason.to_ascii_lowercase().contains("unauthorized") {
                    self.refresh_for_gateway(cx);
                }
                cx.notify();
            }
            NetEvent::NeedAuth => self.refresh_for_gateway(cx),
            NetEvent::Gateway { event, id, data } => {
                if crate::net::ws::is_duplicate(self.last_event_id.as_deref(), &id) {
                    return;
                }
                if !id.is_empty() {
                    self.last_event_id = Some(id);
                }
                if event == "READY" {
                    self.gateway_connected = true;
                    cx.notify();
                    return;
                }
                self.apply_gateway_event(&event, &data, cx);
            }
        }
    }

    fn apply_gateway_event(&mut self, event: &str, data: &serde_json::Value, cx: &mut Context<Self>) {
        match event {
            "MESSAGE_CREATE" => {
                if let Some(dm_id) = data.get("dm_id").and_then(|value| value.as_str()) {
                    if let Ok(dto) = serde_json::from_value::<crate::net::dto::DmMessageDto>(data.clone()) {
                        self.insert_dm_message(dm_id, &dto);
                    }
                } else if let Ok(dto) = serde_json::from_value::<MessageDto>(data.clone())
                    && let Some(key) = self.channel_key_for_id(&dto.channel_id)
                {
                    self.insert_message(&key, &dto);
                }
            }
            "MESSAGE_UPDATE" => {
                if let Some(message) = data.get("message")
                    && let Ok(dto) = serde_json::from_value::<MessageDto>(message.clone())
                {
                    if let Some(key) = self.channel_key_for_id(&dto.channel_id) {
                        self.insert_message(&key, &dto);
                    } else {
                        self.replace_message(&dto);
                    }
                }
            }
            "MESSAGE_DELETE" => {
                if let Some(id) = data.get("id").and_then(|value| value.as_str()) {
                    for messages in self.messages.values_mut() {
                        messages.retain(|message| message.id != id);
                    }
                    for messages in self.dm_messages.values_mut() {
                        messages.retain(|message| message.id != id);
                    }
                    self.close_thread_if_deleted(id);
                }
            }
            "MESSAGE_REACTION_ADD" | "MESSAGE_REACTION_REMOVE" => {
                let Some(message_id) = data.get("message_id").and_then(|value| value.as_str()) else {
                    return;
                };
                let Some(emoji) = data.get("emoji").and_then(|value| value.as_str()) else {
                    return;
                };
                let by_me = data
                    .get("user_id")
                    .and_then(|value| value.as_str())
                    .is_some_and(|user_id| user_id == self.remote.me_id);
                let add = event == "MESSAGE_REACTION_ADD";
                for messages in self.messages.values_mut() {
                    if let Some(message) = messages.iter_mut().find(|message| message.id == message_id) {
                        apply_reaction(message, emoji, add, by_me);
                    }
                }
                for messages in self.dm_messages.values_mut() {
                    if let Some(message) = messages.iter_mut().find(|message| message.id == message_id) {
                        apply_reaction(message, emoji, add, by_me);
                    }
                }
            }
            "TYPING_START" => {
                let channel_id = data.get("channel_id").and_then(|value| value.as_str());
                let active = self.remote.channels.get(&self.active_key()).map(String::as_str);
                if channel_id.is_some() && channel_id == active {
                    let user_id = data.get("user_id").and_then(|value| value.as_str()).unwrap_or("");
                    let name = self
                        .members
                        .iter()
                        .find(|member| {
                            self.remote
                                .members
                                .get(&format!(
                                    "{}/{}",
                                    self.servers[self.active_server].name, member.name
                                ))
                                .map(String::as_str)
                                == Some(user_id)
                        })
                        .map(|member| member.display().to_string())
                        .unwrap_or_else(|| "有人".into());
                    self.show_typing(name, cx);
                }
            }
            "CALL_RING" => {
                if let Some(call_id) = data.get("id").and_then(|value| value.as_str()) {
                    let caller = data.get("caller").cloned().unwrap_or_default();
                    let friend = caller
                        .get("display_name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("来电")
                        .to_string();
                    let color = caller
                        .get("avatar_color")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0x5865f2) as u32;
                    self.dm_call = Some(DmCall {
                        friend,
                        color,
                        online: true,
                        mode: if data.get("kind").and_then(|value| value.as_str()) == Some("video") {
                            CallMode::Video
                        } else {
                            CallMode::Voice
                        },
                        connected: false,
                        mic_on: self.mic_on,
                        camera_on: false,
                        screen_sharing: false,
                        call_id: Some(call_id.to_string()),
                    });
                }
            }
            "CALL_ACCEPT" => {
                if let Some(call) = self.dm_call.as_mut()
                    && call.call_id.as_deref()
                        == data.get("id").and_then(|value| value.as_str())
                {
                    call.connected = true;
                }
            }
            "CALL_DECLINE" | "CALL_END" => {
                if self.dm_call.as_ref().and_then(|call| call.call_id.as_deref())
                    == data.get("id").and_then(|value| value.as_str())
                {
                    self.dm_call = None;
                }
            }
            "POLL_UPDATE" => {
                let Some(poll_id) = data.get("id").and_then(|value| value.as_str()) else {
                    return;
                };
                for messages in self.messages.values_mut() {
                    if let Some(message) = messages
                        .iter_mut()
                        .find(|message| message.poll.as_ref().is_some_and(|poll| poll.id == poll_id))
                        && let Some(poll) = data.get("poll")
                    {
                        update_poll(message, poll);
                    }
                }
            }
            "THREAD_CREATE" | "THREAD_UPDATE" => {
                if let (Some(thread_id), Some(count)) = (
                    data.get("id").and_then(|value| value.as_str()),
                    data.get("reply_count").and_then(|value| value.as_u64()),
                ) {
                    for list in self.threads.values_mut() {
                        if let Some(thread) = list
                            .iter_mut()
                            .find(|thread| thread.id == thread_id)
                        {
                            thread.replies.resize(count as usize, ThreadReply::new("你", "刚刚", String::new(), 0x808080));
                        }
                    }
                }
            }
            "CHANNEL_CREATE" | "CHANNEL_UPDATE" | "CHANNEL_DELETE"
            | "CATEGORY_CREATE" | "CATEGORY_UPDATE" | "CATEGORY_DELETE"
            | "SERVER_UPDATE" | "MEMBER_JOIN" | "MEMBER_LEAVE" | "MEMBER_UPDATE"
            | "ROLE_CREATE" | "ROLE_UPDATE" | "ROLE_DELETE" | "PERMISSIONS_UPDATE"
            | "FRIEND_REQUEST_CREATE" | "FRIEND_REQUEST_ACCEPT" | "FRIEND_REQUEST_REJECT"
            | "FRIEND_REQUEST_DELETE" | "FRIEND_GROUP_CREATE" | "FRIEND_GROUP_UPDATE"
            | "FRIEND_GROUP_DELETE" | "DM_CREATE" | "INVITE_CREATE" | "INVITE_DELETE"
            | "FORUM_POST_CREATE" | "FORUM_POST_UPDATE" | "FORUM_POST_DELETE"
            | "FORUM_REPLY_CREATE" | "FORUM_REPLY_DELETE"
            | "AUTOMOD_RULE_CREATE" | "AUTOMOD_RULE_UPDATE" | "AUTOMOD_RULE_DELETE" => {
                self.refresh_structure(cx);
            }
            "PRESENCE_UPDATE" => {
                if let Some(user_id) = data.get("user_id").and_then(|value| value.as_str()) {
                    let status = data
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("offline");
                    let online = !matches!(status, "offline" | "invisible");
                    let server_name = self
                        .servers
                        .get(self.active_server)
                        .map(|server| server.name.clone())
                        .unwrap_or_default();
                    for member in &mut self.members {
                        let key = format!("{server_name}/{}", member.name);
                        if self.remote.members.get(&key).map(String::as_str) == Some(user_id) {
                            member.online = online;
                        }
                    }
                    for friend in &mut self.friends {
                        if self.remote.friends.get(friend.name).map(String::as_str) == Some(user_id) {
                            friend.online = online;
                        }
                    }
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn channel_key_for_id(&self, channel_id: &str) -> Option<String> {
        self.remote
            .channels
            .iter()
            .find(|(_, id)| id.as_str() == channel_id)
            .map(|(key, _)| key.clone())
    }

    fn insert_message(&mut self, key: &str, dto: &MessageDto) {
        let mut message = hydrate::message_from(dto);
        if dto.author.id == self.remote.me_id {
            message.author = "你";
        }
        self.upsert_message(key, message);
    }

    fn upsert_message(&mut self, key: &str, message: Message) {
        let list = self.messages.entry(key.to_string()).or_default();
        if !message.id.is_empty()
            && let Some(existing) = list.iter_mut().find(|item| item.id == message.id)
        {
            *existing = message;
            return;
        }
        if let Some(nonce) = message.client_nonce.clone()
            && let Some(existing) = list
                .iter_mut()
                .find(|item| item.client_nonce.as_deref() == Some(nonce.as_str()))
        {
            *existing = message;
            return;
        }
        list.push(message);
    }

    fn replace_message(&mut self, dto: &MessageDto) {
        let mut message = hydrate::message_from(dto);
        if dto.author.id == self.remote.me_id {
            message.author = "你";
        }
        for list in self.messages.values_mut() {
            if let Some(existing) = list.iter_mut().find(|item| item.id == message.id) {
                *existing = message;
                return;
            }
        }
    }

    fn insert_dm_message(&mut self, dm_id: &str, dto: &crate::net::dto::DmMessageDto) {
        let Some(friend) = self
            .remote
            .dms
            .iter()
            .find(|(_, id)| id.as_str() == dm_id)
            .map(|(name, _)| name.clone())
        else {
            return;
        };
        let mut message = hydrate::dm_message_from(dto);
        if dto.author.id == self.remote.me_id {
            message.author = "你";
        }
        let list = self.dm_messages.entry(friend.clone()).or_default();
        if let Some(existing) = list.iter_mut().find(|item| item.id == message.id) {
            *existing = message;
        } else {
            list.push(message);
        }
        if let Some(conversation) = self
            .dm_conversations
            .iter_mut()
            .find(|conversation| conversation.friend == friend)
        {
            conversation.preview = Box::leak(dto.content.clone().into_boxed_str());
            if self.active_dm.as_deref() != Some(friend.as_str()) && dto.author.id != self.remote.me_id {
                conversation.unread += 1;
            }
        }
    }

    fn close_thread_if_deleted(&mut self, _message_id: &str) {}

    fn ensure_dm_loaded(&mut self, friend: String, cx: &mut Context<Self>) {
        if !self.is_online() || self.loaded_dms.contains(&friend) {
            return;
        }
        let Some(dm_id) = self.remote.dms.get(&friend).cloned() else {
            return;
        };
        let Some(net) = self.net.clone() else {
            return;
        };
        self.loaded_dms.insert(friend.clone());
        cx.spawn(async move |this, cx| {
            let result = net
                .execute_as::<PageDto<crate::net::dto::DmMessageDto>>(
                    api::dm_messages(&dm_id, None, 50),
                    true,
                )
                .await;
            if result.is_ok() {
                let _ = net.execute(api::mark_dm_read(&dm_id), true).await;
            }
            let _ = this.update(cx, |app, _cx| match result {
                Ok(page) => {
                    let mut messages: Vec<Message> = page
                        .items
                        .iter()
                        .map(|dto| {
                            let mut message = hydrate::dm_message_from(dto);
                            if dto.author.id == app.remote.me_id {
                                message.author = "你";
                            }
                            message
                        })
                        .collect();
                    messages.sort_by(|a, b| a.id.cmp(&b.id));
                    app.dm_messages.insert(friend.clone(), messages);
                }
                Err(error) => {
                    app.loaded_dms.remove(&friend);
                    app.message_request_notice = Some(format!("加载私信失败：{}", error.user_message()));
                }
            });
        })
        .detach();
    }

    fn finish_dm_send(
        &mut self,
        friend: &str,
        nonce: &str,
        result: Result<serde_json::Value, NetError>,
    ) {
        match result {
            Ok(value) => {
                let Ok(dto) = serde_json::from_value::<crate::net::dto::DmMessageDto>(value) else {
                    return;
                };
                let mut message = hydrate::dm_message_from(&dto);
                if dto.author.id == self.remote.me_id {
                    message.author = "你";
                }
                if let Some(list) = self.dm_messages.get_mut(friend)
                    && let Some(existing) = list
                        .iter_mut()
                        .find(|item| item.client_nonce.as_deref() == Some(nonce))
                {
                    *existing = message;
                }
            }
            Err(error) => {
                if let Some(list) = self.dm_messages.get_mut(friend)
                    && let Some(existing) = list
                        .iter_mut()
                        .find(|item| item.client_nonce.as_deref() == Some(nonce))
                {
                    existing.delivery = DeliveryStatus::Failed;
                }
                self.automod_notice = Some(format!("发送失败：{}", error.user_message()));
            }
        }
    }

    fn finish_send(&mut self, key: &str, nonce: &str, result: Result<serde_json::Value, NetError>) {
        match result {
            Ok(value) => {
                let Ok(dto) = serde_json::from_value::<MessageDto>(value) else {
                    return;
                };
                let mut message = hydrate::message_from(&dto);
                if dto.author.id == self.remote.me_id {
                    message.author = "你";
                }
                if let Some(list) = self.messages.get_mut(key)
                    && let Some(existing) = list
                        .iter_mut()
                        .find(|item| item.client_nonce.as_deref() == Some(nonce))
                {
                    *existing = message;
                }
            }
            Err(error) => {
                if let Some(list) = self.messages.get_mut(key)
                    && let Some(existing) = list
                        .iter_mut()
                        .find(|item| item.client_nonce.as_deref() == Some(nonce))
                {
                    existing.delivery = DeliveryStatus::Failed;
                }
                self.automod_notice = Some(format!("发送失败：{}", error.user_message()));
            }
        }
    }

    pub(crate) fn toggle_login_mode(&mut self, cx: &mut Context<Self>) {
        self.login_mode = match self.login_mode {
            LoginMode::Login => LoginMode::Register,
            LoginMode::Register => LoginMode::Login,
        };
        self.login_error = None;
        cx.notify();
    }

    pub(crate) fn fill_demo_account(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.login_mode = LoginMode::Login;
        self.login_email
            .update(cx, |input, cx| input.set_value("demo@example.com", window, cx));
        self.login_password
            .update(cx, |input, cx| input.set_value("password123", window, cx));
        self.login_error = None;
        cx.notify();
    }

    pub(crate) fn toggle_login_remember(&mut self, cx: &mut Context<Self>) {
        self.login_remember = !self.login_remember;
        cx.notify();
    }

    pub(crate) fn submit_auth(&mut self, cx: &mut Context<Self>) {
        if self.login_busy {
            return;
        }
        let email = self.login_email.read(cx).value().trim().to_string();
        let password = self.login_password.read(cx).value().to_string();
        let username = self.login_username.read(cx).value().trim().to_string();
        if email.is_empty() || password.is_empty() {
            self.login_error = Some("请输入邮箱和密码".into());
            cx.notify();
            return;
        }
        let mode = self.login_mode;
        if mode == LoginMode::Register && username.len() < 2 {
            self.login_error = Some("注册需要至少 2 个字符的用户名".into());
            cx.notify();
            return;
        }
        let Some(net) = self.net.clone() else {
            self.login_error = Some("未配置后端地址".into());
            cx.notify();
            return;
        };
        let remember = self.login_remember;
        self.login_busy = true;
        self.login_error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = if mode == LoginMode::Register {
                net.register(email, password, username, remember).await
            } else {
                net.login(email, password, remember).await
            };
            let _ = this.update(cx, |app, cx| {
                app.login_busy = false;
                match result {
                    Ok(_) => app.begin_boot(cx),
                    Err(error) => {
                        app.login_error = Some(error.user_message());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn begin_boot(&mut self, cx: &mut Context<Self>) {
        let Some(net) = self.net.clone() else {
            self.auth_status = AuthStatus::Demo;
            cx.notify();
            return;
        };
        self.auth_status = AuthStatus::Booting;
        self.boot_message = "正在同步服务器…".into();
        self.login_error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = Self::fetch_boot(net).await;
            let _ = this.update(cx, |app, cx| match result {
                Ok(data) => app.apply_hydrated(data, cx),
                Err(error) => {
                    app.auth_status = AuthStatus::Login;
                    app.login_error = Some(format!("首屏同步失败：{}", error.user_message()));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    async fn fetch_boot(net: Arc<NetClient>) -> Result<hydrate::Hydrated, NetError> {
        let me: UserDto = net.execute_as(api::me(), true).await?;
        let servers: Vec<ServerDto> = net.execute_as(api::servers(), true).await?;

        let mut trees = Vec::new();
        let mut members = Vec::new();
        let mut invites = Vec::new();
        let mut forum_posts = Vec::new();
        let mut events = Vec::new();
        let mut boosts = Vec::new();
        let mut welcomes = Vec::new();
        let notifications: crate::net::dto::NotificationPageDto = net
            .execute_as(api::notifications(), true)
            .await?;
        let bookmarks: Vec<crate::net::dto::BookmarkDto> = net.execute_as(api::bookmarks(), true).await?;
        let mut automod_rules = Vec::new();
        let mut audit = Vec::new();
        let mut roles = Vec::new();
        let mut permissions = Vec::new();
        for server in &servers {
            trees.push(net.execute_as::<ServerTreeDto>(api::server_tree(&server.id), true).await?);
            members.push(net.execute_as::<Vec<MemberDto>>(api::server_members(&server.id), true).await?);
            roles.push(net.execute_as::<Vec<RoleDto>>(api::server_roles(&server.id), true).await?);
            if let Ok(rows) = net
                .execute_as::<Vec<crate::net::dto::InviteDto>>(api::server_invites(&server.id), true)
                .await
            {
                invites.extend(rows);
            }
            if let Ok(rows) = net
                .execute_as::<Vec<crate::net::dto::ServerEventDto>>(api::server_events(&server.id), true)
                .await
            {
                events.extend(rows);
            }
            if let Ok(boost) = net
                .execute_as::<crate::net::dto::BoostDto>(api::server_boosts(&server.id), true)
                .await
            {
                boosts.push((server.name.clone(), boost));
            }
            if let Ok(welcome) = net
                .execute_as::<crate::net::dto::WelcomeDto>(api::server_welcome(&server.id), true)
                .await
            {
                welcomes.push((server.name.clone(), welcome));
            }
            if let Ok(rows) = net
                .execute_as::<Vec<crate::net::dto::AutoModRuleDto>>(api::automod_rules(&server.id), true)
                .await
            {
                automod_rules.push((server.name.clone(), rows));
            }
            if let Ok(rows) = net
                .execute_as::<crate::net::dto::AuditPageDto>(api::server_audit(&server.id), true)
                .await
            {
                audit.push((server.name.clone(), rows.items));
            }
            let tree = trees.last().cloned();
            if let Some(tree) = tree {
                for channel in tree.channels.iter().filter(|channel| channel.kind == "forum") {
                    if let Ok(page) = net
                        .execute_as::<crate::net::dto::ForumPostPageDto>(api::forum_posts(&channel.id), true)
                        .await
                    {
                        forum_posts.extend(page.items);
                    }
                }
            }
            permissions.push(
                match net
                    .execute_as::<PermissionMatrixDto>(api::server_permissions(&server.id), true)
                    .await
                {
                    Ok(matrix) => matrix,
                    Err(error) if error.status == Some(403) => PermissionMatrixDto {
                        permissions: Vec::new(),
                        items: Vec::new(),
                    },
                    Err(error) => return Err(error),
                },
            );
        }

        let friends: Vec<FriendDto> = net.execute_as(api::friends(), true).await?;
        let groups: Vec<FriendGroupDto> = net.execute_as(api::friend_groups(), true).await?;
        let request_page: crate::net::dto::FriendRequestPageDto =
            net.execute_as(api::friend_requests(), true).await?;
        let blocks: Vec<crate::net::dto::BlockDto> = net.execute_as(api::blocks(), true).await?;
        let requests: crate::net::dto::MessageRequestPageDto =
            net.execute_as(api::all_message_requests(), true).await?;
        let dms: Vec<DmDto> = net.execute_as(api::dms(), true).await?;
        let counts: UnreadCountsDto = net
            .execute_as(api::unread_counts(), true)
            .await
            .unwrap_or_default();

        let mut active_server_id = String::new();
        let mut active_channel_id = String::new();
        if let Some(server) = servers.first()
            && let Some(tree) = trees.first()
        {
            active_server_id = server.id.clone();
            active_channel_id = tree
                .channels
                .iter()
                .find(|channel| channel.kind == "text")
                .map(|channel| channel.id.clone())
                .unwrap_or_default();
        }
        let active_messages = if active_channel_id.is_empty() {
            Vec::new()
        } else {
            net.execute_as::<PageDto<MessageDto>>(api::messages(&active_channel_id, None, 50), true)
                .await?
                .items
        };

        Ok(hydrate::hydrate(hydrate::BootData {
            me,
            servers,
            trees,
            members,
            roles,
            permissions,
            friends,
            friend_requests: request_page.items.into_iter().chain(request_page.outgoing).collect(),
            blocks,
            message_requests: requests.items,
            invites,
            forum_posts,
            events,
            boosts,
            welcomes,
            notifications: notifications.items,
            bookmarks,
            automod_rules,
            audit,
            groups,
            dms,
            counts,
            active_messages,
            active_server_id,
            active_channel_id,
        }))
    }

    fn apply_hydrated(&mut self, data: hydrate::Hydrated, cx: &mut Context<Self>) {
        self.apply_hydrated_inner(data, false, true, cx);
    }

    fn apply_hydrated_inner(
        &mut self,
        data: hydrate::Hydrated,
        keep_view: bool,
        restart_gateway: bool,
        cx: &mut Context<Self>,
    ) {
        let previous_server = self.servers.get(self.active_server).map(|server| server.name.clone());
        let previous_channel = self.active_channel.clone();
        let previous_dm = self.active_dm.clone();
        let previous_messages = std::mem::take(&mut self.messages);

        self.me = data.me;
        self.remote = data.index;
        self.servers = data.servers;
        self.members_by_server = data.members_by_server;
        self.roles_by_server = data.roles_by_server;
        self.permissions_by_server = data.permissions_by_server;

        if self.servers.is_empty() {
            let (server, members) = hydrate::fallback_server(&self.me.display_name);
            let name = server.name.clone();
            self.members_by_server.insert(name.clone(), members);
            self.roles_by_server.insert(name.clone(), Vec::new());
            self.permissions_by_server.insert(name, Vec::new());
            self.servers = vec![server];
        }

        if let Some(server_id) = self.pending_server_id.take()
            && let Some(index) = self
                .servers
                .iter()
                .position(|server| self.remote.servers.get(&server.name) == Some(&server_id))
        {
            self.active_server = index;
        } else {
        self.active_server = if keep_view {
            previous_server
                .as_deref()
                .and_then(|name| self.servers.iter().position(|server| server.name == name))
                .unwrap_or(data.active_server.min(self.servers.len() - 1))
        } else {
            data.active_server.min(self.servers.len() - 1)
        };
        }
        if let Some(forum_post) = self.pending_forum_post.take()
            && self
                .forum_posts
                .iter()
                .any(|post| post.id == forum_post)
        {
            self.forum_open = Some(forum_post);
        }
        if let Some(channel) = self.pending_channel.take()
            && self.servers[self.active_server]
                .categories
                .iter()
                .flat_map(|category| category.channels.iter())
                .any(|candidate| candidate.name == channel)
        {
            self.active_channel = channel;
        }
        let server_name = self.servers[self.active_server].name.clone();
        let channel_exists = self.servers[self.active_server]
            .categories
            .iter()
            .flat_map(|category| category.channels.iter())
            .any(|channel| channel.name == previous_channel);
        self.active_channel = if keep_view && channel_exists {
            previous_channel
        } else if data.active_channel.is_empty() {
            self.first_text_channel().unwrap_or_default()
        } else {
            data.active_channel
        };
        self.members = self
            .members_by_server
            .get(&server_name)
            .cloned()
            .unwrap_or_default();
        self.roles = self
            .roles_by_server
            .get(&server_name)
            .cloned()
            .unwrap_or_default();
        self.role_permissions = self
            .permissions_by_server
            .get(&server_name)
            .cloned()
            .unwrap_or_default();

        self.friends = data.friends;
        self.friend_groups = data.friend_groups;
        self.friend_requests = data.friend_requests;
        self.blocked_users = data.blocked_users;
        self.message_requests = data.message_requests;
        self.invites = data.invites;
        self.forum_posts = data.forum_posts;
        self.forum_open = None;
        self.events = data.events;
        self.boosts = data.boosts;
        self.welcome_screens = data.welcomes;
        self.welcome_joined = data.welcome_joined;
        self.backend_notifications = data.notifications;
        self.bookmarks = data.bookmarks;
        self.automod_by_server = data.automod_by_server;
        self.audit_by_server = data.audit_by_server;
        self.automod_rules = self
            .automod_by_server
            .get(&self.active_server_name())
            .cloned()
            .unwrap_or_default();
        self.audit = self
            .audit_by_server
            .get(&self.active_server_name())
            .cloned()
            .unwrap_or_default();
        self.dm_conversations = data.dm_conversations;
        self.active_dm = if keep_view
            && previous_dm
                .as_deref()
                .is_some_and(|name| self.dm_conversations.iter().any(|dm| dm.friend == name))
        {
            previous_dm
        } else {
            self.dm_conversations.first().map(|dm| dm.friend.to_string())
        };

        if keep_view {
            self.messages = previous_messages;
        } else {
            self.messages.clear();
            let key = self.active_key();
            self.loaded_channels.clear();
            self.loaded_channels.insert(key.clone());
            self.messages.insert(key, data.messages);
            self.dm_messages.clear();
            self.threads.clear();
            self.events.clear();
            self.audit.clear();
            self.bans.clear();
            self.automod_rules.clear();
            self.forum_posts.clear();
            self.forum_open = None;
                self.welcome_screens.clear();
            self.welcome_joined.clear();
            self.boosts.clear();
        }

        if !keep_view {
            self.view_mode = ViewMode::Server;
            self.show_settings = false;
            self.show_server_settings = false;
            self.show_search_center = false;
            self.show_help_center = false;
        }
        self.auth_status = AuthStatus::Ready;
        self.boot_message.clear();
        self.login_error = None;
        self.current_session = self.session_key();
        self.sync_invite_channel();
        if restart_gateway {
            self.start_gateway(cx);
        }
        self.structure_refreshing = false;
        cx.notify();
    }

    fn refresh_structure(&mut self, cx: &mut Context<Self>) {
        if !self.is_online() || self.structure_refreshing {
            return;
        }
        let Some(net) = self.net.clone() else {
            return;
        };
        self.structure_refreshing = true;
        cx.spawn(async move |this, cx| {
            let result = Self::fetch_boot(net).await;
            let _ = this.update(cx, |app, cx| match result {
                Ok(data) => app.apply_hydrated_inner(data, true, false, cx),
                Err(_) => {
                    app.structure_refreshing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn retry_boot(&mut self, cx: &mut Context<Self>) {
        if self.net.is_some() {
            self.begin_boot(cx);
        }
    }

    pub(crate) fn logout(&mut self, cx: &mut Context<Self>) {
        self.stop_gateway();
        let net = self.net.clone();
        self.auth_status = AuthStatus::Login;
        self.login_busy = false;
        self.login_error = None;
        self.show_settings = false;
        self.show_server_settings = false;
        self.show_search_center = false;
        self.servers.clear();
        self.messages.clear();
        self.dm_messages.clear();
        self.members.clear();
        self.roles.clear();
        self.role_permissions.clear();
        self.friends.clear();
        self.friend_groups.clear();
        self.dm_conversations.clear();
        self.active_dm = None;
        self.loaded_channels.clear();
        self.loaded_dms.clear();
        self.has_more_messages.clear();
        self.bookmarks.clear();
        self.notifications_on = true;
        cx.notify();
        if let Some(net) = net {
            cx.spawn(async move |_this, _cx| {
                let _ = net.logout().await;
            })
            .detach();
        }
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
            friend_groups: self.friend_groups.clone(),
            invites: Some(self.invites.clone()),
            audit: Some(self.audit.clone()),
            bans: Some(self.bans.clone()),
            members: Some(self.members.clone()),
            roles: Some(self.roles.clone()),
            permissions: Some(self.role_permissions.clone()),
            automod_rules: Some(self.automod_rules.clone()),
            events: Some(self.events.clone()),
            message_requests: Some(self.message_requests.clone()),
            forum_posts: Some(self.forum_posts.clone()),
            bookmarks: Some(self.bookmarks.clone()),
            welcome_screens: Some(self.welcome_screens.clone()),
            welcome_joined: self.welcome_joined.clone(),
            boosts: Some(self.boosts.clone()),
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
        // 分组顺序以快照为准；旧快照没有这个字段时保留种子里的默认顺序。
        if !saved.friend_groups.is_empty() {
            self.friend_groups = saved.friend_groups;
        }
        self.friend_requests = saved.friend_requests;
        self.blocked_users = saved.blocked_users;
        // 旧快照没有 invites 字段时保留种子邀请，而不是清空。
        if let Some(invites) = saved.invites {
            self.invites = invites;
        }
        // 审核日志：旧快照没有就保留种子日志。
        if let Some(audit) = saved.audit {
            self.audit = audit;
        }
        // 封禁列表：旧快照没有就保留种子数据。
        if let Some(bans) = saved.bans {
            self.bans = bans;
        }
        // 成员与角色：旧快照没有就保留种子；选中角色若已不存在则清空。
        if let Some(members) = saved.members {
            self.members = members;
        }
        if let Some(roles) = saved.roles {
            self.roles = roles;
        }
        // 权限：旧快照没有就保留种子；丢弃指向已不存在角色的脏记录。
        if let Some(permissions) = saved.permissions {
            self.role_permissions = permissions;
        }
        self.role_permissions
            .retain(|grant| self.roles.iter().any(|role| role.name == grant.role));
        // 自动审核：旧快照没有就保留种子；选中规则若已不存在则清空。
        if let Some(rules) = saved.automod_rules {
            self.automod_rules = rules;
        }
        // 日程：旧快照没有就保留种子；丢弃指向已不存在服务器的脏记录。
        if let Some(events) = saved.events {
            self.events = events;
        }
        self.events.retain(|event| {
            self.servers
                .iter()
                .any(|server| server.name == event.server)
        });
        // 私信请求：旧快照没有就保留种子。
        if let Some(requests) = saved.message_requests {
            self.message_requests = requests;
        }
        // 论坛帖子：旧快照没有就保留种子；丢弃指向已不存在频道/服务器的脏记录。
        if let Some(posts) = saved.forum_posts {
            self.forum_posts = posts;
        }
        self.forum_posts.retain(|post| {
            self.servers.iter().any(|server| {
                server.name == post.server
                    && server.categories.iter().any(|category| {
                        category.channels.iter().any(|channel| {
                            channel.name == post.channel && channel.kind == ChannelKind::Forum
                        })
                    })
            })
        });
        // 消息书签：旧快照没有就保留种子；丢弃指向已不存在的服务器 /
        // 私信好友的脏记录，避免点进去空跳。
        if let Some(markers) = saved.bookmarks {
            self.bookmarks = markers;
        }
        self.bookmarks.retain(|bookmark| {
            if bookmark.is_dm {
                self.friends.iter().any(|friend| friend.name == bookmark.channel)
            } else {
                self.servers.iter().any(|server| {
                    server.name == bookmark.server
                        && server
                            .categories
                            .iter()
                            .any(|category| {
                                category
                                    .channels
                                    .iter()
                                    .any(|channel| channel.name == bookmark.channel)
                            })
                })
            }
        });
        // 欢迎屏幕：旧快照没有就保留种子；丢弃服务器已不存在的配置与已完成记录。
        if let Some(screens) = saved.welcome_screens {
            self.welcome_screens = screens;
        }
        self.welcome_screens
            .retain(|welcome| self.servers.iter().any(|server| server.name == welcome.server));
        self.welcome_joined
            .retain(|name| self.servers.iter().any(|server| &server.name == name));
        // 加成记录同样按服务器名归属，丢弃已不存在的服务器。
        if let Some(boosts) = saved.boosts {
            self.boosts = boosts;
        }
        self.boosts
            .retain(|boost| self.servers.iter().any(|server| server.name == boost.server));
        if let Some(selected) = self.selected_automod.take() {
            self.selected_automod = self
                .automod_rules
                .iter()
                .any(|rule| rule.name == selected)
                .then_some(selected);
        }
        if let Some(selected) = self.selected_role.take() {
            self.selected_role = self
                .roles
                .iter()
                .any(|role| role.name == selected)
                .then_some(selected);
        }
        self.sync_invite_channel();
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
        if self.auth_status == AuthStatus::Ready {
            return;
        }
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
        self.friend_groups = seed_friend_groups();
        self.roles = seed_roles();
        self.selected_role = None;
        self.role_notice = None;
        self.role_permissions = seed_permissions();
        self.permission_notice = None;
        self.automod_rules = seed_automod_rules();
        self.selected_automod = None;
        self.automod_notice = None;
        self.events = seed_events();
        self.show_events = false;
        self.event_filter = EventFilter::All;
        self.selected_event = None;
        self.event_notice = None;
        self.show_event_create = false;
        self.message_requests = seed_message_requests();
        self.show_message_requests = false;
        self.message_request_filter = MessageRequestFilter::Pending;
        self.message_request_notice = None;
        self.forum_posts = seed_forum_posts();
        self.forum_open = None;
        self.forum_sort = ForumSort::Recent;
        self.forum_query.clear();
        self.forum_search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.show_forum_composer = false;
        self.forum_tags_draft.clear();
        self.forum_notice = None;
        self.bookmarks = seed_bookmarks();
        self.show_bookmarks = false;
        self.bookmark_query.clear();
        self.bookmark_search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.bookmark_notice = None;
        self.welcome_screens = seed_welcome_screens();
        self.welcome_target = None;
        self.welcome_rule_agreed = false;
        self.welcome_selected.clear();
        self.welcome_joined.clear();
        self.boosts = seed_boosts();
        self.boost_notice = None;
        self.invites = seed_invites();
        self.invite_copied_code = None;
        self.audit = seed_audit();
        self.audit_filter = None;
        self.bans = seed_bans();
        self.member_filter = MemberFilter::All;
        self.show_member_nick = false;
        self.member_nick_target.clear();
        self.sync_invite_channel();
        self.collapsed_groups.clear();
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
        // 论坛频道接管聊天主区域（本身是一张独立的帖子列表页）。
        let forum_open = self.is_forum_view();

        let main = if search_open {
            search_center::render(self, cx).into_any_element()
        } else {
            match view_mode {
                ViewMode::Friends => friends::render(self, cx).into_any_element(),
                ViewMode::Server if forum_open => forum::render(self, cx).into_any_element(),
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
            .when(self.show_bookmarks && view_mode != ViewMode::Friends, |el| {
                el.child(chat::bookmarks_panel::render(self, cx))
            })
            .when(show_thread, |el| {
                el.child(chat::thread_panel::render(self, cx))
            })
            .when(self.show_inbox, |el| {
                el.child(chat::inbox_panel::render(self, cx))
            })
            .when(self.show_events && view_mode != ViewMode::Friends, |el| {
                el.child(events_panel::render(self, cx))
            })
            .when(self.show_message_requests, |el| {
                el.child(message_requests_panel::render(self, cx))
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
            .when(self.show_create_group || self.show_rename_group, |el| {
                el.child(friend_group_dialog::render(self, cx))
            })
            .when(self.show_member_nick, |el| {
                el.child(member_nick_dialog::render(self, cx))
            })
            .when(self.friend_group_menu.is_some(), |el| {
                el.child(friend_group_menu::render(self, cx))
            })
            .when(self.friend_group_picker.is_some(), |el| {
                el.child(friend_group_menu::render_picker(self, cx))
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
            // 欢迎屏幕是顶层模态，放在最后保证盖住菜单与面板。
            .when(self.welcome_target.is_some(), |el| {
                el.child(welcome::render(self, cx))
            })
            .child(voice_status::render(self, cx))
    }
}

impl Render for DiscordApp {
    #[allow(refining_impl_trait)]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // 内容有变化就顺手落盘；没有变化时这里只是序列化一次做比较。
        self.autosave();
        if !matches!(self.auth_status, AuthStatus::Ready | AuthStatus::Demo) {
            return login::render(self, cx).into_any_element();
        }
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
            .when(self.auth_status == AuthStatus::Demo, |el| {
                el.child(
                    div()
                        .absolute()
                        .right(px(12.))
                        .bottom(px(64.))
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(rgb(palette::bg_footer()))
                        .text_size(px(11.))
                        .text_color(rgb(palette::text_muted()))
                        .id("demo-mode-badge")
                        .child("离线演示"),
                )
            })
            .into_any_element()
    }
}

