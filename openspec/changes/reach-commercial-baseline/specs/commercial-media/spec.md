# Spec Delta

## Purpose

打通真实附件链路：客户端选择文件、后端接收与校验、对象存储保存、消息内可预览，并具备配额与清理能力。

## ADDED Requirements

### Requirement: 客户端真实上传

客户端 SHALL 选择本地文件并上传到后端，上传中 MUST 显示进度，失败 MUST 可重试。消息发送 MUST 引用已上传附件 id。

#### Scenario: 上传并发送

- **WHEN** 用户选择图片并发送
- **THEN** 附件上传成功，消息展示真实文件预览

#### Scenario: 上传失败重试

- **WHEN** 网络中断后点击重试
- **THEN** 不重复创建消息，附件只保留一份

### Requirement: 类型与大小校验

后端 SHALL 校验 MIME 与文件大小，拒绝超限或伪装类型。上限 MUST 可配置。

#### Scenario: 超过大小

- **WHEN** 上传超过配置上限
- **THEN** 返回 413，不保存文件

#### Scenario: 伪装类型

- **WHEN** 扩展名为图片但内容非图片
- **THEN** 后端拒绝并记录失败

### Requirement: 预览与缩略图

图片 SHALL 生成缩略图，视频 MUST 至少返回可播放 URL。消息内 MUST 展示文件名、大小与下载链接。

#### Scenario: 图片预览

- **WHEN** 消息包含图片附件
- **THEN** 客户端展示缩略图，点击打开原图

#### Scenario: 文件卡片

- **WHEN** 附件为普通文件
- **THEN** 展示类型、大小、下载按钮

### Requirement: 配额与清理

系统 SHALL 按用户或服务器限制附件总量，并定期清理未引用孤儿文件。

#### Scenario: 超过配额

- **WHEN** 用户存储超过配额
- **THEN** 拒绝新上传并提示可清理空间

#### Scenario: 孤儿清理

- **WHEN** 附件超过保留期且未挂到消息
- **THEN** 后台删除存储对象与元数据
