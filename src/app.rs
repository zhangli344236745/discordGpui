use std::collections::HashMap;

use gpui_kit::*;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{v_flex, Theme, ThemeMode};
use gpui_kit::prelude::FluentBuilder as _;

use crate::data::{
    seed_discover_servers, seed_dm_conversations, seed_dm_messages, seed_friend_requests,
    seed_friends, seed_members, seed_messages, seed_servers, seed_stickers, sticker_packs, Attachment,
    Category, Channel, ChannelKind, DiscoverServer, DmConversation, ForwardOrigin, ForwardTarget,
    Friend, FriendRequest, FriendTab, Member, MemberGroup, Message, Notification, NotificationKind,
    NotificationTarget, Reaction, SearchHit, Server, Sticker, UserStatus, VoiceParticipant,
};
use crate::ui::theme::palette;
use crate::ui::{
    category_dialog, category_menu, channel_menu, channel_sidebar, chat, create_dialog, dm_sidebar,
    discover, friends, invite_dialog, member_list, profile_card, server_menu, server_rail,
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

/// 频道操作菜单的锚点：鼠标右键位置或标题栏铃铛入口。
#[derive(Clone, Copy)]
pub(crate) enum MenuAnchor {
    /// 窗口坐标（像素），用于右键菜单跟随光标。
    Point(f32, f32),
    /// 聊天标题栏右上角，菜单贴右边缘向下展开。
    Header,
}

/// 频道操作菜单状态：目标频道名 + 展开锚点。
#[derive(Clone)]
pub(crate) struct ChannelMenuState {
    pub(crate) name: String,
    pub(crate) anchor: MenuAnchor,
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
}

/// 外观主题选项。浅色会真正切换色板；「跟随系统」在 Demo 里等同深色（不读系统外观）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeChoice {
    Dark,
    Light,
    System,
}

impl ThemeChoice {
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
    pub(crate) reply_to: Option<&'static str>,
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

    // 服务器下拉菜单（邀请 / 静音 / 标记已读 / 创建频道 / 离开）与邀请弹层。
    pub(crate) show_server_menu: bool,
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

    // 消息转发：源消息下标 + 弹层显隐 + 目标搜索 + 成功提示。
    pub(crate) forward_source: Option<usize>,
    pub(crate) show_forward: bool,
    pub(crate) forward_search: Entity<InputState>,
    pub(crate) forward_query: String,
    /// 转发成功后的顶栏提示文案，切换会话即清除。
    pub(crate) forward_notice: Option<String>,
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
fn hit_matches(message: &Message, query: &str) -> bool {
    message.text.to_lowercase().contains(query) || message.author.to_lowercase().contains(query)
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
        let server_name_edit =
            cx.new(|cx| InputState::new(window, cx).placeholder("服务器名称"));
        let server_desc_edit =
            cx.new(|cx| InputState::new(window, cx).placeholder("用一两句话介绍这个服务器"));
        let discover_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索服务器或关键词"));
        let search_center_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索所有频道与私信"));
        let forward_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索频道或私信"));

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

        Self {
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
            mention_prefix: None,
            mention_candidates: Vec::new(),
            profile_target: None,
            menu_message: None,
            menu_message_point: None,
            show_pins: false,
            voice_channel: None,
            my_status: UserStatus::Online,
            show_status_menu: false,
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
            forward_source: None,
            show_forward: false,
            forward_search,
            forward_query: String::new(),
            forward_notice: None,
        }
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
        self.clear_search(window, cx);
        self.clear_dm_search(window, cx);
        self.emoji_picker_open = false;
        self.emoji_target = None;
        self.reply_to = None;
        self.mention_prefix = None;
        self.mention_candidates.clear();
        self.profile_target = None;
        self.close_message_menu();
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
        self.show_search_center = false;
        self.close_forward();
        self.forward_notice = None;
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

    /// 打开频道操作菜单，并收起其它浮层，避免叠层。
    pub(crate) fn open_channel_menu(&mut self, name: String, anchor: MenuAnchor) {
        self.channel_menu = Some(ChannelMenuState { name, anchor });
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
    }

    /// 静音 / 取消静音当前服务器。
    pub(crate) fn toggle_server_mute(&mut self, cx: &mut Context<Self>) {
        self.servers[self.active_server].muted = !self.servers[self.active_server].muted;
        self.show_server_menu = false;
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
        let mut message = Message::new("你", "刚刚", text, palette::accent());
        if let Some(attachment) = self.attachment_draft.take() {
            message = message.attachment(attachment);
        }
        if let Some(messages) = self.current_messages_mut() {
            messages.push(message);
        }
        self.reply_to = None;
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
        } else {
            self.show_sticker_picker = true;
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

    /// 关闭贴纸弹层。
    pub(crate) fn close_sticker_picker(&mut self) {
        self.show_sticker_picker = false;
    }

    /// 切换贴纸包。
    pub(crate) fn set_sticker_pack(&mut self, pack: &'static str, cx: &mut Context<Self>) {
        self.sticker_pack = pack;
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
        let message = Message::new("你", "刚刚", "", palette::accent()).sticker(sticker);
        if let Some(messages) = self.current_messages_mut() {
            messages.push(message);
        }
        self.reply_to = None;
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

    /// 设置/取消回复目标。
    pub(crate) fn toggle_reply(&mut self, author: &'static str) {
        if self.reply_to == Some(author) {
            self.reply_to = None;
        } else {
            self.reply_to = Some(author);
        }
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

    /// 删除当前会话的第 index 条消息。
    pub(crate) fn delete_message(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(messages) = self.current_messages_mut() {
            if index < messages.len() {
                messages.remove(index);
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
                message.text = text;
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

    /// 切换搜索范围并立即重跑。
    pub(crate) fn set_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        if self.search_scope == scope {
            return;
        }
        self.search_scope = scope;
        self.run_search();
        cx.notify();
    }

    /// 按当前关键词与范围重算结果；关键词为空时结果直接清空。
    fn run_search(&mut self) {
        let query = self.search_center_query.trim().to_lowercase();
        self.search_results.clear();
        if query.is_empty() {
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
                            if !hit_matches(message, &query) {
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
                    if !hit_matches(message, &query) {
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

    /// 打开设置页（覆盖主界面），同时关闭其他浮层。
    pub(crate) fn open_settings(&mut self) {
        self.show_settings = true;
        self.show_server_settings = false;
        self.show_voice_room = false;
        self.show_search_center = false;
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
            .when(self.show_inbox, |el| {
                el.child(chat::inbox_panel::render(self, cx))
            })
            .when(self.member_menu.is_some(), |el| {
                el.child(member_menu::render(self, cx))
            })
            .when(self.menu_message_point.is_some(), |el| {
                el.child(chat::message_menu::render_floating(self, cx))
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
            .when(self.show_voice_room && self.voice_channel.is_some(), |el| {
                el.child(voice_room::render(self, cx))
            })
            .child(voice_status::render(self, cx))
    }
}

impl Render for DiscordApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.show_server_settings {
            server_settings::render(self, cx).into_any_element()
        } else if self.show_settings {
            settings::render(self, cx).into_any_element()
        } else {
            self.render_main(cx).into_any_element()
        }
    }
}
