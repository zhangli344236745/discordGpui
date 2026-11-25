"""所有 ORM 模型集中在这里 import 一次，保证 Base.metadata 完整（建表 / 迁移都靠它）。"""

from app.models.base import Base
from app.models.channel import Category, Channel, ChannelUserState
from app.models.community import (
    Boost,
    EventRsvp,
    ServerEvent,
    VoiceParticipant,
    WelcomeAccept,
    WelcomeRecommended,
    WelcomeRule,
    WelcomeScreen,
)
from app.models.forum import ForumPost, ForumReply, ForumTag
from app.models.guild import (
    Ban,
    ChannelOverride,
    MemberRole,
    PermissionGrant,
    Role,
    Server,
    ServerMember,
)
from app.models.integration import BotToken, SlashCommand, Webhook
from app.models.interaction import (
    Bookmark,
    Poll,
    PollOption,
    PollVote,
    Thread,
    ThreadReply,
)
from app.models.message import (
    Attachment,
    DmChannel,
    DmMessage,
    Message,
    MessageEdit,
    Reaction,
    ReadState,
)
from app.models.misc import CallSession, Draft, MessageRequest, Notification
from app.models.moderation import (
    AuditEntry,
    AutoModKeyword,
    AutoModRule,
    Invite,
    LegalHold,
    Report,
    ReportAction,
    RetentionRule,
)
from app.models.realtime import GatewayEvent
from app.models.security import SecurityAudit
from app.models.social import Block, FriendGroup, FriendRequest, Friendship
from app.models.user import AuthToken, Session, User, UserSettings

__all__ = [
    "Attachment",
    "BotToken",
    "AuditEntry",
    "AuthToken",
    "AutoModKeyword",
    "AutoModRule",
    "Ban",
    "Base",
    "Block",
    "Bookmark",
    "Boost",
    "CallSession",
    "Category",
    "Channel",
    "ChannelOverride",
    "ChannelUserState",
    "DmChannel",
    "DmMessage",
    "Draft",
    "EventRsvp",
    "ForumPost",
    "ForumReply",
    "ForumTag",
    "FriendGroup",
    "FriendRequest",
    "Friendship",
    "GatewayEvent",
    "Invite",
    "LegalHold",
    "MemberRole",
    "Message",
    "MessageEdit",
    "MessageRequest",
    "Notification",
    "PermissionGrant",
    "Poll",
    "PollOption",
    "PollVote",
    "Reaction",
    "Report",
    "ReportAction",
    "RetentionRule",
    "ReadState",
    "Role",
    "Server",
    "SecurityAudit",
    "ServerEvent",
    "ServerMember",
    "SlashCommand",
    "Session",
    "Thread",
    "ThreadReply",
    "User",
    "UserSettings",
    "VoiceParticipant",
    "WelcomeAccept",
    "WelcomeRecommended",
    "WelcomeRule",
    "Webhook",
    "WelcomeScreen",
]
