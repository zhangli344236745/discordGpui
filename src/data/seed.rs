use std::collections::HashMap;

use super::models::{
    Attachment, Category, Channel, DiscoverServer, DmConversation, Friend, FriendRequest, Member,
    Message, Server,
};

/// 前端模拟服务器数据。
pub fn seed_servers() -> Vec<Server> {
    vec![
        Server {
            name: "开发者社区".into(),
            badge: "开".into(),
            color: 0x5865f2,
            muted: false,
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
                        Channel::text("闲聊", "日常摸鱼、随便聊聊").unread(3),
                        Channel::text("前端开发", "React / Vue / CSS 话题"),
                        Channel::text("Rust 讨论", "所有权、异步与生态"),
                        Channel::text("作品展示", "晒一晒最近做的东西"),
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
            muted: false,
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
            muted: false,
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
            muted: false,
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
                "小满",
                "今天 11:40",
                "顺手截了张新的侧边栏效果图，看看层次对不对 👇",
                0xfaa61c,
            )
            .attachment(Attachment::image("sidebar-preview.png", "1.2 MB")),
            Message::new("阿哲", "今天 11:42", "配色稳了，就这个。", 0x5865f2)
                .attachment(Attachment::file("界面走查清单.md", "12 KB")),
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

/// 前端模拟成员数据。
pub fn seed_members() -> Vec<Member> {
    vec![
        Member {
            name: "Nova —— 管理员",
            color: 0x00a8fc,
            online: true,
        },
        Member {
            name: "阿哲 —— 管理员",
            color: 0x5865f2,
            online: true,
        },
        Member {
            name: "小满 —— 版主",
            color: 0xfaa61c,
            online: true,
        },
        Member {
            name: "Lin",
            color: 0x57f287,
            online: true,
        },
        Member {
            name: "Mia",
            color: 0xeb459e,
            online: false,
        },
        Member {
            name: "Kai",
            color: 0x80848e,
            online: false,
        },
    ]
}

/// 前端模拟好友数据。
pub fn seed_friends() -> Vec<Friend> {
    vec![
        Friend {
            name: "Nova",
            color: 0x00a8fc,
            online: true,
            activity: "正在使用 GPUI Kit",
        },
        Friend {
            name: "阿哲",
            color: 0x5865f2,
            online: true,
            activity: "正在玩 Factorio",
        },
        Friend {
            name: "小满",
            color: 0xfaa61c,
            online: true,
            activity: "正在听音乐",
        },
        Friend {
            name: "Lin",
            color: 0x57f287,
            online: true,
            activity: "在线",
        },
        Friend {
            name: "Mia",
            color: 0xeb459e,
            online: false,
            activity: "离线",
        },
        Friend {
            name: "Kai",
            color: 0x80848e,
            online: false,
            activity: "离线",
        },
    ]
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
                .reaction("👍", 2),
            Message::new("Nova", "今天 09:45", "思路清晰。表情选择器别忘了加，使用率挺高的。", 0x00a8fc),
            Message::new("Nova", "今天 09:46", "那我把表情选择器也加上", 0x00a8fc).grouped(),
        ],
    );

    all.insert(
        "阿哲".into(),
        vec![
            Message::new("阿哲", "昨天 20:10", "昨晚那个渲染问题定位到了吗？", 0x5865f2),
            Message::new("你", "昨天 20:15", "定位到了，是 trait 没导入，编译器提示挺明确的。", 0x5865f2),
            Message::new("阿哲", "昨天 20:16", "编译过了，没问题", 0x5865f2).reaction("🎉", 1),
        ],
    );

    all.insert(
        "小满".into(),
        vec![
            Message::new("小满", "今天 11:02", "新界面的柔和色板我很喜欢 👀", 0xfaa61c),
            Message::new("你", "今天 11:05", "都集中到 palette 里了，改一处全站生效。", 0x5865f2),
            Message::new("小满", "今天 11:06", "配色我调好发你", 0xfaa61c),
        ],
    );

    all.insert(
        "Lin".into(),
        vec![Message::new("Lin", "周一 14:30", "周末的Demo很不错！", 0x57f287)],
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
