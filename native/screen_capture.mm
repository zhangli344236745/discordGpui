//! 屏幕采集（CGDisplayStream → NV12）。
//!
//! 输出 420f（NV12 视频范围）帧，经 C 回调交给 Rust；未授权时直接报错。

#import <CoreGraphics/CoreGraphics.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>

#include "screen_capture.h"

static void dgpu_screen_set_error(char *err, size_t err_len, NSString *message) {
  if (err == NULL || err_len == 0) {
    return;
  }
  const char *text = message.UTF8String;
  if (text == NULL) {
    err[0] = '\0';
    return;
  }
  strncpy(err, text, err_len - 1);
  err[err_len - 1] = '\0';
}

dgpu_screen_t *dgpu_screen_start(dgpu_screen_frame_cb callback,
                                 int max_width,
                                 int max_height,
                                 char *err,
                                 size_t err_len) {
  if (callback == NULL) {
    dgpu_screen_set_error(err, err_len, @"回调为空");
    return NULL;
  }
  if (!CGPreflightScreenCaptureAccess()) {
    dgpu_screen_set_error(err, err_len, @"屏幕录制权限未授权");
    return NULL;
  }

  CGDirectDisplayID display = CGMainDisplayID();
  size_t src_w = CGDisplayPixelsWide(display);
  size_t src_h = CGDisplayPixelsHigh(display);
  if (src_w == 0 || src_h == 0) {
    dgpu_screen_set_error(err, err_len, @"没有可用显示器");
    return NULL;
  }

  // 等比缩放到上限内，保证宽高为偶数（YUV420 要求）
  double scale = MIN(1.0, MIN((double)max_width / (double)src_w, (double)max_height / (double)src_h));
  size_t out_w = (size_t)((double)src_w * scale) & ~((size_t)1);
  size_t out_h = (size_t)((double)src_h * scale) & ~((size_t)1);
  if (out_w < 2 || out_h < 2) {
    dgpu_screen_set_error(err, err_len, @"屏幕尺寸不可用");
    return NULL;
  }

  NSDictionary *properties = @{
    (__bridge NSString *)kCGDisplayStreamShowCursor : @YES,
    (__bridge NSString *)kCGDisplayStreamMinimumFrameTime : @(1.0 / 15.0),
    (__bridge NSString *)kCGDisplayStreamPreserveAspectRatio : @YES,
  };

  dispatch_queue_t queue = dispatch_queue_create("workbuddy.dgpui.screen", DISPATCH_QUEUE_SERIAL);
  CGDisplayStreamRef stream = CGDisplayStreamCreateWithDispatchQueue(
      display, out_w, out_h, '420f', (__bridge CFDictionaryRef)properties, queue,
      ^(CGDisplayStreamFrameStatus status, uint64_t time, IOSurfaceRef surface,
        CGDisplayStreamUpdateRef update) {
        (void)time;
        (void)update;
        if (status != kCGDisplayStreamFrameStatusFrameComplete || surface == NULL) {
          return;
        }
        CVPixelBufferRef pixel = NULL;
        NSDictionary *attrs = @{(id)kCVPixelBufferIOSurfacePropertiesKey : @{}};
        if (CVPixelBufferCreateWithIOSurface(kCFAllocatorDefault, surface,
                                            (__bridge CFDictionaryRef)attrs, &pixel) !=
                kCVReturnSuccess ||
            pixel == NULL) {
          return;
        }
        if (CVPixelBufferLockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly) == kCVReturnSuccess) {
          uint8_t *y = (uint8_t *)CVPixelBufferGetBaseAddressOfPlane(pixel, 0);
          uint8_t *uv = (uint8_t *)CVPixelBufferGetBaseAddressOfPlane(pixel, 1);
          if (y != NULL && uv != NULL) {
            callback(y, uv, (int)CVPixelBufferGetWidth(pixel), (int)CVPixelBufferGetHeight(pixel),
                     (int)CVPixelBufferGetBytesPerRowOfPlane(pixel, 0),
                     (int)CVPixelBufferGetBytesPerRowOfPlane(pixel, 1));
          }
          CVPixelBufferUnlockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly);
        }
        CFRelease(pixel);
      });

  if (stream == NULL) {
    dgpu_screen_set_error(err, err_len, @"无法创建屏幕采集流");
    return NULL;
  }
  if (CGDisplayStreamStart(stream) != kCGErrorSuccess) {
    CFRelease(stream);
    dgpu_screen_set_error(err, err_len, @"屏幕采集启动失败");
    return NULL;
  }

  dgpu_screen_t *handle = (dgpu_screen_t *)calloc(1, sizeof(dgpu_screen_t));
  handle->stream = (void *)stream;
  return handle;
}

void dgpu_screen_stop(dgpu_screen_t *handle) {
  if (handle == NULL) {
    return;
  }
  CGDisplayStreamRef stream = (CGDisplayStreamRef)handle->stream;
  if (stream != NULL) {
    CGDisplayStreamStop(stream);
    CFRelease(stream);
  }
  free(handle);
}
