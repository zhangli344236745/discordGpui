# Tasks

## 1. P0 部署基线

- [x] 1.1 新增 `EventBus` 抽象与内存实现，改造 Gateway 广播调用；验证单实例 e2e 仍通过
- [x] 1.2 新增 Redis Pub/Sub EventBus 实现与 `REDIS_URL` 配置；双实例脚本验证跨实例消息实时到达
- [x] 1.3 新增 `Storage` 抽象，现有本地磁盘实现接入；验证附件上传/下载回归通过
- [x] 1.4 新增 S3/MinIO Storage 实现与 `S3_*` 配置；容器环境验证上传后可下载
- [x] 1.5 验证 Postgres URL 可启动并跑迁移；补双后端迁移测试或脚本
- [x] 1.6 编写 `docker-compose.yml` 与 `.env.production.example`；一键启动通过健康检查
- [x] 1.7 编写备份/恢复脚本（DB + 对象存储）并在空容器演练成功

## 2. P1 身份安全

- [x] 2.1 新增 `Mailer` 抽象与测试/SMTP 实现；单测验证不泄露收件内容
- [x] 2.2 实现邮箱验证发送与确认；验证重复确认幂等、过期失效
- [x] 2.3 实现找回密码邮件与新密码设置；验证旧会话全部撤销
- [x] 2.4 实现 TOTP 2FA 绑定、登录校验与恢复码；验证缺码拒登、恢复码一次性
- [x] 2.5 实现安全审计表与登录/密码/2FA/会话事件记录；验证不含敏感明文
- [x] 2.6 实现会话列表、单会话撤销、全局退出；验证 Gateway 即时断开
- [x] 2.7 身份安全 pytest 全覆盖并通过 e2e

## 3. P2 媒体管线

- [x] 3.1 客户端真实文件选择与上传（系统文件对话框 + 上传后发送 + 失败可在消息菜单重试）；验证图片消息展示真实附件
- [x] 3.2 后端 MIME sniff 与大小校验；验证超限 413、伪装类型拒绝
- [x] 3.3 图片缩略图生成与消息预览；验证点击打开原图
- [x] 3.4 附件配额与清理任务；验证超配额拒绝、孤儿对象删除
- [x] 3.5 媒体 pytest 与客户端 e2e 通过

## 4. P3 真实语音

- [x] 4.1 引入 LiveKit Rust SDK（voice-livekit feature）+ VoiceProvider 抽象；未启用/构建失败时回退仅信令
- [x] 4.2 客户端连接语音频道：真实房间 + 麦克风发布 + 平台 ADM 自动播放远端音频（scripts/voice-e2e.sh 验证）
- [x] 4.3 麦克风/摄像头/屏幕共享：真实采集发布、频道与私信双入口开关、服务端状态同步、远端画面渲染（BGRA → RenderImage）
- [x] 4.4 设备选择与权限缺失提示：设置页「设备」行循环切换（引擎 PlatformAudio 枚举/切换），权限拒绝提示且不崩溃
- [x] 4.5 服务器侧静音/移除走 RoomService（LIVEKIT_ROOMSERVICE_ENABLED 开关，不可达回退 Demo 投影）；验证 webhook 状态回写
- [x] 4.6 弱网自动重连：SDK Reconnecting/Reconnected 事件 + 信令层参与者校准；验证重连后无重复参与者
- [x] 4.7 私信真实通话：呼叫/接听返回 token，双端加入 dm: 房间并发布麦克风（scripts/dm-call-e2e.sh 验证）

## 5. P4 机器人与集成

- [x] 5.1 数据模型：bots、bot_tokens、webhooks、slash_commands；迁移可执行
- [x] 5.2 Bot 账号创建/邀请/角色与 BOT 标识；验证 Bot 发消息显示 BOT 标记
- [x] 5.3 Bot token 签发、撤销、限流；验证撤销后 401、超限 429
- [x] 5.4 Webhook 创建/签名/推送/撤销；验证外部 POST 产生消息
- [x] 5.5 斜杠命令注册、权限与执行；验证 `/help` 与无权限拒绝
- [x] 5.6 Bot/Webhook pytest 与示例脚本通过

## 6. P5 合规与举报

- [x] 6.1 数据模型：reports、report_actions、retention_rules、legal_holds；迁移可执行
- [x] 6.2 消息/用户/服务器举报与去重合并；验证处理者收到通知
- [x] 6.3 举报处理状态机与通知；验证删除内容后举报关闭
- [x] 6.4 内容保留策略与法务保留；验证到期清理且 legal hold 跳过
- [x] 6.5 用户数据导出与账号删除；验证导出完整、删除后不可识别
- [x] 6.6 合规 pytest 通过

## 7. P6 客户端可靠性

- [x] 7.1 macOS 安装包/可执行发布；验证全新机器可启动登录
- [x] 7.2 本地缓存最近会话与频道树；断网可读、联网自动同步
- [x] 7.3 REST/Gateway 统一指数退避重连与游标补齐；验证断线 30s 无重复
- [x] 7.4 多设备未读/草稿/书签同步（DRAFT_UPDATE 事件 + 首屏拉取 + 跨设备映射；书签走 BOOKMARK_UPDATE）；验证一端操作另一端可见
- [x] 7.5 性能脚本：冷启动、频道切换、千条消息滚动；记录基线并达标
- [x] 7.6 客户端 e2e 扩展离线与多端场景

## 8. P7 运维可观测

- [x] 8.1 Prometheus 指标端点与核心指标；验证可抓取
- [x] 8.2 JSON 日志与 request_id 贯穿；验证敏感字段脱敏
- [x] 8.3 分布式追踪（W3C traceparent 透传与回写、trace_id/span_id 进 JSON 日志、事件与 RoomService 调用透传）；OTLP 采集器接入留作部署侧可选项
- [x] 8.4 GitHub Actions CI：lint/test/build/image/deploy dry-run
- [x] 8.5 压测脚本与容量报告；输出 P95 与稳定吞吐
- [x] 8.6 备份定时调度与恢复演练自动化
- [x] 8.7 文档更新：生产部署、容量、告警、回滚
