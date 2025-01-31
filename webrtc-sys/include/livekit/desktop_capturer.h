/*
 * Copyright 2025 LiveKit
 *
 * Licensed under the Apache License, Version 2.0 (the “License”);
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an “AS IS” BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#pragma once
#include <memory>

#include "modules/desktop_capture/desktop_capturer.h"
#include "modules/desktop_capture/desktop_capture_options.h"
#include "rust/cxx.h"

namespace livekit {
class DesktopFrame;
class DesktopCapturer;
class Source;
}  // namespace livekit

#include "webrtc-sys/src/desktop_capturer.rs.h"

namespace livekit {

class DesktopCapturer : public webrtc::DesktopCapturer::Callback {
 public:
  explicit DesktopCapturer(rust::Box<DesktopCapturerCallbackWrapper> callback,
                           std::unique_ptr<webrtc::DesktopCapturer> capturer,
                           std::unique_ptr<webrtc::DesktopCapturer> sources_capturer);

  void OnCaptureResult(webrtc::DesktopCapturer::Result result,
                       std::unique_ptr<webrtc::DesktopFrame> frame) final;

  rust::Vec<Source> get_source_list() const;
  bool select_source(uint64_t id) const { return capturer->SelectSource(id); };
  void start() { capturer->Start(this); };
  void capture_frame() const { capturer->CaptureFrame(); };

 private:
  std::unique_ptr<webrtc::DesktopCapturer> capturer;
  // TODO: This is a workaround for fetching the source list
  // it will be removed once the SCContentSharingPicker will be
  // integrated.
  std::unique_ptr<webrtc::DesktopCapturer> sources_capturer;
  rust::Box<DesktopCapturerCallbackWrapper> callback;
};

class DesktopFrame {
 public:
  DesktopFrame(std::unique_ptr<webrtc::DesktopFrame> frame)
      : frame(std::move(frame)) {};
  int32_t width() const { return frame->size().width(); }

  int32_t height() const { return frame->size().height(); }

  int32_t left() const { return frame->rect().left(); }

  int32_t top() const { return frame->rect().top(); }

  int32_t stride() const { return frame->stride(); }

  const uint8_t* data() const { return frame->data(); }

 private:
  std::unique_ptr<webrtc::DesktopFrame> frame;
};

static std::unique_ptr<DesktopCapturer> new_desktop_capturer(
    rust::Box<DesktopCapturerCallbackWrapper> callback,
    bool window_capturer) {
  webrtc::DesktopCaptureOptions options =
      webrtc::DesktopCaptureOptions::CreateDefault();
#ifdef __APPLE__
  options.set_allow_sck_capturer(true);
#endif
  std::unique_ptr<webrtc::DesktopCapturer> capturer = nullptr;
  std::unique_ptr<webrtc::DesktopCapturer> sources_capturer = nullptr;
  if (window_capturer) {
    capturer = webrtc::DesktopCapturer::CreateWindowCapturer(options);
#ifdef __APPLE__
    options.set_allow_sck_capturer(false);
#endif
    sources_capturer = webrtc::DesktopCapturer::CreateWindowCapturer(options);
  } else {
    capturer = webrtc::DesktopCapturer::CreateScreenCapturer(options);
#ifdef __APPLE__
    options.set_allow_sck_capturer(false);
#endif
    sources_capturer = webrtc::DesktopCapturer::CreateScreenCapturer(options);
  }
  if (!capturer || !sources_capturer) {
    return nullptr;
  }
  return std::make_unique<DesktopCapturer>(std::move(callback),
                                           std::move(capturer),
                                           std::move(sources_capturer));
}

}  // namespace livekit