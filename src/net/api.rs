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
    ApiRequest::new(Method::Post, format!("/api/v1/channels/{channel_id}/messages")).with_body(
        json!({
            "content": content,
            "reply_to": reply_to,
            "attachment_ids": [],
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
    ApiRequest::new(Method::Post, format!("/api/v1/dms/{dm_id}/messages")).with_body(json!({
        "content": content,
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
}
