//! 后端 DTO → 现有 UI 模型的水合。
//!
//! 现有界面以显示名/`服务器/频道` 为键，本模块额外维护 `RemoteIndex`
//! （显示名 ↔ 后端 id），C2 起的写操作都从索引取 id，避免按名字反查。

#![allow(dead_code)] // C1 水合子集，C2–C5 继续补字段

use std::collections::HashMap;

use crate::data::{
    Attachment, AttachmentKind, Category, Channel, ChannelKind, DmConversation, Friend,
    FriendRequest, Member, Message, MessageRequest, MyProfile, Notification, NotificationKind,
    NotificationTarget, NotifyLevel, PermissionGrant, PermissionState, Role, Server,
};

use super::dto::{
    AuditDto, AutoModRuleDto, BookmarkDto, BoostDto, ChannelDto, ChannelUnreadDto, DmDto, DmMessageDto, FriendDto, FriendGroupDto,
    FriendRequestDto, ForumPostDto, InviteDto, MemberDto, MessageDto, MessageRequestDto,
    NotificationDto, PermissionMatrixDto, RoleDto, ServerDto, ServerEventDto, ServerTreeDto,
    UnreadCountsDto, UserDto, WelcomeDto,
};

#[derive(Clone, Debug, Default)]
pub struct RemoteIndex {
    pub me_id: String,
    pub servers: HashMap<String, String>,
    pub channels: HashMap<String, String>,
    pub categories: HashMap<String, String>,
    pub members: HashMap<String, String>,
    pub roles: HashMap<String, String>,
    pub friends: HashMap<String, String>,
    /// 显示名/用户名 → user_id（成员与私信对象），用于没有现成私信时创建会话。
    pub people: HashMap<String, String>,
    pub friend_groups: HashMap<String, String>,
    pub blocks: HashMap<String, String>,
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
    pub friend_requests: Vec<FriendRequestDto>,
    pub blocks: Vec<super::dto::BlockDto>,
    pub message_requests: Vec<MessageRequestDto>,
    pub invites: Vec<InviteDto>,
    pub forum_posts: Vec<ForumPostDto>,
    pub events: Vec<ServerEventDto>,
    pub boosts: Vec<(String, BoostDto)>,
    pub welcomes: Vec<(String, WelcomeDto)>,
    pub notifications: Vec<NotificationDto>,
    pub bookmarks: Vec<BookmarkDto>,
    pub automod_rules: Vec<(String, Vec<AutoModRuleDto>)>,
    pub audit: Vec<(String, Vec<AuditDto>)>,
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
    pub friend_requests: Vec<FriendRequest>,
    pub blocked_users: Vec<String>,
    pub message_requests: Vec<MessageRequest>,
    pub invites: Vec<crate::data::Invite>,
    pub forum_posts: Vec<crate::data::ForumPost>,
    pub events: Vec<crate::data::ServerEvent>,
    pub boosts: Vec<crate::data::ServerBoost>,
    pub welcomes: Vec<crate::data::ServerWelcome>,
    pub welcome_joined: Vec<String>,
    pub notifications: Vec<Notification>,
    pub bookmarks: Vec<crate::data::Bookmark>,
    pub automod_by_server: HashMap<String, Vec<crate::data::AutoModRule>>,
    pub audit_by_server: HashMap<String, Vec<crate::data::AuditEntry>>,
    pub dm_conversations: Vec<DmConversation>,
    pub messages: Vec<Message>,
    pub index: RemoteIndex,
}

fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

pub(crate) fn short_time(iso: &str) -> &'static str {
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

pub(crate) fn message_from(dto: &MessageDto) -> Message {
    let mut message = Message::new(
        leak(dto.author.name.clone()),
        short_time(&dto.created_at),
        dto.content.clone(),
        dto.author.color,
    );
    message.id = dto.id.clone();
    message.pinned = dto.pinned;
    if let Some(poll) = dto.poll.as_ref()
        && let (Some(question), Some(options)) = (
            poll.get("question").and_then(|value| value.as_str()),
            poll.get("options").and_then(|value| value.as_array()),
        )
    {
        let mut converted = crate::data::Poll::new(
            question,
            &[],
            poll.get("multi").and_then(|value| value.as_bool()).unwrap_or(false),
        );
        converted.id = poll
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        converted.closed = poll
            .get("closed")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        converted.options = options
            .iter()
            .map(|option| {
                let label = option
                    .get("label")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let votes = option
                    .get("votes")
                    .and_then(|value| value.as_u64())
                    .unwrap_or_default() as usize;
                let mine = option
                    .get("me")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                crate::data::PollOption {
                    id: option
                        .get("id")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    label: label.to_string(),
                    voters: if mine {
                        vec!["你".to_string(); votes.max(1)]
                    } else {
                        vec![String::new(); votes]
                    },
                }
            })
            .collect();
        message.poll = Some(converted);
    }
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

pub(crate) fn dm_message_from(dto: &DmMessageDto) -> Message {
    let mut message = Message::new(
        leak(dto.author.name.clone()),
        short_time(&dto.created_at),
        dto.content.clone(),
        dto.author.color,
    );
    message.id = dto.id.clone();
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
                index
                    .people
                    .insert(member.username.clone(), member.user_id.clone());
                if !member.display_name.is_empty() {
                    index
                        .people
                        .insert(member.display_name.clone(), member.user_id.clone());
                }
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
                index
                    .categories
                    .insert(format!("{}/{}", server_dto.name, category.name), category.id.clone());
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
    for group in &data.groups {
        index
            .friend_groups
            .insert(group.name.clone(), group.id.clone());
    }
    let mut blocked_users = Vec::new();
    for block in &data.blocks {
        index.blocks.insert(block.username.clone(), block.user_id.clone());
        index.blocks.insert(block.display_name.clone(), block.user_id.clone());
        blocked_users.push(if block.display_name.is_empty() {
            block.username.clone()
        } else {
            block.display_name.clone()
        });
    }

    let mut friend_requests = Vec::new();
    for request in &data.friend_requests {
        let incoming = request.direction != "outgoing";
        let user = if incoming {
            &request.from_user
        } else {
            &request.to_user
        };
        friend_requests.push(FriendRequest {
            id: request.id.clone(),
            name: if user.display_name.is_empty() {
                user.username.clone()
            } else {
                user.display_name.clone()
            },
            color: user.avatar_color,
            incoming,
        });
    }

    let mut friends = Vec::new();
    for friend in &data.friends {
        index
            .friends
            .insert(friend.display_name.clone(), friend.id.clone());
        index
            .people
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

    let mut invites = Vec::new();
    for invite in &data.invites {
        let server_name = data
            .servers
            .iter()
            .find(|server| server.id == invite.server_id)
            .map(|server| server.name.clone())
            .unwrap_or_default();
        let channel_name = data
            .trees
            .iter()
            .flat_map(|tree| tree.channels.iter())
            .find(|channel| channel.id == invite.channel_id)
            .map(|channel| channel.name.clone())
            .unwrap_or_default();
        invites.push(crate::data::Invite {
            code: invite.code.clone(),
            server: server_name,
            channel: channel_name,
            inviter: leak(if invite.created_by == index.me_id {
                "你".to_string()
            } else {
                invite.created_at.clone()
            }),
            uses: invite.uses,
            max_uses: invite.max_uses.unwrap_or(0),
            expiry: if invite.expires_at.is_none() {
                crate::data::InviteExpiry::Never
            } else {
                crate::data::InviteExpiry::SevenDays
            },
            created_minutes: 0,
        });
    }

    let mut events = Vec::new();
    for event in &data.events {
        let server_name = data
            .servers
            .iter()
            .find(|server| server.id == event.server_id)
            .map(|server| server.name.clone())
            .unwrap_or_default();
        let channel_name = event
            .channel_id
            .as_deref()
            .and_then(|channel_id| {
                data.trees
                    .iter()
                    .flat_map(|tree| tree.channels.iter())
                    .find(|channel| channel.id == channel_id)
                    .map(|channel| channel.name.clone())
            })
            .unwrap_or_default();
        let category = match event.category.as_str() {
            "game" => crate::data::EventCategory::Game,
            "voice" => crate::data::EventCategory::Voice,
            "community" => crate::data::EventCategory::Community,
            _ => crate::data::EventCategory::Other,
        };
        let mut converted = crate::data::ServerEvent::new(
            event.id.clone(),
            server_name,
            event.title.clone(),
            channel_name,
            leak(event.creator.name.clone()),
            if event.status == "live" { -1 } else { 60 },
            event.duration_minutes,
            category,
        );
        converted.description = event.description.clone();
        converted.attendees = if event.going {
            vec!["你".to_string()]
        } else {
            Vec::new()
        };
        events.push(converted);
    }

    let mut boosts = Vec::new();
    for (server_name, boost) in &data.boosts {
        let mut converted = crate::data::ServerBoost::new(server_name.clone());
        for booster in &boost.boosters {
            converted.boosters.push(crate::data::Booster::new(
                leak(booster.name.clone()),
                booster.color,
                0,
            ));
        }
        boosts.push(converted);
    }

    let mut welcomes = Vec::new();
    let mut welcome_joined = Vec::new();
    for (server_name, welcome) in &data.welcomes {
        if let Some(config) = welcome.welcome.as_ref() {
            let mut converted = crate::data::ServerWelcome::new(server_name.clone(), config.headline.clone(), config.description.clone());
            converted.headline = config.headline.clone();
            converted.description = config.description.clone();
            converted.require_rules = config.require_rules;
            converted.rules = config
                .rules
                .iter()
                .map(|rule| crate::data::WelcomeRule {
                    title: rule.title.clone(),
                    body: rule.body.clone(),
                })
                .collect();
            converted.recommended = config
                .recommended
                .iter()
                .filter_map(|channel_id| {
                    data.trees
                        .iter()
                        .flat_map(|tree| tree.channels.iter())
                        .find(|channel| channel.id == *channel_id)
                        .map(|channel| channel.name.clone())
                })
                .collect();
            welcomes.push(converted);
        }
        if welcome.accepted {
            welcome_joined.push(server_name.clone());
        }
    }

    let mut automod_by_server = HashMap::new();
    for (server_name, rules) in &data.automod_rules {
        let mut converted: Vec<crate::data::AutoModRule> = rules
            .iter()
            .map(|rule| {
                let mut item = crate::data::AutoModRule::new(
                    rule.name.clone(),
                    String::new(),
                    &[],
                    if rule.action == "warn" {
                        crate::data::AutoModAction::Warn
                    } else {
                        crate::data::AutoModAction::Block
                    },
                    rule.enabled,
                );
                item.id = rule.id.clone();
                item.keywords = rule.keywords.clone();
                item
            })
            .collect();
        converted.sort_by(|a, b| a.name.cmp(&b.name));
        automod_by_server.insert(server_name.clone(), converted);
    }

    let mut audit_by_server = HashMap::new();
    for (server_name, entries) in &data.audit {
        let mut converted: Vec<crate::data::AuditEntry> = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let kind = match entry.kind.as_str() {
                "server" => crate::data::AuditKind::Server,
                "channel" => crate::data::AuditKind::Channel,
                "role" => crate::data::AuditKind::Role,
                "member" => crate::data::AuditKind::Member,
                "invite" => crate::data::AuditKind::Invite,
                "message" => crate::data::AuditKind::Channel,
                "boost" => crate::data::AuditKind::Server,
                _ => crate::data::AuditKind::Server,
            };
            converted.push(crate::data::AuditEntry::new(
                index as u32,
                kind,
                entry.actor.name.clone(),
                "操作",
                entry.detail.clone(),
            ));
        }
        converted.reverse();
        audit_by_server.insert(server_name.clone(), converted);
    }

    let mut bookmarks = Vec::new();
    for bookmark in &data.bookmarks {
        let is_dm = bookmark.target_kind == "dm";
        let (server, channel) = if is_dm {
            (
                String::new(),
                index
                    .dms
                    .iter()
                    .find(|(_, id)| id.as_str() == bookmark.scope)
                    .map(|(name, _)| (*name).to_string())
                    .unwrap_or_default(),
            )
        } else {
            let channel = data
                .trees
                .iter()
                .flat_map(|tree| tree.channels.iter())
                .find(|channel| channel.id == bookmark.scope)
                .cloned();
            let server_name = channel
                .as_ref()
                .and_then(|channel| {
                    data.servers
                        .iter()
                        .find(|server| server.id == channel.server_id)
                        .map(|server| server.name.clone())
                })
                .unwrap_or_default();
            (
                server_name,
                channel.as_ref().map(|channel| channel.name.clone()).unwrap_or_default(),
            )
        };
        let author = bookmark.author.clone().unwrap_or_default();
        bookmarks.push(crate::data::Bookmark::new(
            is_dm,
            server,
            channel,
            if author.name.is_empty() { "已注销用户".into() } else { author.name },
            author.color,
            bookmark.content.clone(),
            short_time(&bookmark.saved_at).to_string(),
            0,
        ));
        if let Some(last) = bookmarks.last_mut() {
            last.message_id = bookmark.target_id.clone();
            last.target_kind = bookmark.target_kind.clone();
        }
    }

    let mut notifications = Vec::new();
    for notification in &data.notifications {
        let kind = match notification.kind.as_str() {
            "mention" => NotificationKind::Mention,
            "dm" => NotificationKind::Dm,
            "friend_request" => NotificationKind::FriendRequest,
            _ => NotificationKind::UnreadChannel,
        };
        let target = if notification.target_kind == "dm"
            && let Some(friend) = index
                .dms
                .iter()
                .find(|(_, id)| id.as_str() == notification.target_id)
                .map(|(name, _)| (*name).to_string())
        {
            NotificationTarget::Dm(friend)
        } else if notification.target_kind == "channel" {
            let channel = data
                .trees
                .iter()
                .flat_map(|tree| tree.channels.iter())
                .find(|channel| channel.id == notification.target_id);
            if let Some(channel) = channel
                && let Some(server) = data
                    .servers
                    .iter()
                    .position(|server| server.id == channel.server_id)
            {
                NotificationTarget::Channel {
                    server,
                    channel: channel.name.clone(),
                }
            } else {
                NotificationTarget::Friends
            }
        } else {
            NotificationTarget::Friends
        };
        notifications.push(Notification {
            kind,
            title: notification.title.clone(),
            body: notification.body.clone(),
            time: short_time(&notification.created_at),
            color: 0x5865f2,
            target,
        });
    }

    let mut forum_posts = Vec::new();
    for post in &data.forum_posts {
        let (server_name, channel_name) = data
            .trees
            .iter()
            .flat_map(|tree| tree.channels.iter())
            .find(|channel| channel.id == post.channel_id)
            .map(|channel| {
                data.servers
                    .iter()
                    .find(|server| server.id == channel.server_id)
                    .map(|server| (server.name.clone(), channel.name.clone()))
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if server_name.is_empty() {
            continue;
        }
        let mut converted = crate::data::ForumPost::new(
            post.id.clone(),
            server_name,
            channel_name,
            post.title.clone(),
            leak(post.author.name.clone()),
            post.author.color,
            post.body.clone(),
            0,
        );
        converted.tags = post.tags.clone();
        converted.pinned = post.pinned;
        converted.locked = post.locked;
        converted.reply_count_override = Some(post.reply_count);
        forum_posts.push(converted);
    }

    let message_requests = data
        .message_requests
        .iter()
        .map(|request| MessageRequest {
            id: request.id.clone(),
            sender: if request.from_user.display_name.is_empty() {
                request.from_user.username.clone()
            } else {
                request.from_user.display_name.clone()
            },
            color: request.from_user.avatar_color,
            preview: request.first_message.clone(),
            minutes_ago: 0,
            state: match request.state.as_str() {
                "accepted" => crate::data::MessageRequestState::Accepted,
                "ignored" => crate::data::MessageRequestState::Ignored,
                _ => crate::data::MessageRequestState::Pending,
            },
            visible: request.state != "deleted",
        })
        .collect();

    let dm_unread: HashMap<&str, u32> = data
        .counts
        .dms
        .iter()
        .map(|item| (item.dm_id.as_str(), item.unread))
        .collect();
    let mut dms = Vec::new();
    for dm in &data.dms {
        index.dms.insert(dm.user.display_name.clone(), dm.id.clone());
        // 成员列表使用 username，这里同时登记别名，避免从成员资料呼叫时找不到会话
        index.dms.insert(dm.user.username.clone(), dm.id.clone());
        index
            .people
            .insert(dm.user.display_name.clone(), dm.user.id.clone());
        index
            .people
            .insert(dm.user.username.clone(), dm.user.id.clone());
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
        friend_requests,
        blocked_users,
        message_requests,
        invites,
        forum_posts,
        events,
        boosts,
        welcomes,
        welcome_joined,
        notifications,
        bookmarks,
        automod_by_server,
        audit_by_server,
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
            friend_requests: vec![],
            blocks: vec![],
            message_requests: vec![],
            invites: vec![],
            forum_posts: vec![],
            events: vec![],
            boosts: vec![],
            welcomes: vec![],
            notifications: vec![],
            bookmarks: vec![],
            automod_rules: vec![],
            audit: vec![],
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
        assert_eq!(hydrated.messages[0].id, "m1");
        assert_eq!(hydrated.messages[0].text, "hello");
        assert_eq!(hydrated.messages[0].time, "10:30");
        assert_eq!(hydrated.index.channels.get("开发/一般").map(String::as_str), Some("ch1"));
        assert_eq!(hydrated.index.members.get("开发/alice").map(String::as_str), Some("u1"));
    }

    /// P6 性能预算（可自动化部分）：会话重建 1000 条消息 ≤ 300ms。
    /// 冷启动与滚动流畅度受 GPUI 窗口环境限制，走 README 手测清单。
    fn message_dto(index: usize) -> serde_json::Value {
        serde_json::json!({
            "id": format!("m{index:06}"),
            "channel_id": "c1",
            "author": {"id": "u1", "name": "Nova", "color": 0x5865f2, "bot": false},
            "content": format!("第 {index} 条压测消息：包含一些中等长度的中文正文，用于模拟真实会话。"),
            "type": "text",
            "created_at": "2026-11-25T10:00:00Z",
            "pinned": false,
            "reactions": [],
            "attachments": []
        })
    }

    #[test]
    fn hydrate_1000_messages_under_budget() {
        let dtos: Vec<serde_json::Value> = (0..1000).map(message_dto).collect();

        let started = std::time::Instant::now();
        let messages: Vec<_> = dtos
            .iter()
            .map(|dto| {
                let parsed: super::super::dto::MessageDto = serde_json::from_value(dto.clone()).unwrap();
                super::message_from(&parsed)
            })
            .collect();
        let elapsed = started.elapsed();

        assert_eq!(messages.len(), 1000);
        assert!(
            elapsed.as_millis() < 300,
            "1000 条消息重建耗时 {elapsed:?}，超出 300ms 预算"
        );
    }

    #[test]
    fn parse_1000_message_json_under_budget() {
        let dtos: Vec<serde_json::Value> = (0..1000).map(message_dto).collect();
        let payload = serde_json::to_vec(&dtos).unwrap();

        let started = std::time::Instant::now();
        let parsed: Vec<super::super::dto::MessageDto> = serde_json::from_slice(&payload).unwrap();
        let elapsed = started.elapsed();

        assert_eq!(parsed.len(), 1000);
        assert!(
            elapsed.as_millis() < 300,
            "1000 条消息 JSON 解析耗时 {elapsed:?}，超出 300ms 预算"
        );
    }

    #[test]
    fn fallback_server_keeps_ui_index_safe() {
        let (server, members) = fallback_server("Alice");
        assert_eq!(server.name, "开始");
        assert_eq!(server.categories[0].channels[0].name, "欢迎");
        assert!(members.is_empty());
    }
}
