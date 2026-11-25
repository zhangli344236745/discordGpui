from functools import lru_cache
from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict

BASE_DIR = Path(__file__).resolve().parent.parent


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    app_name: str = "discord-gpui-server"
    api_prefix: str = "/api/v1"
    debug: bool = False

    # SQLite 单文件；测试里覆盖成临时文件；生产可切 Postgres
    database_url: str = f"sqlite+aiosqlite:///{BASE_DIR / 'data' / 'app.db'}"
    # 为空 = 单进程内存广播；配置后启用 Redis Pub/Sub 多实例广播
    redis_url: str = ""

    jwt_secret: str = "dev-only-change-me-please-32bytes!"
    jwt_algorithm: str = "HS256"
    access_ttl_minutes: int = 15
    refresh_ttl_days: int = 30

    # 上传：本地磁盘（生产换 MinIO / S3 时改这里）
    upload_dir: Path = BASE_DIR / "data" / "uploads"
    upload_max_bytes: int = 25 * 1024 * 1024
    # 单用户附件总量配额；0 = 不限制
    upload_user_quota_bytes: int = 512 * 1024 * 1024
    media_base_url: str = "http://127.0.0.1:8080/media"
    # 为空 = 本地磁盘；配置后使用 S3/MinIO 兼容对象存储
    s3_endpoint: str = ""
    # 预签名 URL 的对外地址（浏览器/客户端可达）；为空则用 s3_endpoint
    s3_public_endpoint: str = ""
    s3_access_key: str = ""
    s3_secret_key: str = ""
    s3_bucket: str = "attachments"
    s3_region: str = "us-east-1"
    s3_url_ttl_seconds: int = 3600

    # 邮件：log = 写日志与进程内 outbox；smtp = 真实发送
    mail_backend: str = "log"
    smtp_host: str = ""
    smtp_port: int = 587
    smtp_user: str = ""
    smtp_password: str = ""
    smtp_from: str = ""
    smtp_starttls: bool = True
    app_base_url: str = "http://127.0.0.1:8080"
    email_token_ttl_minutes: int = 30

    # 2FA
    totp_issuer: str = "Discord GPUI"

    # LiveKit
    livekit_url: str = "wss://localhost:7880"
    # RoomService 的 HTTP 地址；为空则从 livekit_url 推导
    livekit_http_url: str = ""
    # 服务器侧静音/踢人是否真实调用 RoomService；未启用只改本地投影（Demo）
    livekit_roomservice_enabled: bool = False
    livekit_api_key: str = "devkey"
    livekit_api_secret: str = "devsecret-devsecret-devsecret-1234"
    livekit_token_ttl_hours: int = 2

    # 限流（简单内存计数，生产换 Redis）
    rate_limit_messages: int = 5
    rate_limit_window_seconds: int = 5
    rate_limit_logins: int = 10
    rate_limit_login_window_seconds: int = 60

    # 实时事件保留与清理
    gateway_event_retention_hours: int = 24
    gateway_heartbeat_timeout_seconds: int = 60
    retention_interval_seconds: int = 3600

    # 私信呼叫
    call_ring_timeout_seconds: int = 60
    call_watchdog_interval_seconds: int = 15

    cors_origins: list[str] = ["*"]


@lru_cache
def get_settings() -> Settings:
    return Settings()


settings = get_settings()
