# Spec Delta

## ADDED Requirements

### Requirement: 视频画面聚焦

客户端 SHALL 允许用户把任意一路视频放大查看，并能返回平铺视图。

#### Scenario: 放大某一路

- **WHEN** 用户点击平铺中的某格画面
- **THEN** 该路画面以更大尺寸展示，并标注参与者与来源

#### Scenario: 还原

- **WHEN** 用户点击「还原」或再次点击聚焦画面
- **THEN** 恢复到多路平铺布局

#### Scenario: 目标消失

- **WHEN** 聚焦的参与者离开或关闭摄像头
- **THEN** 自动退出聚焦并回到平铺，不显示空白大图
