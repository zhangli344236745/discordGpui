from fastapi import APIRouter

from app.api.v1 import (
    auth,
    boosts,
    bots,
    calls,
    channels,
    compliance,
    content,
    dms,
    events,
    forums,
    invites,
    me,
    members,
    messages,
    moderation,
    notifications,
    search,
    servers,
    social,
    system,
    uploads,
    users,
    voice,
    welcome,
)

api_router = APIRouter()
api_router.include_router(system.router)
api_router.include_router(auth.router)
api_router.include_router(bots.router)
api_router.include_router(users.router)
api_router.include_router(servers.router)
api_router.include_router(servers.discover_router)
api_router.include_router(channels.router)
api_router.include_router(messages.router)
api_router.include_router(uploads.router)
api_router.include_router(voice.router)
api_router.include_router(social.router)
api_router.include_router(dms.router)
api_router.include_router(calls.router)
api_router.include_router(members.router)
api_router.include_router(invites.router)
api_router.include_router(content.router)
api_router.include_router(forums.router)
api_router.include_router(events.router)
api_router.include_router(welcome.router)
api_router.include_router(boosts.router)
api_router.include_router(search.router)
api_router.include_router(me.router)
api_router.include_router(notifications.router)
api_router.include_router(moderation.router)
api_router.include_router(compliance.router)

__all__ = ["api_router"]
