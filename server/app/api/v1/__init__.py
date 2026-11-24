from fastapi import APIRouter

from app.api.v1 import auth, channels, messages, servers, stubs, system, uploads, users, voice

api_router = APIRouter()
api_router.include_router(system.router)
api_router.include_router(auth.router)
api_router.include_router(users.router)
api_router.include_router(servers.router)
api_router.include_router(servers.discover_router)
api_router.include_router(channels.router)
api_router.include_router(messages.router)
api_router.include_router(uploads.router)
api_router.include_router(voice.router)
# 占位路由放最后：具体路径优先于这里注册的同名前缀通配
api_router.include_router(stubs.router)

__all__ = ["api_router"]
