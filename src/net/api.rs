//! 按域分组的请求构造。UI 只调用这些函数，不直接拼 URL。
//!
//! C0 覆盖认证、用户、服务器、频道、消息、好友、私信的读路径与少量写路径；
//! 其余域在后续阶段继续补。
#![allow(dead_code)]

use serde_json::{json, Value};

use super::Method;

#[derive(Clone, Debug, PartialEq)]
pub struct ApiRequest {
    pub method: Method,
    pub path: String,
    pub body: Option<Value>,
}

impl ApiRequest {
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            body: None,
        }
    }

    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }
}

// ---------------------------------------------------------------------------
// 系统与认证
// ---------------------------------------------------------------------------

fn encode_component(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn search(q: &str, scope: &str, server_id: Option<&str>, limit: u32) -> ApiRequest {
    let mut path = format!(
        "/api/v1/search?q={}&scope={}&limit={limit}",
        encode_component(q),
        scope
    );
    if let Some(server_id) = server_id {
        path.push_str("&server_id=");
        path.push_str(server_id);
    }
    ApiRequest::new(Method::Get, path)
}

pub fn forum_posts(channel_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/channels/{channel_id}/posts"))
}

pub fn forum_post(post_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/posts/{post_id}"))
}

pub fn create_forum_post(channel_id: &str, title: &str, body: &str, tags: &[String]) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/channels/{channel_id}/posts")).with_body(json!({
        "title": title,
        "body": body,
        "tags": tags,
    }))
}

pub fn create_forum_reply(post_id: &str, body: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/posts/{post_id}/replies"))
        .with_body(json!({ "body": body }))
}

pub fn patch_forum_post(post_id: &str, pinned: Option<bool>, locked: Option<bool>) -> ApiRequest {
    let mut payload = serde_json::Map::new();
    if let Some(value) = pinned {
        payload.insert("pinned".into(), Value::from(value));
    }
    if let Some(value) = locked {
        payload.insert("locked".into(), Value::from(value));
    }
    ApiRequest::new(Method::Patch, format!("/api/v1/posts/{post_id}"))
        .with_body(Value::Object(payload))
}

pub fn create_poll(channel_id: &str, question: &str, options: &[String], multi: bool) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/channels/{channel_id}/polls")).with_body(json!({
        "question": question,
        "options": options,
        "multi": multi,
    }))
}

pub fn vote_poll(poll_id: &str, option_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/polls/{poll_id}/votes"))
        .with_body(json!({ "option_id": option_id }))
}

pub fn clear_poll_vote(poll_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/polls/{poll_id}/votes"))
}

pub fn close_poll(poll_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/polls/{poll_id}/close"))
}

pub fn create_thread(message_id: &str, title: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/messages/{message_id}/threads"))
        .with_body(json!({ "title": title }))
}

pub fn create_thread_reply(thread_id: &str, content: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/threads/{thread_id}/replies"))
        .with_body(json!({ "content": content }))
}

pub fn server_events(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/events"))
}

pub fn create_event(
    server_id: &str,
    title: &str,
    description: &str,
    category: &str,
    channel_id: Option<&str>,
    starts_at: &str,
    duration_minutes: u32,
) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/events")).with_body(json!({
        "title": title,
        "description": description,
        "category": category,
        "channel_id": channel_id,
        "starts_at": starts_at,
        "duration_minutes": duration_minutes,
    }))
}

pub fn delete_event(event_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/events/{event_id}"))
}

pub fn rsvp_event(event_id: &str, going: bool) -> ApiRequest {
    ApiRequest::new(Method::Put, format!("/api/v1/events/{event_id}/rsvp"))
        .with_body(json!({ "going": going }))
}

pub fn server_welcome(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/welcome"))
}

pub fn accept_welcome(server_id: &str, rule_agreed: bool, channels: &[String]) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/welcome/accept")).with_body(
        json!({ "rule_agreed": rule_agreed, "channels": channels }),
    )
}

pub fn server_boosts(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/boosts"))
}

pub fn toggle_boost(server_id: &str, active: bool) -> ApiRequest {
    ApiRequest::new(Method::Put, format!("/api/v1/servers/{server_id}/boosts/me"))
        .with_body(json!({ "active": active }))
}

pub fn notifications() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/notifications")
}

pub fn mark_notifications(ids: &[String]) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/notifications/read")
        .with_body(json!({ "ids": ids }))
}

pub fn mark_all_notifications() -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/notifications/read").with_body(json!({ "all": true }))
}

pub fn voice_join(channel_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Post,
        format!("/api/v1/channels/{channel_id}/voice/join"),
    )
}

pub fn voice_leave(channel_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Post,
        format!("/api/v1/channels/{channel_id}/voice/leave"),
    )
}

pub fn voice_participants(channel_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Get,
        format!("/api/v1/channels/{channel_id}/voice/participants"),
    )
}

pub fn voice_set_state(room: &str, mic: bool, camera: bool, screen: bool) -> ApiRequest {
    ApiRequest::new(
        Method::Put,
        format!("/api/v1/voice/{}/me", encode_component(room)),
    )
    .with_body(json!({ "mic": mic, "camera": camera, "screen": screen }))
}

pub fn server_commands(server_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Get,
        format!("/api/v1/servers/{server_id}/commands"),
    )
}

pub fn run_command(channel_id: &str, name: &str, args: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Post,
        format!(
            "/api/v1/channels/{channel_id}/commands/{}",
            encode_component(name)
        ),
    )
    .with_body(json!({ "args": args }))
}

pub fn drafts() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/me/drafts")
}

pub fn save_draft(scope: &str, content: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Put,
        format!("/api/v1/me/drafts/{}", encode_component(scope)),
    )
    .with_body(json!({ "content": content, "attachments": [] }))
}

pub fn bookmarks() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/bookmarks")
}

pub fn save_bookmark(message_id: &str, kind: &str) -> ApiRequest {
    ApiRequest::new(Method::Put, format!("/api/v1/bookmarks/{message_id}"))
        .with_body(json!({ "saved": true, "kind": kind }))
}

pub fn remove_bookmark(message_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Put, format!("/api/v1/bookmarks/{message_id}"))
        .with_body(json!({ "saved": false }))
}

pub fn automod_rules(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/automod/rules"))
}

pub fn create_automod_rule(server_id: &str, name: &str, action: &str, keywords: &[String]) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/automod/rules")).with_body(json!({
        "name": name,
        "action": action,
        "keywords": keywords,
    }))
}

pub fn patch_automod_rule(
    rule_id: &str,
    name: Option<&str>,
    action: Option<&str>,
    enabled: Option<bool>,
    keywords: Option<&[String]>,
) -> ApiRequest {
    let mut payload = serde_json::Map::new();
    if let Some(value) = name {
        payload.insert("name".into(), Value::String(value.into()));
    }
    if let Some(value) = action {
        payload.insert("action".into(), Value::String(value.into()));
    }
    if let Some(value) = enabled {
        payload.insert("enabled".into(), Value::from(value));
    }
    if let Some(value) = keywords {
        payload.insert("keywords".into(), Value::from(value));
    }
    ApiRequest::new(Method::Patch, format!("/api/v1/automod/rules/{rule_id}"))
        .with_body(Value::Object(payload))
}

pub fn delete_automod_rule(rule_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/automod/rules/{rule_id}"))
}

pub fn server_audit(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/audit"))
}

pub fn clear_audit(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/servers/{server_id}/audit"))
}

pub fn health() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/health")
}

pub fn login(email: &str, password: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/auth/login")
        .with_body(json!({ "email": email, "password": password }))
}

pub fn register(email: &str, password: &str, username: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/auth/register").with_body(json!({
        "email": email,
        "password": password,
        "username": username,
    }))
}

pub fn refresh(refresh_token: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/auth/refresh")
        .with_body(json!({ "refresh_token": refresh_token }))
}

pub fn logout(refresh_token: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/auth/logout")
        .with_body(json!({ "refresh_token": refresh_token }))
}

// ---------------------------------------------------------------------------
// 用户
// ---------------------------------------------------------------------------

pub fn me() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/users/@me")
}

pub fn update_me(display_name: Option<&str>, about: Option<&str>) -> ApiRequest {
    let mut body = serde_json::Map::new();
    if let Some(value) = display_name {
        body.insert("display_name".into(), Value::String(value.into()));
    }
    if let Some(value) = about {
        body.insert("about".into(), Value::String(value.into()));
    }
    ApiRequest::new(Method::Patch, "/api/v1/users/@me").with_body(Value::Object(body))
}

pub fn unread_counts() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/notifications/unread-counts")
}

// ---------------------------------------------------------------------------
// 服务器 / 频道
// ---------------------------------------------------------------------------

pub fn servers() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/servers")
}

pub fn server(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}"))
}

pub fn server_tree(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/channels"))
}

pub fn server_members(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/members"))
}

pub fn server_roles(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/roles"))
}

pub fn server_permissions(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/permissions"))
}

pub fn patch_server(
    server_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    icon_color: Option<u32>,
) -> ApiRequest {
    let mut body = serde_json::Map::new();
    if let Some(value) = name {
        body.insert("name".into(), Value::String(value.into()));
    }
    if let Some(value) = description {
        body.insert("description".into(), Value::String(value.into()));
    }
    if let Some(value) = icon_color {
        body.insert("icon_color".into(), Value::from(value));
    }
    ApiRequest::new(Method::Patch, format!("/api/v1/servers/{server_id}"))
        .with_body(Value::Object(body))
}

pub fn delete_server(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/servers/{server_id}"))
}

pub fn leave_server(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/leave"))
}

pub fn create_category(server_id: &str, name: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/categories"))
        .with_body(json!({ "name": name }))
}

pub fn patch_category(category_id: &str, name: &str) -> ApiRequest {
    ApiRequest::new(Method::Patch, format!("/api/v1/categories/{category_id}"))
        .with_body(json!({ "name": name }))
}

pub fn delete_category(category_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/categories/{category_id}"))
}

pub fn create_channel(
    server_id: &str,
    name: &str,
    kind: &str,
    category_id: Option<&str>,
    topic: &str,
) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/channels")).with_body(json!({
        "name": name,
        "kind": kind,
        "category_id": category_id,
        "topic": topic,
    }))
}

pub fn patch_channel(
    channel_id: &str,
    name: Option<&str>,
    topic: Option<&str>,
    category_id: Option<&str>,
) -> ApiRequest {
    let mut body = serde_json::Map::new();
    if let Some(value) = name {
        body.insert("name".into(), Value::String(value.into()));
    }
    if let Some(value) = topic {
        body.insert("topic".into(), Value::String(value.into()));
    }
    if let Some(value) = category_id {
        body.insert("category_id".into(), Value::String(value.into()));
    }
    ApiRequest::new(Method::Patch, format!("/api/v1/channels/{channel_id}"))
        .with_body(Value::Object(body))
}

pub fn set_channel_notify(channel_id: &str, level: &str) -> ApiRequest {
    ApiRequest::new(Method::Patch, format!("/api/v1/channels/{channel_id}/notify"))
        .with_body(json!({ "notify_level": level }))
}

pub fn set_server_notify(server_id: &str, level: &str) -> ApiRequest {
    ApiRequest::new(Method::Patch, format!("/api/v1/servers/{server_id}/notify"))
        .with_body(json!({ "notify_level": level }))
}

pub fn delete_channel(channel_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/channels/{channel_id}"))
}

pub fn server_invites(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/invites"))
}

pub fn create_invite(channel_id: &str, expiry: &str, max_uses: Option<u32>) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/channels/{channel_id}/invites")).with_body(
        json!({ "expiry": expiry, "max_uses": max_uses }),
    )
}

pub fn accept_invite(code: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/invites/{code}/accept"))
}

pub fn revoke_invite(code: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/invites/{code}"))
}

pub fn server_bans(server_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/servers/{server_id}/bans"))
}

pub fn create_role(server_id: &str, name: &str, color: u32) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/roles"))
        .with_body(json!({ "name": name, "color": color }))
}

pub fn patch_role(role_id: &str, name: Option<&str>, color: Option<u32>) -> ApiRequest {
    let mut body = serde_json::Map::new();
    if let Some(value) = name {
        body.insert("name".into(), Value::String(value.into()));
    }
    if let Some(value) = color {
        body.insert("color".into(), Value::from(value));
    }
    ApiRequest::new(Method::Patch, format!("/api/v1/roles/{role_id}"))
        .with_body(Value::Object(body))
}

pub fn delete_role(role_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/roles/{role_id}"))
}

pub fn assign_role(server_id: &str, user_id: &str, role_id: &str, assigned: bool) -> ApiRequest {
    ApiRequest::new(
        Method::Put,
        format!("/api/v1/servers/{server_id}/members/{user_id}/roles/{role_id}"),
    )
    .with_body(json!({ "assigned": assigned }))
}

pub fn patch_member(server_id: &str, user_id: &str, nickname: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Patch,
        format!("/api/v1/servers/{server_id}/members/{user_id}"),
    )
    .with_body(json!({ "nickname": nickname }))
}

pub fn kick_member(server_id: &str, user_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Delete,
        format!("/api/v1/servers/{server_id}/members/{user_id}"),
    )
}

pub fn ban_member(server_id: &str, user_id: &str, reason: &str, delete_days: u32) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/servers/{server_id}/bans")).with_body(json!({
        "user_id": user_id,
        "reason": reason,
        "delete_days": delete_days,
    }))
}

pub fn unban_member(server_id: &str, user_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Delete,
        format!("/api/v1/servers/{server_id}/bans/{user_id}"),
    )
}

pub fn write_permissions(server_id: &str, items: Value) -> ApiRequest {
    ApiRequest::new(Method::Put, format!("/api/v1/servers/{server_id}/permissions"))
        .with_body(json!({ "items": items }))
}

pub fn create_server(name: &str, template_key: Option<&str>, is_public: bool) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/servers").with_body(json!({
        "name": name,
        "template_key": template_key,
        "is_public": is_public,
    }))
}

// ---------------------------------------------------------------------------
// 好友 / 分组 / 屏蔽
// ---------------------------------------------------------------------------

pub fn friends() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/friends")
}

pub fn online_friends() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/friends/online")
}

pub fn friend_requests() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/friends/requests")
}

pub fn send_friend_request(username: &str, tag: &str, message: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/friends/requests").with_body(json!({
        "username": username,
        "tag": tag,
        "message": message,
    }))
}

pub fn accept_friend_request(request_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Post,
        format!("/api/v1/friends/requests/{request_id}/accept"),
    )
}

pub fn reject_friend_request(request_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Post,
        format!("/api/v1/friends/requests/{request_id}/reject"),
    )
}

pub fn delete_friend(user_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/friends/{user_id}"))
}

pub fn friend_groups() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/friend-groups")
}

pub fn create_friend_group(name: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/friend-groups").with_body(json!({ "name": name }))
}

pub fn rename_friend_group(group_id: &str, name: &str) -> ApiRequest {
    ApiRequest::new(Method::Patch, format!("/api/v1/friend-groups/{group_id}"))
        .with_body(json!({ "name": name }))
}

pub fn delete_friend_group(group_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/friend-groups/{group_id}"))
}

pub fn order_friend_groups(ids: &[String]) -> ApiRequest {
    ApiRequest::new(Method::Patch, "/api/v1/friend-groups/order").with_body(json!({ "ids": ids }))
}

pub fn move_friend(user_id: &str, group_id: Option<&str>) -> ApiRequest {
    ApiRequest::new(Method::Patch, format!("/api/v1/friends/{user_id}"))
        .with_body(json!({ "group_id": group_id }))
}

pub fn blocks() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/blocks")
}

pub fn block_user(user_id: &str, reason: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/blocks")
        .with_body(json!({ "user_id": user_id, "reason": reason }))
}

pub fn unblock_user(user_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/blocks/{user_id}"))
}

// ---------------------------------------------------------------------------
// 消息
// ---------------------------------------------------------------------------

pub fn messages(channel_id: &str, before: Option<&str>, limit: u32) -> ApiRequest {
    let mut path = format!("/api/v1/channels/{channel_id}/messages?limit={limit}");
    if let Some(before) = before {
        path.push_str("&before=");
        path.push_str(before);
    }
    ApiRequest::new(Method::Get, path)
}

pub fn message(message_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Get, format!("/api/v1/messages/{message_id}"))
}

pub fn send_message(
    channel_id: &str,
    content: &str,
    reply_to: Option<&str>,
    client_nonce: Option<&str>,
) -> ApiRequest {
    send_message_with_attachments(channel_id, content, reply_to, client_nonce, &[])
}

pub fn send_message_with_attachments(
    channel_id: &str,
    content: &str,
    reply_to: Option<&str>,
    client_nonce: Option<&str>,
    attachment_ids: &[String],
) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/channels/{channel_id}/messages")).with_body(
        json!({
            "content": content,
            "reply_to": reply_to,
            "attachment_ids": attachment_ids,
            "client_nonce": client_nonce,
        }),
    )
}

pub fn edit_message(message_id: &str, content: &str, version: Option<i32>) -> ApiRequest {
    ApiRequest::new(Method::Patch, format!("/api/v1/messages/{message_id}"))
        .with_body(json!({ "content": content, "version": version }))
}

pub fn pin_message(message_id: &str, pinned: bool) -> ApiRequest {
    ApiRequest::new(Method::Put, format!("/api/v1/messages/{message_id}/pin"))
        .with_body(json!({ "pinned": pinned }))
}

pub fn delete_message(message_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Delete, format!("/api/v1/messages/{message_id}"))
}

pub fn add_reaction(message_id: &str, emoji: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/messages/{message_id}/reactions"))
        .with_body(json!({ "emoji": emoji }))
}

pub fn remove_reaction(message_id: &str, emoji: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Delete,
        format!("/api/v1/messages/{message_id}/reactions/{emoji}"),
    )
}

pub fn mark_channel_read(channel_id: &str, message_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/channels/{channel_id}/read"))
        .with_body(json!({ "last_read_message_id": message_id }))
}

pub fn typing(channel_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/channels/{channel_id}/typing"))
}

// ---------------------------------------------------------------------------
// 私信
// ---------------------------------------------------------------------------

pub fn dms() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/dms")
}

pub fn open_dm(user_id: &str, first_message: Option<&str>) -> ApiRequest {
    ApiRequest::new(Method::Post, "/api/v1/dms")
        .with_body(json!({ "user_id": user_id, "first_message": first_message }))
}

pub fn dm_messages(dm_id: &str, before: Option<&str>, limit: u32) -> ApiRequest {
    let mut path = format!("/api/v1/dms/{dm_id}/messages?limit={limit}");
    if let Some(before) = before {
        path.push_str("&before=");
        path.push_str(before);
    }
    ApiRequest::new(Method::Get, path)
}

pub fn send_dm_message(dm_id: &str, content: &str, client_nonce: Option<&str>) -> ApiRequest {
    send_dm_message_with_reply(dm_id, content, None, client_nonce)
}

pub fn send_dm_message_with_reply(
    dm_id: &str,
    content: &str,
    reply_to: Option<&str>,
    client_nonce: Option<&str>,
) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/dms/{dm_id}/messages")).with_body(json!({
        "content": content,
        "reply_to": reply_to,
        "client_nonce": client_nonce,
    }))
}

pub fn start_dm_call(dm_id: &str, kind: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/dms/{dm_id}/call"))
        .with_body(json!({ "kind": kind }))
}

pub fn accept_dm_call(dm_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/dms/{dm_id}/call/accept"))
}

pub fn decline_dm_call(dm_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/dms/{dm_id}/call/decline"))
}

pub fn hangup_dm_call(dm_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Post, format!("/api/v1/dms/{dm_id}/call/hangup"))
}

pub fn mark_dm_read(dm_id: &str) -> ApiRequest {
    ApiRequest::new(Method::Patch, format!("/api/v1/dms/{dm_id}/read")).with_body(json!({}))
}

pub fn message_requests() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/message-requests?state=pending")
}

pub fn all_message_requests() -> ApiRequest {
    ApiRequest::new(Method::Get, "/api/v1/message-requests?state=all")
}

pub fn ignore_message_request(request_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Post,
        format!("/api/v1/message-requests/{request_id}/ignore"),
    )
}

pub fn delete_message_request(request_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Delete,
        format!("/api/v1/message-requests/{request_id}"),
    )
}

pub fn mark_channel_read_api(channel_id: &str, message_id: &str) -> ApiRequest {
    mark_channel_read(channel_id, message_id)
}

pub fn accept_message_request(request_id: &str) -> ApiRequest {
    ApiRequest::new(
        Method::Post,
        format!("/api/v1/message-requests/{request_id}/accept"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_message_cursor() {
        let request = messages("ch1", Some("900"), 50);
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/api/v1/channels/ch1/messages?limit=50&before=900");
        assert!(request.body.is_none());

        let request = dm_messages("dm1", None, 25);
        assert_eq!(request.path, "/api/v1/dms/dm1/messages?limit=25");
    }

    #[test]
    fn builds_auth_and_send_bodies() {
        let login = login("a@b.com", "pw");
        assert_eq!(login.path, "/api/v1/auth/login");
        assert_eq!(login.body.unwrap()["email"], "a@b.com");

        let send = send_message("ch1", "hi", None, Some("nonce"));
        assert_eq!(send.method, Method::Post);
        assert_eq!(send.body.unwrap()["client_nonce"], "nonce");

        let reaction = remove_reaction("m1", "👍");
        assert_eq!(reaction.path, "/api/v1/messages/m1/reactions/👍");
    }

    #[test]
    fn sends_reply_ids() {
        let channel = send_message("ch1", "hi", Some("m1"), Some("n1"));
        assert_eq!(channel.body.unwrap()["reply_to"], "m1");

        let with_attachment = send_message_with_attachments("ch1", "", Some("m1"), None, &["f1".into()]);
        let body = with_attachment.body.unwrap();
        assert_eq!(body["reply_to"], "m1");
        assert_eq!(body["attachment_ids"][0], "f1");

        let dm = send_dm_message_with_reply("d1", "hi", Some("m2"), Some("n2"));
        let body = dm.body.unwrap();
        assert_eq!(body["reply_to"], "m2");
        assert_eq!(body["client_nonce"], "n2");
    }

    #[test]
    fn builds_voice_requests() {
        let join = voice_join("ch1");
        assert_eq!(join.method, Method::Post);
        assert_eq!(join.path, "/api/v1/channels/ch1/voice/join");

        let leave = voice_leave("ch1");
        assert_eq!(leave.path, "/api/v1/channels/ch1/voice/leave");

        let state = voice_set_state("srv:s1:ch1", false, true, false);
        assert_eq!(state.method, Method::Put);
        assert_eq!(state.path, "/api/v1/voice/srv%3As1%3Ach1/me");
        let body = state.body.unwrap();
        assert_eq!(body["mic"], false);
        assert_eq!(body["camera"], true);
        assert_eq!(body["screen"], false);
    }

    #[test]
    fn builds_draft_requests() {
        let list = drafts();
        assert_eq!(list.path, "/api/v1/me/drafts");

        let save = save_draft("channel:ch1", "半截消息");
        assert_eq!(save.method, Method::Put);
        assert_eq!(save.path, "/api/v1/me/drafts/channel%3Ach1");
        assert_eq!(save.body.unwrap()["content"], "半截消息");
    }
}
