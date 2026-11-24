//! 本地持久化：把界面背后的全部「内容状态」序列化成一个 JSON 快照。
//!
//! 只持久化数据，不持久化浮层显隐、输入框焦点这类一次性 UI 状态。加载成功后
//! 覆盖内存里的种子数据；任何一次快照内容变化都会在下一帧自动写回磁盘。
//!
//! 模型里大量字段是 `&'static str`（作者名、时间、表情等），反序列化时统一用
//! `Box::leak` 转成静态字符串——只在启动加载时发生一次，量级可忽略。

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use gpui_kit::{rgb, Rgba};
use serde::{Deserialize, Serialize, Serializer};

use super::models::{
    AuditEntry, AutoModRule, BannedUser, ChatThread, DmConversation, Friend, FriendRequest, Invite,
    InviteExpiry, Member, Message, MyProfile, PermissionGrant, Role, Server, ServerEvent, UserStatus,
};

/// 快照版本号；字段语义变化时递增，加载端按需丢弃旧数据。
pub const VERSION: u32 = 1;

/// 一个完整的本地快照。所有字段都带 `#[serde(default)]`，缺字段时退回空值，
/// 便于以后加字段而不破坏旧文件。
#[derive(Serialize, Deserialize, Default)]
pub struct Snapshot {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub servers: Vec<Server>,
    #[serde(default)]
    pub messages: HashMap<String, Vec<Message>>,
    #[serde(default)]
    pub dm_conversations: Vec<DmConversation>,
    #[serde(default)]
    pub dm_messages: HashMap<String, Vec<Message>>,
    #[serde(default)]
    pub threads: HashMap<String, Vec<ChatThread>>,
    #[serde(default)]
    pub friends: Vec<Friend>,
    /// 成员与角色。用 Option 区分「旧快照没有」与「用户清空了」：前者保留种子。
    #[serde(default)]
    pub members: Option<Vec<Member>>,
    #[serde(default)]
    pub roles: Option<Vec<Role>>,
    /// 服务器邀请列表（按 `Invite.server` 归属服务器）。
    /// 用 `Option` 区分「旧快照没有这个字段」与「用户把邀请全删了」：前者保留种子数据。
    #[serde(default)]
    pub invites: Option<Vec<Invite>>,
    /// 审核日志（按 `seq` 升序）。旧快照缺字段时保留种子日志。
    #[serde(default)]
    pub audit: Option<Vec<AuditEntry>>,
    /// 封禁列表。旧快照缺字段时保留种子数据。
    #[serde(default)]
    pub bans: Option<Vec<BannedUser>>,
    /// 角色权限位（只存非默认项）。旧快照缺字段时保留种子权限。
    #[serde(default)]
    pub permissions: Option<Vec<PermissionGrant>>,
    /// 自动审核规则。旧快照缺字段时保留种子规则。
    #[serde(default)]
    pub automod_rules: Option<Vec<AutoModRule>>,
    /// 服务器活动。旧快照缺字段时保留种子活动。
    #[serde(default)]
    pub events: Option<Vec<ServerEvent>>,
    /// 好友分组的顺序（只存分组名，成员关系在 `Friend.group` 上）。
    #[serde(default)]
    pub friend_groups: Vec<String>,
    #[serde(default)]
    pub friend_requests: Vec<FriendRequest>,
    #[serde(default)]
    pub blocked_users: Vec<String>,
    #[serde(default)]
    pub me: MyProfile,
    #[serde(default)]
    pub active_server: usize,
    #[serde(default)]
    pub active_channel: String,
    #[serde(default)]
    pub active_dm: Option<String>,
    #[serde(default)]
    pub notifications_on: bool,
    #[serde(default)]
    pub notification_sound: bool,
    #[serde(default)]
    pub my_status: UserStatus,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub show_members: bool,
    #[serde(default)]
    pub collapsed: Vec<String>,
    #[serde(default)]
    pub collapsed_roles: Vec<String>,
    #[serde(default)]
    pub read_notifications: Vec<String>,
}

/// `&'static str` 的序列化适配器：直接写成普通字符串。
/// 反序列化不走这里——静态字符串字段由下方 `*Owned` 镜像统一处理。
pub mod leaked_str {
    use super::*;

    pub fn serialize<S: Serializer>(value: &&'static str, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value)
    }
}

/// `Option<&'static str>` 的序列化适配器：Some 写字符串、None 写 null。
pub mod opt_leaked_str {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &Option<&'static str>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(text) => serializer.serialize_some(text),
            None => serializer.serialize_none(),
        }
    }
}

/// `Rgba` 的序列化适配器：存成 0xRRGGBB；反序列化由 `*Owned` 镜像用 `rgb()` 还原。
pub mod rgba_u32 {
    use super::*;

    pub fn serialize<S: Serializer>(value: &Rgba, serializer: S) -> Result<S::Ok, S::Error> {
        let r = (value.r.clamp(0., 1.) * 255.).round() as u32;
        let g = (value.g.clamp(0., 1.) * 255.).round() as u32;
        let b = (value.b.clamp(0., 1.) * 255.).round() as u32;
        serializer.serialize_u32((r << 16) | (g << 8) | b)
    }
}

/// 快照文件位置：macOS 用 Application Support，其他平台退回家目录下的隐藏目录。
pub fn state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    if cfg!(target_os = "macos") {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("discord-gpui")
            .join("state.json")
    } else {
        PathBuf::from(home).join(".discord-gpui").join("state.json")
    }
}

/// 写入快照（自动建目录），返回实际写入路径。
pub fn save(snapshot: &Snapshot) -> io::Result<PathBuf> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(snapshot).map_err(io::Error::other)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// 读取快照；文件不存在或解析失败时返回 None（调用方退回种子数据）。
pub fn load() -> Option<Snapshot> {
    let json = fs::read_to_string(state_path()).ok()?;
    let snapshot: Snapshot = serde_json::from_str(&json).ok()?;
    (snapshot.version == VERSION && !snapshot.servers.is_empty()).then_some(snapshot)
}

/// 删除快照文件（恢复示例数据时用）。
pub fn clear() -> io::Result<()> {
    match fs::remove_file(state_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

// ---------------------------------------------------------------------------
// 反序列化用的「拥有所有权的镜像类型」
//
// 模型里作者名 / 时间 / 表情 / 贴纸文案都是 `&'static str`，serde 的派生会在字段
// 类型带 `'static` 时给 `Deserialize<'de>` 加上 `'de: 'static` 约束，导致无法从
// 普通字符串解析。这里给这些类型各写一份 String 版镜像，用 `#[serde(from = ...)]`
// 让模型自己实现 Deserialize：先解析镜像，再一次性把字符串 leak 成静态引用。
// ---------------------------------------------------------------------------

/// 把运行时读到的字符串转成静态引用（只在启动加载时调用一次）。
pub fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

#[derive(Deserialize)]
pub struct ReactionOwned {
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub mine: bool,
}

impl From<ReactionOwned> for super::models::Reaction {
    fn from(value: ReactionOwned) -> Self {
        Self {
            emoji: leak(value.emoji),
            count: value.count,
            mine: value.mine,
        }
    }
}

#[derive(Deserialize)]
pub struct AttachmentOwned {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub size: String,
    #[serde(default = "default_attachment_kind")]
    pub kind: super::models::AttachmentKind,
}

fn default_attachment_kind() -> super::models::AttachmentKind {
    super::models::AttachmentKind::File
}

impl From<AttachmentOwned> for super::models::Attachment {
    fn from(value: AttachmentOwned) -> Self {
        Self {
            name: value.name,
            size: leak(value.size),
            kind: value.kind,
        }
    }
}

#[derive(Deserialize)]
pub struct StickerOwned {
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub pack: String,
}

impl From<StickerOwned> for super::models::Sticker {
    fn from(value: StickerOwned) -> Self {
        Self {
            emoji: leak(value.emoji),
            label: leak(value.label),
            pack: leak(value.pack),
        }
    }
}

#[derive(Deserialize)]
pub struct GifOwned {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: String,
}

impl From<GifOwned> for super::models::Gif {
    fn from(value: GifOwned) -> Self {
        Self {
            title: leak(value.title),
            emoji: leak(value.emoji),
            color: value.color,
            category: leak(value.category),
            tags: leak(value.tags),
        }
    }
}

#[derive(Deserialize)]
pub struct EditRevisionOwned {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub time: String,
}

impl From<EditRevisionOwned> for super::models::EditRevision {
    fn from(value: EditRevisionOwned) -> Self {
        Self {
            text: value.text,
            time: leak(value.time),
        }
    }
}

#[derive(Deserialize)]
pub struct ReplyRefOwned {
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub excerpt: String,
}

impl From<ReplyRefOwned> for super::models::ReplyRef {
    fn from(value: ReplyRefOwned) -> Self {
        Self {
            author: leak(value.author),
            excerpt: value.excerpt,
        }
    }
}

#[derive(Deserialize)]
pub struct ReadReceiptOwned {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub color: u32,
}

impl From<ReadReceiptOwned> for super::models::ReadReceipt {
    fn from(value: ReadReceiptOwned) -> Self {
        Self {
            name: leak(value.name),
            time: leak(value.time),
            color: value.color,
        }
    }
}

#[derive(Deserialize)]
pub struct ThreadReplyOwned {
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub color: u32,
}

impl From<ThreadReplyOwned> for super::models::ThreadReply {
    fn from(value: ThreadReplyOwned) -> Self {
        Self {
            author: leak(value.author),
            time: leak(value.time),
            text: value.text,
            color: rgb(value.color),
        }
    }
}

#[derive(Deserialize)]
pub struct MessageOwned {
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub grouped: bool,
    #[serde(default)]
    pub reactions: Vec<ReactionOwned>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub attachment: Option<AttachmentOwned>,
    #[serde(default)]
    pub sticker: Option<StickerOwned>,
    #[serde(default)]
    pub gif: Option<GifOwned>,
    #[serde(default)]
    pub edit_history: Vec<EditRevisionOwned>,
    #[serde(default)]
    pub forwarded: Option<super::models::ForwardOrigin>,
    #[serde(default)]
    pub reply_to: Option<ReplyRefOwned>,
    #[serde(default = "default_delivery")]
    pub delivery: super::models::DeliveryStatus,
    #[serde(default)]
    pub readers: Vec<ReadReceiptOwned>,
    #[serde(default)]
    pub poll: Option<super::models::Poll>,
}

fn default_delivery() -> super::models::DeliveryStatus {
    super::models::DeliveryStatus::Sent
}

impl From<MessageOwned> for super::models::Message {
    fn from(value: MessageOwned) -> Self {
        Self {
            author: leak(value.author),
            time: leak(value.time),
            text: value.text,
            color: rgb(value.color),
            grouped: value.grouped,
            reactions: value.reactions.into_iter().map(Into::into).collect(),
            pinned: value.pinned,
            attachment: value.attachment.map(Into::into),
            sticker: value.sticker.map(Into::into),
            gif: value.gif.map(Into::into),
            edit_history: value.edit_history.into_iter().map(Into::into).collect(),
            forwarded: value.forwarded,
            reply_to: value.reply_to.map(Into::into),
            delivery: value.delivery,
            readers: value.readers.into_iter().map(Into::into).collect(),
            poll: value.poll,
        }
    }
}

#[derive(Deserialize)]
pub struct MemberOwned {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub nick: Option<String>,
    #[serde(default)]
    pub joined_minutes: u32,
}

impl From<MemberOwned> for super::models::Member {
    fn from(value: MemberOwned) -> Self {
        Self {
            name: leak(value.name),
            color: value.color,
            online: value.online,
            role: value.role.map(leak),
            nick: value.nick.map(leak),
            joined_minutes: value.joined_minutes,
        }
    }
}

#[derive(Deserialize)]
pub struct FriendOwned {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub activity: String,
    #[serde(default)]
    pub group: String,
}

impl From<FriendOwned> for super::models::Friend {
    fn from(value: FriendOwned) -> Self {
        Self {
            name: leak(value.name),
            color: value.color,
            online: value.online,
            activity: leak(value.activity),
            group: value.group,
        }
    }
}

#[derive(Deserialize)]
pub struct InviteOwned {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub server: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub inviter: String,
    #[serde(default)]
    pub uses: u32,
    #[serde(default)]
    pub max_uses: u32,
    #[serde(default = "default_invite_expiry")]
    pub expiry: InviteExpiry,
    #[serde(default)]
    pub created_minutes: u32,
}

fn default_invite_expiry() -> InviteExpiry {
    InviteExpiry::Never
}

impl From<InviteOwned> for Invite {
    fn from(value: InviteOwned) -> Self {
        Self {
            code: value.code,
            server: value.server,
            channel: value.channel,
            inviter: leak(value.inviter),
            uses: value.uses,
            max_uses: value.max_uses,
            expiry: value.expiry,
            created_minutes: value.created_minutes,
        }
    }
}

#[derive(Deserialize)]
pub struct DmConversationOwned {
    #[serde(default)]
    pub friend: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub unread: usize,
    #[serde(default)]
    pub preview: String,
}

impl From<DmConversationOwned> for super::models::DmConversation {
    fn from(value: DmConversationOwned) -> Self {
        Self {
            friend: leak(value.friend),
            color: value.color,
            online: value.online,
            unread: value.unread,
            preview: leak(value.preview),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 快照必须能完整往返：模型里的 `&'static str` 与 `Rgba` 都通过镜像类型还原。
    #[test]
    fn snapshot_roundtrip_preserves_content() {
        let snapshot = Snapshot {
            version: VERSION,
            servers: crate::data::seed_servers(),
            messages: crate::data::seed_messages(),
            dm_conversations: crate::data::seed_dm_conversations(),
            friends: crate::data::seed_friends(),
            friend_groups: crate::data::seed_friend_groups(),
            invites: Some(crate::data::seed_invites()),
            members: Some(crate::data::seed_members()),
            roles: Some(crate::data::seed_roles()),
            permissions: Some(crate::data::seed_permissions()),
            automod_rules: Some(crate::data::seed_automod_rules()),
            events: Some(crate::data::seed_events()),
            audit: Some(crate::data::seed_audit()),
            bans: Some(crate::data::seed_bans()),
            me: crate::data::seed_me(),
            ..Default::default()
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let back: Snapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(back.version, VERSION);
        assert_eq!(back.servers.len(), snapshot.servers.len());
        assert_eq!(
            back.servers[0].categories[1].channels[0].notify,
            snapshot.servers[0].categories[1].channels[0].notify
        );

        let key = "开发者社区/闲聊";
        let before = &snapshot.messages[key];
        let after = &back.messages[key];
        assert_eq!(after.len(), before.len());
        assert_eq!(after[0].author, before[0].author);
        assert_eq!(after[0].time, before[0].time);
        assert_eq!(after[0].text, before[0].text);
        assert_eq!(after[0].color.r, before[0].color.r);
        assert_eq!(after[0].color.g, before[0].color.g);
        assert_eq!(after[0].color.b, before[0].color.b);

        // 投票：问题 / 选项 / 每位投票者都要完整往返（种子末尾有一条投票消息）。
        let poll_before = before.iter().find_map(|m| m.poll.as_ref());
        let poll_after = after.iter().find_map(|m| m.poll.as_ref());
        assert!(poll_before.is_some(), "种子里应有一条投票消息");
        let (pb, pa) = (poll_before.unwrap(), poll_after.unwrap());
        assert_eq!(pa.question, pb.question);
        assert_eq!(pa.options.len(), pb.options.len());
        assert_eq!(pa.options[0].voters, pb.options[0].voters);
        assert_eq!(pa.total_votes(), pb.total_votes());

        // 服务器活动：归属服务器 / 相对时间 / 报名名单 / 分类都要完整往返。
        let events_before = snapshot.events.clone().unwrap_or_default();
        let events_after = back.events.clone().unwrap_or_default();
        assert_eq!(events_after.len(), events_before.len());
        assert_eq!(events_after[0].server, events_before[0].server);
        assert_eq!(events_after[0].title, events_before[0].title);
        assert_eq!(events_after[0].starts_in_minutes, events_before[0].starts_in_minutes);
        assert_eq!(events_after[0].attendees, events_before[0].attendees);
        assert_eq!(events_after[0].category, events_before[0].category);

        // 好友分组：顺序表与成员身上的分组名都要能完整往返。
        assert_eq!(back.friend_groups, snapshot.friend_groups);
        assert_eq!(back.friends.len(), snapshot.friends.len());
        assert_eq!(back.friends[0].group, snapshot.friends[0].group);
        assert_eq!(back.friends[0].name, snapshot.friends[0].name);

        assert_eq!(back.me.display_name, snapshot.me.display_name);
        assert_eq!(back.dm_conversations.len(), snapshot.dm_conversations.len());
        assert_eq!(back.dm_conversations[0].friend, snapshot.dm_conversations[0].friend);

        // 邀请：服务器 / 邀请人静态串 / 有效期都要完整往返。
        let before_invites = snapshot.invites.clone().unwrap_or_default();
        let after_invites = back.invites.clone().unwrap_or_default();
        assert_eq!(after_invites.len(), before_invites.len());
        assert_eq!(after_invites[0].code, before_invites[0].code);
        assert_eq!(after_invites[0].server, before_invites[0].server);
        assert_eq!(after_invites[0].inviter, before_invites[0].inviter);
        assert_eq!(after_invites[0].expiry, before_invites[0].expiry);
        assert_eq!(after_invites[0].max_uses, before_invites[0].max_uses);

        // 成员与角色：角色名（静态串）+ 角色表的顺序都要能恢复。
        let before_members = snapshot.members.clone().unwrap_or_default();
        let after_members = back.members.clone().unwrap_or_default();
        assert_eq!(after_members.len(), before_members.len());
        assert_eq!(after_members[0].name, before_members[0].name);
        assert_eq!(after_members[0].role, before_members[0].role);
        assert_eq!(after_members[0].nick, before_members[0].nick);
        assert_eq!(after_members[0].joined_minutes, before_members[0].joined_minutes);
        let before_roles = snapshot.roles.clone().unwrap_or_default();
        let after_roles = back.roles.clone().unwrap_or_default();
        assert_eq!(after_roles.len(), before_roles.len());
        assert_eq!(after_roles[0].name, before_roles[0].name);
        assert_eq!(after_roles[0].color, before_roles[0].color);

        // 权限：角色归属 / 权限 key / 三态枚举都要完整往返。
        let before_permissions = snapshot.permissions.clone().unwrap_or_default();
        let after_permissions = back.permissions.clone().unwrap_or_default();
        assert_eq!(after_permissions.len(), before_permissions.len());
        assert_eq!(after_permissions[0].role, before_permissions[0].role);
        assert_eq!(after_permissions[0].permission, before_permissions[0].permission);
        assert_eq!(after_permissions[0].state, before_permissions[0].state);

        // 自动审核规则：名称 / 关键词清单 / 动作 / 启用状态都要完整往返。
        let before_rules = snapshot.automod_rules.clone().unwrap_or_default();
        let after_rules = back.automod_rules.clone().unwrap_or_default();
        assert_eq!(after_rules.len(), before_rules.len());
        assert_eq!(after_rules[0].name, before_rules[0].name);
        assert_eq!(after_rules[0].keywords, before_rules[0].keywords);
        assert_eq!(after_rules[0].action, before_rules[0].action);
        assert_eq!(after_rules[0].enabled, before_rules[0].enabled);

        // 审核日志：类别枚举、序号与两段文本都要完整往返。
        let before_audit = snapshot.audit.clone().unwrap_or_default();
        let after_audit = back.audit.clone().unwrap_or_default();
        assert_eq!(after_audit.len(), before_audit.len());
        assert_eq!(after_audit[0].kind, before_audit[0].kind);
        assert_eq!(after_audit[0].seq, before_audit[0].seq);
        assert_eq!(after_audit[0].actor, before_audit[0].actor);
        assert_eq!(after_audit[0].action, before_audit[0].action);
        assert_eq!(after_audit[0].detail, before_audit[0].detail);

        // 封禁列表：原因 / 操作人与时间都是自有字符串，直接往返。
        let before_bans = snapshot.bans.clone().unwrap_or_default();
        let after_bans = back.bans.clone().unwrap_or_default();
        assert_eq!(after_bans.len(), before_bans.len());
        assert_eq!(after_bans[0].name, before_bans[0].name);
        assert_eq!(after_bans[0].reason, before_bans[0].reason);
        assert_eq!(after_bans[0].by, before_bans[0].by);
        assert_eq!(after_bans[0].minutes, before_bans[0].minutes);
    }
}
