# Tasks

## 1. 原生采集

- [x] 1.1 `dgpu_screen_list_json` 枚举显示器与窗口（同步等待 + 超时 + 权限预检）
- [x] 1.2 `dgpu_screen_start_target` 用 SCContentFilter/SCStream 采集指定目标（NV12、15fps）
- [x] 1.3 句柄区分 CGDisplayStream / SCStream，停止与释放正确
- [x] 1.4 保持整屏 CGDisplayStream 回退可用

## 2. 客户端接线

- [x] 2.1 Rust `ScreenTarget` + `list_targets`/`start_target`
- [x] 2.2 引擎命令 `ListScreenTargets` / `SetScreenTarget`，保留 `SetScreen(bool)` 回退
- [x] 2.3 App：请求共享时拉取目标并打开选择器，选中后启动，再次点击停止
- [x] 2.4 UI：`screen_picker` 模态（显示器分组、窗口列表、取消）

## 3. 验证与文档

- [x] 3.1 voice-e2e 走「枚举→共享第一个显示器」，断言目标数与 SCREEN_SHARE webhook
- [x] 3.2 dm-call-e2e 保留整屏回退路径并通过
- [ ] 3.3 更新 voice 文档与 README
- [ ] 3.4 `openspec validate add-screen-share-picker --strict` 通过
