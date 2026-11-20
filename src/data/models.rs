use gpui_kit::{rgb, Rgba};

/// 单条消息上的表情回应。
#[derive(Clone)]
pub struct Reaction {
    pub emoji: &'static str,
    pub count: usize,
    pub mine: bool,
}

/// 附件展示形态：图片给缩略图，其他文件给文件卡。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    Image,
    File,
}

/// 消息附件（纯 Demo：只保存文件名与大小文案，不做真实上传）。
#[derive(Clone)]
pub struct Attachment {
    pub name: String,
    pub size: &'static str,
    pub kind: AttachmentKind,
}

impl Attachment {
    pub fn image(name: impl Into<String>, size: &'static str) -> Self {
        Self {
            name: name.into(),
            size,
            kind: AttachmentKind::Image,
        }
    }

    pub fn file(name: impl Into<String>, size: &'static str) -> Self {
        Self {
            name: name.into(),
            size,
            kind: AttachmentKind::File,
        }
    }

    pub fn is_image(&self) -> bool {
        self.kind == AttachmentKind::Image
    }
}

/// 聊天消息。
#[derive(Clone)]
pub struct Message {
    pub author: &'static str,
    pub time: &'static str,
    pub text: String,
    pub color: Rgba,
    pub grouped: bool,
    pub reactions: Vec<Reaction>,
    pub pinned: bool,
    pub attachment: Option<Attachment>,
}

impl Message {
    pub fn new(author: &'static str, time: &'static str, text: impl Into<String>, color: u32) -> Self {
        Self {
            author,
            time,
            text: text.into(),
            color: rgb(color),
            grouped: false,
            reactions: Vec::new(),
            pinned: false,
            attachment: None,
        }
    }

    pub fn grouped(mut self) -> Self {
        self.grouped = true;
        self
    }

    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    pub fn reaction(mut self, emoji: &'static str, count: usize) -> Self {
        self.reactions.push(Reaction {
            emoji,
            count,
            mine: false,
        });
        self
    }

    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachment = Some(attachment);
        self
    }
}

/// 用户在线状态：影响头像状态点与底部状态文字。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Online,
    Idle,
    Dnd,
    Invisible,
}

impl UserStatus {
    pub fn label(self) -> &'static str {
        match self {
            UserStatus::Online => "在线",
            UserStatus::Idle => "闲置",
            UserStatus::Dnd => "请勿打扰",
            UserStatus::Invisible => "隐身",
        }
    }

    /// Discord 标准状态色。
    pub fn color(self) -> u32 {
        match self {
            UserStatus::Online => 0x23a55a,
            UserStatus::Idle => 0xf0b232,
            UserStatus::Dnd => 0xf23f43,
            UserStatus::Invisible => 0x80848e,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Text,
    Voice,
}

/// 频道（文字或语音）。
#[derive(Clone)]
pub struct Channel {
    pub name: String,
    pub kind: ChannelKind,
    pub unread: usize,
    pub topic: String,
}

impl Channel {
    pub fn text(name: &str, topic: &str) -> Self {
        Self {
            name: name.into(),
            kind: ChannelKind::Text,
            unread: 0,
            topic: topic.into(),
        }
    }

    pub fn voice(name: &str) -> Self {
        Self {
            name: name.into(),
            kind: ChannelKind::Voice,
            unread: 0,
            topic: String::new(),
        }
    }

    pub fn unread(mut self, count: usize) -> Self {
        self.unread = count;
        self
    }
}

/// 频道分类。名称用 String，支持运行时重命名。
#[derive(Clone)]
pub struct Category {
    pub name: String,
    pub channels: Vec<Channel>,
}

impl Category {
    pub fn new(name: impl Into<String>, channels: Vec<Channel>) -> Self {
        Self {
            name: name.into(),
            channels,
        }
    }
}

/// 服务器。名称与徽标用 String，支持运行时新建。
pub struct Server {
    pub name: String,
    pub badge: String,
    pub color: u32,
    /// 是否静音：静音后菜单显示「取消静音」，侧栏标题旁出现静音图标。
    pub muted: bool,
    /// 服务器简介，在服务器设置「概览」里展示与编辑。
    pub description: String,
    pub categories: Vec<Category>,
}

impl Server {
    /// 用名称首字符作为徽标创建一个带默认文字频道的新服务器。
    pub fn new(name: impl Into<String>, color: u32) -> Self {
        let name = name.into();
        let badge = name.chars().next().unwrap_or('?').to_string();
        Self {
            name,
            badge,
            color,
            muted: false,
            description: "刚创建的新服务器，快去邀请朋友吧。".into(),
            categories: vec![Category::new(
                "文字频道",
                vec![Channel::text("闲聊", "欢迎来到新服务器")],
            )],
        }
    }
}

/// 全量搜索的一条命中结果（跨频道 / 私信）。
#[derive(Clone)]
pub struct SearchHit {
    /// 频道所属服务器名；私信结果固定为「私信」。
    pub server: String,
    /// 频道名；私信结果为好友名。
    pub channel: String,
    pub is_dm: bool,
    pub author: &'static str,
    pub time: &'static str,
    pub text: String,
    pub color: Rgba,
}

impl SearchHit {
    /// 结果行的归属标签：服务器频道用「服务器 ▸ #频道」，私信用「私信 ▸ 好友」。
    pub fn scope_label(&self) -> String {
        if self.is_dm {
            format!("私信 ▸ {}", self.channel)
        } else {
            format!("{} ▸ #{}", self.server, self.channel)
        }
    }
}

/// 探索页里的公开服务器卡片。joined 表示是否已加入（加入后会进入服务器栏）。
#[derive(Clone)]
pub struct DiscoverServer {
    pub name: String,
    pub badge: String,
    pub color: u32,
    pub category: &'static str,
    pub members: usize,
    pub description: String,
    pub joined: bool,
}

impl DiscoverServer {
    pub fn new(
        name: &str,
        color: u32,
        category: &'static str,
        members: usize,
        description: &str,
    ) -> Self {
        Self {
            name: name.into(),
            badge: name.chars().next().unwrap_or('?').to_string(),
            color,
            category,
            members,
            description: description.into(),
            joined: false,
        }
    }
}

/// 成员栏条目。name 可形如「Nova —— 管理员」，后缀为角色名。
#[derive(Clone, Copy)]
pub struct Member {
    pub name: &'static str,
    pub color: u32,
    pub online: bool,
}

/// 语音房间里的其他参与者（Demo：连接语音时从在线成员里抽几位）。
#[derive(Clone)]
pub struct VoiceParticipant {
    pub name: &'static str,
    pub color: u32,
    /// 是否被服务器静音（点击可切换）。
    pub muted: bool,
}

/// 成员栏角色分组：由 app 层按角色聚合成员后供视图渲染。
pub struct MemberGroup {
    /// 分组标题（角色名 / 在线 / 离线）。
    pub title: String,
    /// 标题与人数 badge 的颜色。
    pub color: u32,
    /// 折叠状态用的稳定 key。
    pub key: String,
    /// 该组成员在 app.members 中的索引。
    pub member_indices: Vec<usize>,
}

/// 好友列表条目。
#[derive(Clone, Copy)]
pub struct Friend {
    pub name: &'static str,
    pub color: u32,
    pub online: bool,
    pub activity: &'static str,
}

/// 好友请求：incoming 为收到的请求，否则为发出的请求。
#[derive(Clone)]
pub struct FriendRequest {
    pub name: String,
    pub color: u32,
    pub incoming: bool,
}

/// 好友页标签。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FriendTab {
    Online,
    Pending,
    All,
    Blocked,
    AddFriend,
}

/// 私信会话列表条目。
#[derive(Clone, Copy)]
pub struct DmConversation {
    pub friend: &'static str,
    pub color: u32,
    pub online: bool,
    pub unread: usize,
    pub preview: &'static str,
}

/// 通知来源类型，决定图标与文案。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Mention,
    UnreadChannel,
    Dm,
    FriendRequest,
}

/// 通知跳转目标。
#[derive(Clone)]
pub enum NotificationTarget {
    Channel { server: usize, channel: String },
    Dm(String),
    Friends,
}

/// 收件箱通知条目，由 app 层从未读数据聚合而来。
#[derive(Clone)]
pub struct Notification {
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
    pub time: &'static str,
    pub color: u32,
    pub target: NotificationTarget,
}
