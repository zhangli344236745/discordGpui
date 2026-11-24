//! Gateway WebSocket 线程：连接、心跳、退避重连与 ?since 补偿。
//!
//! 线程只负责网络与 JSON 解析，事件通过 async-channel 交给 UI reducer。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use async_channel::Sender;
use serde_json::Value;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message as WsMessage, WebSocket};

use super::NetConfig;
use super::auth::SessionStore;

#[derive(Clone, Debug)]
pub enum NetEvent {
    Connected,
    Gateway {
        event: String,
        id: String,
        data: Value,
    },
    Disconnected {
        reason: String,
    },
    NeedAuth,
}

pub fn is_duplicate(last: Option<&str>, id: &str) -> bool {
    !id.is_empty() && last == Some(id)
}

pub struct WsClient {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl WsClient {
    pub fn start(
        config: NetConfig,
        store: Arc<SessionStore>,
        events: Sender<NetEvent>,
        since: Option<String>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let join = thread::Builder::new()
            .name("discord-ws".into())
            .spawn(move || run(config, store, events, since, worker_stop))
            .expect("failed to spawn ws thread");
        Self {
            stop,
            join: Some(join),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WsClient {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(
    config: NetConfig,
    store: Arc<SessionStore>,
    events: Sender<NetEvent>,
    mut since: Option<String>,
    stop: Arc<AtomicBool>,
) {
    let mut delay = 1u64;
    while !stop.load(Ordering::Relaxed) {
        let Some(token) = store.access_token() else {
            let _ = events.send_blocking(NetEvent::NeedAuth);
            return;
        };
        let url = gateway_url(&config.base, &token, since.as_deref());
        match tungstenite::connect(&url) {
            Ok((mut socket, _response)) => {
                let _ = events.send_blocking(NetEvent::Connected);
                delay = 1;
                configure_read_timeout(&mut socket);
                let mut last_ping = Instant::now();
                loop {
                    if stop.load(Ordering::Relaxed) {
                        let _ = socket.close(None);
                        return;
                    }
                    match socket.read() {
                        Ok(WsMessage::Text(text)) => {
                            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                let event = value
                                    .get("event")
                                    .and_then(|item| item.as_str())
                                    .unwrap_or("UNKNOWN")
                                    .to_string();
                                let id = value
                                    .get("id")
                                    .and_then(|item| item.as_str())
                                    .unwrap_or_default()
                                    .to_string();
                                if !id.is_empty() {
                                    since = Some(id.clone());
                                }
                                let data = value.get("data").cloned().unwrap_or(Value::Null);
                                let _ = events.send_blocking(NetEvent::Gateway { event, id, data });
                            }
                        }
                        Ok(WsMessage::Ping(payload)) => {
                            let _ = socket.send(WsMessage::Pong(payload));
                        }
                        Ok(WsMessage::Close(frame)) => {
                            let reason = frame
                                .map(|frame| format!("code={} {}", frame.code, frame.reason))
                                .unwrap_or_else(|| "closed".into());
                            let _ = events.send_blocking(NetEvent::Disconnected { reason });
                            break;
                        }
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(error))
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) => {}
                        Err(error) => {
                            let _ = events.send_blocking(NetEvent::Disconnected {
                                reason: error.to_string(),
                            });
                            break;
                        }
                    }
                    if last_ping.elapsed() >= Duration::from_secs(30) {
                        let _ = socket.send(WsMessage::Ping(Vec::new()));
                        last_ping = Instant::now();
                    }
                }
            }
            Err(error) => {
                let _ = events.send_blocking(NetEvent::Disconnected {
                    reason: error.to_string(),
                });
            }
        }

        // 退避等待，期间仍可被 stop 打断。
        let deadline = Instant::now() + Duration::from_secs(delay);
        while Instant::now() < deadline {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        delay = (delay * 2).min(15);
    }
}

fn gateway_url(base: &str, token: &str, since: Option<&str>) -> String {
    let base = base.trim_end_matches('/');
    let ws_base = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        base.to_string()
    };
    let mut url = format!("{ws_base}/ws/gateway?token={token}");
    if let Some(cursor) = since
        && !cursor.is_empty()
    {
        url.push_str("&since=");
        url.push_str(cursor);
    }
    url
}

fn configure_read_timeout(socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>) {
    match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        }
        MaybeTlsStream::Rustls(stream) => {
            let _ = stream
                .get_ref()
                .set_read_timeout(Some(Duration::from_millis(500)));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_events_in_order() {
        use std::net::TcpListener;
        use std::sync::Mutex;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let served = Arc::new(Mutex::new(0));
        let served_thread = served.clone();
        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let mut socket = tungstenite::accept(stream).unwrap();
                for (index, id) in ["1", "2", "3"].iter().enumerate() {
                    let payload = serde_json::json!({
                        "event": "MESSAGE_CREATE",
                        "id": id,
                        "data": {"index": index},
                    });
                    if socket
                        .send(WsMessage::Text(payload.to_string()))
                        .is_err()
                    {
                        return;
                    }
                }
                *served_thread.lock().unwrap() = 3;
                thread::sleep(Duration::from_millis(300));
            }
        });

        let config = NetConfig::new(
            format!("http://127.0.0.1:{port}"),
            Duration::from_secs(2),
            false,
            std::path::PathBuf::from("/tmp/dgpui-ws-test.json"),
        );
        let store = Arc::new(SessionStore::new());
        store.set(super::super::auth::Tokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            token_type: "Bearer".into(),
            expires_in: 900,
        });
        let (tx, rx) = async_channel::unbounded();
        let mut client = WsClient::start(config, store, tx, None);

        assert!(matches!(
            pollster::block_on(rx.recv()).unwrap(),
            NetEvent::Connected
        ));
        for expected in ["1", "2", "3"] {
            match pollster::block_on(rx.recv()).unwrap() {
                NetEvent::Gateway { id, .. } => assert_eq!(id, expected),
                other => panic!("unexpected event: {other:?}"),
            }
        }
        client.stop();
        assert_eq!(*served.lock().unwrap(), 3);
    }

    #[test]
    fn duplicate_cursor_is_detected() {
        assert!(is_duplicate(Some("10"), "10"));
        assert!(!is_duplicate(Some("10"), "11"));
        assert!(!is_duplicate(None, ""));
    }

    #[test]
    fn builds_ws_url_with_since() {
        assert_eq!(
            gateway_url("http://127.0.0.1:8080", "abc", None),
            "ws://127.0.0.1:8080/ws/gateway?token=abc"
        );
        assert_eq!(
            gateway_url("https://api.example.com/", "abc", Some("42")),
            "wss://api.example.com/ws/gateway?token=abc&since=42"
        );
    }
}
