from functools import lru_cache
from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict

BASE_DIR = Path(__file__).resolve().parent.parent


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    app_name: str = "discord-gpui-server"
    api_prefix: str = "/api/v1"
    debug: bool = False

    # SQLite 单文件；测试里覆盖成临时文件
    database_url: str = f"sqlite+aiosqlite:///{BASE_DIR / 'data' / 'app.db'}"

    jwt_secret: str = "dev-only-change-me-please-32bytes!"
    jwt_algorithm: str = "HS256"
    access_ttl_minutes: int = 15
    refresh_ttl_days: int = 30

    # 上传：本地磁盘（生产换 MinIO / S3 时改这里）
    upload_dir: Path = BASE_DIR / "data" / "uploads"
    upload_max_bytes: int = 25 * 1024 * 1024
    media_base_url: str = "http://127.0.0.1:8080/media"

    # LiveKit
    livekit_url: str = "wss://localhost:7880"
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
