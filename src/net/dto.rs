//! 后端 DTO。字段与 `docs/backend-api.md` / FastAPI 输出保持 snake_case。
//!
//! C0 只定义登录与首屏需要的结构；其余域在 C2–C5 接线时继续补。
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ErrorBody {
    pub code: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub retry_after: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct HealthDto {
    pub status: String,
    #[serde(default)]
    pub db: bool,
    #[serde(default)]
    pub livekit: bool,
    #[serde(default)]
    pub gateway_connections: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AuthorDto {
    pub id: String,
    pub name: String,
    pub color: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub tag: String,
    pub display_name: String,
    pub avatar_color: u32,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ServerDto {
    pub id: String,
    pub name: String,
    pub icon_color: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub owner_id: String,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub member_count: u32,
    #[serde(default)]
    pub notify_level: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CategoryDto {
    pub id: String,
    pub server_id: String,
    pub name: String,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ChannelDto {
    pub id: String,
    pub server_id: String,
    pub category_id: Option<String>,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub notify_level: String,
    #[serde(default)]
    pub last_read_message_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ServerTreeDto {
    pub categories: Vec<CategoryDto>,
    pub channels: Vec<ChannelDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AttachmentDto {
    pub id: String,
    pub filename: String,
    pub mime: String,
    pub size: u64,
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MessageDto {
    pub id: String,
    pub channel_id: String,
    pub author: AuthorDto,
    #[serde(default)]
    pub content: String,
    #[serde(rename = "type", default = "default_message_type")]
    pub msg_type: String,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub client_nonce: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub edited_at: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub reactions: Vec<serde_json::Value>,
    #[serde(default)]
    pub attachments: Vec<AttachmentDto>,
    #[serde(default)]
    pub poll: Option<serde_json::Value>,
    #[serde(default)]
    pub warnings: Vec<serde_json::Value>,
}

fn default_message_type() -> String {
    "text".into()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct FriendDto {
    pub id: String,
    pub username: String,
    pub tag: String,
    pub display_name: String,
    pub avatar_color: u32,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct MemberDto {
    pub user_id: String,
    pub username: String,
    pub tag: String,
    pub display_name: String,
    #[serde(default)]
    pub nickname: String,
    pub avatar_color: u32,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub role_ids: Vec<String>,
    #[serde(default)]
    pub joined_at: String,
    #[serde(default)]
    pub is_owner: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RoleDto {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub color: u32,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub is_everyone: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PermissionItemDto {
    pub role_id: String,
    pub permission: String,
    pub state: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PermissionMatrixDto {
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub items: Vec<PermissionItemDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct FriendRequestUserDto {
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub avatar_color: u32,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct FriendRequestDto {
    pub id: String,
    pub from_user: FriendRequestUserDto,
    pub to_user: FriendRequestUserDto,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct FriendRequestPageDto {
    #[serde(default)]
    pub items: Vec<FriendRequestDto>,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub outgoing: Vec<FriendRequestDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct BlockDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct FriendGroupDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub count: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DmUserDto {
    pub id: String,
    pub username: String,
    pub tag: String,
    pub display_name: String,
    pub avatar_color: u32,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DmLastMessageDto {
    pub id: String,
    pub author_id: String,
    #[serde(default)]
    pub content: String,
    #[serde(rename = "type", default = "default_message_type")]
    pub msg_type: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DmDto {
    pub id: String,
    pub user: DmUserDto,
    #[serde(default)]
    pub last_message: Option<DmLastMessageDto>,
    #[serde(default)]
    pub unread: u32,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub has_draft: bool,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DmMessageDto {
    pub id: String,
    pub dm_id: String,
    pub author: AuthorDto,
    #[serde(default)]
    pub content: String,
    #[serde(rename = "type", default = "default_message_type")]
    pub msg_type: String,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub edited_at: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PageDto<T> {
    #[serde(default = "Vec::new")]
    pub items: Vec<T>,
    #[serde(default)]
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ChannelUnreadDto {
    pub channel_id: String,
    #[serde(default)]
    pub server_id: String,
    #[serde(default)]
    pub unread: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DmUnreadDto {
    pub dm_id: String,
    #[serde(default)]
    pub unread: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct UnreadCountsDto {
    #[serde(default)]
    pub mentions: u32,
    #[serde(default)]
    pub friend_requests: u32,
    #[serde(default)]
    pub notifications: u32,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub servers: Vec<serde_json::Value>,
    #[serde(default)]
    pub channels: Vec<ChannelUnreadDto>,
    #[serde(default)]
    pub dms: Vec<DmUnreadDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TokensDto {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: u64,
}

/// 登录/注册请求体，放在 DTO 模块便于 worker 直接序列化。
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RegisterDto {
    pub email: String,
    pub password: String,
    pub username: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RefreshDto {
    pub refresh_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_message_and_tree() {
        let message: MessageDto = serde_json::from_str(
            r#"{"id":"1","channel_id":"2","author":{"id":"3","name":"Alice","color":5865},
                "content":"hi","type":"text","pinned":false,"edited_at":null,
                "created_at":"2026-11-24T00:00:00Z","reactions":[],"attachments":[]}"#,
        )
        .unwrap();
        assert_eq!(message.id, "1");
        assert_eq!(message.msg_type, "text");
        assert_eq!(message.author.name, "Alice");

        let tree: ServerTreeDto = serde_json::from_str(
            r#"{"categories":[{"id":"c1","server_id":"s1","name":"文字","position":0,"collapsed":false}],
                "channels":[{"id":"ch1","server_id":"s1","category_id":"c1","name":"一般",
                "kind":"text","topic":"","position":0,"notify_level":"inherit",
                "last_read_message_id":null}]}"#,
        )
        .unwrap();
        assert_eq!(tree.categories.len(), 1);
        assert_eq!(tree.channels[0].name, "一般");
    }

    #[test]
    fn parses_tokens_and_error() {
        let tokens: TokensDto = serde_json::from_str(
            r#"{"access_token":"a","refresh_token":"r","token_type":"Bearer","expires_in":900}"#,
        )
        .unwrap();
        assert_eq!(tokens.expires_in, 900);

        let envelope: ErrorEnvelope =
            serde_json::from_str(r#"{"error":{"code":"CONFLICT","message":"重名"}}"#).unwrap();
        assert_eq!(envelope.error.code, "CONFLICT");
    }
}
