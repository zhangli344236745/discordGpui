# Spec Delta

## Purpose

提升客户端到可发布水平：可安装、可离线、可恢复、可在多设备间保持一致状态，并满足基本性能预算。

## ADDED Requirements

### Requirement: 可安装发布

客户端 SHALL 提供可分发的安装包或可执行产物，首次启动 MUST 可完成登录与主界面，无需开发者命令。

#### Scenario: 安装启动

- **WHEN** 用户安装后首次打开
- **THEN** 出现登录页并可正常登录

#### Scenario: 升级

- **WHEN** 升级到新版本
- **THEN** 旧配置与会话按兼容策略迁移

### Requirement: 离线缓存

客户端 SHALL 缓存最近会话、频道树与消息，断网时 MUST 可查看缓存内容并明确显示离线状态。

#### Scenario: 断网浏览

- **WHEN** 网络断开
- **THEN** 最近会话仍可阅读，输入被阻止或标记为待发送

#### Scenario: 恢复联网

- **WHEN** 网络恢复
- **THEN** 客户端自动同步缺失内容

### Requirement: 弱网重连

客户端 SHALL 对 REST 与 Gateway 使用指数退避重连，恢复后 MUST 通过游标补齐事件，不重复消息。

#### Scenario: 断线补偿

- **WHEN** 断线 30 秒后恢复
- **THEN** 事件按序补齐且无重复

#### Scenario: 长时间断线

- **WHEN** 超过补偿窗口
- **THEN** 客户端提示重新同步并重建状态

### Requirement: 多设备一致性

同一账号多端登录时 SHALL 同步未读、草稿、书签与设置。一端操作 MUST 在另一端可见。

#### Scenario: 多端书签

- **WHEN** 一端保存书签
- **THEN** 另一端收到更新并展示

#### Scenario: 多端退出

- **WHEN** 一端退出账号
- **THEN** 其他端按会话策略处理并显示登录页

### Requirement: 性能预算

客户端 SHALL 满足基本性能预算：冷启动 ≤ 3 秒、频道切换 ≤ 300ms、千条消息滚动不卡顿。回归 MUST 用脚本验证。

#### Scenario: 冷启动

- **WHEN** 启动到可交互
- **THEN** 常规机器耗时不超过 3 秒

#### Scenario: 大会话滚动

- **WHEN** 打开千条消息频道
- **THEN** 滚动保持流畅，无内存持续增长
