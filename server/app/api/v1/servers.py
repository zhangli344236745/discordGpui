from fastapi import APIRouter, Depends, Query, status
from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db
from app.deps import ServerContext, current_user, server_context
from app.errors import conflict, not_found
from app.ids import new_id
from app.models import (
    Category,
    Channel,
    MemberRole,
    PermissionGrant,
    Role,
    Server,
    ServerMember,
    User,
    WelcomeAccept,
    WelcomeRecommended,
    WelcomeRule,
    WelcomeScreen,
)
from app.realtime.events import Event
from app.realtime.hub import hub
from app.schemas.guild import ServerCreate, ServerOut, ServerPatch
from app.services import audit
from app.services.permissions import BASE_PERMISSIONS
from app.services.templates import Template, get_template, list_templates

router = APIRouter(prefix="/servers", tags=["servers"])
discover_router = APIRouter(prefix="/discover", tags=["discover"])


async def _default_layout(db: AsyncSession, server_id: str, template: Template | None) -> list[str]:
    """铺分类与频道，返回推荐频道 id（按模板 recommended 名称匹配）。"""
    created: dict[str, str] = {}
    if template is None:
        specs = [
            ("文字频道", [("一般", "text", "")]),
            ("语音频道", [("大厅", "voice", "")]),
        ]
    else:
        specs = [(cat.name, [(ch.name, ch.kind, ch.topic) for ch in cat.channels]) for cat in template.categories]

    for cat_pos, (cat_name, channels) in enumerate(specs):
        category = Category(id=new_id(), server_id=server_id, name=cat_name, position=cat_pos)
        db.add(category)
        await db.flush()  # 频道要引用 category.id，先落库
        for ch_pos, (ch_name, kind, topic) in enumerate(channels):
            channel = Channel(
                id=new_id(),
                server_id=server_id,
                category_id=category.id,
                name=ch_name,
                kind=kind,
                topic=topic,
                position=ch_pos,
            )
            db.add(channel)
            created.setdefault(ch_name, channel.id)

    if template is None:
        return []
    return [created[name] for name in template.recommended if name in created]


async def _provision_welcome(
    db: AsyncSession, server_id: str, template: Template | None, recommended: list[str]
) -> None:
    if template is None:
        db.add(
            WelcomeScreen(
                server_id=server_id,
                headline="欢迎加入",
                description="先看看规则，再去推荐的频道打个招呼。",
                require_rules=True,
            )
        )
        db.add(WelcomeRule(server_id=server_id, position=1, title="友善交流", body="对事不对人。"))
        db.add(WelcomeRule(server_id=server_id, position=2, title="不要刷屏", body="把内容发到对的频道。"))
        return

    db.add(
        WelcomeScreen(
            server_id=server_id,
            headline=template.headline,
            description=template.description,
            require_rules=True,
        )
    )
    for i, (title, body) in enumerate(template.rules, start=1):
        db.add(WelcomeRule(server_id=server_id, position=i, title=title, body=body))
    for ch_id in recommended:
        db.add(WelcomeRecommended(server_id=server_id, channel_id=ch_id))


@router.get("", response_model=list[ServerOut])
async def my_servers(db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> list[ServerOut]:
    rows = (
        await db.scalars(
            select(Server)
            .join(ServerMember, ServerMember.server_id == Server.id)
            .where(ServerMember.user_id == user.id)
            .order_by(Server.position, Server.id)
        )
    ).all()
    return [ServerOut.model_validate(s) for s in rows]


@router.post("", response_model=ServerOut, status_code=status.HTTP_201_CREATED)
async def create_server(
    body: ServerCreate, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)
) -> ServerOut:
    template = get_template(body.template_key)
    name = body.name.strip() or (template.name if template else "未命名服务器")

    dup = await db.scalar(
        select(Server.id)
        .join(ServerMember, ServerMember.server_id == Server.id)
        .where(ServerMember.user_id == user.id, func.lower(Server.name) == name.lower())
    )
    if dup is not None:
        raise conflict("你已经有一个同名服务器", "name")

    position = await db.scalar(select(func.count()).select_from(ServerMember).where(ServerMember.user_id == user.id))
    server = Server(
        id=new_id(),
        name=name,
        icon_color=body.icon_color,
        description=template.description if template else "",
        owner_id=user.id,
        is_public=body.is_public,
        category="other",
        template_key=template.key if template else None,
        member_count=1,
        position=int(position or 0),
    )
    db.add(server)
    await db.flush()  # 先把 servers 行写下去，后面 categories/channels 的外键才能过

    everyone = Role(
        id=new_id(),
        server_id=server.id,
        name="@everyone",
        color=0x99AAB5,
        position=0,
        is_everyone=True,
    )
    db.add(everyone)
    await db.flush()
    for perm in BASE_PERMISSIONS:
        db.add(PermissionGrant(server_id=server.id, role_id=everyone.id, permission=perm, state="allow"))

    db.add(ServerMember(server_id=server.id, user_id=user.id, nickname=""))
    db.add(MemberRole(server_id=server.id, user_id=user.id, role_id=everyone.id))

    recommended = await _default_layout(db, server.id, template)
    await _provision_welcome(db, server.id, template, recommended)

    detail = f"创建了服务器 / {name}"
    if template:
        detail += f"（模板：{template.name}）"
    audit.log(db, server.id, user.id, audit.KIND_SERVER, detail)
    await db.commit()
    await db.refresh(server)

    payload = {"server": ServerOut.model_validate(server).model_dump()}
    await hub.to_user(user.id, Event.SERVER_UPDATE, payload)
    return ServerOut.model_validate(server)


@router.get("/{server_id}", response_model=ServerOut)
async def get_server(ctx: ServerContext = Depends(server_context)) -> ServerOut:
    return ServerOut.model_validate(ctx.server)


@router.patch("/{server_id}", response_model=ServerOut)
async def patch_server(
    body: ServerPatch,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> ServerOut:
    ctx.require("MANAGE_SERVER")
    if body.name is not None and body.name != ctx.server.name:
        old = ctx.server.name
        ctx.server.name = body.name.strip()
        audit.log(
            db,
            ctx.server.id,
            ctx.user.id,
            audit.KIND_SERVER,
            f"更新了服务器名称 / {old} → {ctx.server.name}",
        )
    if body.description is not None:
        ctx.server.description = body.description
        audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_SERVER, "更新了服务器简介")
    if body.icon_color is not None and body.icon_color != ctx.server.icon_color:
        ctx.server.icon_color = body.icon_color
        audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_SERVER, "更新了服务器颜色")
    if body.is_public is not None:
        ctx.server.is_public = body.is_public
    if body.notify_level is not None:
        ctx.server.notify_level = body.notify_level

    await db.commit()
    payload = {"server": ServerOut.model_validate(ctx.server).model_dump()}
    await hub.to_server(ctx.server.id, Event.SERVER_UPDATE, payload)
    return ServerOut.model_validate(ctx.server)


@router.delete("/{server_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_server(ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)) -> None:
    if not ctx.is_owner:
        from app.errors import forbidden

        raise forbidden("MANAGE_SERVER")
    server_id = ctx.server.id
    await db.delete(ctx.server)
    await db.commit()
    await hub.to_server(server_id, Event.SERVER_UPDATE, {"server_id": server_id, "deleted": True})


@router.post("/{server_id}/leave", status_code=status.HTTP_204_NO_CONTENT)
async def leave_server(ctx: ServerContext = Depends(server_context), db: AsyncSession = Depends(get_db)) -> None:
    if ctx.is_owner:
        raise conflict("服务器所有者不能离开，请先转让或解散", "owner_id")
    await db.delete(ctx.member)
    ctx.server.member_count = max(0, ctx.server.member_count - 1)
    await db.commit()
    await hub.to_server(ctx.server.id, Event.MEMBER_LEAVE, {"server_id": ctx.server.id, "user_id": ctx.user.id})


@router.put("/{server_id}/order", status_code=status.HTTP_204_NO_CONTENT)
async def reorder_server(
    body: dict,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> None:
    position = int(body.get("position", 0))
    ctx.server.position = position
    await db.commit()


@router.get("/{server_id}/templates")
async def templates_for_server(ctx: ServerContext = Depends(server_context)) -> list[dict]:
    return list_templates()


@router.post("/{server_id}/templates/apply", status_code=status.HTTP_201_CREATED)
async def apply_template(
    body: dict,
    ctx: ServerContext = Depends(server_context),
    db: AsyncSession = Depends(get_db),
) -> dict:
    ctx.require("MANAGE_SERVER")
    template = get_template(body.get("template_key"))
    if template is None:
        raise not_found("模板")

    recommended = await _default_layout(db, ctx.server.id, template)
    for model, column in (
        (WelcomeRule, WelcomeRule.server_id),
        (WelcomeRecommended, WelcomeRecommended.server_id),
        (WelcomeScreen, WelcomeScreen.server_id),
    ):
        existing = await db.scalar(select(model).where(column == ctx.server.id))
        if existing is not None:
            await db.delete(existing)
    await db.flush()
    await _provision_welcome(db, ctx.server.id, template, recommended)
    ctx.server.template_key = template.key
    audit.log(db, ctx.server.id, ctx.user.id, audit.KIND_SERVER, f"应用了模板 / {template.name}")
    await db.commit()
    return {"applied": template.key, "channels": template.channel_count}


@discover_router.get("/servers")
async def discover(
    q: str = "",
    category: str = "",
    limit: int = Query(default=20, le=50),
    db: AsyncSession = Depends(get_db),
    user: User = Depends(current_user),
) -> list[dict]:
    stmt = select(Server).where(Server.is_public.is_(True))
    if q:
        stmt = stmt.where(func.lower(Server.name).contains(q.lower()))
    if category and category != "all":
        stmt = stmt.where(Server.category == category)
    rows = (await db.scalars(stmt.order_by(Server.member_count.desc()).limit(limit))).all()

    joined = set((await db.scalars(select(ServerMember.server_id).where(ServerMember.user_id == user.id))).all())
    return [
        {
            **ServerOut.model_validate(s).model_dump(),
            "joined": s.id in joined,
        }
        for s in rows
    ]


@discover_router.post("/servers/{server_id}/join")
async def join_server(server_id: str, db: AsyncSession = Depends(get_db), user: User = Depends(current_user)) -> dict:
    server = await db.get(Server, server_id)
    if server is None or not server.is_public:
        raise not_found("服务器")

    member = await db.scalar(
        select(ServerMember).where(ServerMember.server_id == server_id, ServerMember.user_id == user.id)
    )
    if member is None:
        db.add(ServerMember(server_id=server_id, user_id=user.id, nickname=""))
        server.member_count += 1
        # 默认给 @everyone
        everyone = await db.scalar(select(Role).where(Role.server_id == server_id, Role.is_everyone.is_(True)))
        if everyone is not None:
            db.add(MemberRole(server_id=server_id, user_id=user.id, role_id=everyone.id))
        await db.commit()
        await hub.to_server(server_id, Event.MEMBER_JOIN, {"server_id": server_id, "user_id": user.id})

    welcome = await db.get(WelcomeScreen, server_id)
    accepted = await db.get(WelcomeAccept, (server_id, user.id))
    rules = (
        await db.scalars(select(WelcomeRule).where(WelcomeRule.server_id == server_id).order_by(WelcomeRule.position))
    ).all()
    recommended = (
        await db.scalars(select(WelcomeRecommended.channel_id).where(WelcomeRecommended.server_id == server_id))
    ).all()

    return {
        "server": ServerOut.model_validate(server).model_dump(),
        "welcome": None
        if welcome is None
        else {
            "headline": welcome.headline,
            "description": welcome.description,
            "require_rules": welcome.require_rules,
            "rules": [{"title": r.title, "body": r.body} for r in rules],
            "recommended": list(recommended),
            "accepted": accepted is not None,
        },
    }
