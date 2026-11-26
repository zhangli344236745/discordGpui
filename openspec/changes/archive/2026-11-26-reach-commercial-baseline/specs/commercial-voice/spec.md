# Spec Delta

## Purpose

建立真实音视频与屏幕共享链路：LiveKit 房间生命周期、客户端采集与渲染、设备控制、弱网重连与服务器侧治理。

## ADDED Requirements

### Requirement: 房间生命周期

系统 SHALL 在用户加入语音频道或接受私信通话时创建 LiveKit 房间，在空房或结束时回收。房间名与权限 MUST 由服务端下发。

#### Scenario: 加入频道语音

- **WHEN** 用户点击语音频道
- **THEN** 客户端连接 LiveKit 房间并显示本端与远端参与者

#### Scenario: 空房回收

- **WHEN** 最后一个参与者离开超过 5 分钟
- **THEN** 房间被回收且后续加入重新创建

### Requirement: 音视频采集

客户端 SHALL 支持麦克风、摄像头与屏幕共享，且可独立开关。切换状态 MUST 同步到服务端与所有参与者。

#### Scenario: 开摄像头

- **WHEN** 用户开启摄像头
- **THEN** 其他参与者看到视频流

#### Scenario: 静音

- **WHEN** 用户关闭麦克风
- **THEN** 其他参与者看到静音标记，音频流停止

### Requirement: 设备选择与权限

客户端 SHALL 提供输入/输出设备选择，并在权限缺失时给出可操作提示。

#### Scenario: 麦克风权限拒绝

- **WHEN** 系统拒绝麦克风权限
- **THEN** 客户端提示开启权限，不崩溃

#### Scenario: 切换设备

- **WHEN** 用户选择另一麦克风
- **THEN** 本端流切换且连接保持

### Requirement: 弱网与重连

客户端 SHALL 在 LiveKit 断开后自动重连，并保持 UI 状态一致。重连失败 MUST 明确提示。

#### Scenario: 自动重连

- **WHEN** 网络短暂中断
- **THEN** 客户端重连并恢复音视频

#### Scenario: 服务器静音

- **WHEN** 管理员静音某参与者
- **THEN** 该端麦克风被禁用并收到状态更新
