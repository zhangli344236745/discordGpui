//! 摄像头采集最小封装（Objective-C++）。
//!
//! 只做一件事：AVCaptureSession 输出 NV12 帧，通过 C 回调交给 Rust；
//! 权限、错误都通过 `err` 缓冲回传，避免异常穿透 FFI。

#import <AVFoundation/AVFoundation.h>
#import <CoreMedia/CoreMedia.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>

#include "camera_capture.h"

@interface DGPUCameraCapture : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate>
@property(nonatomic, assign) dgpu_frame_cb callback;
@property(nonatomic, strong) AVCaptureSession *session;
@end

@implementation DGPUCameraCapture
- (void)captureOutput:(AVCaptureOutput *)output
    didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
           fromConnection:(AVCaptureConnection *)connection {
  (void)output;
  (void)connection;
  CVPixelBufferRef pixel = CMSampleBufferGetImageBuffer(sampleBuffer);
  if (pixel == NULL || self.callback == NULL) {
    return;
  }
  if (CVPixelBufferLockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) {
    return;
  }
  uint8_t *y = (uint8_t *)CVPixelBufferGetBaseAddressOfPlane(pixel, 0);
  uint8_t *uv = (uint8_t *)CVPixelBufferGetBaseAddressOfPlane(pixel, 1);
  int width = (int)CVPixelBufferGetWidth(pixel);
  int height = (int)CVPixelBufferGetHeight(pixel);
  int stride_y = (int)CVPixelBufferGetBytesPerRowOfPlane(pixel, 0);
  int stride_uv = (int)CVPixelBufferGetBytesPerRowOfPlane(pixel, 1);
  if (y != NULL && uv != NULL) {
    self.callback(y, uv, width, height, stride_y, stride_uv);
  }
  CVPixelBufferUnlockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly);
}
@end

static void dgpu_set_error(char *err, size_t err_len, NSString *message) {
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

dgpu_camera_t *dgpu_camera_start(dgpu_frame_cb callback, char *err, size_t err_len) {
  if (callback == NULL) {
    dgpu_set_error(err, err_len, @"回调为空");
    return NULL;
  }

  // 阻塞等待权限结果（首次会弹系统授权框；语音线程不是主线程，安全）
  AVAuthorizationStatus status = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
  if (status == AVAuthorizationStatusNotDetermined) {
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeVideo
                             completionHandler:^(BOOL granted) {
                               (void)granted;
                               dispatch_semaphore_signal(semaphore);
                             }];
    dispatch_semaphore_wait(semaphore, dispatch_time(DISPATCH_TIME_NOW, (int64_t)(8 * NSEC_PER_SEC)));
    status = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
  }
  if (status != AVAuthorizationStatusAuthorized) {
    dgpu_set_error(err, err_len, @"摄像头权限未授权");
    return NULL;
  }

  AVCaptureDevice *device = [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeVideo];
  if (device == nil) {
    dgpu_set_error(err, err_len, @"未找到摄像头设备");
    return NULL;
  }

  NSError *error = nil;
  AVCaptureDeviceInput *input = [AVCaptureDeviceInput deviceInputWithDevice:device error:&error];
  if (input == nil) {
    dgpu_set_error(err, err_len, error.localizedDescription ?: @"无法打开摄像头");
    return NULL;
  }

  DGPUCameraCapture *delegate = [[DGPUCameraCapture alloc] init];
  delegate.callback = callback;

  AVCaptureSession *session = [[AVCaptureSession alloc] init];
  [session beginConfiguration];
  session.sessionPreset = AVCaptureSessionPreset1280x720;
  if (![session canAddInput:input]) {
    dgpu_set_error(err, err_len, @"摄像头输入不可用");
    [session commitConfiguration];
    return NULL;
  }
  [session addInput:input];

  AVCaptureVideoDataOutput *output = [[AVCaptureVideoDataOutput alloc] init];
  output.videoSettings = @{
    (id)kCVPixelBufferPixelFormatTypeKey : @(kCVPixelFormatType_420YpCbCr8BiPlanarFullRange)
  };
  output.alwaysDiscardsLateVideoFrames = YES;
  dispatch_queue_t queue = dispatch_queue_create("workbuddy.dgpui.camera", DISPATCH_QUEUE_SERIAL);
  [output setSampleBufferDelegate:delegate queue:queue];
  if (![session canAddOutput:output]) {
    dgpu_set_error(err, err_len, @"摄像头输出不可用");
    [session commitConfiguration];
    return NULL;
  }
  [session addOutput:output];
  [session commitConfiguration];

  [session startRunning];

  dgpu_camera_t *handle = (dgpu_camera_t *)calloc(1, sizeof(dgpu_camera_t));
  handle->session = (__bridge_retained void *)session;
  handle->delegate = (__bridge_retained void *)delegate;
  return handle;
}

void dgpu_camera_stop(dgpu_camera_t *handle) {
  if (handle == NULL) {
    return;
  }
  AVCaptureSession *session = (__bridge_transfer AVCaptureSession *)handle->session;
  DGPUCameraCapture *delegate = (__bridge_transfer DGPUCameraCapture *)handle->delegate;
  delegate.callback = NULL;
  [session stopRunning];
  (void)session;
  (void)delegate;
  free(handle);
}
