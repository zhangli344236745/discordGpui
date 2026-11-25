//! 客户端网络底座：配置、REST worker、认证与 DTO。
//!
//! 所有 HTTP 调用都在专用线程执行，UI 侧只通过 `async-channel` 收发消息。
//! C0 阶段先提供类型与 worker，C1 起接入登录与启动同步。
#![allow(dead_code)] // C0 骨架：C1/C2 接线后逐项清理未使用告警

pub mod api;
pub mod auth;
pub mod dto;
pub mod hydrate;
pub mod worker;
pub mod ws;

pub use worker::NetClient;

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_API_BASE: &str = "http://127.0.0.1:8080";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub const REFRESH_MARGIN: Duration = Duration::from_secs(30);

/// 启动模式：在线走真实后端，离线只读本地种子/快照。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMode {
    Online,
    Demo,
}

/// HTTP 方法，避免在 UI 层依赖具体 HTTP 库的类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Patch,
    Put,
    Delete,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Patch => "PATCH",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetErrorKind {
    Config,
    Network,
    Timeout,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    Validation,
    RateLimited,
    Blocked,
    Server,
    Decode,
}

/// 统一的错误形态：后端返回 `{error:{code,message,field,request_id}}` 时尽量保留原始信息。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetError {
    pub kind: NetErrorKind,
    pub status: Option<u16>,
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub request_id: Option<String>,
    pub retry_after: Option<u64>,
}

impl NetError {
    pub fn new(kind: NetErrorKind, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            status: None,
            code: code.into(),
            message: message.into(),
            field: None,
            request_id: None,
            retry_after: None,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::new(NetErrorKind::Config, "CONFIG_ERROR", message)
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new(NetErrorKind::Network, "NETWORK_ERROR", message)
    }

    pub fn timeout() -> Self {
        Self::new(NetErrorKind::Timeout, "NETWORK_TIMEOUT", "请求超时，请重试")
    }

    pub fn decode(message: impl Into<String>) -> Self {
        Self::new(NetErrorKind::Decode, "DECODE_ERROR", message)
    }

    pub fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(NetErrorKind::Unauthorized, code, message)
    }

    /// 从后端的统一错误体解析；解析失败时按 HTTP 状态给出兜底文案。
    pub fn from_response(status: u16, body: &str) -> Self {
        let parsed: Option<dto::ErrorEnvelope> = serde_json::from_str(body).ok();
        let mut error = match parsed {
            Some(envelope) => Self {
                kind: kind_for(status, &envelope.error.code),
                status: Some(status),
                code: envelope.error.code,
                message: envelope.error.message,
                field: envelope.error.field,
                request_id: envelope.error.request_id,
                retry_after: envelope.error.retry_after,
            },
            None => Self {
                kind: kind_for(status, ""),
                status: Some(status),
                code: format!("HTTP_{status}"),
                message: fallback_message(status),
                field: None,
                request_id: None,
                retry_after: None,
            },
        };
        if error.status.is_none() {
            error.status = Some(status);
        }
        error
    }

    pub fn is_auth_expired(&self) -> bool {
        self.code == "TOKEN_EXPIRED" || (self.status == Some(401) && self.code != "UNAUTHORIZED")
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status == Some(401)
    }

    pub fn user_message(&self) -> String {
        match self.kind {
            NetErrorKind::RateLimited => match self.retry_after {
                Some(seconds) => format!("请求过于频繁，{} 秒后重试", seconds),
                None => "请求过于频繁，请稍后重试".into(),
            },
            NetErrorKind::Timeout => "网络超时，请重试".into(),
            NetErrorKind::Network => format!("连接失败：{}", self.message),
            NetErrorKind::Forbidden => self.message.clone(),
            NetErrorKind::Blocked => self.message.clone(),
            NetErrorKind::Unauthorized => {
                if self.code == "TOKEN_EXPIRED" {
                    "登录已过期，请重新登录".into()
                } else if self.message.is_empty() {
                    "邮箱或密码不正确".into()
                } else {
                    self.message.clone()
                }
            }
            _ => {
                if self.message.is_empty() {
                    "请求失败，请重试".into()
                } else {
                    self.message.clone()
                }
            }
        }
    }
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            Some(status) => write!(f, "[{} {}] {}", status, self.code, self.message),
            None => write!(f, "[{}] {}", self.code, self.message),
        }
    }
}

impl std::error::Error for NetError {}

fn kind_for(status: u16, code: &str) -> NetErrorKind {
    if code == "AUTOMOD_BLOCKED" {
        return NetErrorKind::Blocked;
    }
    match status {
        401 => NetErrorKind::Unauthorized,
        403 => NetErrorKind::Forbidden,
        404 => NetErrorKind::NotFound,
        409 => NetErrorKind::Conflict,
        422 => NetErrorKind::Validation,
        429 => NetErrorKind::RateLimited,
        500..=599 => NetErrorKind::Server,
        _ => NetErrorKind::Validation,
    }
}

fn fallback_message(status: u16) -> String {
    match status {
        401 => "登录已过期".into(),
        403 => "没有权限执行该操作".into(),
        404 => "资源不存在".into(),
        409 => "操作冲突，请刷新后重试".into(),
        422 => "请求内容不合法".into(),
        429 => "请求过于频繁".into(),
        500..=599 => "服务端异常".into(),
        _ => format!("请求失败（HTTP {status}）"),
    }
}

/// 传输层请求；worker 只依赖这个结构，测试可以注入 mock transport。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawResponse {
    pub status: u16,
    pub body: String,
}

pub trait Transport: Send + Sync {
    fn execute(&self, request: &HttpRequest) -> Result<RawResponse, NetError>;
}

/// 网络配置：地址、超时、离线开关与会话文件位置。
#[derive(Clone, Debug)]
pub struct NetConfig {
    pub base: String,
    pub timeout: Duration,
    pub demo: bool,
    pub session_file: PathBuf,
}

impl NetConfig {
    pub fn new(base: impl Into<String>, timeout: Duration, demo: bool, session_file: PathBuf) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            timeout,
            demo,
            session_file,
        }
    }

    /// 从环境变量读取；`DISCORD_API_BASE` 非法时返回 Config 错误，由调用方回退 Demo。
    pub fn from_env() -> Result<Self, NetError> {
        let base = std::env::var("DISCORD_API_BASE").unwrap_or_else(|_| DEFAULT_API_BASE.to_string());
        let base = validate_base(&base)?;
        let timeout = match std::env::var("DISCORD_HTTP_TIMEOUT") {
            Ok(raw) => parse_timeout(&raw)?,
            Err(_) => DEFAULT_TIMEOUT,
        };
        let demo = match std::env::var("DISCORD_DEMO") {
            Ok(raw) => !matches!(raw.trim().to_ascii_lowercase().as_str(), "" | "0" | "false" | "no"),
            Err(_) => false,
        };
        Ok(Self::new(base, timeout, demo, default_session_file()))
    }

    pub fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else if path.starts_with('/') {
            format!("{}{}", self.base, path)
        } else {
            format!("{}/{}", self.base, path)
        }
    }
}

pub fn validate_base(raw: &str) -> Result<String, NetError> {
    let value = raw.trim().trim_end_matches('/');
    let lower = value.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) || lower.len() <= 8 {
        return Err(NetError::config("API 地址必须以 http:// 或 https:// 开头"));
    }
    if value.contains(char::is_whitespace) {
        return Err(NetError::config("API 地址不能包含空格"));
    }
    Ok(value.to_string())
}

pub fn parse_timeout(raw: &str) -> Result<Duration, NetError> {
    let seconds: u64 = raw
        .trim()
        .parse()
        .map_err(|_| NetError::config("DISCORD_HTTP_TIMEOUT 必须是整数秒"))?;
    if !(1..=120).contains(&seconds) {
        return Err(NetError::config("DISCORD_HTTP_TIMEOUT 必须在 1..=120 秒之间"));
    }
    Ok(Duration::from_secs(seconds))
}

fn default_session_file() -> PathBuf {
    if let Ok(path) = std::env::var("DISCORD_SESSION_FILE") {
        return PathBuf::from(path);
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("discord-gpui").join("session.json");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("discord-gpui")
            .join("session.json");
    }
    PathBuf::from(".discord-gpui-session.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_base_url() {
        assert_eq!(validate_base("http://127.0.0.1:8080/").unwrap(), "http://127.0.0.1:8080");
        assert_eq!(validate_base(" https://api.example.com ").unwrap(), "https://api.example.com");
        assert!(validate_base("ftp://example.com").is_err());
        assert!(validate_base("http://").is_err());
        assert!(validate_base("http://a b.com").is_err());
    }

    #[test]
    fn parses_timeout_bounds() {
        assert_eq!(parse_timeout("10").unwrap(), Duration::from_secs(10));
        assert!(parse_timeout("0").is_err());
        assert!(parse_timeout("121").is_err());
        assert!(parse_timeout("abc").is_err());
    }

    #[test]
    fn maps_error_body() {
        let body = r#"{"error":{"code":"MISSING_PERMISSION","message":"缺少 MANAGE_ROLES","field":"permission","request_id":"req_1"}}"#;
        let error = NetError::from_response(403, body);
        assert_eq!(error.kind, NetErrorKind::Forbidden);
        assert_eq!(error.code, "MISSING_PERMISSION");
        assert_eq!(error.request_id.as_deref(), Some("req_1"));

        let limited = NetError::from_response(
            429,
            r#"{"error":{"code":"RATE_LIMITED","message":"慢点","retry_after":7}}"#,
        );
        assert_eq!(limited.kind, NetErrorKind::RateLimited);
        assert_eq!(limited.retry_after, Some(7));
        assert!(limited.user_message().contains('7'));

        let expired = NetError::from_response(401, r#"{"error":{"code":"TOKEN_EXPIRED","message":"过期"}}"#);
        assert!(expired.is_auth_expired());

        let blocked = NetError::from_response(422, r#"{"error":{"code":"AUTOMOD_BLOCKED","message":"拦截"}}"#);
        assert_eq!(blocked.kind, NetErrorKind::Blocked);
    }

    #[test]
    fn falls_back_for_non_json_error() {
        let error = NetError::from_response(500, "boom");
        assert_eq!(error.kind, NetErrorKind::Server);
        assert_eq!(error.code, "HTTP_500");
        assert!(error.message.contains("服务端"));
    }

    #[test]
    fn builds_urls() {
        let config = NetConfig::new(
            "http://127.0.0.1:8080/",
            DEFAULT_TIMEOUT,
            false,
            PathBuf::from("/tmp/s.json"),
        );
        assert_eq!(config.url("/api/v1/health"), "http://127.0.0.1:8080/api/v1/health");
        assert_eq!(config.url("api/v1/health"), "http://127.0.0.1:8080/api/v1/health");
    }
}
