import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles

from app.api.v1 import api_router
from app.config import settings
from app.db import init_db
from app.errors import ApiError, api_error_handler
from app.realtime import outbox  # noqa: F401  注册 after_commit 广播监听
from app.realtime.bus import close_bus, init_bus
from app.realtime.gateway import router as gateway_router
from app.services import calls as call_service
from app.services import retention
from app.services.mailer import init_mailer
from app.services.storage import init_storage

logging.basicConfig(level=logging.INFO, format="%(levelname)s %(name)s: %(message)s")
log = logging.getLogger("server")


@asynccontextmanager
async def lifespan(app: FastAPI):  # noqa: ANN201
    settings.upload_dir.mkdir(parents=True, exist_ok=True)
    await init_db()
    init_mailer()
    init_storage()
    await init_bus()
    cleaner = retention.start()
    call_watchdog = call_service.start()
    log.info("ready: %s", settings.database_url)
    try:
        yield
    finally:
        await retention.stop(cleaner)
        await call_service.stop(call_watchdog)
        await close_bus()


def create_app() -> FastAPI:
    app = FastAPI(
        title="discord-gpui-server",
        version="0.1.0",
        description="Discord 风格桌面客户端的后端骨架。接口清单见 docs/backend-api.md。",
        lifespan=lifespan,
    )

    app.add_middleware(
        CORSMiddleware,
        allow_origins=settings.cors_origins,
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    app.add_exception_handler(ApiError, api_error_handler)

    @app.exception_handler(Exception)
    async def unhandled(request: Request, exc: Exception) -> JSONResponse:  # noqa: ANN001
        log.exception("unhandled error on %s", request.url.path)
        return JSONResponse(
            status_code=500,
            content={
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "服务端异常",
                    "field": None,
                    "request_id": request.headers.get("x-request-id", ""),
                }
            },
        )

    app.include_router(api_router, prefix=settings.api_prefix)
    app.include_router(gateway_router)
    app.mount("/media", StaticFiles(directory=str(settings.upload_dir), check_dir=False), name="media")
    return app


app = create_app()
