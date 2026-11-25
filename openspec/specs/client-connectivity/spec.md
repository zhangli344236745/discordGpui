# client-connectivity Specification

## Purpose

定义客户端与后端之间的连接基础：地址配置、登录态、REST 调用、token 刷新、错误呈现与离线回退。它决定界面能否在真实后端和本地演示模式之间可靠切换。

## Requirements

### Requirement: 后端地址与启动模式

客户端 SHALL 通过 `DISCORD_API_BASE` 环境变量或设置项确定后端地址，默认 `http://127.0.0.1:8080`。启动时 MUST 探测 `GET /api/v1/health`：可达进入在线模式；不可达或未配置时 MUST 进入离线演示模式，并在界面明确显示当前模式。

#### Scenario: 在线启动

- **WHEN** 客户端启动且 health 返回 `status=ok`
- **THEN** 显示登录界面，后续数据全部来自后端

#### Scenario: 后端不可达

- **WHEN** health 请求超时或连接被拒绝
- **THEN** 进入离线演示模式，加载本地种子/快照，并显示「离线演示」标识，不弹出登录

#### Scenario: 地址变更

- **WHEN** 用户把 API 地址改为另一个可达实例并重启
- **THEN** 新地址生效，旧 token 不发送到新主机

### Requirement: 登录、注册与退出

客户端 SHALL 支持邮箱 + 密码登录、邮箱 + 密码 + 用户名注册，以及退出登录。登录成功后 MUST 在内存保存 access/refresh token；退出 MUST 调用 `/auth/logout` 并清空 token、用户缓存与实时连接。

#### Scenario: 登录成功

- **WHEN** 用户输入正确邮箱与密码
- **THEN** 进入主界面并开始首屏同步，窗口标题/用户栏显示真实资料

#### Scenario: 密码错误

- **WHEN** 后端返回 401 `UNAUTHORIZED`
- **THEN** 登录表单显示「邮箱或密码不正确」，不进入主界面，不保留任何 token

#### Scenario: 登录限流

- **WHEN** 后端返回 429 且带 `retry_after`
- **THEN** 登录按钮禁用并在提示中显示剩余秒数，倒计时结束后恢复

#### Scenario: 退出登录

- **WHEN** 用户点击退出
- **THEN** 调用登出接口、关闭 WebSocket、清空内存状态并回到登录界面

### Requirement: Token 刷新与 401 处理

access token 过期时客户端 MUST 使用 refresh token 静默刷新并重放原请求一次。刷新失败（refresh 过期或已撤销）MUST 清理会话并回到登录界面，不得循环重试。

#### Scenario: 静默刷新

- **WHEN** 任意 REST 请求返回 401 `TOKEN_EXPIRED`
- **THEN** 客户端刷新 token 后重发一次，用户无感，界面不跳转

#### Scenario: 刷新失效

- **WHEN** `/auth/refresh` 返回 401
- **THEN** 客户端清空会话、断开 WS、回到登录界面并提示「登录已过期」

#### Scenario: 并发请求

- **WHEN** 多个请求同时遇到过期
- **THEN** 只发起一次刷新，其余请求等待刷新结果后重放

### Requirement: 后台网络执行与错误映射

所有网络 I/O MUST 在 UI 线程之外执行，UI 只消费结果。REST 超时 SHALL 为 10 秒；失败 MUST 转换成可展示错误（含后端 `error.code` 与 `request_id`），不得 panic 或阻塞界面。

#### Scenario: 请求超时

- **WHEN** 请求超过 10 秒无响应
- **THEN** 界面显示「网络超时，请重试」，当前输入与草稿保留

#### Scenario: 业务错误

- **WHEN** 后端返回 `403 MISSING_PERMISSION`
- **THEN** 界面提示缺少的权限位，不修改本地状态

#### Scenario: 服务端 5xx

- **WHEN** 后端返回 500
- **THEN** 显示通用错误与 `request_id`，客户端保持可继续操作

### Requirement: 凭据安全

密码 MUST NOT 写入磁盘或日志。refresh token 默认只存内存；启用「记住我」时 MUST 写入权限为 `0600` 的本地文件，退出登录时删除。

#### Scenario: 默认不落盘

- **WHEN** 用户未勾选「记住我」并关闭客户端
- **THEN** 磁盘上没有 refresh token，下次启动要求重新登录

#### Scenario: 记住我

- **WHEN** 用户勾选「记住我」并登录
- **THEN** refresh token 写入 0600 文件，下次启动自动刷新并进入主界面

#### Scenario: 退出清理

- **WHEN** 用户退出登录
- **THEN** 内存与磁盘上的 token 都被删除
