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
} dgpu_screen_t;

/// 启动主屏采集（缩放到不超过 max_width x max_height）。
/// 需要「屏幕录制」权限；未授权时返回 NULL 并写入 err。
dgpu_screen_t *dgpu_screen_start(dgpu_screen_frame_cb callback,
                                 int max_width,
                                 int max_height,
                                 char *err,
                                 size_t err_len);

void dgpu_screen_stop(dgpu_screen_t *handle);

#ifdef __cplusplus
}
#endif

#endif
