#ifndef DGPUI_CAMERA_CAPTURE_H
#define DGPUI_CAMERA_CAPTURE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// NV12 帧回调：Y 平面 + UV 交织平面 + 尺寸/步长。
/// 回调在采集队列上调用，数据仅在调用期间有效。
typedef void (*dgpu_frame_cb)(const uint8_t *y,
                              const uint8_t *uv,
                              int width,
                              int height,
                              int stride_y,
                              int stride_uv);

typedef struct dgpu_camera_t {
  void *session;
  void *delegate;
} dgpu_camera_t;

/// 启动默认摄像头；失败返回 NULL 并写入 err。
dgpu_camera_t *dgpu_camera_start(dgpu_frame_cb callback, char *err, size_t err_len);

/// 停止并释放。
void dgpu_camera_stop(dgpu_camera_t *handle);

#ifdef __cplusplus
}
#endif

#endif
