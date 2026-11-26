# Spec Delta

## Purpose

建立商用级账号安全基线：真实邮件验证、找回密码、可选双因素认证、会话撤销与审计，使账号体系达到可对外运营标准。

## ADDED Requirements

### Requirement: 邮箱验证与找回密码

系统 SHALL 通过邮件发送验证码与重置链接，并要求用户在有效期完成操作。邮件服务 MUST 可配置，测试模式 MUST 可拦截输出。

#### Scenario: 注册验证

- **WHEN** 用户注册并提交验证码
- **THEN** 账号标记为已验证，重复验证幂等

#### Scenario: 重置密码

- **WHEN** 用户通过邮件链接设置新密码
- **THEN** 旧密码失效，全部会话撤销

### Requirement: 双因素认证

系统 SHALL 支持可选 TOTP 双因素认证。启用后未提供验证码 MUST 拒绝登录。

#### Scenario: 启用 2FA

- **WHEN** 用户绑定 TOTP 并下次登录
- **THEN** 缺少验证码时登录失败

#### Scenario: 恢复码

- **WHEN** 用户使用恢复码登录
- **THEN** 登录成功且该恢复码失效

### Requirement: 会话与令牌撤销

系统 SHALL 支持列出、撤销单个会话与撤销全部会话。密码修改 MUST 使其他会话失效。

#### Scenario: 撤销全部会话

- **WHEN** 用户触发全局退出
- **THEN** 所有 refresh token 失效，Gateway 连接关闭

#### Scenario: 单设备撤销

- **WHEN** 用户撤销某个设备会话
- **THEN** 该设备后续刷新失败，其他设备不受影响

### Requirement: 安全审计

系统 SHALL 记录登录成功/失败、密码修改、2FA 变更与会话撤销，并支持导出。日志 MUST 不包含明文密码或完整 token。

#### Scenario: 登录失败审计

- **WHEN** 密码错误
- **THEN** 生成安全审计记录，不记录密码

#### Scenario: 导出审计

- **WHEN** 管理员导出安全审计
- **THEN** 得到带时间、IP、设备与结果的结构化数据
