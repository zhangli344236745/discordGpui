//! 屏幕采集（CGDisplayStream → NV12）。
//!
//! 输出 420f（NV12 视频范围）帧，经 C 回调交给 Rust；未授权时直接报错。

#import <CoreGraphics/CoreGraphics.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>

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
  if (handle->kind == 1) {
    SCStream *stream = (__bridge_transfer SCStream *)handle->stream;
    id delegate = (__bridge_transfer id)handle->delegate;
    (void)delegate;
    [stream stopCaptureWithCompletionHandler:^(NSError *error) {
      (void)error;
    }];
  } else {
    CGDisplayStreamRef stream = (CGDisplayStreamRef)handle->stream;
    if (stream != NULL) {
      CGDisplayStreamStop(stream);
      CFRelease(stream);
    }
  }
  free(handle);
}


#pragma mark - ScreenCaptureKit（窗口/显示器选择）

@interface DGPUSCKOutput : NSObject <SCStreamOutput>
@property(nonatomic, assign) dgpu_screen_frame_cb callback;
@end

@implementation DGPUSCKOutput
- (void)stream:(SCStream *)stream
    didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
                   ofType:(SCStreamOutputType)type {
  (void)stream;
  if (type != SCStreamOutputTypeScreen || self.callback == NULL) {
    return;
  }
  CVPixelBufferRef pixel = CMSampleBufferGetImageBuffer(sampleBuffer);
  if (pixel == NULL) {
    return;
  }
  if (CVPixelBufferLockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) {
    return;
  }
  uint8_t *y = (uint8_t *)CVPixelBufferGetBaseAddressOfPlane(pixel, 0);
  uint8_t *uv = (uint8_t *)CVPixelBufferGetBaseAddressOfPlane(pixel, 1);
  if (y != NULL && uv != NULL) {
    self.callback(y, uv, (int)CVPixelBufferGetWidth(pixel), (int)CVPixelBufferGetHeight(pixel),
                  (int)CVPixelBufferGetBytesPerRowOfPlane(pixel, 0),
                  (int)CVPixelBufferGetBytesPerRowOfPlane(pixel, 1));
  }
  CVPixelBufferUnlockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly);
}
@end

static SCShareableContent *dgpu_fetch_content(NSError **out_error) {
  dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
  __block SCShareableContent *content = nil;
  __block NSError *fetch_error = nil;
  [SCShareableContent getShareableContentExcludingDesktopWindows:NO
                                             onScreenWindowsOnly:YES
                                              completionHandler:^(SCShareableContent *result,
                                                                  NSError *error) {
                                                content = result;
                                                fetch_error = error;
                                                dispatch_semaphore_signal(semaphore);
                                              }];
  if (dispatch_semaphore_wait(semaphore,
                              dispatch_time(DISPATCH_TIME_NOW, (int64_t)(6 * NSEC_PER_SEC))) != 0) {
    if (out_error != NULL) {
      *out_error = [NSError errorWithDomain:@"dgpui.screen"
                                       code:1
                                   userInfo:@{NSLocalizedDescriptionKey : @"枚举共享内容超时"}];
    }
    return nil;
  }
  if (out_error != NULL) {
    *out_error = fetch_error;
  }
  return content;
}

char *dgpu_screen_list_json(char *err, size_t err_len) {
  if (!CGPreflightScreenCaptureAccess()) {
    dgpu_screen_set_error(err, err_len, @"屏幕录制权限未授权");
    return NULL;
  }
  NSError *error = nil;
  SCShareableContent *content = dgpu_fetch_content(&error);
  if (content == nil) {
    dgpu_screen_set_error(err, err_len, error.localizedDescription ?: @"无法枚举共享内容");
    return NULL;
  }

  NSMutableArray *items = [NSMutableArray array];
  int display_index = 1;
  for (SCDisplay *display in content.displays) {
    [items addObject:@{
      @"id" : @(display.displayID),
      @"kind" : @"display",
      @"title" : [NSString stringWithFormat:@"显示器 %d", display_index++],
      @"app" : @"",
      @"width" : @(display.width),
      @"height" : @(display.height),
    }];
  }
  for (SCWindow *window in content.windows) {
    if (window.frame.size.width < 120 || window.frame.size.height < 80) {
      continue;
    }
    NSString *title = window.title ?: @"";
    NSString *app = window.owningApplication.applicationName ?: @"";
    if (title.length == 0 && app.length == 0) {
      continue;
    }
    [items addObject:@{
      @"id" : @(window.windowID),
      @"kind" : @"window",
      @"title" : title,
      @"app" : app,
      @"width" : @(window.frame.size.width),
      @"height" : @(window.frame.size.height),
    }];
  }

  NSData *data = [NSJSONSerialization dataWithJSONObject:items options:0 error:&error];
  if (data == nil) {
    dgpu_screen_set_error(err, err_len, error.localizedDescription ?: @"序列化共享列表失败");
    return NULL;
  }
  NSString *json = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
  return strdup(json.UTF8String ?: "[]");
}

dgpu_screen_t *dgpu_screen_start_target(uint32_t target_id,
                                        int is_window,
                                        int max_width,
                                        int max_height,
                                        dgpu_screen_frame_cb callback,
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

  NSError *error = nil;
  SCShareableContent *content = dgpu_fetch_content(&error);
  if (content == nil) {
    dgpu_screen_set_error(err, err_len, error.localizedDescription ?: @"无法枚举共享内容");
    return NULL;
  }

  SCContentFilter *filter = nil;
  CGFloat src_width = 0;
  CGFloat src_height = 0;
  if (is_window) {
    SCWindow *target = nil;
    for (SCWindow *window in content.windows) {
      if (window.windowID == target_id) {
        target = window;
        break;
      }
    }
    if (target == nil) {
      dgpu_screen_set_error(err, err_len, @"窗口不存在或已关闭");
      return NULL;
    }
    filter = [[SCContentFilter alloc] initWithDesktopIndependentWindow:target];
    src_width = target.frame.size.width;
    src_height = target.frame.size.height;
  } else {
    SCDisplay *target = nil;
    for (SCDisplay *display in content.displays) {
      if (display.displayID == target_id) {
        target = display;
        break;
      }
    }
    if (target == nil) {
      dgpu_screen_set_error(err, err_len, @"显示器不存在");
      return NULL;
    }
    filter = [[SCContentFilter alloc] initWithDisplay:target excludingWindows:@[]];
    src_width = target.width;
    src_height = target.height;
  }

  double scale = MIN(1.0, MIN((double)max_width / (double)src_width,
                              (double)max_height / (double)src_height));
  size_t out_w = (size_t)(src_width * scale) & ~((size_t)1);
  size_t out_h = (size_t)(src_height * scale) & ~((size_t)1);
  if (out_w < 2 || out_h < 2) {
    dgpu_screen_set_error(err, err_len, @"共享区域尺寸不可用");
    return NULL;
  }

  SCStreamConfiguration *config = [[SCStreamConfiguration alloc] init];
  config.width = out_w;
  config.height = out_h;
  config.minimumFrameInterval = CMTimeMake(1, 15);
  config.pixelFormat = kCVPixelFormatType_420YpCbCr8BiPlanarFullRange;
  config.showsCursor = YES;
  config.queueDepth = 3;

  DGPUSCKOutput *output = [[DGPUSCKOutput alloc] init];
  output.callback = callback;

  SCStream *stream = [[SCStream alloc] initWithFilter:filter configuration:config delegate:nil];
  dispatch_queue_t queue = dispatch_queue_create("workbuddy.dgpui.sck", DISPATCH_QUEUE_SERIAL);
  NSError *add_error = nil;
  if (![stream addStreamOutput:output
                          type:SCStreamOutputTypeScreen
            sampleHandlerQueue:queue
                         error:&add_error]) {
    dgpu_screen_set_error(err, err_len, add_error.localizedDescription ?: @"无法挂载屏幕输出");
    return NULL;
  }

  dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
  __block NSError *start_error = nil;
  [stream startCaptureWithCompletionHandler:^(NSError *capture_error) {
    start_error = capture_error;
    dispatch_semaphore_signal(semaphore);
  }];
  if (dispatch_semaphore_wait(semaphore,
                              dispatch_time(DISPATCH_TIME_NOW, (int64_t)(6 * NSEC_PER_SEC))) != 0 ||
      start_error != nil) {
    dgpu_screen_set_error(err, err_len,
                          start_error.localizedDescription ?: @"屏幕共享启动超时");
    return NULL;
  }

  dgpu_screen_t *handle = (dgpu_screen_t *)calloc(1, sizeof(dgpu_screen_t));
  handle->kind = 1;
  handle->stream = (__bridge_retained void *)stream;
  handle->delegate = (__bridge_retained void *)output;
  return handle;
}

void dgpu_free_string(char *value) {
  if (value != NULL) {
    free(value);
  }
}
