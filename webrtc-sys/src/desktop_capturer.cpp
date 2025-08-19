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

#include "livekit/desktop_capturer.h"

using SourceList = webrtc::DesktopCapturer::SourceList;

namespace livekit {
DesktopCapturer::DesktopCapturer(
    rust::Box<DesktopCapturerCallbackWrapper> callback,
    std::unique_ptr<webrtc::DesktopCapturer> capturer)
    : callback(std::move(callback)), capturer(std::move(capturer)) {}

void DesktopCapturer::OnCaptureResult(
    webrtc::DesktopCapturer::Result result,
    std::unique_ptr<webrtc::DesktopFrame> frame) {
  CaptureResult ret_result = CaptureResult::Success;
  switch (result) {
    case webrtc::DesktopCapturer::Result::SUCCESS:
      ret_result = CaptureResult::Success;
      break;
    case webrtc::DesktopCapturer::Result::ERROR_PERMANENT:
      ret_result = CaptureResult::ErrorPermanent;
      break;
    case webrtc::DesktopCapturer::Result::ERROR_TEMPORARY:
      ret_result = CaptureResult::ErrorTemporary;
      break;
    case webrtc::DesktopCapturer::Result::ERROR_USER_STOPPED:
      ret_result = CaptureResult::ErrorUserStopped;
      break;
    default:
      break;
  }
  callback->on_capture_result(ret_result,
                              std::make_unique<DesktopFrame>(std::move(frame)));
}

void DesktopCapturer::set_excluded_applications(
    rust::Vec<uint64_t> excluded_applications) const {
  std::vector<uint64_t> excluded_apps{};
  for (auto& app : excluded_applications) {
    excluded_apps.push_back(app);
  }
  capturer->SetExcludedApplications(std::move(excluded_apps));
}

rust::Vec<Source> DesktopCapturer::get_source_list() const {
  SourceList list{};
  bool res = capturer->GetSourceList(&list);
  rust::Vec<Source> source_list{};
  if (res) {
    for (auto& source : list) {
      source_list.push_back(Source{static_cast<uint64_t>(source.id),
                                   source.title, source.display_id});
    }
  }
  return source_list;
}

DesktopRect DesktopCapturer::get_source_rect() const {
  DesktopRect rect{};

#ifdef __linux__
  auto metadata = capturer->GetMetadata();
  rect.top = metadata.session_details.top;
  rect.left = metadata.session_details.left;
  rect.width = metadata.session_details.width;
  rect.height = metadata.session_details.height;
#endif

  return rect;
}
}  // namespace livekit
