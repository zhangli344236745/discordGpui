"""P1 / P2 端点的占位实现。

这些路由**存在且出现在 OpenAPI 里**（契约可见），但统一返回 501，
客户端可以先把界面接上去，服务端逐个替换即可。
"""

from fastapi import APIRouter, Request

from app.errors import ApiError, ErrorCode

router = APIRouter(tags=["pending"])

# (methods, path, 文档节, 说明)
PENDING: list[tuple[list[str], str, str, str]] = [
    # 3.3 好友
    (["GET"], "/friends", "3.3", "好友列表"),
    (["GET"], "/friends/online", "3.3", "在线好友"),
    (["POST"], "/friends/requests", "3.3", "发送好友请求"),
    (["GET"], "/friends/requests", "3.3", "好友请求列表"),
    (["POST"], "/friends/requests/{request_id}/accept", "3.3", "接受好友请求"),
    (["POST"], "/friends/requests/{request_id}/reject", "3.3", "拒绝好友请求"),
    (["DELETE"], "/friends/{user_id}", "3.3", "删除好友"),
    (["POST"], "/blocks", "3.3", "屏蔽用户"),
    (["DELETE"], "/blocks/{user_id}", "3.3", "取消屏蔽"),
    (["GET"], "/blocks", "3.3", "屏蔽列表"),
    # 3.4 好友分组
    (["GET"], "/friend-groups", "3.4", "好友分组"),
    (["POST"], "/friend-groups", "3.4", "新建分组"),
    (["PATCH"], "/friend-groups/order", "3.4", "分组排序"),
    (["PATCH"], "/friend-groups/{group_id}", "3.4", "重命名分组"),
    (["DELETE"], "/friend-groups/{group_id}", "3.4", "删除分组"),
    (["PATCH"], "/friends/{user_id}", "3.4", "移动好友分组"),
    # 3.9 投票 / 话题 / 书签
    (["POST"], "/channels/{channel_id}/polls", "3.9", "发起投票"),
    (["POST"], "/polls/{poll_id}/votes", "3.9", "投票"),
    (["DELETE"], "/polls/{poll_id}/votes", "3.9", "取消投票"),
    (["POST"], "/polls/{poll_id}/close", "3.9", "结束投票"),
    (["POST"], "/messages/{message_id}/threads", "3.9", "开话题"),
    (["GET"], "/threads/{thread_id}", "3.9", "话题详情"),
    (["POST"], "/threads/{thread_id}/replies", "3.9", "话题回复"),
    (["GET"], "/bookmarks", "3.9", "书签列表"),
    (["PUT"], "/bookmarks/{message_id}", "3.9", "保存 / 取消保存"),
    (["DELETE"], "/bookmarks/{bookmark_id}", "3.9", "移除书签"),
    # 3.10 私信与消息请求
    (["GET"], "/dms", "3.10", "私信会话列表"),
    (["POST"], "/dms", "3.10", "创建私信会话"),
    (["GET"], "/dms/{dm_id}/messages", "3.10", "私信历史"),
    (["POST"], "/dms/{dm_id}/messages", "3.10", "发送私信"),
    (["PATCH"], "/dms/{dm_id}", "3.10", "置顶 / 关闭会话"),
    (["PATCH"], "/dms/{dm_id}/read", "3.10", "标记私信已读"),
    (["GET"], "/message-requests", "3.10", "消息请求列表"),
    (["POST"], "/message-requests/{request_id}/accept", "3.10", "接受消息请求"),
    (["POST"], "/message-requests/{request_id}/ignore", "3.10", "忽略消息请求"),
    (["DELETE"], "/message-requests/{request_id}", "3.10", "删除消息请求"),
    # 3.11 论坛
    (["GET"], "/channels/{channel_id}/posts", "3.11", "帖子列表"),
    (["POST"], "/channels/{channel_id}/posts", "3.11", "发帖"),
    (["GET"], "/posts/{post_id}", "3.11", "帖子详情"),
    (["PATCH"], "/posts/{post_id}", "3.11", "编辑 / 置顶 / 锁定"),
    (["DELETE"], "/posts/{post_id}", "3.11", "删除帖子"),
    (["POST"], "/posts/{post_id}/replies", "3.11", "回帖"),
    (["DELETE"], "/replies/{reply_id}", "3.11", "删除回帖"),
    (["GET"], "/channels/{channel_id}/tags", "3.11", "频道标签"),
    # 3.12 成员 / 角色 / 权限
    (["GET"], "/servers/{server_id}/members", "3.12", "成员列表"),
    (["PATCH"], "/servers/{server_id}/members/{user_id}", "3.12", "改服务器昵称"),
    (["DELETE"], "/servers/{server_id}/members/{user_id}", "3.12", "移出成员"),
    (["GET"], "/servers/{server_id}/roles", "3.12", "角色列表"),
    (["POST"], "/servers/{server_id}/roles", "3.12", "新建角色"),
    (["PATCH"], "/roles/{role_id}", "3.12", "编辑角色"),
    (["DELETE"], "/roles/{role_id}", "3.12", "删除角色"),
    (["PUT"], "/servers/{server_id}/members/{user_id}/roles/{role_id}", "3.12", "授予 / 移除角色"),
    (["GET"], "/servers/{server_id}/permissions", "3.12", "权限矩阵"),
    (["PUT"], "/servers/{server_id}/permissions", "3.12", "写入权限矩阵"),
    (["GET"], "/channels/{channel_id}/overrides", "3.12", "频道权限覆盖"),
    (["PUT"], "/channels/{channel_id}/overrides", "3.12", "写频道权限覆盖"),
    (["GET"], "/servers/{server_id}/bans", "3.12", "封禁列表"),
    (["POST"], "/servers/{server_id}/bans", "3.12", "封禁成员"),
    (["DELETE"], "/servers/{server_id}/bans/{user_id}", "3.12", "撤销封禁"),
    # 3.13 邀请
    (["POST"], "/channels/{channel_id}/invites", "3.13", "创建邀请"),
    (["GET"], "/servers/{server_id}/invites", "3.13", "邀请列表"),
    (["DELETE"], "/invites/{code}", "3.13", "撤销邀请"),
    (["GET"], "/invites/{code}", "3.13", "解析邀请"),
    (["POST"], "/invites/{code}/accept", "3.13", "接受邀请"),
    # 3.14 审核日志与自动审核
    (["GET"], "/servers/{server_id}/audit", "3.14", "审核日志"),
    (["DELETE"], "/servers/{server_id}/audit", "3.14", "清空审核日志"),
    (["GET"], "/servers/{server_id}/automod/rules", "3.14", "自动审核规则"),
    (["POST"], "/servers/{server_id}/automod/rules", "3.14", "新建规则"),
    (["PATCH"], "/automod/rules/{rule_id}", "3.14", "编辑规则"),
    (["DELETE"], "/automod/rules/{rule_id}", "3.14", "删除规则"),
    (["POST"], "/servers/{server_id}/automod/test", "3.14", "试跑规则"),
    # 3.15 日程
    (["GET"], "/servers/{server_id}/events", "3.15", "活动列表"),
    (["POST"], "/servers/{server_id}/events", "3.15", "新建活动"),
    (["PATCH"], "/events/{event_id}", "3.15", "编辑活动"),
    (["DELETE"], "/events/{event_id}", "3.15", "删除活动"),
    (["PUT"], "/events/{event_id}/rsvp", "3.15", "报名 / 取消"),
    (["GET"], "/events/{event_id}/attendees", "3.15", "报名名单"),
    # 3.16 欢迎屏幕
    (["GET"], "/servers/{server_id}/welcome", "3.16", "欢迎配置"),
    (["PATCH"], "/servers/{server_id}/welcome", "3.16", "编辑欢迎配置"),
    (["POST"], "/servers/{server_id}/welcome/accept", "3.16", "同意规则"),
    (["GET"], "/servers/{server_id}/welcome/state", "3.16", "我的同意状态"),
    (["POST"], "/channels/{channel_id}/welcome-read", "3.16", "标记频道已读"),
    # 3.17 加成
    (["GET"], "/servers/{server_id}/boosts", "3.17", "加成状态"),
    (["PUT"], "/servers/{server_id}/boosts/me", "3.17", "加成 / 撤销"),
    (["GET"], "/servers/{server_id}/boost/tiers", "3.17", "特权档位"),
    (["POST"], "/servers/{server_id}/boost/subscribe", "3.17", "订阅计费"),
    # 3.18 搜索
    (["GET"], "/search", "3.18", "全局搜索"),
    (["GET"], "/channels/{channel_id}/search", "3.18", "会话内搜索"),
    (["GET"], "/search/suggest", "3.18", "搜索补全"),
    # 3.19 用户级视图状态
    (["GET"], "/me/channel-state", "3.19", "频道视图状态"),
    (["PUT"], "/me/channel-state/{channel_id}", "3.19", "更新视图状态"),
    (["GET"], "/me/drafts", "3.19", "草稿列表"),
    (["PUT"], "/me/drafts/{scope_id}", "3.19", "保存草稿"),
    # 3.20 通知
    (["GET"], "/notifications", "3.20", "通知列表"),
    (["POST"], "/notifications/read", "3.20", "批量已读"),
    (["GET"], "/notifications/unread-counts", "3.20", "未读计数"),
    (["POST"], "/push/subscribe", "3.20", "注册推送"),
    # 3.21 私信通话
    (["POST"], "/dms/{dm_id}/call", "3.21", "发起通话"),
    (["POST"], "/dms/{dm_id}/call/accept", "3.21", "接听"),
    (["POST"], "/dms/{dm_id}/call/decline", "3.21", "拒接"),
    (["POST"], "/dms/{dm_id}/call/hangup", "3.21", "挂断"),
]


def _make_handler(section: str, note: str):
    async def handler(request: Request):  # noqa: ANN202
        raise ApiError(
            501,
            ErrorCode.NOT_IMPLEMENTED,
            f"{note} 尚未实现（对应文档 {section}）",
        )

    handler.__name__ = f"pending_{abs(hash((section, note))) % 10_000_000}"
    return handler


for _methods, _path, _section, _note in PENDING:
    router.add_api_route(
        _path,
        _make_handler(_section, _note),
        methods=_methods,
        summary=f"[待实现] {_note}",
        description=f"计划见 docs/backend-api.md 第 {_section} 节；当前返回 501。",
        status_code=200,
    )
