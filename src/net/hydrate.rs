//! 后端 DTO → 现有 UI 模型的水合。
//!
//! 现有界面以显示名/`服务器/频道` 为键，本模块额外维护 `RemoteIndex`
//! （显示名 ↔ 后端 id），C2 起的写操作都从索引取 id，避免按名字反查。

#![allow(dead_code)] // C1 水合子集，C2–C5 继续补字段

use std::collections::HashMap;

use crate::data::{
    Attachment, AttachmentKind, Category, Channel, ChannelKind, DmConversation, Friend,
    Member, Message, MyProfile, NotifyLevel, PermissionGrant, PermissionState, Role,
    Server,
};

use super::dto::{
    ChannelDto, ChannelUnreadDto, DmDto, FriendDto, FriendGroupDto, MemberDto, MessageDto,
    PermissionMatrixDto, RoleDto, ServerDto, ServerTreeDto, UnreadCountsDto, UserDto,
};

#[derive(Clone, Debug, Default)]
pub struct RemoteIndex {
    pub me_id: String,
    pub servers: HashMap<String, String>,
    pub channels: HashMap<String, String>,
    pub members: HashMap<String, String>,
    pub roles: HashMap<String, String>,
    pub friends: HashMap<String, String>,
    pub dms: HashMap<String, String>,
}

pub struct BootData {
    pub me: UserDto,
    pub servers: Vec<ServerDto>,
    pub trees: Vec<ServerTreeDto>,
    pub members: Vec<Vec<MemberDto>>,
    pub roles: Vec<Vec<RoleDto>>,
    pub permissions: Vec<PermissionMatrixDto>,
    pub friends: Vec<FriendDto>,
    pub groups: Vec<FriendGroupDto>,
    pub dms: Vec<DmDto>,
    pub counts: UnreadCountsDto,
    pub active_messages: Vec<MessageDto>,
    pub active_server_id: String,
    pub active_channel_id: String,
}

pub struct Hydrated {
    pub me: MyProfile,
    pub servers: Vec<Server>,
    pub active_server: usize,
    pub active_channel: String,
    pub members_by_server: HashMap<String, Vec<Member>>,
    pub roles_by_server: HashMap<String, Vec<Role>>,
    pub permissions_by_server: HashMap<String, Vec<PermissionGrant>>,
    pub members_active: Vec<Member>,
    pub roles_active: Vec<Role>,
    pub friends: Vec<Friend>,
    pub friend_groups: Vec<String>,
    pub dm_conversations: Vec<DmConversation>,
    pub messages: Vec<Message>,
    pub index: RemoteIndex,
}

fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn short_time(iso: &str) -> &'static str {
    if let Some((_, rest)) = iso.split_once('T')
        && rest.len() >= 5
    {
        return leak(rest[..5].to_string());
    }
    "刚刚"
}

fn online(status: &str) -> bool {
    !matches!(status, "offline" | "invisible" | "")
}

fn notify_level(value: &str) -> NotifyLevel {
    match value {
        "all" => NotifyLevel::All,
        "mentions" => NotifyLevel::Mentions,
        "none" => NotifyLevel::Nothing,
        _ => NotifyLevel::Inherit,
    }
}

fn channel_kind(value: &str) -> ChannelKind {
    match value {
        "voice" => ChannelKind::Voice,
        "forum" => ChannelKind::Forum,
        _ => ChannelKind::Text,
    }
}

fn permission_state(value: &str) -> PermissionState {
    match value {
        "allow" => PermissionState::Allow,
        "deny" => PermissionState::Deny,
        _ => PermissionState::Unset,
    }
}

fn size_label(size: u64) -> &'static str {
    let text = if size >= 1024 * 1024 {
        format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
    } else if size >= 1024 {
        format!("{:.0} KB", size as f64 / 1024.0)
    } else {
        format!("{size} B")
    };
    leak(text)
}

fn message_from(dto: &MessageDto) -> Message {
    let mut message = Message::new(
        leak(dto.author.name.clone()),
        short_time(&dto.created_at),
        dto.content.clone(),
        dto.author.color,
    );
    message.pinned = dto.pinned;
    if let Some(attachment) = dto.attachments.first() {
        let kind = if attachment.mime.starts_with("image/") {
            AttachmentKind::Image
        } else {
            AttachmentKind::File
        };
        message.attachment = Some(Attachment {
            name: attachment.filename.clone(),
            size: size_label(attachment.size),
            kind,
        });
    }
    for reaction in &dto.reactions {
        if let (Some(emoji), Some(count)) = (
            reaction.get("emoji").and_then(|value| value.as_str()),
            reaction.get("count").and_then(|value| value.as_u64()),
        ) {
            message.reactions.push(crate::data::Reaction {
                emoji: leak(emoji.to_string()),
                count: count as usize,
                mine: reaction.get("me").and_then(|value| value.as_bool()).unwrap_or(false),
            });
        }
    }
    message
}

fn channel_from(dto: &ChannelDto, unread: u32) -> Channel {
    Channel {
        name: dto.name.clone(),
        kind: channel_kind(&dto.kind),
        unread: unread as usize,
        topic: dto.topic.clone(),
        notify: notify_level(&dto.notify_level),
    }
}

pub fn hydrate(data: BootData) -> Hydrated {
    let mut index = RemoteIndex {
        me_id: data.me.id.clone(),
        ..Default::default()
    };
    let me = MyProfile::new(
        data.me.display_name.clone(),
        format!("{}#{}", data.me.username, data.me.tag),
        data.me.email.clone().unwrap_or_default(),
        data.me.about.clone(),
        data.me.avatar_color,
    );

    let mut channel_unread: HashMap<&str, u32> = HashMap::new();
    for ChannelUnreadDto {
        channel_id, unread, ..
    } in &data.counts.channels
    {
        channel_unread.insert(channel_id.as_str(), *unread);
    }

    let mut members_by_server = HashMap::new();
    let mut roles_by_server = HashMap::new();
    let mut permissions_by_server = HashMap::new();
    let mut servers = Vec::new();

    for (position, server_dto) in data.servers.iter().enumerate() {
        index
            .servers
            .insert(server_dto.name.clone(), server_dto.id.clone());
        let tree = data.trees.get(position);
        let role_dtos = data.roles.get(position).cloned().unwrap_or_default();
        let role_names: HashMap<&str, &str> = role_dtos
            .iter()
            .map(|role| (role.id.as_str(), role.name.as_str()))
            .collect();
        let member_role_names: HashMap<&str, &str> = role_dtos
            .iter()
            .filter(|role| !role.is_everyone)
            .map(|role| (role.id.as_str(), role.name.as_str()))
            .collect();

        let mut roles: Vec<Role> = role_dtos
            .iter()
            .filter(|role| !role.is_everyone)
            .map(|role| Role::new(role.name.clone(), role.color))
            .collect();
        roles.sort_by(|a, b| a.name.cmp(&b.name));
        for role in &role_dtos {
            index
                .roles
                .insert(format!("{}/{}", server_dto.name, role.name), role.id.clone());
        }

        let mut members: Vec<Member> = data
            .members
            .get(position)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|member| {
                let display = if member.nickname.trim().is_empty() {
                    member.display_name.clone()
                } else {
                    member.nickname.clone()
                };
                index.members.insert(
                    format!("{}/{}", server_dto.name, member.username),
                    member.user_id.clone(),
                );
                let role = member
                    .role_ids
                    .iter()
                    .find_map(|role_id| member_role_names.get(role_id.as_str()).copied())
                    .map(|name| leak(name.to_string()));
                Member {
                    name: leak(member.username.clone()),
                    color: member.avatar_color,
                    online: online(&member.status) || member.is_owner,
                    role,
                    nick: if display == member.display_name {
                        None
                    } else {
                        Some(leak(display))
                    },
                    joined_minutes: 0,
                }
            })
            .collect();
        members.sort_by(|a, b| a.display().cmp(b.display()));

        let permissions = data
            .permissions
            .get(position)
            .map(|matrix| {
                matrix
                    .items
                    .iter()
                    .filter_map(|item| {
                        role_names.get(item.role_id.as_str()).map(|role_name| PermissionGrant {
                            role: (*role_name).to_string(),
                            permission: item.permission.clone(),
                            state: permission_state(&item.state),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut categories = Vec::new();
        if let Some(tree) = tree {
            let mut grouped: HashMap<Option<&str>, Vec<&ChannelDto>> = HashMap::new();
            for channel in &tree.channels {
                grouped
                    .entry(channel.category_id.as_deref())
                    .or_default()
                    .push(channel);
                index.channels.insert(
                    format!("{}/{}", server_dto.name, channel.name),
                    channel.id.clone(),
                );
            }
            for category in &tree.categories {
                let channels = grouped
                    .remove(&Some(category.id.as_str()))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|channel| {
                        channel_from(channel, *channel_unread.get(channel.id.as_str()).unwrap_or(&0))
                    })
                    .collect();
                categories.push(Category::new(category.name.clone(), channels));
            }
            if let Some(rest) = grouped.remove(&None)
                && !rest.is_empty()
            {
                let channels = rest
                    .into_iter()
                    .map(|channel| {
                        channel_from(channel, *channel_unread.get(channel.id.as_str()).unwrap_or(&0))
                    })
                    .collect();
                categories.push(Category::new("未分类", channels));
            }
        }

        servers.push(Server {
            name: server_dto.name.clone(),
            badge: server_dto.name.chars().next().unwrap_or('?').to_string(),
            color: server_dto.icon_color,
            notify: notify_level(&server_dto.notify_level),
            description: server_dto.description.clone(),
            categories,
        });
        members_by_server.insert(server_dto.name.clone(), members);
        roles_by_server.insert(server_dto.name.clone(), roles);
        permissions_by_server.insert(server_dto.name.clone(), permissions);
    }

    let active_server = data
        .servers
        .iter()
        .position(|server| server.id == data.active_server_id)
        .unwrap_or(0);
    let active_server_name = data
        .servers
        .get(active_server)
        .map(|server| server.name.clone())
        .unwrap_or_default();
    let active_channel = data
        .trees
        .get(active_server)
        .and_then(|tree| tree.channels.iter().find(|channel| channel.id == data.active_channel_id))
        .map(|channel| channel.name.clone())
        .or_else(|| {
            servers
                .get(active_server)
                .and_then(|server| {
                    server
                        .categories
                        .iter()
                        .flat_map(|category| category.channels.iter())
                        .find(|channel| channel.kind == ChannelKind::Text)
                })
                .map(|channel| channel.name.clone())
        })
        .unwrap_or_default();

    let group_names: HashMap<&str, &str> = data
        .groups
        .iter()
        .map(|group| (group.id.as_str(), group.name.as_str()))
        .collect();
    let groups = data.groups.iter().map(|group| group.name.clone()).collect();

    let mut friends = Vec::new();
    for friend in &data.friends {
        index
            .friends
            .insert(friend.display_name.clone(), friend.id.clone());
        friends.push(Friend {
            name: leak(friend.display_name.clone()),
            color: friend.avatar_color,
            online: friend.online,
            activity: "",
            group: friend
                .group_id
                .as_deref()
                .and_then(|id| group_names.get(id))
                .map(|name| (*name).to_string())
                .unwrap_or_default(),
        });
    }
    friends.sort_by(|a, b| a.name.cmp(b.name));

    let dm_unread: HashMap<&str, u32> = data
        .counts
        .dms
        .iter()
        .map(|item| (item.dm_id.as_str(), item.unread))
        .collect();
    let mut dms = Vec::new();
    for dm in &data.dms {
        index.dms.insert(dm.user.display_name.clone(), dm.id.clone());
        dms.push(DmConversation {
            friend: leak(dm.user.display_name.clone()),
            color: dm.user.avatar_color,
            online: online(&dm.user.status),
            unread: *dm_unread.get(dm.id.as_str()).unwrap_or(&dm.unread) as usize,
            preview: leak(
                dm.last_message
                    .as_ref()
                    .map(|message| message.content.clone())
                    .unwrap_or_default(),
            ),
        });
    }

    let messages = data
        .active_messages
        .iter()
        .map(message_from)
        .collect::<Vec<_>>();

    Hydrated {
        me,
        servers,
        active_server,
        active_channel,
        members_active: members_by_server
            .get(&active_server_name)
            .cloned()
            .unwrap_or_default(),
        roles_active: roles_by_server
            .get(&active_server_name)
            .cloned()
            .unwrap_or_default(),
        members_by_server,
        roles_by_server,
        permissions_by_server,
        friends,
        friend_groups: groups,
        dm_conversations: dms,
        messages,
        index,
    }
}

/// 空账号兜底：没有任何服务器时给一个本地占位，避免界面索引越界。
pub fn fallback_server(display_name: &str) -> (Server, Vec<Member>) {
    let name = if display_name.trim().is_empty() {
        "我的服务器".to_string()
    } else {
        "开始".to_string()
    };
    (
        Server {
            name: name.clone(),
            badge: name.chars().next().unwrap_or('?').to_string(),
            color: 0x5865f2,
            notify: NotifyLevel::All,
            description: "还没有加入任何服务器。".into(),
            categories: vec![Category::new("开始", vec![Channel::text("欢迎", "创建一个服务器开始聊天")])],
        },
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dto<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn hydrates_server_tree_members_and_messages() {
        let data = BootData {
            me: dto(json!({"id":"u1","username":"alice","tag":"0001","display_name":"Alice",
                "avatar_color":5865,"about":"hi","status":"online","email":"a@b.com"})),
            servers: vec![dto(json!({"id":"s1","name":"开发","icon_color":5865,
                "description":"desc","owner_id":"u1","member_count":2,"notify_level":"all"}))],
            trees: vec![dto(json!({
                "categories":[{"id":"c1","server_id":"s1","name":"文字","position":0,"collapsed":false}],
                "channels":[
                    {"id":"ch1","server_id":"s1","category_id":"c1","name":"一般","kind":"text",
                     "topic":"","position":0,"notify_level":"inherit","last_read_message_id":null},
                    {"id":"ch2","server_id":"s1","category_id":"c1","name":"大厅","kind":"voice",
                     "topic":"","position":1,"notify_level":"inherit","last_read_message_id":null}
                ]}))],
            members: vec![vec![
                dto(json!({"user_id":"u1","username":"alice","tag":"0001","display_name":"Alice",
                    "nickname":"","avatar_color":5865,"status":"online","role_ids":["r1"],"is_owner":true})),
                dto(json!({"user_id":"u2","username":"bob","tag":"0002","display_name":"Bob",
                    "nickname":"小鲍","avatar_color":123,"status":"offline","role_ids":[]})),
            ]],
            roles: vec![vec![
                dto(json!({"id":"r0","server_id":"s1","name":"@everyone","color":99,"position":0,"is_everyone":true})),
                dto(json!({"id":"r1","server_id":"s1","name":"管理员","color":5865,"position":5})),
            ]],
            permissions: vec![dto(json!({"items":[{"role_id":"r1","permission":"MANAGE_ROLES","state":"allow"}]}))],
            friends: vec![],
            groups: vec![],
            dms: vec![],
            counts: dto(json!({"channels":[{"channel_id":"ch1","server_id":"s1","unread":3}]})),
            active_messages: vec![dto(json!({"id":"m1","channel_id":"ch1",
                "author":{"id":"u1","name":"Alice","color":5865},"content":"hello","type":"text",
                "pinned":false,"created_at":"2026-11-24T10:30:00Z","reactions":[],"attachments":[]}))],
            active_server_id: "s1".into(),
            active_channel_id: "ch1".into(),
        };

        let hydrated = hydrate(data);
        assert_eq!(hydrated.me.display_name, "Alice");
        assert_eq!(hydrated.servers.len(), 1);
        assert_eq!(hydrated.active_channel, "一般");
        assert_eq!(hydrated.servers[0].categories[0].channels[0].unread, 3);
        assert_eq!(hydrated.members_active.len(), 2);
        assert_eq!(hydrated.members_active[1].display(), "小鲍");
        assert_eq!(hydrated.roles_active.len(), 1);
        assert_eq!(hydrated.permissions_by_server["开发"][0].permission, "MANAGE_ROLES");
        assert_eq!(hydrated.messages[0].text, "hello");
        assert_eq!(hydrated.messages[0].time, "10:30");
        assert_eq!(hydrated.index.channels.get("开发/一般").map(String::as_str), Some("ch1"));
        assert_eq!(hydrated.index.members.get("开发/alice").map(String::as_str), Some("u1"));
    }

    #[test]
    fn fallback_server_keeps_ui_index_safe() {
        let (server, members) = fallback_server("Alice");
        assert_eq!(server.name, "开始");
        assert_eq!(server.categories[0].channels[0].name, "欢迎");
        assert!(members.is_empty());
    }
}
