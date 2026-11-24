use std::collections::HashMap;

use super::models::{
    Attachment, AuditEntry, AuditKind, AutoModAction, AutoModRule, BannedUser, Category, Channel,
    ChatThread, DiscoverServer, DmConversation, Friend, FriendRequest, Gif, HelpArticle, Invite,
    InviteExpiry, Member, Message, MyProfile, NotifyLevel, PermissionGrant, PermissionState,
    Poll, ReadReceipt, Role, Server, Sticker, ThreadReply,
};

/// 前端模拟服务器数据。
pub fn seed_servers() -> Vec<Server> {
    vec![
        Server {
            name: "开发者社区".into(),
            badge: "开".into(),
            color: 0x5865f2,
            notify: NotifyLevel::All,
            description: "一个聊 Rust、前端与工具链的开发者聚集地，欢迎分享作品与踩坑记录。".into(),
            categories: vec![
                Category {
                    name: "信息中心".into(),
                    channels: vec![
                        Channel::text("公告", "社区公告与版本更新"),
                        Channel::text("规则", "请先阅读社区规则").unread(1),
                        Channel::text("资源分享", "好用的开发工具与资料"),
                    ],
                },
                Category {
                    name: "文字频道".into(),
                    channels: vec![
                        Channel::text("闲聊", "日常摸鱼、随便聊聊")
                            .unread(3)
                            .notify(NotifyLevel::Mentions),
                        Channel::text("前端开发", "React / Vue / CSS 话题"),
                        Channel::text("Rust 讨论", "所有权、异步与生态"),
                        Channel::text("作品展示", "晒一晒最近做的东西")
                            .unread(2)
                            .notify(NotifyLevel::Nothing),
                    ],
                },
                Category {
                    name: "语音频道".into(),
                    channels: vec![Channel::voice("一般"), Channel::voice("技术闲聊")],
                },
            ],
        },
        Server {
            name: "设计工坊".into(),
            badge: "设".into(),
            color: 0xeb459e,
            notify: NotifyLevel::All,
            description: "视觉与交互设计的交流空间，随手发灵感、互评设计稿。".into(),
            categories: vec![
                Category {
                    name: "工作台".into(),
                    channels: vec![
                        Channel::text("灵感", "每天一点好设计"),
                        Channel::text("反馈", "设计稿互评"),
                        Channel::text("素材", "免费图库与字体"),
                    ],
                },
                Category {
                    name: "语音频道".into(),
                    channels: vec![Channel::voice("一起画画")],
                },
            ],
        },
        Server {
            name: "音乐房".into(),
            badge: "音".into(),
            color: 0x57f287,
            notify: NotifyLevel::All,
            description: "一起听歌、聊编曲与设备的地方，语音频道常驻。".into(),
            categories: vec![Category {
                name: "音乐".into(),
                channels: vec![
                    Channel::text("点歌", "歌单接龙"),
                    Channel::text("分享", "最近播放的歌"),
                ],
            }],
        },
        Server {
            name: "游戏夜".into(),
            badge: "游".into(),
            color: 0xfee75c,
            notify: NotifyLevel::All,
            description: "开黑、组队与赛事讨论，晚上语音房最热闹。".into(),
            categories: vec![Category {
                name: "组队".into(),
                channels: vec![
                    Channel::text("找队友", "今晚开黑吗"),
                    Channel::text("战绩", "晒晒段位"),
                ],
            }],
        },
    ]
}

/// 前端模拟消息数据，键为 `服务器/频道`。
pub fn seed_messages() -> HashMap<String, Vec<Message>> {
    let mut all = HashMap::new();

    all.insert(
        "开发者社区/闲聊".into(),
        vec![
            Message::new(
                "小满",
                "昨天 21:34",
                "这个新界面的配色好舒服，像换了工作台 🎨",
                0xfaa61c,
            )
            .reaction("❤️", 4)
            .reaction("🎉", 2),
            Message::new(
                "小满",
                "昨天 21:35",
                "尤其是侧边栏那个灰蓝色的层次，做得挺克制。",
                0xfaa61c,
            )
            .grouped(),
            Message::new(
                "阿哲",
                "昨天 22:08",
                "GPUI 这套渲染链路确实快，滚动列表基本没有掉帧。",
                0x5865f2,
            )
            .reaction("👍", 7),
            Message::new(
                "Nova",
                "今天 09:12",
                "早上好！今天准备把消息搜索补上，@小满 你昨天说的筛选逻辑我看了，思路没问题。",
                0x00a8fc,
            )
            .pinned(),
            Message::new("Nova", "今天 09:13", "另外表情回应也一起做，先把界面交互跑通。", 0x00a8fc)
                .grouped(),
            Message::new("阿哲", "今天 10:02", "赞，那我把成员列表的状态点也加上。", 0x5865f2),
            Message::new(
                "Lin",
                "今天 11:26",
                "刚把语音频道的图标换成耳机样式了，看起来更接近 Discord。",
                0x57f287,
            )
            .reaction("🔥", 3),
            Message::new(
                "Lin",
                "今天 11:28",
                "设计稿放在 https://example.com/gpui-kit-design 了，有空可以看看。",
                0x57f287,
            ),
            Message::new("小满", "今天 11:38", "", 0xfaa61c).gif(Gif::new(
                "撒花",
                "🎉",
                0xfee75c,
                "热门",
                "庆祝 撒花 party",
            )),
            Message::new(
                "小满",
                "今天 11:40",
                "顺手截了张新的侧边栏效果图，看看层次对不对 👇",
                0xfaa61c,
            )
            .attachment(Attachment::image("sidebar-preview.png", "1.2 MB")),
            Message::new("阿哲", "今天 11:42", "配色稳了，就这个。", 0x5865f2)
                .attachment(Attachment::file("界面走查清单.md", "12 KB")),
            Message::new("小满", "今天 11:45", "", 0xfaa61c)
                .sticker(Sticker::new("🎉", "撒花", "日常")),
            Message::new("Lin", "今天 11:46", "贴纸也能直接发了，点输入框右边的贴纸图标。", 0x57f287),
            Message::new(
                "你",
                "今天 11:50",
                "已读回执也补上了，点状态就能看到谁读过。",
                0x5865f2,
            )
            .read_by(vec![
                ReadReceipt::new("Nova", "今天 11:51", 0x00a8fc),
                ReadReceipt::new("小满", "今天 11:52", 0xfaa61c),
                ReadReceipt::new("阿哲", "今天 11:53", 0x5865f2),
            ]),
            Message::new(
                "Nova",
                "今天 12:00",
                "下周的周会定在哪天？投个票 👇",
                0x00a8fc,
            )
            .poll({
                let mut poll = Poll::new(
                    "下周周会时间",
                    &["周一 10:00", "周三 15:00", "周五 16:00"],
                    false,
                );
                poll.options[0].voters = vec!["小满".into(), "阿哲".into()];
                poll.options[1].voters = vec!["Lin".into()];
                poll
            }),
        ],
    );

    all.insert(
        "开发者社区/前端开发".into(),
        vec![
            Message::new(
                "Lin",
                "今天 08:45",
                "CSS `:has()` 现在主流浏览器都支持了，很多以前要写 JS 的布局可以直接搞定。",
                0x57f287,
            ),
            Message::new(
                "小满",
                "今天 08:52",
                "对，我用它做表单校验高亮，少了一大堆状态。",
                0xfaa61c,
            )
            .reaction("💡", 5),
        ],
    );

    all.insert(
        "开发者社区/Rust 讨论".into(),
        vec![
            Message::new(
                "阿哲",
                "今天 09:30",
                "GPUI 里用 cx.subscribe_in 订阅输入事件很顺手，实体生命周期也不用自己管。",
                0x5865f2,
            ),
            Message::new(
                "Nova",
                "今天 09:41",
                "嗯，渲染和状态分开之后，界面代码清楚很多。",
                0x00a8fc,
            ),
        ],
    );

    all.insert(
        "开发者社区/作品展示".into(),
        vec![Message::new(
            "小满",
            "今天 10:15",
            "周末做了个桌面端记账工具，等整理好仓库地址发出来～",
            0xfaa61c,
        )
        .reaction("👀", 6)],
    );

    all.insert(
        "开发者社区/公告".into(),
        vec![Message::new(
            "管理员",
            "2026/11/18 12:00",
            "社区规则已更新，请大家发言前先阅读 #规则 频道。",
            0xed4245,
        )
        .pinned()
        .reaction("✅", 12)],
    );

    all.insert(
        "设计工坊/灵感".into(),
        vec![
            Message::new("Mia", "今天 09:05", "分享一套低饱和度的配色，做工具类界面很耐看。", 0xeb459e),
            Message::new("小满", "今天 09:20", "收藏了，正好用在下一个项目上。", 0xfaa61c),
        ],
    );

    all
}

/// 服务器角色（顺序即成员栏分组顺序，也是角色列表的展示顺序）。
pub fn seed_roles() -> Vec<Role> {
    vec![
        Role::new("管理员", 0xf23f43),
        Role::new("版主", 0x57f287),
        Role::new("贡献者", 0x00a8fc),
    ]
}

/// 角色权限种子：管理员几乎全部允许，版主有 moderation 权限并禁用管理频道，
/// 贡献者只允许基础互动，其余权限位留空（继承默认）。
pub fn seed_permissions() -> Vec<PermissionGrant> {
    let mut grants = Vec::new();
    let grant_all = |role: &str, keys: &[&str], state: PermissionState, out: &mut Vec<PermissionGrant>| {
        for key in keys {
            out.push(PermissionGrant::new(role, *key, state));
        }
    };

    grant_all(
        "管理员",
        &[
            "view_channels",
            "invite_people",
            "change_nickname",
            "send_messages",
            "embed_links",
            "attach_files",
            "add_reactions",
            "mention_everyone",
            "connect",
            "speak",
            "screen_share",
            "manage_messages",
        ],
        PermissionState::Allow,
        &mut grants,
    );
    grant_all(
        "版主",
        &[
            "view_channels",
            "invite_people",
            "change_nickname",
            "send_messages",
            "embed_links",
            "attach_files",
            "add_reactions",
            "mention_everyone",
            "connect",
            "speak",
            "screen_share",
            "manage_messages",
        ],
        PermissionState::Allow,
        &mut grants,
    );
    grant_all(
        "版主",
        &["manage_channels"],
        PermissionState::Deny,
        &mut grants,
    );
    grant_all(
        "贡献者",
        &[
            "view_channels",
            "send_messages",
            "embed_links",
            "attach_files",
            "add_reactions",
            "connect",
            "speak",
        ],
        PermissionState::Allow,
        &mut grants,
    );
    grant_all(
        "贡献者",
        &["manage_messages", "manage_channels", "mention_everyone"],
        PermissionState::Deny,
        &mut grants,
    );
    grants
}

/// 自动审核种子：默认启用一条拦截广告链接的规则与一条警告刷屏的规则，
/// 另有一条停用规则演示「启用」开关。
pub fn seed_automod_rules() -> Vec<AutoModRule> {
    vec![
        AutoModRule::new(
            "广告链接拦截",
            "拦截常见推广与外链，保护社区免受垃圾广告打扰。",
            &["http://", "https://", "免费领取", "点击链接", "spam"],
            AutoModAction::Block,
            true,
        ),
        AutoModRule::new(
            "刷屏提醒",
            "命中后仍可发送，但会提示注意刷屏。",
            &["刷屏", " flood ", "@@@"],
            AutoModAction::Warn,
            true,
        ),
        AutoModRule::new(
            "不友善用语",
            "命中后拦截人身攻击类词汇（默认停用）。",
            &["笨蛋", "傻子"],
            AutoModAction::Block,
            false,
        ),
    ]
}

/// 前端模拟成员数据。角色按名称归属 `seed_roles()`，None 表示无角色。
pub fn seed_members() -> Vec<Member> {
    vec![
        Member {
            name: "Nova",
            color: 0x00a8fc,
            online: true,
            role: Some("管理员"),
            nick: None,
            joined_minutes: 180 * 24 * 60,
        },
        Member {
            name: "阿哲",
            color: 0x5865f2,
            online: true,
            role: Some("管理员"),
            nick: Some("阿哲（夜猫子）"),
            joined_minutes: 150 * 24 * 60,
        },
        Member {
            name: "小满",
            color: 0xfaa61c,
            online: true,
            role: Some("版主"),
            nick: None,
            joined_minutes: 96 * 24 * 60,
        },
        Member {
            name: "Lin",
            color: 0x57f287,
            online: true,
            role: Some("贡献者"),
            nick: Some("Lin 🌿"),
            joined_minutes: 40 * 24 * 60,
        },
        Member {
            name: "Mia",
            color: 0xeb459e,
            online: false,
            role: None,
            nick: None,
            joined_minutes: 12 * 24 * 60,
        },
        Member {
            name: "Kai",
            color: 0x80848e,
            online: false,
            role: None,
            nick: None,
            joined_minutes: 3 * 24 * 60,
        },
    ]
}

/// 初始封禁列表：演示态，只在本机留存。
pub fn seed_bans() -> Vec<BannedUser> {
    vec![BannedUser {
        name: "raider#4040".into(),
        reason: "刷屏与恶意邀请链接".into(),
        minutes: 6 * 24 * 60,
        by: "Nova".into(),
    }]
}

/// 前端模拟好友数据。
pub fn seed_friends() -> Vec<Friend> {
    vec![
        Friend {
            name: "Nova",
            color: 0x00a8fc,
            online: true,
            activity: "正在使用 GPUI Kit",
            group: "一起打游戏".into(),
        },
        Friend {
            name: "阿哲",
            color: 0x5865f2,
            online: true,
            activity: "正在玩 Factorio",
            group: "一起打游戏".into(),
        },
        Friend {
            name: "小满",
            color: 0xfaa61c,
            online: true,
            activity: "正在听音乐",
            group: "现实朋友".into(),
        },
        Friend {
            name: "Lin",
            color: 0x57f287,
            online: true,
            activity: "在线",
            group: "现实朋友".into(),
        },
        Friend {
            name: "小柯",
            color: 0x9b59b6,
            online: true,
            activity: "正在写周报",
            group: "现实朋友".into(),
        },
        Friend {
            name: "Mia",
            color: 0xeb459e,
            online: false,
            activity: "离线",
            group: "同事".into(),
        },
        Friend {
            name: "Ada",
            color: 0x1abc9c,
            online: false,
            activity: "离线",
            group: "同事".into(),
        },
        // 刻意留一个未分组好友，用来验证「未分组」分组的兜底渲染。
        Friend {
            name: "Kai",
            color: 0x80848e,
            online: false,
            activity: "离线",
            group: String::new(),
        },
    ]
}

/// 好友分组的默认顺序；好友身上出现但这里没有的分组会按出现顺序补在后面。
pub fn seed_friend_groups() -> Vec<String> {
    vec!["一起打游戏".into(), "现实朋友".into(), "同事".into()]
}

/// 前端模拟私信会话数据。
pub fn seed_dm_conversations() -> Vec<DmConversation> {
    vec![
        DmConversation {
            friend: "Nova",
            color: 0x00a8fc,
            online: true,
            unread: 2,
            preview: "那我把表情选择器也加上",
        },
        DmConversation {
            friend: "阿哲",
            color: 0x5865f2,
            online: true,
            unread: 0,
            preview: "编译过了，没问题",
        },
        DmConversation {
            friend: "小满",
            color: 0xfaa61c,
            online: true,
            unread: 0,
            preview: "配色我调好发你",
        },
        DmConversation {
            friend: "Lin",
            color: 0x57f287,
            online: true,
            unread: 0,
            preview: "好的，晚点看",
        },
    ]
}

/// 前端模拟好友请求：两条收到的、一条发出的。
pub fn seed_friend_requests() -> Vec<FriendRequest> {
    vec![
        FriendRequest {
            name: "Zoe".into(),
            color: 0xf0b232,
            incoming: true,
        },
        FriendRequest {
            name: "Ash".into(),
            color: 0x9b59b6,
            incoming: true,
        },
        FriendRequest {
            name: "Rin".into(),
            color: 0x00a8fc,
            incoming: false,
        },
    ]
}

/// 前端模拟私信消息数据，键为好友名。
pub fn seed_dm_messages() -> HashMap<String, Vec<Message>> {
    let mut all = HashMap::new();

    all.insert(
        "Nova".into(),
        vec![
            Message::new("Nova", "今天 09:41", "嗨，私信页面的布局你打算怎么做？", 0x00a8fc),
            Message::new("你", "今天 09:43", "沿用频道那套四栏，把频道栏换成会话列表就行。", 0x5865f2)
                .reaction("👍", 2)
                .revised("频道栏可以直接复用服务器那套，会话列表另做。", "今天 09:41")
                .read_by(vec![ReadReceipt::new("Nova", "今天 09:44", 0x00a8fc)]),
            Message::new("Nova", "今天 09:45", "思路清晰。表情选择器别忘了加，使用率挺高的。", 0x00a8fc),
            Message::new("Nova", "今天 09:46", "那我把表情选择器也加上", 0x00a8fc).grouped(),
        ],
    );

    all.insert(
        "阿哲".into(),
        vec![
            Message::new("阿哲", "昨天 20:10", "昨晚那个渲染问题定位到了吗？", 0x5865f2),
            Message::new("你", "昨天 20:15", "定位到了，是 trait 没导入，编译器提示挺明确的。", 0x5865f2)
                .revised("还没定位到，可能是编译缓存的问题。", "昨天 20:13")
                .delivered(),
            Message::new("阿哲", "昨天 20:16", "编译过了，没问题", 0x5865f2).reaction("🎉", 1),
        ],
    );

    all.insert(
        "小满".into(),
        vec![
            Message::new("小满", "今天 11:02", "新界面的柔和色板我很喜欢 👀", 0xfaa61c),
            Message::new("你", "今天 11:05", "都集中到 palette 里了，改一处全站生效。", 0x5865f2)
                .read_by(vec![ReadReceipt::new("小满", "今天 11:06", 0xfaa61c)]),
            Message::new("小满", "今天 11:06", "配色我调好发你", 0xfaa61c),
        ],
    );

    all.insert(
        "Lin".into(),
        vec![Message::new("Lin", "周一 14:30", "周末的Demo很不错！", 0x57f287)],
    );

    all
}

/// 话题演示数据：键为会话键（频道「服务器/频道」、私信「dm:好友」），
/// root_index 指向该会话主消息列表里的原消息。
pub fn seed_threads() -> HashMap<String, Vec<ChatThread>> {
    let mut all = HashMap::new();

    all.insert(
        "开发者社区/闲聊".into(),
        vec![ChatThread::new(
            3,
            "消息搜索与表情回应",
            vec![
                ThreadReply::new("小满", "今天 09:18", "筛选逻辑我可以补一个按作者过滤的入口。", 0xfaa61c),
                ThreadReply::new("阿哲", "今天 09:20", "回应计数建议本地先加一再等接口，手感会好很多。", 0x5865f2),
                ThreadReply::new("你", "今天 09:22", "收到，先把话题面板这条链路跑通。", 0x5865f2),
            ],
        )],
    );

    all.insert(
        "dm:Nova".into(),
        vec![ChatThread::new(
            1,
            "私信布局讨论",
            vec![ThreadReply::new(
                "Nova",
                "今天 09:47",
                "会话列表和频道栏共用一套侧栏组件就够了。",
                0x00a8fc,
            )],
        )],
    );

    all
}

/// 附件选择器里的模拟文件清单（纯前端，不读真实磁盘）。
pub fn seed_attachments() -> Vec<Attachment> {
    vec![
        Attachment::image("sidebar-preview.png", "1.2 MB"),
        Attachment::image("mockup-dark.png", "860 KB"),
        Attachment::file("design-spec.pdf", "480 KB"),
        Attachment::file("界面走查清单.md", "12 KB"),
        Attachment::file("release-notes.txt", "3 KB"),
    ]
}

/// 探索页的公开服务器候选（纯前端模拟，可加入）。
pub fn seed_discover_servers() -> Vec<DiscoverServer> {
    vec![
        DiscoverServer::new(
            "像素酒馆",
            0xeb459e,
            "游戏",
            12840,
            "独立游戏与像素美术爱好者的据点，每周有联机活动。",
        ),
        DiscoverServer::new(
            "GPU 研究所",
            0x5865f2,
            "科技",
            35620,
            "聊显卡、推理框架与本地大模型部署。",
        ),
        DiscoverServer::new(
            "排版诊所",
            0xfaa61c,
            "设计",
            7420,
            "把设计稿贴上来，一起看看字距和留白。",
        ),
        DiscoverServer::new(
            "深夜电台",
            0x57f287,
            "音乐",
            21400,
            "一起听歌、聊黑胶与合成器，语音房常驻。",
        ),
        DiscoverServer::new(
            "Rust 学堂",
            0xf23f43,
            "学习",
            4680,
            "从所有权讲到异步，作业互评，氛围友好。",
        ),
        DiscoverServer::new(
            "放映厅",
            0x00a8fc,
            "影视",
            18930,
            "每周共看一部电影，映后语音讨论。",
        ),
        DiscoverServer::new(
            "开源咖啡馆",
            0x5865f2,
            "科技",
            9870,
            "维护开源项目的人在这里互相答疑与招人。",
        ),
        DiscoverServer::new(
            "指尖工作台",
            0xeb459e,
            "设计",
            5230,
            "图标、动效与设计系统的日常分享。",
        ),
    ]
}

/// 贴纸弹层里的全部贴纸：按 pack 分成三组，pack 名即弹层里的分类标签。
pub fn seed_stickers() -> Vec<Sticker> {
    vec![
        Sticker::new("😂", "笑到模糊", "日常"),
        Sticker::new("🥹", "泪目", "日常"),
        Sticker::new("🤔", "陷入思考", "日常"),
        Sticker::new("😎", "稳了", "日常"),
        Sticker::new("🥳", "开香槟", "日常"),
        Sticker::new("👍", "收到", "日常"),
        Sticker::new("🎉", "撒花", "日常"),
        Sticker::new("🔥", "上分", "日常"),
        Sticker::new("🐱", "猫猫探头", "猫咪"),
        Sticker::new("😺", "猫猫开心", "猫咪"),
        Sticker::new("😸", "猫猫坏笑", "猫咪"),
        Sticker::new("😹", "猫猫爆笑", "猫咪"),
        Sticker::new("😻", "猫猫心动", "猫咪"),
        Sticker::new("🙀", "猫猫震惊", "猫咪"),
        Sticker::new("😿", "猫猫委屈", "猫咪"),
        Sticker::new("😾", "猫猫生气", "猫咪"),
        Sticker::new("🎁", "礼物", "礼物"),
        Sticker::new("🎂", "生日蛋糕", "礼物"),
        Sticker::new("🌹", "玫瑰", "礼物"),
        Sticker::new("🍰", "来块甜品", "礼物"),
        Sticker::new("🥂", "干杯", "礼物"),
        Sticker::new("💝", "心意", "礼物"),
        Sticker::new("🎈", "庆祝气球", "礼物"),
        Sticker::new("🧸", "抱抱熊", "礼物"),
    ]
}

/// 贴纸包的展示顺序（弹层顶部标签）。
pub fn sticker_packs() -> Vec<&'static str> {
    vec!["日常", "猫咪", "礼物"]
}

/// GIF 弹层里的全部条目：色块 + emoji 字形代替真实动图，category 即分类标签。
pub fn seed_gifs() -> Vec<Gif> {
    vec![
        Gif::new("爆笑", "😂", 0xfee75c, "热门", "笑 哈哈 爆笑 laugh haha"),
        Gif::new("鼓掌", "👏", 0x57f287, "热门", "鼓掌 拍手 赞 clap"),
        Gif::new("点赞", "👍", 0x5865f2, "热门", "赞 好 点赞 thumbs up"),
        Gif::new("震惊", "😱", 0xed4245, "热门", "震惊 惊吓 wow shock"),
        Gif::new("无语", "😑", 0x95a5a6, "热门", "无语 无聊 面无表情 meh"),
        Gif::new("同意", "✅", 0x57f287, "反应", "同意 对 好的 yes ok"),
        Gif::new("拒绝", "❌", 0xed4245, "反应", "拒绝 不要 错 no"),
        Gif::new("生气", "😡", 0xed4245, "反应", "生气 愤怒 angry mad"),
        Gif::new("思考", "🤔", 0x9b59b6, "反应", "思考 想想 think"),
        Gif::new("感谢", "🙏", 0x00b0f4, "反应", "感谢 谢谢 thanks"),
        Gif::new("跳舞", "🕺", 0xeb459e, "搞怪", "跳舞 舞 dance"),
        Gif::new("庆祝", "🎉", 0xfee75c, "搞怪", "庆祝 撒花 party"),
        Gif::new("摸鱼", "🐟", 0x00b0f4, "搞怪", "摸鱼 划水 fish"),
        Gif::new("炸了", "💥", 0xed4245, "搞怪", "爆炸 炸 boom"),
        Gif::new("躺平", "🛌", 0x95a5a6, "搞怪", "躺平 睡觉 sleep"),
        Gif::new("猫猫", "🐱", 0xfee75c, "动物", "猫 猫猫 cat"),
        Gif::new("狗狗", "🐶", 0xe67e22, "动物", "狗 狗狗 dog"),
        Gif::new("熊猫", "🐼", 0x1abc9c, "动物", "熊猫 panda"),
        Gif::new("兔子", "🐰", 0xeb459e, "动物", "兔子 rabbit"),
        Gif::new("企鹅", "🐧", 0x5865f2, "动物", "企鹅 penguin"),
    ]
}

/// GIF 分类的展示顺序（弹层顶部标签）。
pub fn gif_categories() -> Vec<&'static str> {
    vec!["热门", "反应", "搞怪", "动物"]
}

/// 帮助中心的分类顺序（左侧导航）。
pub fn help_categories() -> Vec<&'static str> {
    vec!["入门", "消息", "会话", "语音", "设置"]
}

/// 帮助中心的本地演示文档：内容与当前已实现的功能一一对应。
pub fn seed_help_articles() -> Vec<HelpArticle> {
    vec![
        HelpArticle::new(
            "ui-tour",
            "入门",
            "界面速览：三栏都在做什么",
            "从左到右依次是服务器栏、侧栏与聊天区，搞清楚这三块就入门了。",
            &[
                "最左侧是服务器栏：每个圆形图标是一个服务器，底部 + 号用来创建新服务器，指南针进入「探索」页面。",
                "第二栏是侧栏：服务器模式下按分类列出文字频道与语音频道，私信模式下则换成会话列表与好友入口。",
                "右侧是聊天区：顶部标题栏放频道名与各类开关，中间是消息列表，底部是输入条。",
            ],
            &[
                "点击服务器图标即可切换服务器，当前服务器左侧会显示白色指示条",
                "按 Ctrl/⌘ + K 打开快速切换器，输入频道或私信名后回车直达",
                "点击左下角头像旁的齿轮打开用户设置",
            ],
        ),
        HelpArticle::new(
            "servers",
            "入门",
            "创建服务器、频道与分类",
            "服务器装频道，频道装消息；分类只是给频道分组。",
            &[
                "创建服务器：点服务器栏底部的 + 号，填写名称与简介即可。新服务器会自带一组默认频道。",
                "创建频道：悬停分类标题会出现 + 号，可选择创建文字频道或语音频道，并指定所属分类。",
                "右键频道或分类可以重命名、调整顺序或删除；删除频道会连同该频道的消息一起移除。",
            ],
            &[
                "频道名支持中文与空格，标题栏会原样展示",
                "语音频道在侧栏里用喇叭图标区分，点一下即加入",
                "服务器设置里可以改名称、简介、查看角色与表情",
            ],
        ),
        HelpArticle::new(
            "composer",
            "消息",
            "发送消息与输入条用法",
            "Enter 发送、Shift + Enter 换行，输入条右侧还有四种内容入口。",
            &[
                "在输入框里打字，按 Enter 发送，按 Shift + Enter 换行。输入 @ 会弹出成员补全，选中后插入提及。",
                "输入条右侧依次是附件、贴纸、礼物、GIF、表情五个入口：附件会生成文件卡片，贴纸与 GIF 点一下即发送，表情会插入到光标处。",
                "未发送的内容会按会话分别保存为草稿，切走再切回来仍在原处，侧栏对应频道会显示草稿标记。",
            ],
            &[
                "发送后消息列表自动滚到底部",
                "消息里出现 @你的名字 时会记入收件箱的提及列表",
                "附件与 GIF 都是本地点位数据，不会真的上传或联网",
            ],
        ),
        HelpArticle::new(
            "reactions",
            "消息",
            "表情回应、贴纸与 GIF",
            "回应做情绪反馈，贴纸和 GIF 用来代替文字。",
            &[
                "悬停消息会出现工具栏，点笑脸添加表情回应；同一表情多人回应会合并计数，再点一次可取消自己的回应。",
                "贴纸入口按「日常 / 猫咪 / 礼物」分包，点任意贴纸立即作为一条消息发出；GIF 面板支持按标题或标签跨分类搜索。",
                "GIF 有热门、反应、搞怪、动物四类，回车直接发送搜索到的第一张。",
            ],
            &[
                "回应计数会跟随消息一起显示在正文下方",
                "转发贴纸或 GIF 消息时，摘要会写成 [贴纸] / [GIF] + 名称",
                "当前贴纸与 GIF 都是色块加 emoji 的占位渲染，不加载真实图片",
            ],
        ),
        HelpArticle::new(
            "search",
            "消息",
            "搜索与高级过滤语法",
            "放大镜进入搜索中心，除了关键词还能用 from: / has: / after: / before: 精确筛选。",
            &[
                "点标题栏放大镜进入搜索中心，可把范围切到当前服务器、全部服务器、私信或全部，命中文字会高亮，点结果直接跳过去。",
                "from:作者 只看某人发的消息，可以写多个取并集；has: 支持 image、file、attachment、sticker、gif、link、reaction、edited、mention、pinned，多个取交集。",
                "after:today / before:yesterday 这类时间前缀可以限定消息时间；只有过滤条件、不写关键词也能出结果。搜索栏下方会列出已应用的条件，点标签即移除。",
            ],
            &[
                "示例：from:小满 has:image 侧边栏",
                "示例：has:gif 庆祝",
                "时间过滤基于消息时间的「今天 / 昨天」前缀，不是真实日期区间",
            ],
        ),
        HelpArticle::new(
            "edit-thread",
            "消息",
            "编辑、话题与转发",
            "改错字用编辑，展开讨论用话题，搬到别处用转发。",
            &[
                "只有自己发的消息能编辑：右键或在省略号菜单里选「编辑消息」，改完 Enter 保存、Esc 取消。正文确实变化时，时间戳旁会出现「（已编辑）」，点开可看历史版本。",
                "话题适合把一条消息的讨论单独拉出来：工具栏的话题按钮或右键菜单都能创建，右侧面板里有原消息、回复列表和独立输入框，回车发送。",
                "转发会打开目标选择器，可搜索频道或私信，转发后的消息带来源标记，点来源可跳回原会话。",
            ],
            &[
                "话题按会话隔离保存，切会话后不会串到别的频道",
                "删除话题的根消息会同时删掉整个话题",
                "历史版本只展示，不提供回滚",
            ],
        ),
        HelpArticle::new(
            "friends-dm",
            "会话",
            "好友与私信",
            "好友页管理关系，私信侧栏发起一对一会话。",
            &[
                "点服务器栏的「好友」图标进入好友页，可切换在线、全部、待处理等分组；收到的请求可以接受或忽略，接受后进入好友列表。",
                "私信侧栏顶部有真实搜索框，同时过滤会话、好友，也可以直接搜名字发起新对话。",
                "点消息里的作者名或成员行会打开资料卡，里面能发消息、发起通话，也能看到在线状态与共同服务器。",
            ],
            &[
                "会话有未读时侧栏名字右侧显示红点计数",
                "资料卡点遮罩或 Esc 关闭",
                "当前没有真实网络层，好友请求与在线状态都是本地演示数据",
            ],
        ),
        HelpArticle::new(
            "unread",
            "会话",
            "通知、未读与收件箱",
            "收件箱把所有需要你注意的事聚到一处。",
            &[
                "标题栏铃铛打开收件箱，提及、未读频道、私信与好友请求会按类型聚合，点条目直接跳到对应位置。",
                "进入有未读的频道时，消息列表里会出现「新消息」分隔线，被 @ 的消息会高亮；滚上去之后右下角会出现「跳到最新」浮标。",
                "标题栏还有独立的通知开关，关掉后铃铛变红，表示当前会话不再提醒。",
            ],
            &[
                "未读数来自本地模拟状态，不会随真实消息推送变化",
                "Esc 可逐层关闭收件箱与其他浮层",
                "设置里的通知页可以控制桌面提醒与提示音开关",
            ],
        ),
        HelpArticle::new(
            "voice-channel",
            "语音",
            "加入服务器语音频道",
            "点语音频道即连接，左下角会出现语音状态条。",
            &[
                "点击侧栏里的语音频道即可加入，右侧会打开语音房间面板，列出当前参与者与各自的麦克风状态。",
                "面板里可以切换静音、关闭耳机（Deafen）；底部语音状态条也会同步显示当前频道与两个开关，点挂断图标断开连接。",
                "加入语音频道与私信通话互斥：进入语音频道会自动结束正在进行的私信通话。",
            ],
            &[
                "切换频道时语音连接会保持在原频道，直到手动断开",
                "语音房间面板可随时用标题栏的喇叭图标重新打开",
                "当前不采集真实音频，只演示状态流转",
            ],
        ),
        HelpArticle::new(
            "dm-call",
            "语音",
            "私信语音与视频通话",
            "在私信里点电话或摄像头按钮就能发起通话。",
            &[
                "私信标题栏有电话与摄像头两个按钮，资料卡里也有同样的一对入口；语音通话与视频通话走同一个浮窗。",
                "浮窗先进入「呼叫中」，点「模拟接通」进入通话态，此时可以切换麦克风、摄像头与屏幕分享，视频或共享时浮窗会展开预览区。",
                "点挂断或浮窗右上角关闭即可结束通话；浮窗挂在最外层，切频道、切私信、进设置都不会消失。",
            ],
            &[
                "通话浮窗始终位于右下角，不遮挡输入条",
                "视频与屏幕分享是占位预览，不采集真实画面",
                "发起私信通话会断开服务器语音连接",
            ],
        ),
        HelpArticle::new(
            "appearance",
            "设置",
            "外观、主题与消息显示",
            "深色、浅色、跟随系统三选一，消息显示项可以逐条开关。",
            &[
                "设置的外观页可以切换深色、浅色与跟随系统；主题色板集中在一处，改一处全站生效。",
                "「文字与图片」页控制消息区的显示细节，例如是否显示时间戳、是否默认展开成员列表。",
                "语言页目前提供简体中文与 English 的展示项，界面文案仍以中文为准。",
            ],
            &[
                "跟随系统只在点选时读取一次窗口外观",
                "浅色主题下所有色值都有对应的浅色版本",
                "消息密度与字号暂未做成可调项",
            ],
        ),
        HelpArticle::new(
            "account",
            "设置",
            "账户资料与头像",
            "显示名称、用户名、邮箱与「关于我」都能改，头像颜色点一下就换。",
            &[
                "设置 → 账户顶部是资料预览：头像、显示名称、用户名、「关于我」与当前在线状态，改完立刻同步到这里。",
                "下面依次是显示名称、用户名、邮箱与「关于我」四个输入框。显示名称与用户名必填，留空保存会提示失败；邮箱与简介可以留空。",
                "头像颜色是 8 个色块，点一下立即生效，不需要点保存。改完点「保存改动」会同步到底部用户栏，「重置」则丢弃这次编辑。",
            ],
            &[
                "显示名称会显示在侧栏底部用户栏与资料预览里",
                "已经发出的消息不会跟着改名，仍保留当时的署名",
                "资料只存在内存里，重启应用恢复为 Riven",
            ],
        ),
        HelpArticle::new(
            "shortcuts",
            "设置",
            "快捷键与隐私",
            "常用按键只有几个，记住 Esc 和 Ctrl/⌘ + K 就够用。",
            &[
                "Enter 发送消息或保存编辑，Shift + Enter 换行，Ctrl/⌘ + K 打开快速切换器，↑/↓ 在结果里循环选择。",
                "Esc 按「最上层优先」逐层关闭：先关编辑态与浮层，再关话题面板、设置页，最后才轮到页面本身。",
                "隐私与安全页提供两个全局开关，分别控制陌生人私信与在线状态展示。",
            ],
            &[
                "右键消息可以打开完整操作菜单",
                "悬停消息左侧工具栏包含回应、回复、话题与更多",
                "设置页的快捷键列表与这里保持一致",
            ],
        ),
    ]
}

/// 当前用户的初始资料（账户页可改，重启后恢复这份）。
pub fn seed_me() -> MyProfile {
    MyProfile::new(
        "Riven",
        "riven#0001",
        "riven@example.com",
        "专注界面与交互，喜欢把细节打磨到顺滑。",
        0x5865f2,
    )
}

/// 服务器邀请的种子数据：一条有效、一条已用完、一条已过期，覆盖三种状态。
pub fn seed_invites() -> Vec<Invite> {
    vec![
        Invite {
            code: "dev-community".into(),
            server: "开发者社区".into(),
            channel: "闲聊".into(),
            inviter: "Riven",
            uses: 6,
            max_uses: 0,
            expiry: InviteExpiry::Never,
            created_minutes: 3 * 24 * 60,
        },
        Invite {
            code: "dev-weekly-5".into(),
            server: "开发者社区".into(),
            channel: "作品展示".into(),
            inviter: "Nova",
            uses: 5,
            max_uses: 5,
            expiry: InviteExpiry::SevenDays,
            created_minutes: 2 * 24 * 60,
        },
        Invite {
            code: "dev-tmp-30m".into(),
            server: "开发者社区".into(),
            channel: "闲聊".into(),
            inviter: "阿哲",
            uses: 1,
            max_uses: 10,
            expiry: InviteExpiry::ThirtyMinutes,
            created_minutes: 96,
        },
        Invite {
            code: "rust-lang".into(),
            server: "Rust 学习小组".into(),
            channel: "日常交流".into(),
            inviter: "Riven",
            uses: 2,
            max_uses: 25,
            expiry: InviteExpiry::OneDay,
            created_minutes: 40,
        },
    ]
}

/// 审核日志的初始条目：`seq` 升序，最大的一条是「刚刚」。
pub fn seed_audit() -> Vec<AuditEntry> {
    vec![
        AuditEntry::new(
            1,
            AuditKind::Server,
            "Riven",
            "创建了服务器",
            "开发者社区",
        ),
        AuditEntry::new(2, AuditKind::Channel, "Riven", "创建了频道", "#闲聊"),
        AuditEntry::new(3, AuditKind::Channel, "阿哲", "创建了频道", "#作品展示"),
        AuditEntry::new(4, AuditKind::Role, "Nova", "创建了角色", "管理员"),
        AuditEntry::new(
            5,
            AuditKind::Emoji,
            "小满",
            "移除了表情",
            ":party_blob:",
        ),
        AuditEntry::new(
            6,
            AuditKind::Role,
            "Nova",
            "给成员授予了角色",
            "Lin → 贡献者",
        ),
        AuditEntry::new(
            7,
            AuditKind::Invite,
            "阿哲",
            "创建了邀请",
            "discord.gg/dev-tmp-30m · #闲聊",
        ),
        AuditEntry::new(
            8,
            AuditKind::Server,
            "Riven",
            "更新了服务器简介",
            "欢迎分享作品与踩坑记录",
        ),
        AuditEntry::new(
            9,
            AuditKind::Member,
            "Nova",
            "封禁了成员",
            "raider#4040 · 刷屏与恶意邀请链接",
        ),
        AuditEntry::new(
            10,
            AuditKind::Member,
            "阿哲",
            "将成员移出了服务器",
            "旧号#0002",
        ),
    ]
}
