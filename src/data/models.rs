use gpui_kit::{rgb, Rgba};
use serde::{Deserialize, Serialize};

/// 单条消息上的表情回应。
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::ReactionOwned")]
pub struct Reaction {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub emoji: &'static str,
    pub count: usize,
    pub mine: bool,
}

/// 附件展示形态：图片给缩略图，其他文件给文件卡。
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentKind {
    Image,
    File,
}

/// 消息附件（纯 Demo：只保存文件名与大小文案，不做真实上传）。
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::AttachmentOwned")]
pub struct Attachment {
    pub name: String,
    #[serde(with = "crate::data::persist::leaked_str")]
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

/// 贴纸：Demo 用 emoji 字形 + 名称代替真实贴纸资源，按 pack 分组展示。
#[derive(Clone, Copy, Serialize)]
#[serde(from = "crate::data::persist::StickerOwned")]
pub struct Sticker {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub emoji: &'static str,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub label: &'static str,
    /// 所属贴纸包，同时也是弹层里的分类名。
    #[serde(with = "crate::data::persist::leaked_str")]
    pub pack: &'static str,
}

impl Sticker {
    pub const fn new(emoji: &'static str, label: &'static str, pack: &'static str) -> Self {
        Self { emoji, label, pack }
    }
}

/// GIF：Demo 用色块 + emoji 字形模拟动图缩略图，按分类分组、按标签搜索。
#[derive(Clone, Copy, Serialize)]
#[serde(from = "crate::data::persist::GifOwned")]
pub struct Gif {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub title: &'static str,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub emoji: &'static str,
    pub color: u32,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub category: &'static str,
    /// 空格分隔的搜索关键词，搜索框按子串匹配标题或标签。
    #[serde(with = "crate::data::persist::leaked_str")]
    pub tags: &'static str,
}

impl Gif {
    pub const fn new(
        title: &'static str,
        emoji: &'static str,
        color: u32,
        category: &'static str,
        tags: &'static str,
    ) -> Self {
        Self {
            title,
            emoji,
            color,
            category,
            tags,
        }
    }
}

/// 消息的一次历史版本：保存编辑前的正文与编辑时间。
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::EditRevisionOwned")]
pub struct EditRevision {
    pub text: String,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub time: &'static str,
}

/// 转发来源：记录消息从哪个频道/私信转发而来，供正文上方的小字与点击跳转使用。
#[derive(Clone, Serialize, Deserialize)]
pub struct ForwardOrigin {
    /// 展示文案，如「#闲聊」「@Nova」。
    pub label: String,
    pub is_dm: bool,
    /// 频道所属服务器；私信为 None。
    pub server: Option<String>,
    /// 频道名或好友名，跳转时使用。
    pub channel: String,
}

impl ForwardOrigin {
    pub fn channel(server: impl Into<String>, channel: impl Into<String>) -> Self {
        let channel = channel.into();
        Self {
            label: format!("#{channel}"),
            is_dm: false,
            server: Some(server.into()),
            channel,
        }
    }

    pub fn dm(friend: impl Into<String>) -> Self {
        let friend = friend.into();
        Self {
            label: format!("@{friend}"),
            is_dm: true,
            server: None,
            channel: friend,
        }
    }
}

/// 转发弹层里的一个可选目标：某个文字频道或某条私信。
#[derive(Clone, Serialize, Deserialize)]
pub struct ForwardTarget {
    /// 频道所属服务器名；私信为「私信」。
    pub server: String,
    /// 频道名或好友名。
    pub channel: String,
    pub is_dm: bool,
    /// 行首图标/文字的高亮色。
    pub color: u32,
}

impl ForwardTarget {
    pub fn channel(server: impl Into<String>, channel: impl Into<String>, color: u32) -> Self {
        Self {
            server: server.into(),
            channel: channel.into(),
            is_dm: false,
            color,
        }
    }

    pub fn dm(friend: impl Into<String>, color: u32) -> Self {
        Self {
            server: "私信".into(),
            channel: friend.into(),
            is_dm: true,
            color,
        }
    }

    /// 目标标题：「#闲聊」或「@Nova」。
    pub fn label(&self) -> String {
        if self.is_dm {
            format!("@{}", self.channel)
        } else {
            format!("#{}", self.channel)
        }
    }

    /// 目标归属：「开发者社区」或「私信」。
    pub fn scope(&self) -> String {
        if self.is_dm {
            "私信".into()
        } else {
            self.server.clone()
        }
    }
}

/// 消息回复引用：作者 + 原消息摘要，点击可跳回原消息。
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::ReplyRefOwned")]
pub struct ReplyRef {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub author: &'static str,
    pub excerpt: String,
}

/// 自己发出的消息的送达状态：已发送 → 已送达 → 已读。
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Sent,
    Delivered,
    Read,
}

/// 已读回执中的一位读者（Demo 用静态名与时间，不接后端）。
#[derive(Clone, Copy, Serialize)]
#[serde(from = "crate::data::persist::ReadReceiptOwned")]
pub struct ReadReceipt {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub name: &'static str,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub time: &'static str,
    pub color: u32,
}

impl ReadReceipt {
    pub const fn new(name: &'static str, time: &'static str, color: u32) -> Self {
        Self { name, time, color }
    }
}

/// 话题里的一条回复：复用消息的最小展示字段，不参与主消息列表。
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::ThreadReplyOwned")]
pub struct ThreadReply {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub author: &'static str,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub time: &'static str,
    pub text: String,
    #[serde(with = "crate::data::persist::rgba_u32")]
    pub color: Rgba,
}

impl ThreadReply {
    pub fn new(
        author: &'static str,
        time: &'static str,
        text: impl Into<String>,
        color: u32,
    ) -> Self {
        Self {
            author,
            time,
            text: text.into(),
            color: rgb(color),
        }
    }
}

/// 挂在某条主消息上的话题：root_index 是它在当前会话消息列表里的下标。
#[derive(Clone, Serialize, Deserialize)]
pub struct ChatThread {
    pub root_index: usize,
    pub title: String,
    pub replies: Vec<ThreadReply>,
}

impl ChatThread {
    pub fn new(
        root_index: usize,
        title: impl Into<String>,
        replies: Vec<ThreadReply>,
    ) -> Self {
        Self {
            root_index,
            title: title.into(),
            replies,
        }
    }
}

/// 聊天消息。
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::MessageOwned")]
pub struct Message {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub author: &'static str,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub time: &'static str,
    pub text: String,
    #[serde(with = "crate::data::persist::rgba_u32")]
    pub color: Rgba,
    pub grouped: bool,
    pub reactions: Vec<Reaction>,
    pub pinned: bool,
    pub attachment: Option<Attachment>,
    /// 贴纸消息：正文通常为空，只渲染贴纸卡。
    pub sticker: Option<Sticker>,
    /// GIF 消息：正文通常为空，只渲染 GIF 卡（Demo 为色块占位）。
    pub gif: Option<Gif>,
    /// 编辑历史：每次成功编辑前把旧正文存一份；非空即显示「已编辑」。
    pub edit_history: Vec<EditRevision>,
    /// 转发来源；Some 时正文上方显示「转发自 …」。
    pub forwarded: Option<ForwardOrigin>,
    /// 回复引用；Some 时正文上方显示「回复 @作者：摘要」。
    pub reply_to: Option<ReplyRef>,
    /// 自己发出的消息的送达状态；他人消息保持默认的「已发送」但不渲染。
    pub delivery: DeliveryStatus,
    /// 已读者列表，非空时状态行可点开详情。
    pub readers: Vec<ReadReceipt>,
    /// 投票消息：正文可作为投票标题，另渲染投票卡。
    pub poll: Option<Poll>,
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
            sticker: None,
            gif: None,
            edit_history: Vec::new(),
            forwarded: None,
            reply_to: None,
            delivery: DeliveryStatus::Sent,
            readers: Vec::new(),
            poll: None,
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

    /// 挂一张贴纸（消息正文可为空）。
    pub fn sticker(mut self, sticker: Sticker) -> Self {
        self.sticker = Some(sticker);
        self
    }

    /// 挂一个 GIF（消息正文可为空）。
    pub fn gif(mut self, gif: Gif) -> Self {
        self.gif = Some(gif);
        self
    }

    /// 挂一张投票卡（正文可作为投票说明）。
    pub fn poll(mut self, poll: Poll) -> Self {
        self.poll = Some(poll);
        self
    }

    /// 追加一个编辑前的历史版本。
    pub fn revised(mut self, text: impl Into<String>, time: &'static str) -> Self {
        self.edit_history.push(EditRevision {
            text: text.into(),
            time,
        });
        self
    }

    /// 是否编辑过（有历史版本即视为编辑过）。
    pub fn is_edited(&self) -> bool {
        !self.edit_history.is_empty()
    }

    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachment = Some(attachment);
        self
    }

    /// 标记为转发消息，并记录来源。
    pub fn forwarded(mut self, origin: ForwardOrigin) -> Self {
        self.forwarded = Some(origin);
        self
    }

    /// 标记为回复消息，记录被回复作者与摘要。
    pub fn replying_to(mut self, author: &'static str, excerpt: impl Into<String>) -> Self {
        self.reply_to = Some(ReplyRef {
            author,
            excerpt: excerpt.into(),
        });
        self
    }

    /// 标记为已送达（尚未有人读）。
    pub fn delivered(mut self) -> Self {
        self.delivery = DeliveryStatus::Delivered;
        self
    }

    /// 标记为已读并记录读者；空列表按「已送达」处理。
    pub fn read_by(mut self, readers: Vec<ReadReceipt>) -> Self {
        self.delivery = if readers.is_empty() {
            DeliveryStatus::Delivered
        } else {
            DeliveryStatus::Read
        };
        self.readers = readers;
        self
    }
}

/// 用户在线状态：影响头像状态点与底部状态文字。
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Online,
    Idle,
    Dnd,
    Invisible,
}

impl Default for UserStatus {
    fn default() -> Self {
        UserStatus::Online
    }
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

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    Text,
    Voice,
    /// 论坛频道：不承载聊天消息，而是展示可按标签/热度排序的帖子列表。
    Forum,
}

/// 频道通知级别。`Inherit` 表示跟随服务器默认，其余三级各自生效。
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub enum NotifyLevel {
    #[default]
    Inherit,
    All,
    Mentions,
    Nothing,
}

impl NotifyLevel {
    /// 单选项标题。
    pub fn label(self) -> &'static str {
        match self {
            NotifyLevel::Inherit => "使用服务器默认",
            NotifyLevel::All => "全部消息",
            NotifyLevel::Mentions => "仅 @提及",
            NotifyLevel::Nothing => "关闭通知",
        }
    }

    /// 单选项副标题，解释该级别具体会推什么。
    pub fn hint(self) -> &'static str {
        match self {
            NotifyLevel::Inherit => "跟随时通知设置，不做单独覆盖",
            NotifyLevel::All => "每条新消息都提醒",
            NotifyLevel::Mentions => "只有提到你或 @所有人 时提醒",
            NotifyLevel::Nothing => "不提醒，侧栏仅显示灰点",
        }
    }

    /// 菜单里的一行摘要（「通知设置 · 仅 @提及」）。
    pub fn short(self) -> &'static str {
        match self {
            NotifyLevel::Inherit => "跟随服务器",
            NotifyLevel::All => "全部消息",
            NotifyLevel::Mentions => "仅 @提及",
            NotifyLevel::Nothing => "已静音",
        }
    }

    /// 是否算静音：静音频道在侧栏显示铃铛斜线、未读角标置灰。
    pub fn is_muted(self) -> bool {
        self == NotifyLevel::Nothing
    }

    /// 稳定的索引，供单选项遍历与回调传值。
    pub fn index(self) -> usize {
        match self {
            NotifyLevel::Inherit => 0,
            NotifyLevel::All => 1,
            NotifyLevel::Mentions => 2,
            NotifyLevel::Nothing => 3,
        }
    }
}

/// 频道（文字或语音）。
#[derive(Clone, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub kind: ChannelKind,
    pub unread: usize,
    pub topic: String,
    pub notify: NotifyLevel,
}

impl Channel {
    pub fn text(name: &str, topic: &str) -> Self {
        Self {
            name: name.into(),
            kind: ChannelKind::Text,
            unread: 0,
            topic: topic.into(),
            notify: NotifyLevel::Inherit,
        }
    }

    pub fn voice(name: &str) -> Self {
        Self {
            name: name.into(),
            kind: ChannelKind::Voice,
            unread: 0,
            topic: String::new(),
            notify: NotifyLevel::Inherit,
        }
    }

    pub fn forum(name: &str, topic: &str) -> Self {
        Self {
            name: name.into(),
            kind: ChannelKind::Forum,
            unread: 0,
            topic: topic.into(),
            notify: NotifyLevel::Inherit,
        }
    }

    /// 种子数据里给某频道预设通知级别，用来演示静音态。
    pub fn notify(mut self, level: NotifyLevel) -> Self {
        self.notify = level;
        self
    }

    pub fn unread(mut self, count: usize) -> Self {
        self.unread = count;
        self
    }
}

/// 频道分类。名称用 String，支持运行时重命名。
#[derive(Clone, Serialize, Deserialize)]
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
#[derive(Clone, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub badge: String,
    pub color: u32,
    /// 服务器默认通知级别：频道选「跟随服务器」时继承这个值；`Nothing` 等同服务器静音。
    pub notify: NotifyLevel,
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
            notify: NotifyLevel::All,
            description: "刚创建的新服务器，快去邀请朋友吧。".into(),
            categories: vec![Category::new(
                "文字频道",
                vec![Channel::text("闲聊", "欢迎来到新服务器")],
            )],
        }
    }
}

/// 邀请链接的有效期选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteExpiry {
    ThirtyMinutes,
    OneHour,
    SixHours,
    TwelveHours,
    OneDay,
    SevenDays,
    Never,
}

impl InviteExpiry {
    /// 表单里按顺序展示的全部选项。
    pub const ALL: [InviteExpiry; 7] = [
        InviteExpiry::ThirtyMinutes,
        InviteExpiry::OneHour,
        InviteExpiry::SixHours,
        InviteExpiry::TwelveHours,
        InviteExpiry::OneDay,
        InviteExpiry::SevenDays,
        InviteExpiry::Never,
    ];

    pub fn label(self) -> &'static str {
        match self {
            InviteExpiry::ThirtyMinutes => "30 分钟",
            InviteExpiry::OneHour => "1 小时",
            InviteExpiry::SixHours => "6 小时",
            InviteExpiry::TwelveHours => "12 小时",
            InviteExpiry::OneDay => "1 天",
            InviteExpiry::SevenDays => "7 天",
            InviteExpiry::Never => "永不",
        }
    }

    /// 有效期分钟数；0 表示永不过期。
    pub fn minutes(self) -> u32 {
        match self {
            InviteExpiry::ThirtyMinutes => 30,
            InviteExpiry::OneHour => 60,
            InviteExpiry::SixHours => 6 * 60,
            InviteExpiry::TwelveHours => 12 * 60,
            InviteExpiry::OneDay => 24 * 60,
            InviteExpiry::SevenDays => 7 * 24 * 60,
            InviteExpiry::Never => 0,
        }
    }
}

/// 一条服务器邀请。Demo 里只在本机生成与撤销，不真正走后端。
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::InviteOwned")]
pub struct Invite {
    /// 邀请码，链接即 `discord.gg/{code}`。
    pub code: String,
    /// 所属服务器名；与消息键一致，按名称归属。
    pub server: String,
    /// 邀请着陆的文字频道名。
    pub channel: String,
    /// 创建者显示名。
    #[serde(with = "crate::data::persist::leaked_str")]
    pub inviter: &'static str,
    /// 已被使用次数。
    pub uses: u32,
    /// 最大使用次数；0 表示不限。
    pub max_uses: u32,
    pub expiry: InviteExpiry,
    /// 创建于多少分钟前（演示值是静态的，新建的从 0 开始）。
    pub created_minutes: u32,
}

impl Invite {
    pub fn link(&self) -> String {
        format!("discord.gg/{}", self.code)
    }

    /// 次数已用满（不限次数时永远为 false）。
    pub fn used_up(&self) -> bool {
        self.max_uses > 0 && self.uses >= self.max_uses
    }

    pub fn expired(&self) -> bool {
        let total = self.expiry.minutes();
        total > 0 && self.created_minutes >= total
    }

    /// 失效状态标签；仍有效时返回 None。
    pub fn status(&self) -> Option<&'static str> {
        if self.expired() {
            Some("已过期")
        } else if self.used_up() {
            Some("已用完")
        } else {
            None
        }
    }

    /// 使用次数文案：`3 / 10` 或 `3 / 不限`。
    pub fn uses_label(&self) -> String {
        if self.max_uses == 0 {
            format!("{} / 不限", self.uses)
        } else {
            format!("{} / {}", self.uses, self.max_uses)
        }
    }

    /// 有效期文案：剩余时间 / 已过期 / 永不过期。
    pub fn expiry_label(&self) -> String {
        let total = self.expiry.minutes();
        if total == 0 {
            return "永不过期".into();
        }
        if self.created_minutes >= total {
            return "已过期".into();
        }
        let left = total - self.created_minutes;
        if left < 60 {
            format!("{left} 分钟后过期")
        } else if left < 24 * 60 {
            format!("{} 小时后过期", left / 60)
        } else {
            format!("{} 天后过期", left / (24 * 60))
        }
    }
}

/// 审核日志的类别：决定筛选条、图标与配色。
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum AuditKind {
    Server,
    Channel,
    Role,
    Member,
    Invite,
    Emoji,
}

impl AuditKind {
    /// 全部类别，顺序即筛选条的展示顺序。
    pub const ALL: [AuditKind; 6] = [
        AuditKind::Server,
        AuditKind::Channel,
        AuditKind::Role,
        AuditKind::Member,
        AuditKind::Invite,
        AuditKind::Emoji,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AuditKind::Server => "服务器",
            AuditKind::Channel => "频道",
            AuditKind::Role => "角色",
            AuditKind::Member => "成员",
            AuditKind::Invite => "邀请",
            AuditKind::Emoji => "表情",
        }
    }

    /// 行首图标的底色。
    pub fn color(self) -> u32 {
        match self {
            AuditKind::Server => 0x5865f2,
            AuditKind::Channel => 0x00a8fc,
            AuditKind::Role => 0xfaa61c,
            AuditKind::Member => 0xf23f43,
            AuditKind::Invite => 0x57f287,
            AuditKind::Emoji => 0xeb459e,
        }
    }
}

/// 一条审核日志。全部字段都是自有 String，可直接派发 serde，无需 `*Owned` 镜像。
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AuditEntry {
    /// 全局递增序号：越大越新，同时用来推算「多久以前」。
    pub seq: u32,
    pub kind: AuditKind,
    /// 操作者显示名。
    pub actor: String,
    /// 做了什么，如「更新了服务器名称」。
    pub action: String,
    /// 具体内容，如「开发者社区 → Dev Community」；允许为空。
    pub detail: String,
}

impl AuditEntry {
    pub fn new(
        seq: u32,
        kind: AuditKind,
        actor: impl Into<String>,
        action: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            seq,
            kind,
            actor: actor.into(),
            action: action.into(),
            detail: detail.into(),
        }
    }

    /// 相对时间文案：最新一条是「刚刚」，其余按序号差推分钟 / 小时 / 天。
    /// 序号差当作「中间隔了几次操作」，每次操作按一分钟计。
    pub fn time_label(&self, newest: u32) -> String {
        let diff = newest.saturating_sub(self.seq);
        match diff {
            0 => "刚刚".into(),
            1..=59 => format!("{diff} 分钟前"),
            60..=1439 => format!("{} 小时前", diff / 60),
            _ => format!("{} 天前", diff / (24 * 60)),
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

/// 服务器角色：名称 + 颜色。成员按**角色名**归属，改名时由 app 层同步所有成员。
#[derive(Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub color: u32,
}

impl Role {
    pub fn new(name: impl Into<String>, color: u32) -> Self {
        Self {
            name: name.into(),
            color,
        }
    }
}

/// 角色权限的三态。`Unset` 表示继承服务器默认，`Allow` / `Deny` 为显式覆盖。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum PermissionState {
    #[default]
    Unset,
    Allow,
    Deny,
}

impl PermissionState {
    /// 按钮/单元格文案。
    pub fn label(self) -> &'static str {
        match self {
            PermissionState::Unset => "继承",
            PermissionState::Allow => "允许",
            PermissionState::Deny => "拒绝",
        }
    }

    /// 点击时在 继承 → 允许 → 拒绝 → 继承 之间循环。
    pub fn next(self) -> Self {
        match self {
            PermissionState::Unset => PermissionState::Allow,
            PermissionState::Allow => PermissionState::Deny,
            PermissionState::Deny => PermissionState::Unset,
        }
    }
}

/// 权限分组：决定权限矩阵里的分组标题与顺序。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PermissionGroup {
    General,
    Text,
    Voice,
    Advanced,
}

impl PermissionGroup {
    /// 全部分组，顺序即矩阵展示顺序。
    pub const ALL: [PermissionGroup; 4] = [
        PermissionGroup::General,
        PermissionGroup::Text,
        PermissionGroup::Voice,
        PermissionGroup::Advanced,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PermissionGroup::General => "通用权限",
            PermissionGroup::Text => "文字频道",
            PermissionGroup::Voice => "语音频道",
            PermissionGroup::Advanced => "高级权限",
        }
    }
}

/// 一条权限位的静态定义。`key` 是持久化用的稳定键，改动会破坏旧快照里的设置。
#[derive(Clone, Copy)]
pub struct PermissionDef {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub group: PermissionGroup,
}

/// 权限清单：视图遍历它渲染矩阵，不持有任何状态。
pub const PERMISSIONS: &[PermissionDef] = &[
    PermissionDef {
        key: "view_channels",
        label: "查看频道",
        hint: "在服务器栏看到并进入频道",
        group: PermissionGroup::General,
    },
    PermissionDef {
        key: "invite_people",
        label: "邀请好友",
        hint: "生成邀请链接并邀请他人加入",
        group: PermissionGroup::General,
    },
    PermissionDef {
        key: "change_nickname",
        label: "修改昵称",
        hint: "更改自己在本服务器的昵称",
        group: PermissionGroup::General,
    },
    PermissionDef {
        key: "send_messages",
        label: "发送消息",
        hint: "在文字频道发送消息与附件",
        group: PermissionGroup::Text,
    },
    PermissionDef {
        key: "embed_links",
        label: "嵌入链接",
        hint: "消息中的链接展示为预览卡片",
        group: PermissionGroup::Text,
    },
    PermissionDef {
        key: "attach_files",
        label: "上传附件",
        hint: "在消息中附加文件与图片",
        group: PermissionGroup::Text,
    },
    PermissionDef {
        key: "add_reactions",
        label: "添加回应",
        hint: "给消息添加表情回应",
        group: PermissionGroup::Text,
    },
    PermissionDef {
        key: "mention_everyone",
        label: "提及所有人",
        hint: "使用 @everyone 与 @here",
        group: PermissionGroup::Text,
    },
    PermissionDef {
        key: "connect",
        label: "连接语音",
        hint: "加入语音频道",
        group: PermissionGroup::Voice,
    },
    PermissionDef {
        key: "speak",
        label: "语音发言",
        hint: "在语音频道中说话",
        group: PermissionGroup::Voice,
    },
    PermissionDef {
        key: "screen_share",
        label: "屏幕分享",
        hint: "在语音或通话中分享屏幕",
        group: PermissionGroup::Voice,
    },
    PermissionDef {
        key: "manage_messages",
        label: "管理消息",
        hint: "删除他人消息与置顶消息",
        group: PermissionGroup::Advanced,
    },
    PermissionDef {
        key: "manage_channels",
        label: "管理频道",
        hint: "创建、编辑与删除频道",
        group: PermissionGroup::Advanced,
    },
];

/// 一个角色对一条权限的设置。只保存非默认项；查不到的键一律按 `Unset` 处理。
#[derive(Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub role: String,
    pub permission: String,
    pub state: PermissionState,
}

impl PermissionGrant {
    pub fn new(role: impl Into<String>, permission: impl Into<String>, state: PermissionState) -> Self {
        Self {
            role: role.into(),
            permission: permission.into(),
            state,
        }
    }
}

/// 自动审核规则命中关键词后的处理方式。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AutoModAction {
    /// 拦截：消息不会被发送，原文保留在输入框。
    Block,
    /// 警告：消息照常发送，同时给一条提示。
    Warn,
}

impl AutoModAction {
    pub fn label(self) -> &'static str {
        match self {
            AutoModAction::Block => "拦截消息",
            AutoModAction::Warn => "发出警告",
        }
    }
}

/// 一条自动审核规则：名字 + 说明 + 关键词清单 + 处理方式 + 是否启用。
/// 全部字段自有，serde 直接派生，无需 `*Owned` 镜像。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoModRule {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub action: AutoModAction,
    #[serde(default = "default_automod_enabled")]
    pub enabled: bool,
}

fn default_automod_enabled() -> bool {
    true
}

impl AutoModRule {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        keywords: &[&str],
        action: AutoModAction,
        enabled: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            keywords: keywords.iter().map(|key| (*key).to_string()).collect(),
            action,
            enabled,
        }
    }

    /// 文本是否命中任一关键词（大小写不敏感）；返回命中的原始关键词。
    pub fn matched_keyword(&self, text: &str) -> Option<&str> {
        let lower = text.to_lowercase();
        self.keywords
            .iter()
            .find(|keyword| !keyword.is_empty() && lower.contains(&keyword.to_lowercase()))
            .map(|keyword| keyword.as_str())
    }
}

/// 投票的单个选项：`voters` 存投票者身份键（"你"表示本机用户），
/// 用名单而非纯计数，既能显示票数也能判断"我是否已投 / 投了哪项"。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PollOption {
    pub label: String,
    #[serde(default)]
    pub voters: Vec<String>,
}

impl PollOption {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            voters: Vec::new(),
        }
    }
}

/// 一条投票：问题 + 选项 + 是否多选 + 是否已结束。
/// 全部字段自有，serde 直接派生，无需 `*Owned` 镜像。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Poll {
    pub question: String,
    pub options: Vec<PollOption>,
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub closed: bool,
}

impl Poll {
    pub fn new(question: impl Into<String>, labels: &[&str], multiple: bool) -> Self {
        Self {
            question: question.into(),
            options: labels.iter().map(|label| PollOption::new(*label)).collect(),
            multiple,
            closed: false,
        }
    }

    /// 总票数（含重复投票者在不同选项上的票）。
    pub fn total_votes(&self) -> usize {
        self.options.iter().map(|option| option.voters.len()).sum()
    }

    pub fn option_votes(&self, index: usize) -> usize {
        self.options
            .get(index)
            .map(|option| option.voters.len())
            .unwrap_or(0)
    }

    /// 某选项的得票百分比（总票为 0 时返回 0）。
    pub fn percent(&self, index: usize) -> usize {
        let total = self.total_votes();
        if total == 0 {
            0
        } else {
            self.option_votes(index) * 100 / total
        }
    }

    /// 某个投票者投了哪些选项的下标。
    pub fn voted_options(&self, voter: &str) -> Vec<usize> {
        self.options
            .iter()
            .enumerate()
            .filter(|(_, option)| option.voters.iter().any(|name| name == voter))
            .map(|(index, _)| index)
            .collect()
    }

    /// 当前领先选项的下标集合（并列可能多个；无人投票时为空）。
    pub fn leading_options(&self) -> Vec<usize> {
        let max = self.options.iter().map(|option| option.voters.len()).max().unwrap_or(0);
        if max == 0 {
            return Vec::new();
        }
        self.options
            .iter()
            .enumerate()
            .filter(|(_, option)| option.voters.len() == max)
            .map(|(index, _)| index)
            .collect()
    }
}

/// 服务器活动分类：决定卡片图标与配色。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum EventCategory {
    Game,
    Voice,
    Community,
    Other,
}

impl EventCategory {
    pub const ALL: [EventCategory; 4] = [
        EventCategory::Game,
        EventCategory::Voice,
        EventCategory::Community,
        EventCategory::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EventCategory::Game => "游戏",
            EventCategory::Voice => "语音",
            EventCategory::Community => "社区",
            EventCategory::Other => "其他",
        }
    }

    pub fn color(self) -> u32 {
        match self {
            EventCategory::Game => 0x5865f2,
            EventCategory::Voice => 0x57f287,
            EventCategory::Community => 0xeb459e,
            EventCategory::Other => 0xfaa61c,
        }
    }
}

/// 活动状态：由开始时间相对当前分钟的位置推算（见 `ServerEvent::status`）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServerEventStatus {
    Upcoming,
    Live,
    Ended,
}

impl ServerEventStatus {
    pub fn label(self) -> &'static str {
        match self {
            ServerEventStatus::Upcoming => "即将开始",
            ServerEventStatus::Live => "进行中",
            ServerEventStatus::Ended => "已结束",
        }
    }
}

/// 一条服务器活动（Discord Scheduled Event 的本地演示）。
/// 时间用「距现在多少分钟」的相对值，和审核日志一样不依赖真实时钟；
/// 全部字段自有，serde 直接派生。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerEvent {
    pub id: String,
    /// 归属服务器名（与 `Server.name` 对应）。
    pub server: String,
    pub title: String,
    pub description: String,
    /// 活动落地语音/文字频道，仅作展示。
    pub channel: String,
    /// 发起人身份键。
    pub host: String,
    /// 报名者身份键名单（"你"表示本机用户）。
    pub attendees: Vec<String>,
    /// 距开始的分钟数：正数=多久后开始，负数=已开始多久。
    pub starts_in_minutes: i32,
    /// 持续分钟数。
    pub duration_minutes: u32,
    pub category: EventCategory,
}

impl ServerEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        server: impl Into<String>,
        title: impl Into<String>,
        channel: impl Into<String>,
        host: impl Into<String>,
        starts_in_minutes: i32,
        duration_minutes: u32,
        category: EventCategory,
    ) -> Self {
        Self {
            id: id.into(),
            server: server.into(),
            title: title.into(),
            description: String::new(),
            channel: channel.into(),
            host: host.into(),
            attendees: Vec::new(),
            starts_in_minutes,
            duration_minutes,
            category,
        }
    }

    /// 相对状态：未到开始时间=即将开始；开始后仍在时长内=进行中；否则已结束。
    pub fn status(&self) -> ServerEventStatus {
        if self.starts_in_minutes > 0 {
            ServerEventStatus::Upcoming
        } else if self.starts_in_minutes + self.duration_minutes as i32 > 0 {
            ServerEventStatus::Live
        } else {
            ServerEventStatus::Ended
        }
    }

    /// 人类可读的开始时间（相对现在）。
    pub fn time_label(&self) -> String {
        let minutes = self.starts_in_minutes;
        match self.status() {
            ServerEventStatus::Upcoming => {
                if minutes < 60 {
                    format!("{minutes} 分钟后开始")
                } else {
                    format!("{} 小时后开始", minutes / 60)
                }
            }
            ServerEventStatus::Live => format!("已开始 {} 分钟", -minutes),
            ServerEventStatus::Ended => {
                let ended = (-minutes) - self.duration_minutes as i32;
                if ended < 60 {
                    format!("{ended} 分钟前结束")
                } else {
                    format!("{} 小时前结束", ended / 60)
                }
            }
        }
    }

    pub fn attendee_count(&self) -> usize {
        self.attendees.len()
    }

    pub fn is_attending(&self, name: &str) -> bool {
        self.attendees.iter().any(|person| person == name)
    }
}

/// 成员条目。`role` 存角色名，None 表示无角色（归入在线 / 离线兜底组）；
/// `nick` 是服务器昵称，只影响展示，`name` 始终是身份键。
#[derive(Clone, Copy, Serialize)]
#[serde(from = "crate::data::persist::MemberOwned")]
pub struct Member {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub name: &'static str,
    pub color: u32,
    pub online: bool,
    #[serde(with = "crate::data::persist::opt_leaked_str")]
    pub role: Option<&'static str>,
    #[serde(with = "crate::data::persist::opt_leaked_str")]
    pub nick: Option<&'static str>,
    /// 加入于多少分钟前（演示值是静态的）。
    pub joined_minutes: u32,
}

impl Member {
    /// 展示名：设置过昵称就用昵称。
    pub fn display(&self) -> &'static str {
        self.nick.unwrap_or(self.name)
    }

    /// 加入时间文案：刚刚 / 分钟 / 小时 / 天。
    pub fn joined_label(&self) -> String {
        let minutes = self.joined_minutes;
        if minutes == 0 {
            "刚刚加入".into()
        } else if minutes < 60 {
            format!("{minutes} 分钟前加入")
        } else if minutes < 24 * 60 {
            format!("{} 小时前加入", minutes / 60)
        } else {
            format!("{} 天前加入", minutes / (24 * 60))
        }
    }
}

/// 被封禁的用户。Demo 里只在本机列表留存，不连任何后端。
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BannedUser {
    pub name: String,
    /// 封禁原因，空串按「未填写原因」展示。
    pub reason: String,
    /// 封禁于多少分钟前。
    pub minutes: u32,
    /// 操作人显示名。
    pub by: String,
}

impl BannedUser {
    pub fn reason_label(&self) -> &str {
        if self.reason.trim().is_empty() {
            "未填写原因"
        } else {
            &self.reason
        }
    }

    pub fn time_label(&self) -> String {
        let minutes = self.minutes;
        if minutes == 0 {
            "刚刚封禁".into()
        } else if minutes < 60 {
            format!("{minutes} 分钟前封禁")
        } else if minutes < 24 * 60 {
            format!("{} 小时前封禁", minutes / 60)
        } else {
            format!("{} 天前封禁", minutes / (24 * 60))
        }
    }
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
#[derive(Clone, Serialize)]
#[serde(from = "crate::data::persist::FriendOwned")]
pub struct Friend {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub name: &'static str,
    pub color: u32,
    pub online: bool,
    #[serde(with = "crate::data::persist::leaked_str")]
    pub activity: &'static str,
    /// 所属好友分组名；空字符串表示「未分组」。分组顺序记录在 app 层。
    pub group: String,
}

/// 好友请求：incoming 为收到的请求，否则为发出的请求。
#[derive(Clone, Serialize, Deserialize)]
pub struct FriendRequest {
    pub name: String,
    pub color: u32,
    pub incoming: bool,
}

/// 消息请求的处理状态。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MessageRequestState {
    /// 待处理（新请求，计入侧栏角标）。
    Pending,
    /// 已接受（转为正常私信会话）。
    Accepted,
    /// 已忽略（保留记录但不再提醒）。
    Ignored,
}

impl MessageRequestState {
    pub fn label(self) -> &'static str {
        match self {
            MessageRequestState::Pending => "待处理",
            MessageRequestState::Accepted => "已接受",
            MessageRequestState::Ignored => "已忽略",
        }
    }
}

/// 陌生人发来的私信请求。接受后转为正常私信会话，忽略后仅保留记录。
#[derive(Clone, Serialize, Deserialize)]
pub struct MessageRequest {
    pub sender: String,
    pub color: u32,
    /// 首条消息预览。
    pub preview: String,
    /// 收到距今的分钟数（用于相对时间标签）。
    pub minutes_ago: u32,
    pub state: MessageRequestState,
    /// 是否还在请求箱里展示（删除后为 false）。
    pub visible: bool,
}

impl MessageRequest {
    pub fn new(sender: impl Into<String>, color: u32, preview: impl Into<String>, minutes_ago: u32) -> Self {
        Self {
            sender: sender.into(),
            color,
            preview: preview.into(),
            minutes_ago,
            state: MessageRequestState::Pending,
            visible: true,
        }
    }

    /// 相对时间标签：分钟 / 小时 / 天。
    pub fn time_label(&self) -> String {
        match self.minutes_ago {
            0..=1 => "刚刚".into(),
            2..=59 => format!("{} 分钟前", self.minutes_ago),
            60..=1439 => format!("{} 小时前", self.minutes_ago / 60),
            _ => format!("{} 天前", self.minutes_ago / 1440),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.state == MessageRequestState::Pending && self.visible
    }
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
#[derive(Clone, Copy, Serialize)]
#[serde(from = "crate::data::persist::DmConversationOwned")]
pub struct DmConversation {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub friend: &'static str,
    pub color: u32,
    pub online: bool,
    pub unread: usize,
    #[serde(with = "crate::data::persist::leaked_str")]
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

/// 帮助中心的一篇文档：分类 + 标题 + 摘要 + 正文段落 + 要点提示。
#[derive(Clone, Copy)]
pub struct HelpArticle {
    pub id: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub body: &'static [&'static str],
    pub tips: &'static [&'static str],
}

impl HelpArticle {
    pub const fn new(
        id: &'static str,
        category: &'static str,
        title: &'static str,
        summary: &'static str,
        body: &'static [&'static str],
        tips: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            category,
            title,
            summary,
            body,
            tips,
        }
    }

    /// 搜索匹配：标题、摘要、正文与要点都参与（query 已转小写）。
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        self.title.to_lowercase().contains(query)
            || self.summary.to_lowercase().contains(query)
            || self.category.to_lowercase().contains(query)
            || self.body.iter().any(|p| p.to_lowercase().contains(query))
            || self.tips.iter().any(|t| t.to_lowercase().contains(query))
    }
}

/// 当前登录用户（本地演示资料）：账户页可编辑，底部用户栏与资料预览共用。
#[derive(Clone, Serialize, Deserialize)]
pub struct MyProfile {
    pub display_name: String,
    pub username: String,
    pub email: String,
    pub bio: String,
    pub color: u32,
}

impl Default for MyProfile {
    fn default() -> Self {
        Self {
            display_name: "Riven".into(),
            username: "riven".into(),
            email: String::new(),
            bio: String::new(),
            color: 0x5865f2,
        }
    }
}

impl MyProfile {
    pub fn new(
        display_name: impl Into<String>,
        username: impl Into<String>,
        email: impl Into<String>,
        bio: impl Into<String>,
        color: u32,
    ) -> Self {
        Self {
            display_name: display_name.into(),
            username: username.into(),
            email: email.into(),
            bio: bio.into(),
            color,
        }
    }

    /// 头像上的首字：取显示名第一个字符，空名退化为 'R'。
    pub fn initial(&self) -> char {
        self.display_name
            .chars()
            .next()
            .and_then(|c| c.to_uppercase().next())
            .unwrap_or('R')
    }
}

/// 论坛帖子的排序方式。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ForumSort {
    /// 最近活动优先（按最后一条回帖时间）。
    Recent,
    /// 最多回应优先。
    Popular,
    /// 只看还没有回应的帖子。
    Unanswered,
}

impl ForumSort {
    pub fn label(self) -> &'static str {
        match self {
            ForumSort::Recent => "最新",
            ForumSort::Popular => "最热",
            ForumSort::Unanswered => "未回应",
        }
    }
}

/// 论坛帖子的一条回帖（纯 Demo，作者用自有字符串，不走 leak 镜像）。
#[derive(Clone, Serialize, Deserialize)]
pub struct ForumReply {
    pub author: String,
    pub color: u32,
    pub body: String,
    /// 回帖距今分钟数，用于相对时间与「最后活动」排序。
    pub minutes_ago: u32,
}

impl ForumReply {
    pub fn new(author: impl Into<String>, color: u32, body: impl Into<String>, minutes_ago: u32) -> Self {
        Self {
            author: author.into(),
            color,
            body: body.into(),
            minutes_ago,
        }
    }
}

/// 论坛频道里的一个帖子。归属用 `server` + `channel` 两个字符串标记，
/// 字段全部为自有字符串，快照可直接派生 serde、不需要 Owned 镜像。
#[derive(Clone, Serialize, Deserialize)]
pub struct ForumPost {
    pub id: String,
    pub server: String,
    pub channel: String,
    pub title: String,
    pub author: String,
    pub color: u32,
    pub body: String,
    /// 自由标签（Demo 里用逗号分隔输入）。
    pub tags: Vec<String>,
    pub minutes_ago: u32,
    pub pinned: bool,
    pub locked: bool,
    pub replies: Vec<ForumReply>,
}

impl ForumPost {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        server: impl Into<String>,
        channel: impl Into<String>,
        title: impl Into<String>,
        author: impl Into<String>,
        color: u32,
        body: impl Into<String>,
        minutes_ago: u32,
    ) -> Self {
        Self {
            id: id.into(),
            server: server.into(),
            channel: channel.into(),
            title: title.into(),
            author: author.into(),
            color,
            body: body.into(),
            tags: Vec::new(),
            minutes_ago,
            pinned: false,
            locked: false,
            replies: Vec::new(),
        }
    }

    pub fn tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|tag| tag.to_string()).collect();
        self
    }

    pub fn replies(mut self, replies: Vec<ForumReply>) -> Self {
        self.replies = replies;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// 相对时间标签：分钟 / 小时 / 天。
    pub fn time_label(&self) -> String {
        relative_label(self.minutes_ago)
    }

    pub fn reply_count(&self) -> usize {
        self.replies.len()
    }

    /// 最后活动时间：有回帖则取最近的一条，否则用发帖时间。
    pub fn last_activity(&self) -> u32 {
        self.replies
            .iter()
            .map(|reply| reply.minutes_ago)
            .min()
            .unwrap_or(self.minutes_ago)
            .min(self.minutes_ago)
    }

    /// 搜索匹配：标题、正文、作者与标签都参与（query 已转小写）。
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        self.title.to_lowercase().contains(query)
            || self.body.to_lowercase().contains(query)
            || self.author.to_lowercase().contains(query)
            || self.tags.iter().any(|tag| tag.to_lowercase().contains(query))
    }
}

/// 分钟数转相对时间标签（发帖与回帖共用）。
fn relative_label(minutes: u32) -> String {
    match minutes {
        0..=1 => "刚刚".into(),
        2..=59 => format!("{} 分钟前", minutes),
        60..=1439 => format!("{} 小时前", minutes / 60),
        _ => format!("{} 天前", minutes / 1440),
    }
}

// ---------------------------------------------------------------------------
// 反序列化：静态字符串字段让 serde 派生会给 `Deserialize` 加上 `'de: 'static`，
// 无法从普通字符串解析。因此这些类型手写 `Deserialize`，先解析 persist 层的
// String 镜像，再转成模型（字符串一次性 leak 成 `&'static str`）。
// ---------------------------------------------------------------------------
macro_rules! deserialize_via_owned {
    ($($model:ty => $owned:ty),* $(,)?) => {
        $(
            impl<'de> Deserialize<'de> for $model {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    <$owned as Deserialize>::deserialize(deserializer).map(Into::into)
                }
            }
        )*
    };
}

deserialize_via_owned!(
    Reaction => crate::data::persist::ReactionOwned,
    Attachment => crate::data::persist::AttachmentOwned,
    Sticker => crate::data::persist::StickerOwned,
    Gif => crate::data::persist::GifOwned,
    EditRevision => crate::data::persist::EditRevisionOwned,
    ReplyRef => crate::data::persist::ReplyRefOwned,
    ReadReceipt => crate::data::persist::ReadReceiptOwned,
    ThreadReply => crate::data::persist::ThreadReplyOwned,
    Message => crate::data::persist::MessageOwned,
    Member => crate::data::persist::MemberOwned,
    Friend => crate::data::persist::FriendOwned,
    DmConversation => crate::data::persist::DmConversationOwned,
    Invite => crate::data::persist::InviteOwned,
);

#[cfg(test)]
mod tests {
    use super::*;

    fn invite(max_uses: u32, expiry: InviteExpiry, created_minutes: u32, uses: u32) -> Invite {
        Invite {
            code: "demo-1".into(),
            server: "开发者社区".into(),
            channel: "闲聊".into(),
            inviter: "Riven",
            uses,
            max_uses,
            expiry,
            created_minutes,
        }
    }

    /// 三种失效状态的判定顺序：过期优先于用完，仍有效时返回 None。
    #[test]
    fn invite_status_follows_expiry_then_uses() {
        assert_eq!(invite(0, InviteExpiry::Never, 9999, 3).status(), None);
        assert_eq!(invite(5, InviteExpiry::OneDay, 10, 5).status(), Some("已用完"));
        assert_eq!(invite(0, InviteExpiry::ThirtyMinutes, 45, 0).status(), Some("已过期"));

        // 次数用完且同时过期时，过期优先。
        let both = invite(1, InviteExpiry::OneHour, 90, 1);
        assert!(both.expired() && both.used_up());
        assert_eq!(both.status(), Some("已过期"));
    }

    /// 文案随剩余时间切换分钟 / 小时 / 天，不限次数用「不限」。
    #[test]
    fn invite_labels_read_naturally() {
        assert_eq!(invite(0, InviteExpiry::Never, 0, 12).uses_label(), "12 / 不限");
        assert_eq!(invite(10, InviteExpiry::Never, 0, 2).uses_label(), "2 / 10");
        assert_eq!(
            invite(0, InviteExpiry::ThirtyMinutes, 0, 0).expiry_label(),
            "30 分钟后过期"
        );
        assert_eq!(
            invite(0, InviteExpiry::SixHours, 60, 0).expiry_label(),
            "5 小时后过期"
        );
        assert_eq!(
            invite(0, InviteExpiry::SevenDays, 2 * 24 * 60, 0).expiry_label(),
            "5 天后过期"
        );
        assert_eq!(
            invite(0, InviteExpiry::Never, 0, 0).expiry_label(),
            "永不过期"
        );
        assert_eq!(invite(0, InviteExpiry::OneHour, 61, 0).expiry_label(), "已过期");
        assert_eq!(invite(0, InviteExpiry::Never, 0, 0).link(), "discord.gg/demo-1");
    }

    /// 成员加入时间与封禁原因兜底文案。
    #[test]
    fn member_and_ban_labels_read_naturally() {
        let member = |minutes| Member {
            name: "Nova",
            color: 0x00a8fc,
            online: true,
            role: None,
            nick: None,
            joined_minutes: minutes,
        };
        assert_eq!(member(0).joined_label(), "刚刚加入");
        assert_eq!(member(30).joined_label(), "30 分钟前加入");
        assert_eq!(member(5 * 60).joined_label(), "5 小时前加入");
        assert_eq!(member(3 * 24 * 60).joined_label(), "3 天前加入");

        // 有昵称优先用昵称展示，原名仍是身份键。
        let aliased = Member {
            nick: Some("Nova ✨"),
            ..member(10)
        };
        assert_eq!(aliased.display(), "Nova ✨");
        assert_eq!(aliased.name, "Nova");

        let ban = |reason: &str| BannedUser {
            name: "raider".into(),
            reason: reason.into(),
            minutes: 0,
            by: "Riven".into(),
        };
        assert_eq!(ban("  ").reason_label(), "未填写原因");
        assert_eq!(ban("刷屏").reason_label(), "刷屏");
        assert_eq!(ban("").time_label(), "刚刚封禁");
    }

    /// 审核日志的相对时间：最新一条「刚刚」，其余按序号差换成分钟 / 小时 / 天。
    #[test]
    fn audit_time_label_counts_back_from_newest() {
        let entry = |seq| AuditEntry::new(seq, AuditKind::Channel, "Riven", "创建了频道", "#闲聊");
        let newest = 5_000;
        assert_eq!(entry(newest).time_label(newest), "刚刚");
        assert_eq!(entry(newest - 1).time_label(newest), "1 分钟前");
        assert_eq!(entry(newest - 59).time_label(newest), "59 分钟前");
        assert_eq!(entry(newest - 120).time_label(newest), "2 小时前");
        assert_eq!(entry(newest - 3 * 24 * 60).time_label(newest), "3 天前");
        // 序号比最新还大（理论上不会发生）也不应下溢。
        assert_eq!(entry(newest + 3).time_label(newest), "刚刚");
    }

    /// 权限三态循环一圈回到原点，文案与默认状态稳定。
    #[test]
    fn permission_state_cycles_and_labels() {
        assert_eq!(PermissionState::default(), PermissionState::Unset);
        let mut state = PermissionState::Unset;
        assert_eq!(state.label(), "继承");
        state = state.next();
        assert_eq!(state, PermissionState::Allow);
        assert_eq!(state.label(), "允许");
        state = state.next();
        assert_eq!(state, PermissionState::Deny);
        assert_eq!(state.label(), "拒绝");
        state = state.next();
        assert_eq!(state, PermissionState::Unset);

        // 权限清单的 key 必须唯一且与分组对应，避免矩阵与快照错位。
        let mut keys = std::collections::HashSet::new();
        for def in PERMISSIONS {
            assert!(keys.insert(def.key), "重复的权限 key: {}", def.key);
            assert!(!def.label.is_empty());
            assert!(PermissionGroup::ALL.contains(&def.group));
        }
    }

    /// 自动审核关键词大小写不敏感，空关键词不参与匹配。
    #[test]
    fn automod_keyword_matching_is_case_insensitive() {
        let rule = AutoModRule::new(
            "广告",
            "test",
            &["SPAM", "", "https://"],
            AutoModAction::Block,
            true,
        );
        assert_eq!(rule.matched_keyword("please spam me"), Some("SPAM"));
        assert_eq!(rule.matched_keyword("去 HTTPS://example.com"), Some("https://"));
        assert_eq!(rule.matched_keyword("clean message"), None);
        assert_eq!(AutoModAction::Block.label(), "拦截消息");
        assert_eq!(AutoModAction::Warn.label(), "发出警告");
    }

    #[test]
    fn poll_tally_percent_and_voter_lookup() {
        let mut poll = Poll::new("下周周会", &["周一", "周三", "周五"], false);
        assert_eq!(poll.total_votes(), 0);
        assert_eq!(poll.percent(0), 0);
        assert!(poll.leading_options().is_empty());

        poll.options[0].voters = vec!["小满".into(), "阿哲".into()];
        poll.options[1].voters = vec!["Lin".into()];
        assert_eq!(poll.total_votes(), 3);
        assert_eq!(poll.option_votes(0), 2);
        assert_eq!(poll.percent(0), 66);
        assert_eq!(poll.percent(1), 33);
        assert_eq!(poll.leading_options(), vec![0]);
        assert!(poll.voted_options("你").is_empty());
        assert_eq!(poll.voted_options("阿哲"), vec![0]);
    }

    #[test]
    fn poll_ties_return_multiple_leaders() {
        let mut poll = Poll::new("午餐", &["A", "B", "C"], true);
        assert!(poll.multiple);
        poll.options[0].voters = vec!["你".into()];
        poll.options[2].voters = vec!["Lin".into()];
        assert_eq!(poll.leading_options(), vec![0, 2]);
        assert_eq!(poll.voted_options("你"), vec![0]);
    }

    #[test]
    fn server_event_status_follows_relative_time() {
        let mut upcoming = ServerEvent::new(
            "a", "S", "t", "c", "h", 45, 90, EventCategory::Community,
        );
        assert_eq!(upcoming.status(), ServerEventStatus::Upcoming);
        assert_eq!(upcoming.time_label(), "45 分钟后开始");

        upcoming.starts_in_minutes = 120;
        assert_eq!(upcoming.time_label(), "2 小时后开始");

        // 已开始但仍在时长内 = 进行中。
        upcoming.starts_in_minutes = -20;
        assert_eq!(upcoming.status(), ServerEventStatus::Live);
        assert_eq!(upcoming.time_label(), "已开始 20 分钟");

        // 超过时长 = 已结束。
        upcoming.starts_in_minutes = -180;
        assert_eq!(upcoming.status(), ServerEventStatus::Ended);
        assert_eq!(upcoming.time_label(), "1 小时前结束");

        // 恰好在开始点（-0）且时长未过仍算进行中，不出现 -0 时间戳。
        upcoming.starts_in_minutes = 0;
        assert_eq!(upcoming.status(), ServerEventStatus::Live);
        assert_eq!(upcoming.time_label(), "已开始 0 分钟");
    }

    #[test]
    fn server_event_attendee_lookup() {
        let mut event = ServerEvent::new("b", "S", "t", "c", "h", 30, 60, EventCategory::Game);
        assert_eq!(event.attendee_count(), 0);
        assert!(!event.is_attending("你"));
        event.attendees.push("你".into());
        event.attendees.push("小满".into());
        assert_eq!(event.attendee_count(), 2);
        assert!(event.is_attending("你"));
        assert_eq!(EventCategory::Game.label(), "游戏");
    }

    #[test]
    fn message_request_state_and_time_label() {
        let req = MessageRequest::new("Kai", 0xeb459e, "你好", 18);
        assert_eq!(req.state, MessageRequestState::Pending);
        assert!(req.visible);
        assert!(req.is_pending());
        assert_eq!(req.time_label(), "18 分钟前");
        assert_eq!(req.state.label(), "待处理");

        let accepted = MessageRequest {
            state: MessageRequestState::Accepted,
            ..MessageRequest::new("Iris", 0x57f287, "hi", 60)
        };
        assert!(!accepted.is_pending());
        assert_eq!(accepted.time_label(), "1 小时前");

        let deleted = MessageRequest {
            visible: false,
            ..MessageRequest::new("x", 0, "x", 5000)
        };
        assert!(!deleted.is_pending());
        assert_eq!(deleted.time_label(), "3 天前");
    }

    #[test]
    fn forum_post_activity_search_and_time() {
        let mut post = ForumPost::new("p1", "S", "求助", "如何配置", "Riven", 0, "正文", 120)
            .tags(&["Rust", "构建"]);
        assert_eq!(post.time_label(), "2 小时前");
        assert_eq!(post.reply_count(), 0);
        assert_eq!(post.last_activity(), 120);
        assert!(post.matches("rust"));
        assert!(post.matches("构建"));
        assert!(post.matches("riven"));
        assert!(!post.matches("不存在"));

        post.replies.push(ForumReply::new("Kai", 0, "看文档", 30));
        assert_eq!(post.reply_count(), 1);
        // 最后活动取最新的回帖（距今分钟数最小）。
        assert_eq!(post.last_activity(), 30);
        assert_eq!(ForumSort::Recent.label(), "最新");
        assert_eq!(ForumSort::Popular.label(), "最热");
        assert_eq!(ForumSort::Unanswered.label(), "未回应");
    }
}
