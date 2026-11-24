"""服务器模板：与前端 data/seed.rs 的 4 个模板同源（游戏公会 / 学习小组 / 设计工作室 / 开源项目）。"""

from dataclasses import dataclass, field


@dataclass(frozen=True)
class TemplateChannel:
    name: str
    kind: str = "text"
    topic: str = ""


@dataclass(frozen=True)
class TemplateCategory:
    name: str
    channels: tuple[TemplateChannel, ...]


@dataclass(frozen=True)
class Template:
    key: str
    name: str
    tagline: str
    badge: str
    color: int
    categories: tuple[TemplateCategory, ...]
    headline: str
    description: str
    rules: tuple[tuple[str, str], ...] = field(default_factory=tuple)
    recommended: tuple[str, ...] = field(default_factory=tuple)

    @property
    def channel_count(self) -> int:
        return sum(len(c.channels) for c in self.categories)


TEMPLATES: dict[str, Template] = {
    "gaming": Template(
        key="gaming",
        name="游戏公会",
        tagline="组队、开黑、赛后复盘",
        badge="游",
        color=0xED4245,
        categories=(
            TemplateCategory(
                "信息",
                (
                    TemplateChannel("欢迎", "text", "先看规则再开麦"),
                    TemplateChannel("公告", "text", "活动与版本更新"),
                ),
            ),
            TemplateCategory(
                "文字",
                (
                    TemplateChannel("闲聊", "text", "什么都聊"),
                    TemplateChannel("组队", "text", "发房间号"),
                    TemplateChannel("战绩", "text", "晒截图"),
                ),
            ),
            TemplateCategory("语音", (TemplateChannel("开黑大厅", "voice"),)),
            TemplateCategory("论坛", (TemplateChannel("找队友", "forum", "发帖约时间"),)),
        ),
        headline="欢迎来到公会",
        description="先读规则，然后去 #组队 找队友。",
        rules=(("尊重队友", "不骂人、不甩锅"), ("拒绝外挂", "发现即移出"), ("活动准时", "迟到请提前说")),
        recommended=("欢迎", "组队"),
    ),
    "study": Template(
        key="study",
        name="学习小组",
        tagline="自习室、打卡、资料共享",
        badge="学",
        color=0x3BA55D,
        categories=(
            TemplateCategory(
                "信息",
                (
                    TemplateChannel("欢迎", "text", "介绍你自己"),
                    TemplateChannel("公告", "text", "每周计划"),
                ),
            ),
            TemplateCategory(
                "学习",
                (
                    TemplateChannel("自习室", "text", "番茄钟打卡"),
                    TemplateChannel("答疑", "text", "贴题目"),
                    TemplateChannel("资料", "text", "笔记与链接"),
                ),
            ),
            TemplateCategory("语音", (TemplateChannel("专注自习", "voice"),)),
        ),
        headline="欢迎加入学习小组",
        description="每天打卡，互相监督。",
        rules=(
            ("专注", "自习频道不闲聊"),
            ("分享", "资料先搜再问"),
        ),
        recommended=("自习室", "公告"),
    ),
    "design": Template(
        key="design",
        name="设计工作室",
        tagline="作品评审、灵感、接单",
        badge="设",
        color=0xFAA61A,
        categories=(
            TemplateCategory(
                "信息",
                (
                    TemplateChannel("欢迎", "text", "看规则"),
                    TemplateChannel("公告", "text", "工作室动态"),
                ),
            ),
            TemplateCategory(
                "创作",
                (
                    TemplateChannel("作品展示", "text", "发图配说明"),
                    TemplateChannel("灵感", "text", "参考与配色"),
                    TemplateChannel("作品反馈", "forum", "求点评请发帖"),
                ),
            ),
            TemplateCategory("语音", (TemplateChannel("评审间", "voice"),)),
        ),
        headline="欢迎来到设计工作室",
        description="发作品请附上下文：目标、限制、想要什么反馈。",
        rules=(
            ("具体", "反馈要说清哪里、为什么"),
            ("署名", "转载注明来源"),
        ),
        recommended=("作品展示", "灵感"),
    ),
    "oss": Template(
        key="oss",
        name="开源项目",
        tagline="Issue、PR、发布节奏",
        badge="源",
        color=0x5865F2,
        categories=(
            TemplateCategory(
                "信息",
                (
                    TemplateChannel("公告", "text", "版本与路线图"),
                    TemplateChannel("贡献指南", "text", "怎么提 PR"),
                ),
            ),
            TemplateCategory(
                "协作",
                (
                    TemplateChannel("闲聊", "text", "随便聊"),
                    TemplateChannel("问题反馈", "forum", "一个帖子一个 issue"),
                    TemplateChannel("Pull Request", "text", "贴链接求 review"),
                ),
            ),
            TemplateCategory("语音", (TemplateChannel("维护者会议", "voice"),)),
        ),
        headline="欢迎参与开源项目",
        description="提问前先搜 issue；提 PR 请附测试。",
        rules=(("先搜再问", "避免重复 issue"), ("小步提交", "一个 PR 一件事"), ("友善", "对事不对人")),
        recommended=("公告", "贡献指南"),
    ),
}


def get_template(key: str | None) -> Template | None:
    return TEMPLATES.get(key) if key else None


def list_templates() -> list[dict]:
    return [
        {
            "key": t.key,
            "name": t.name,
            "tagline": t.tagline,
            "badge": t.badge,
            "color": t.color,
            "categories": len(t.categories),
            "channels": t.channel_count,
        }
        for t in TEMPLATES.values()
    ]
