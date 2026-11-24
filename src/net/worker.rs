//! REST worker：专用线程串行执行请求，UI 通过 async-channel 等待结果。
//!
//! 串行模型让 token 刷新天然只发生一次：第一个过期请求刷新后，后续请求看到
//! 新 token 不再刷新。HTTP 传输通过 `Transport` 抽象，测试可注入 mock。

use std::sync::Arc;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use async_channel::{Receiver, Sender, bounded};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::api::ApiRequest;
use super::auth::{self, SessionStore, Tokens};
use super::dto::{LoginDto, RefreshDto, RegisterDto, TokensDto};
use super::{
    AppMode, HttpRequest, Method, NetConfig, NetError, NetErrorKind, RawResponse, Transport,
};

type Reply<T> = Sender<Result<T, NetError>>;

enum Command {
    Http {
        request: ApiRequest,
        auth: bool,
        reply: Reply<Value>,
    },
    Login {
        email: String,
        password: String,
        remember: bool,
        reply: Reply<Tokens>,
    },
    Register {
        email: String,
        password: String,
        username: String,
        remember: bool,
        reply: Reply<Tokens>,
    },
    Refresh {
        reply: Reply<()>,
    },
    Logout {
        reply: Reply<()>,
    },
    Shutdown,
}

/// 客户端持有的网络句柄；Drop 时会关闭 worker。
pub struct NetClient {
    config: NetConfig,
    store: Arc<SessionStore>,
    tx: Sender<Command>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl NetClient {
    pub fn spawn(config: NetConfig) -> Self {
        let transport: Box<dyn Transport> = Box::new(UreqTransport::new(config.timeout));
        Self::spawn_with_transport(config, transport)
    }

    /// 测试入口：注入自定义 Transport（例如 mock）。
    pub fn spawn_with_transport(config: NetConfig, transport: Box<dyn Transport>) -> Self {
        let store = Arc::new(SessionStore::new());
        let (tx, rx) = async_channel::unbounded();
        let worker_store = store.clone();
        let worker_config = config.clone();
        let join = thread::Builder::new()
            .name("discord-net".into())
            .spawn(move || {
                let worker = Worker {
                    config: worker_config,
                    store: worker_store,
                    transport,
                    remember: false,
                };
                worker.run(rx);
            })
            .expect("failed to spawn net worker");
        Self {
            config,
            store,
            tx,
            join: Mutex::new(Some(join)),
        }
    }

    pub fn config(&self) -> &NetConfig {
        &self.config
    }

    pub fn mode(&self) -> AppMode {
        if self.config.demo {
            AppMode::Demo
        } else {
            AppMode::Online
        }
    }

    pub fn session(&self) -> Arc<SessionStore> {
        self.store.clone()
    }

    pub fn is_logged_in(&self) -> bool {
        self.store.is_logged_in()
    }

    pub fn tokens(&self) -> Option<Tokens> {
        self.store.snapshot()
    }

    pub async fn login(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
        remember: bool,
    ) -> Result<Tokens, NetError> {
        let (tx, rx) = bounded(1);
        self.tx
            .send(Command::Login {
                email: email.into(),
                password: password.into(),
                remember,
                reply: tx,
            })
            .await
            .map_err(|_| NetError::network("网络线程已退出"))?;
        rx.recv().await.map_err(|_| NetError::network("网络线程已退出"))?
    }

    pub async fn register(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
        username: impl Into<String>,
        remember: bool,
    ) -> Result<Tokens, NetError> {
        let (tx, rx) = bounded(1);
        self.tx
            .send(Command::Register {
                email: email.into(),
                password: password.into(),
                username: username.into(),
                remember,
                reply: tx,
            })
            .await
            .map_err(|_| NetError::network("网络线程已退出"))?;
        rx.recv().await.map_err(|_| NetError::network("网络线程已退出"))?
    }

    pub async fn refresh(&self) -> Result<(), NetError> {
        let (tx, rx) = bounded(1);
        self.tx
            .send(Command::Refresh { reply: tx })
            .await
            .map_err(|_| NetError::network("网络线程已退出"))?;
        rx.recv().await.map_err(|_| NetError::network("网络线程已退出"))?
    }

    pub async fn logout(&self) -> Result<(), NetError> {
        let (tx, rx) = bounded(1);
        self.tx
            .send(Command::Logout { reply: tx })
            .await
            .map_err(|_| NetError::network("网络线程已退出"))?;
        rx.recv().await.map_err(|_| NetError::network("网络线程已退出"))?
    }

    /// 执行一个 typed 请求；`auth=true` 自动附带并刷新 access token。
    pub async fn execute(&self, request: ApiRequest, auth: bool) -> Result<Value, NetError> {
        let (tx, rx) = bounded(1);
        self.tx
            .send(Command::Http {
                request,
                auth,
                reply: tx,
            })
            .await
            .map_err(|_| NetError::network("网络线程已退出"))?;
        rx.recv().await.map_err(|_| NetError::network("网络线程已退出"))?
    }

    pub async fn execute_as<T: DeserializeOwned>(
        &self,
        request: ApiRequest,
        auth: bool,
    ) -> Result<T, NetError> {
        let value = self.execute(request, auth).await?;
        serde_json::from_value(value).map_err(|error| NetError::decode(error.to_string()))
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send_blocking(Command::Shutdown);
        if let Ok(mut guard) = self.join.lock()
            && let Some(handle) = guard.take()
        {
            let _ = handle.join();
        }
    }
}

impl Drop for NetClient {
    fn drop(&mut self) {
        let _ = self.tx.send_blocking(Command::Shutdown);
        if let Ok(mut guard) = self.join.lock()
            && let Some(handle) = guard.take()
        {
            let _ = handle.join();
        }
    }
}

struct Worker {
    config: NetConfig,
    store: Arc<SessionStore>,
    transport: Box<dyn Transport>,
    remember: bool,
}

impl Worker {
    fn run(mut self, rx: Receiver<Command>) {
        while let Ok(command) = rx.recv_blocking() {
            match command {
                Command::Http {
                    request,
                    auth,
                    reply,
                } => {
                    let result = self.handle_http(&request, auth);
                    let _ = reply.send_blocking(result);
                }
                Command::Login {
                    email,
                    password,
                    remember,
                    reply,
                } => {
                    let result = self.login(&email, &password, remember);
                    let _ = reply.send_blocking(result);
                }
                Command::Register {
                    email,
                    password,
                    username,
                    remember,
                    reply,
                } => {
                    let result = self.register(&email, &password, &username, remember);
                    let _ = reply.send_blocking(result);
                }
                Command::Refresh { reply } => {
                    let result = self.refresh();
                    let _ = reply.send_blocking(result);
                }
                Command::Logout { reply } => {
                    let result = self.logout();
                    let _ = reply.send_blocking(result);
                }
                Command::Shutdown => break,
            }
        }
    }

    fn handle_http(&mut self, request: &ApiRequest, auth: bool) -> Result<Value, NetError> {
        let raw = self.send(request, auth)?;
        if auth && raw.status == 401 {
            let error = NetError::from_response(raw.status, &raw.body);
            if error.is_auth_expired() {
                self.refresh()?;
                let retried = self.send(request, true)?;
                return decode_response(retried);
            }
            self.store.clear();
            if self.remember {
                auth::clear_refresh_token(&self.config.session_file);
            }
            return Err(error);
        }
        decode_response(raw)
    }

    fn send(&mut self, request: &ApiRequest, auth: bool) -> Result<RawResponse, NetError> {
        let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
        if auth {
            self.ensure_access_token()?;
            let token = self
                .store
                .access_token()
                .ok_or_else(|| NetError::unauthorized("UNAUTHORIZED", "未登录"))?;
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        let body = request
            .body
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| NetError::decode(format!("请求序列化失败：{error}")))?;
        if body.is_some() {
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }
        self.transport.execute(&HttpRequest {
            method: request.method,
            url: self.config.url(&request.path),
            headers,
            body,
        })
    }

    fn ensure_access_token(&mut self) -> Result<(), NetError> {
        if !self.store.is_logged_in() {
            return Err(NetError::unauthorized("UNAUTHORIZED", "未登录"));
        }
        if self.store.needs_refresh() {
            self.refresh()?;
        }
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), NetError> {
        let refresh_token = self
            .store
            .refresh_token()
            .ok_or_else(|| NetError::unauthorized("TOKEN_EXPIRED", "登录已过期，请重新登录"))?;
        let body = serde_json::to_string(&RefreshDto {
            refresh_token: refresh_token.clone(),
        })
        .map_err(|error| NetError::decode(format!("请求序列化失败：{error}")))?;
        let raw = self.transport.execute(&HttpRequest {
            method: Method::Post,
            url: self.config.url("/api/v1/auth/refresh"),
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(body),
        })?;
        if !(200..300).contains(&raw.status) {
            self.store.clear();
            auth::clear_refresh_token(&self.config.session_file);
            return Err(NetError::from_response(raw.status, &raw.body));
        }
        let dto: TokensDto = serde_json::from_str(&raw.body)
            .map_err(|error| NetError::decode(format!("刷新响应解析失败：{error}")))?;
        let tokens = Tokens::from(dto);
        self.store.set(tokens.clone());
        if self.remember {
            let _ = auth::save_refresh_token(
                &self.config.session_file,
                &self.config.base,
                &tokens.refresh_token,
            );
        }
        Ok(())
    }

    fn login(&mut self, email: &str, password: &str, remember: bool) -> Result<Tokens, NetError> {
        if email.trim().is_empty() || password.is_empty() {
            return Err(NetError::new(
                NetErrorKind::Validation,
                "VALIDATION_ERROR",
                "请输入邮箱和密码",
            ));
        }
        let body = serde_json::to_string(&LoginDto {
            email: email.trim().to_string(),
            password: password.to_string(),
        })
        .map_err(|error| NetError::decode(format!("请求序列化失败：{error}")))?;
        let raw = self.transport.execute(&HttpRequest {
            method: Method::Post,
            url: self.config.url("/api/v1/auth/login"),
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(body),
        })?;
        if !(200..300).contains(&raw.status) {
            return Err(NetError::from_response(raw.status, &raw.body));
        }
        let dto: TokensDto = serde_json::from_str(&raw.body)
            .map_err(|error| NetError::decode(format!("登录响应解析失败：{error}")))?;
        let tokens = Tokens::from(dto);
        self.store.set(tokens.clone());
        self.remember = remember;
        if remember {
            auth::save_refresh_token(
                &self.config.session_file,
                &self.config.base,
                &tokens.refresh_token,
            )?;
        } else {
            auth::clear_refresh_token(&self.config.session_file);
        }
        Ok(tokens)
    }

    fn register(
        &mut self,
        email: &str,
        password: &str,
        username: &str,
        remember: bool,
    ) -> Result<Tokens, NetError> {
        if email.trim().is_empty() || password.len() < 8 || username.trim().len() < 2 {
            return Err(NetError::new(
                NetErrorKind::Validation,
                "VALIDATION_ERROR",
                "请填写有效邮箱、至少 8 位密码和用户名",
            ));
        }
        let body = serde_json::to_string(&RegisterDto {
            email: email.trim().to_string(),
            password: password.to_string(),
            username: username.trim().to_string(),
        })
        .map_err(|error| NetError::decode(format!("请求序列化失败：{error}")))?;
        let raw = self.transport.execute(&HttpRequest {
            method: Method::Post,
            url: self.config.url("/api/v1/auth/register"),
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(body),
        })?;
        if !(200..300).contains(&raw.status) {
            return Err(NetError::from_response(raw.status, &raw.body));
        }
        let dto: TokensDto = serde_json::from_str(&raw.body)
            .map_err(|error| NetError::decode(format!("注册响应解析失败：{error}")))?;
        let tokens = Tokens::from(dto);
        self.store.set(tokens.clone());
        self.remember = remember;
        if remember {
            auth::save_refresh_token(
                &self.config.session_file,
                &self.config.base,
                &tokens.refresh_token,
            )?;
        } else {
            auth::clear_refresh_token(&self.config.session_file);
        }
        Ok(tokens)
    }

    fn logout(&mut self) -> Result<(), NetError> {
        if let Some(refresh_token) = self.store.refresh_token() {
            if let Ok(body) = serde_json::to_string(&RefreshDto { refresh_token }) {
                let _ = self.transport.execute(&HttpRequest {
                    method: Method::Post,
                    url: self.config.url("/api/v1/auth/logout"),
                    headers: vec![
                        ("Accept".to_string(), "application/json".to_string()),
                        ("Content-Type".to_string(), "application/json".to_string()),
                    ],
                    body: Some(body),
                });
            }
        }
        self.store.clear();
        auth::clear_refresh_token(&self.config.session_file);
        self.remember = false;
        Ok(())
    }
}

fn decode_response(raw: RawResponse) -> Result<Value, NetError> {
    if !(200..300).contains(&raw.status) {
        return Err(NetError::from_response(raw.status, &raw.body));
    }
    if raw.body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&raw.body)
        .map_err(|error| NetError::decode(format!("响应不是合法 JSON：{error}")))
}

/// 生产传输：ureq 的阻塞调用，只在 net 线程使用。
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    pub fn new(timeout: Duration) -> Self {
        Self {
            agent: ureq::AgentBuilder::new().timeout(timeout).build(),
        }
    }
}

impl Transport for UreqTransport {
    fn execute(&self, request: &HttpRequest) -> Result<RawResponse, NetError> {
        let mut outgoing = self.agent.request(request.method.as_str(), &request.url);
        for (key, value) in &request.headers {
            outgoing = outgoing.set(key, value);
        }
        let result = match &request.body {
            Some(body) => outgoing.send_string(body),
            None => outgoing.call(),
        };
        match result {
            Ok(response) => read_response(response),
            Err(ureq::Error::Status(_status, response)) => read_response(response),
            Err(ureq::Error::Transport(error)) => {
                let message = error.to_string();
                if message.to_ascii_lowercase().contains("timed out")
                    || message.to_ascii_lowercase().contains("timeout")
                {
                    Err(NetError::timeout())
                } else {
                    Err(NetError::network(message))
                }
            }
        }
    }
}

fn read_response(response: ureq::Response) -> Result<RawResponse, NetError> {
    let status = response.status();
    let body = response
        .into_string()
        .map_err(|error| NetError::network(format!("读取响应失败：{error}")))?;
    Ok(RawResponse { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct MockTransport {
        requests: Mutex<Vec<HttpRequest>>,
        responses: Mutex<VecDeque<Result<RawResponse, NetError>>>,
    }

    impl MockTransport {
        fn new(responses: Vec<Result<RawResponse, NetError>>) -> Arc<Self> {
            Arc::new(Self {
                requests: Mutex::new(Vec::new()),
                responses: Mutex::new(responses.into()),
            })
        }
    }

    impl Transport for Arc<MockTransport> {
        fn execute(&self, request: &HttpRequest) -> Result<RawResponse, NetError> {
            self.requests.lock().unwrap().push(request.clone());
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err(NetError::network("mock exhausted")))
        }
    }

    fn config() -> NetConfig {
        NetConfig::new(
            "http://127.0.0.1:8080",
            Duration::from_secs(1),
            false,
            PathBuf::from("/tmp/dgpui-test-session.json"),
        )
    }

    fn ok(body: &str) -> Result<RawResponse, NetError> {
        Ok(RawResponse {
            status: 200,
            body: body.to_string(),
        })
    }

    fn status(code: u16, body: &str) -> Result<RawResponse, NetError> {
        Ok(RawResponse {
            status: code,
            body: body.to_string(),
        })
    }

    fn tokens_json(access: &str, refresh: &str, expires_in: u64) -> String {
        format!(
            r#"{{"access_token":"{access}","refresh_token":"{refresh}","token_type":"Bearer","expires_in":{expires_in}}}"#
        )
    }

    #[test]
    fn executes_json_and_reports_decode_errors() {
        let mock = MockTransport::new(vec![ok(r#"{"status":"ok"}"#), ok("not json")]);
        let client = NetClient::spawn_with_transport(config(), Box::new(mock.clone()));
        let value = pollster::block_on(client.execute(super::super::api::health(), false)).unwrap();
        assert_eq!(value["status"], "ok");

        let error = pollster::block_on(client.execute(super::super::api::health(), false)).unwrap_err();
        assert_eq!(error.kind, NetErrorKind::Decode);
        assert_eq!(mock.requests.lock().unwrap().len(), 2);
        client.shutdown();
    }

    #[test]
    fn propagates_timeout_and_http_errors() {
        let mock = MockTransport::new(vec![
            Err(NetError::timeout()),
            status(403, r#"{"error":{"code":"MISSING_PERMISSION","message":"缺少 X"}}"#),
        ]);
        let client = NetClient::spawn_with_transport(config(), Box::new(mock.clone()));
        let timeout = pollster::block_on(client.execute(super::super::api::health(), false)).unwrap_err();
        assert_eq!(timeout.kind, NetErrorKind::Timeout);

        let denied = pollster::block_on(client.execute(super::super::api::health(), false)).unwrap_err();
        assert_eq!(denied.kind, NetErrorKind::Forbidden);
        client.shutdown();
    }

    #[test]
    fn refreshes_expired_token_once_before_request() {
        let mock = MockTransport::new(vec![
            ok(&tokens_json("access-2", "refresh-2", 900)),
            ok(r#"{"ok":true}"#),
            ok(r#"{"ok":true}"#),
        ]);
        let client = NetClient::spawn_with_transport(config(), Box::new(mock.clone()));
        client.session().set(Tokens {
            access_token: "access-1".into(),
            refresh_token: "refresh-1".into(),
            token_type: "Bearer".into(),
            expires_in: 1,
        });

        for _ in 0..2 {
            pollster::block_on(client.execute(super::super::api::me(), true)).unwrap();
        }

        let requests = mock.requests.lock().unwrap();
        assert_eq!(requests[0].url, "http://127.0.0.1:8080/api/v1/auth/refresh");
        assert!(requests[1].headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer access-2"));
        assert!(requests[2].headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer access-2"));
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.url.ends_with("/auth/refresh"))
                .count(),
            1
        );
        client.shutdown();
    }

    #[test]
    fn retries_once_on_token_expired_response() {
        let mock = MockTransport::new(vec![
            status(401, r#"{"error":{"code":"TOKEN_EXPIRED","message":"过期"}}"#),
            ok(&tokens_json("access-2", "refresh-2", 900)),
            ok(r#"{"id":"u1","username":"a","tag":"0001","display_name":"A","avatar_color":1}"#),
        ]);
        let client = NetClient::spawn_with_transport(config(), Box::new(mock.clone()));
        client.session().set(Tokens {
            access_token: "access-1".into(),
            refresh_token: "refresh-1".into(),
            token_type: "Bearer".into(),
            expires_in: 900,
        });
        let value = pollster::block_on(client.execute(super::super::api::me(), true)).unwrap();
        assert_eq!(value["id"], "u1");
        let requests = mock.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[1].url.ends_with("/auth/refresh"));
        client.shutdown();
    }

    #[test]
    fn concurrent_expired_requests_refresh_once() {
        let mock = MockTransport::new(vec![
            ok(&tokens_json("access-2", "refresh-2", 900)),
            ok(r#"{"ok":1}"#),
            ok(r#"{"ok":2}"#),
        ]);
        let client = Arc::new(NetClient::spawn_with_transport(config(), Box::new(mock.clone())));
        client.session().set(Tokens {
            access_token: "access-1".into(),
            refresh_token: "refresh-1".into(),
            token_type: "Bearer".into(),
            expires_in: 1,
        });

        let handles: Vec<_> = (0..2)
            .map(|_| {
                let client = client.clone();
                std::thread::spawn(move || {
                    pollster::block_on(client.execute(super::super::api::me(), true))
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        let requests = mock.requests.lock().unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.url.ends_with("/auth/refresh"))
                .count(),
            1
        );
    }

    #[test]
    fn clears_session_when_refresh_fails() {
        let mock = MockTransport::new(vec![status(
            401,
            r#"{"error":{"code":"UNAUTHORIZED","message":"失效"}}"#,
        )]);
        let client = NetClient::spawn_with_transport(config(), Box::new(mock.clone()));
        client.session().set(Tokens {
            access_token: "access-1".into(),
            refresh_token: "refresh-1".into(),
            token_type: "Bearer".into(),
            expires_in: 1,
        });
        let error = pollster::block_on(client.execute(super::super::api::me(), true)).unwrap_err();
        assert_eq!(error.kind, NetErrorKind::Unauthorized);
        assert!(!client.is_logged_in());
        client.shutdown();
    }
}
