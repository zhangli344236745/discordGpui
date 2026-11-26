#ifndef DGPUI_SCREEN_CAPTURE_H
#define DGPUI_SCREEN_CAPTURE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// NV12 帧回调（与摄像头共用签名）。
typedef void (*dgpu_screen_frame_cb)(const uint8_t *y,
                                     const uint8_t *uv,
                                     int width,
                                     int height,
                                     int stride_y,
                                     int stride_uv);

typedef struct dgpu_screen_t {
  void *stream;
  void *delegate;
  /// 0 = CGDisplayStream（整屏回退），1 = ScreenCaptureKit（可选择窗口/显示器）
  int kind;
} dgpu_screen_t;

/// 启动主屏采集（缩放到不超过 max_width x max_height）。
/// 需要「屏幕录制」权限；未授权时返回 NULL 并写入 err。
dgpu_screen_t *dgpu_screen_start(dgpu_screen_frame_cb callback,
                                 int max_width,
                                 int max_height,
                                 char *err,
                                 size_t err_len);

void dgpu_screen_stop(dgpu_screen_t *handle);

/// 枚举可共享的显示器与窗口，返回 JSON 数组字符串（调用方用 dgpu_free_string 释放）。
/// 失败返回 NULL 并写入 err。
char *dgpu_screen_list_json(char *err, size_t err_len);

/// 按目标启动采集：target_id 为 displayID 或 windowID。
dgpu_screen_t *dgpu_screen_start_target(uint32_t target_id,
                                        int is_window,
                                        int max_width,
                                        int max_height,
                                        dgpu_screen_frame_cb callback,
                                        char *err,
                                        size_t err_len);

void dgpu_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif
