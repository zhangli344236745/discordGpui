from fastapi import APIRouter, Depends
from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.db import get_db
from app.realtime.hub import hub

router = APIRouter(tags=["system"])

GIF_CATEGORIES = ["热门", "反应", "搞怪", "动物"]

GIFS = [
    {"title": "鼓掌", "emoji": "👏", "color": 0xFAA61A, "category": "热门", "tags": ["赞", "庆祝"]},
    {"title": "放烟花", "emoji": "🎆", "color": 0x5865F2, "category": "热门", "tags": ["庆祝", "开局"]},
    {"title": "比心", "emoji": "🫶", "color": 0xEB459E, "category": "热门", "tags": ["爱", "谢谢"]},
    {"title": "欢呼", "emoji": "🙌", "color": 0x3BA55D, "category": "热门", "tags": ["赢", "庆祝"]},
    {"title": "闪亮", "emoji": "✨", "color": 0xFEE75C, "category": "热门", "tags": ["新", "好看"]},
    {"title": "笑哭", "emoji": "😂", "color": 0xFEE75C, "category": "反应", "tags": ["笑", "哈哈"]},
    {"title": "震惊", "emoji": "😱", "color": 0x9B59B6, "category": "反应", "tags": ["惊", "什么"]},
    {"title": "无语", "emoji": "😑", "color": 0x95A5A6, "category": "反应", "tags": ["无语", "服了"]},
    {"title": "思考", "emoji": "🤔", "color": 0x3498DB, "category": "反应", "tags": ["想", "为什么"]},
    {"title": "点赞", "emoji": "👍", "color": 0x3BA55D, "category": "反应", "tags": ["好", "同意"]},
    {"title": "蹦迪", "emoji": "🕺", "color": 0xEB459E, "category": "搞怪", "tags": ["嗨", "音乐"]},
    {"title": "翻车", "emoji": "🚗", "color": 0xED4245, "category": "搞怪", "tags": ["失败", "尴尬"]},
    {"title": "做鬼脸", "emoji": "😜", "color": 0xFAA61A, "category": "搞怪", "tags": ["调皮", "略"]},
    {"title": "转圈", "emoji": "🌀", "color": 0x5865F2, "category": "搞怪", "tags": ["晕", "循环"]},
    {"title": "假装思考", "emoji": "🧐", "color": 0x95A5A6, "category": "搞怪", "tags": ["演技", "认真"]},
    {"title": "猫猫点头", "emoji": "🐱", "color": 0xFAA61A, "category": "动物", "tags": ["猫", "可爱"]},
    {"title": "柴犬歪头", "emoji": "🐶", "color": 0xD35400, "category": "动物", "tags": ["狗", "疑惑"]},
    {"title": "仓鼠吃东西", "emoji": "🐹", "color": 0xE67E22, "category": "动物", "tags": ["鼠", "吃"]},
    {"title": "企鹅摇摆", "emoji": "🐧", "color": 0x34495E, "category": "动物", "tags": ["企鹅", "走"]},
    {"title": "兔兔跳", "emoji": "🐰", "color": 0xEB459E, "category": "动物", "tags": ["兔", "跳"]},
]

HELP_ARTICLES = [
    (
        "入门",
        "第一次打开",
        "三个区域：服务器栏、频道栏、聊天区。",
        "点服务器切换社区；点频道切换会话；底部齿轮进设置。",
    ),
    (
        "入门",
        "创建自己的服务器",
        "服务器栏底部 + 可以建服务器，也可以选模板。",
        "模板会一次铺好分类、文字 / 语音 / 论坛频道和欢迎屏幕。",
    ),
    ("入门", "加入别人的服务器", "用邀请链接或探索页。", "探索页按分类浏览公开服务器，加入后自动弹欢迎屏幕。"),
    ("消息", "发送与编辑", "Enter 发送，Shift+Enter 换行。", "自己的消息可以在菜单里编辑，编辑历史会保留。"),
    ("消息", "回复与话题", "回复会带引用条，话题在右侧独立面板。", "点引用条能跳回原消息并高亮。"),
    ("消息", "表情、贴纸与 GIF", "输入框右侧的三个按钮。", "表情也可以直接加到消息上作为回应。"),
    ("消息", "搜索", "标题栏放大镜进入搜索中心。", "支持 from: / has: / after: / before: 过滤。"),
    ("会话", "好友与私信", "好友页五个标签；点好友即建私信。", "消息请求会在侧栏单独提示。"),
    ("会话", "通知与静音", "频道右键或标题栏铃铛。", "频道可以跟随服务器默认级别。"),
    ("语音", "连接语音频道", "点语音频道即连接。", "底部状态条展开房间面板，可以静音或移出成员。"),
    ("语音", "私信通话", "私信标题栏的电话 / 摄像头按钮。", "通话与语音频道互斥。"),
    ("设置", "主题与数据", "设置 → 外观 / 数据与存储。", "浅色主题会真正切换整套色板。"),
]


@router.get("/health")
async def health(db: AsyncSession = Depends(get_db)) -> dict:
    db_ok = True
    try:
        await db.execute(text("SELECT 1"))
    except Exception:  # pragma: no cover - 只在 DB 故障时触发
        db_ok = False
    return {
        "status": "ok" if db_ok else "degraded",
        "db": db_ok,
        "livekit": bool(settings.livekit_url),
        "gateway_connections": hub.size,
    }


@router.get("/config")
async def config() -> dict:
    return {
        "api_version": "v1",
        "upload_max_bytes": settings.upload_max_bytes,
        "gif_categories": GIF_CATEGORIES,
        "voice_region": "local",
        "features": {
            "forum": True,
            "voice": True,
            "automod": True,
            "boosts": True,
            "templates": True,
        },
    }


@router.get("/gifs")
async def gif_categories() -> list[dict]:
    return [{"name": c, "count": sum(1 for g in GIFS if g["category"] == c)} for c in GIF_CATEGORIES]


@router.get("/gifs/search")
async def gif_search(q: str = "", category: str = "") -> list[dict]:
    items = GIFS
    if category and category != "全部":
        items = [g for g in items if g["category"] == category]
    if q:
        needle = q.lower()
        items = [g for g in items if needle in g["title"].lower() or any(needle in t for t in g["tags"])]
    return items


@router.get("/help/articles")
async def help_articles() -> list[dict]:
    return [
        {"id": str(i), "category": cat, "title": title, "summary": summary, "body": body}
        for i, (cat, title, summary, body) in enumerate(HELP_ARTICLES, start=1)
    ]
