from typing import Any

from fastapi import Request
from fastapi.responses import JSONResponse


class ErrorCode:
    VALIDATION_ERROR = "VALIDATION_ERROR"
    UNAUTHORIZED = "UNAUTHORIZED"
    TOKEN_EXPIRED = "TOKEN_EXPIRED"
    FORBIDDEN = "MISSING_PERMISSION"
    NOT_FOUND = "NOT_FOUND"
    CONFLICT = "CONFLICT"
    AUTOMOD_BLOCKED = "AUTOMOD_BLOCKED"
    CHANNEL_LOCKED = "CHANNEL_LOCKED"
    RATE_LIMITED = "RATE_LIMITED"
    NOT_IMPLEMENTED = "NOT_IMPLEMENTED"


class ApiError(Exception):
    """业务异常，统一由 handler 渲染成 {error:{code,message,field,request_id}}。"""

    def __init__(
        self,
        status_code: int,
        code: str,
        message: str,
        field: str | None = None,
        extra: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> None:
        self.status_code = status_code
        self.code = code
        self.message = message
        self.field = field
        self.extra = extra or {}
        self.headers = headers or {}


def not_found(what: str = "资源") -> ApiError:
    return ApiError(404, ErrorCode.NOT_FOUND, f"{what}不存在")


def forbidden(permission: str) -> ApiError:
    return ApiError(403, ErrorCode.FORBIDDEN, f"缺少 {permission}", field="permission")


def conflict(message: str, field: str | None = None) -> ApiError:
    return ApiError(409, ErrorCode.CONFLICT, message, field=field)


def payload_too_large(message: str, field: str | None = None) -> ApiError:
    return ApiError(413, "PAYLOAD_TOO_LARGE", message, field)


def bad_request(message: str, field: str | None = None) -> ApiError:
    return ApiError(400, ErrorCode.VALIDATION_ERROR, message, field=field)


def unauthorized(message: str = "未登录", code: str = ErrorCode.UNAUTHORIZED) -> ApiError:
    return ApiError(401, code, message)


async def api_error_handler(request: Request, exc: ApiError) -> JSONResponse:
    body: dict[str, Any] = {
        "error": {
            "code": exc.code,
            "message": exc.message,
            "field": exc.field,
            "request_id": request.headers.get("x-request-id", ""),
        }
    }
    body["error"].update(exc.extra)
    return JSONResponse(status_code=exc.status_code, content=body, headers=exc.headers or None)
