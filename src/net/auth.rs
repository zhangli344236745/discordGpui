//! 认证状态与 token 文件。
//!
//! access 只在内存保存；refresh 默认也在内存，“记住我”时写入 0600 文件。
//! 密码从不由本模块接触或持久化。

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::dto::TokensDto;
use super::{NetError, REFRESH_MARGIN};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

impl From<TokensDto> for Tokens {
    fn from(dto: TokensDto) -> Self {
        Self {
            access_token: dto.access_token,
            refresh_token: dto.refresh_token,
            token_type: if dto.token_type.is_empty() {
                "Bearer".into()
            } else {
                dto.token_type
            },
            expires_in: dto.expires_in,
        }
    }
}

#[derive(Clone, Debug)]
struct StoredTokens {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_at: Instant,
}

/// 线程之间共享的会话；worker 刷新，UI 只读快照。
#[derive(Default)]
pub struct SessionStore {
    inner: Mutex<Option<StoredTokens>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, tokens: Tokens) {
        let ttl = Duration::from_secs(tokens.expires_in.max(1));
        let stored = StoredTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            token_type: tokens.token_type,
            expires_at: Instant::now() + ttl,
        };
        *self.inner.lock().expect("session mutex poisoned") = Some(stored);
    }

    pub fn clear(&self) {
        *self.inner.lock().expect("session mutex poisoned") = None;
    }

    /// 只有 refresh token（启动自动登录）：access 置空、立即过期，等 worker 刷新。
    pub fn set_refresh_only(&self, refresh_token: String) {
        let stored = StoredTokens {
            access_token: String::new(),
            refresh_token,
            token_type: "Bearer".into(),
            expires_at: Instant::now(),
        };
        *self.inner.lock().expect("session mutex poisoned") = Some(stored);
    }

    pub fn is_logged_in(&self) -> bool {
        self.inner.lock().expect("session mutex poisoned").is_some()
    }

    pub fn access_token(&self) -> Option<String> {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .as_ref()
            .map(|tokens| tokens.access_token.clone())
    }

    pub fn refresh_token(&self) -> Option<String> {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .as_ref()
            .map(|tokens| tokens.refresh_token.clone())
    }

    pub fn needs_refresh(&self) -> bool {
        let guard = self.inner.lock().expect("session mutex poisoned");
        match guard.as_ref() {
            Some(tokens) => tokens.expires_at <= Instant::now() + REFRESH_MARGIN,
            None => true,
        }
    }

    pub fn snapshot(&self) -> Option<Tokens> {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .as_ref()
            .map(|tokens| Tokens {
                access_token: tokens.access_token.clone(),
                refresh_token: tokens.refresh_token.clone(),
                token_type: tokens.token_type.clone(),
                expires_in: 0,
            })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedSession {
    base: String,
    refresh_token: String,
}

/// 保存 refresh token；Unix 下文件权限设为 0600。
pub fn save_refresh_token(path: &Path, base: &str, refresh_token: &str) -> Result<(), NetError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| NetError::config(format!("无法创建会话目录：{error}")))?;
    }
    let payload = PersistedSession {
        base: base.to_string(),
        refresh_token: refresh_token.to_string(),
    };
    let json = serde_json::to_vec(&payload)
        .map_err(|error| NetError::config(format!("序列化会话失败：{error}")))?;
    let mut file = fs::File::create(path)
        .map_err(|error| NetError::config(format!("无法写入会话文件：{error}")))?;
    file.write_all(&json)
        .map_err(|error| NetError::config(format!("无法写入会话文件：{error}")))?;
    drop(file);
    restrict_permissions(path)?;
    Ok(())
}

/// 读取 refresh token；base 不同则视为无会话，避免把 token 发到别的实例。
pub fn load_refresh_token(path: &Path, base: &str) -> Option<String> {
    let raw = fs::read(path).ok()?;
    let session: PersistedSession = serde_json::from_slice(&raw).ok()?;
    (session.base == base).then_some(session.refresh_token)
}

pub fn clear_refresh_token(path: &Path) {
    let _ = fs::remove_file(path);
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), NetError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| NetError::config(format!("无法设置会话文件权限：{error}")))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), NetError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dgpui-{name}-{}-{stamp}.json", std::process::id()))
    }

    #[test]
    fn session_store_tracks_expiry() {
        let store = SessionStore::new();
        assert!(store.needs_refresh());
        store.set(Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            token_type: "Bearer".into(),
            expires_in: 900,
        });
        assert!(!store.needs_refresh());
        assert_eq!(store.access_token().as_deref(), Some("a"));
        store.clear();
        assert!(store.snapshot().is_none());
    }

    #[test]
    fn session_file_roundtrip_and_base_isolation() {
        let path = temp_path("session");
        save_refresh_token(&path, "http://127.0.0.1:8080", "refresh-1").unwrap();
        assert_eq!(
            load_refresh_token(&path, "http://127.0.0.1:8080").as_deref(),
            Some("refresh-1")
        );
        assert_eq!(load_refresh_token(&path, "http://other:8080"), None);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        clear_refresh_token(&path);
        assert!(!path.exists());
    }

    #[test]
    fn corrupt_session_file_is_ignored() {
        let path = temp_path("corrupt");
        fs::write(&path, b"not json").unwrap();
        assert_eq!(load_refresh_token(&path, "http://127.0.0.1:8080"), None);
        let _ = fs::remove_file(path);
    }
}
