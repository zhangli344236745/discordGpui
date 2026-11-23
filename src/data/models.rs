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
    Invite,
    Emoji,
}

impl AuditKind {
    /// 全部类别，顺序即筛选条的展示顺序。
    pub const ALL: [AuditKind; 5] = [
        AuditKind::Server,
        AuditKind::Channel,
        AuditKind::Role,
        AuditKind::Invite,
        AuditKind::Emoji,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AuditKind::Server => "服务器",
            AuditKind::Channel => "频道",
            AuditKind::Role => "角色",
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

/// 成员栏条目。`role` 存角色名，None 表示无角色（归入在线 / 离线兜底组）。
#[derive(Clone, Copy, Serialize)]
#[serde(from = "crate::data::persist::MemberOwned")]
pub struct Member {
    #[serde(with = "crate::data::persist::leaked_str")]
    pub name: &'static str,
    pub color: u32,
    pub online: bool,
    #[serde(with = "crate::data::persist::opt_leaked_str")]
    pub role: Option<&'static str>,
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
}
