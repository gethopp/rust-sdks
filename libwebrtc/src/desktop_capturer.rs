// Copyright 2025 LiveKit, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::imp::desktop_capturer as imp_dc;

pub struct DesktopCapturer {
    handle: imp_dc::DesktopCapturer,
}

impl DesktopCapturer {
    pub fn new<T>(callback: T, window_capturer: bool) -> Option<Self>
    where
        T: Fn(CaptureResult, DesktopFrame) + Send + 'static,
    {
        let inner_callback = move |result: imp_dc::CaptureResult, frame: imp_dc::DesktopFrame| {
            callback(capture_result_from_sys(result), DesktopFrame::new(frame));
        };
        let desktop_capturer = imp_dc::DesktopCapturer::new(inner_callback, window_capturer);
        if desktop_capturer.is_none() {
            return None;
        }
        Some(Self { handle: desktop_capturer.unwrap() })
    }

    pub fn start_capture(&mut self, source: CaptureSource) {
        self.handle.select_source(source.sys_handle.id());
        self.handle.start();
    }

    pub fn capture_frame(&mut self) {
        self.handle.capture_frame();
    }

    pub fn get_source_list(&self) -> Vec<CaptureSource> {
        let source_list = self.handle.get_source_list();
        source_list.into_iter().map(|source| CaptureSource { sys_handle: source }).collect()
    }

    pub fn set_excluded_applications(&self, applications: Vec<u64>) {
        self.handle.set_excluded_applications(applications);
    }
}

pub struct DesktopFrame {
    pub(crate) sys_handle: imp_dc::DesktopFrame,
}

impl DesktopFrame {
    pub fn new(sys_handle: imp_dc::DesktopFrame) -> Self {
        Self { sys_handle }
    }

    pub fn width(&self) -> i32 {
        self.sys_handle.width() as i32
    }
    pub fn height(&self) -> i32 {
        self.sys_handle.height() as i32
    }
    pub fn stride(&self) -> u32 {
        self.sys_handle.stride() as u32
    }
    pub fn left(&self) -> i32 {
        self.sys_handle.left()
    }
    pub fn top(&self) -> i32 {
        self.sys_handle.top()
    }
    pub fn data(&self) -> &[u8] {
        &self.sys_handle.data()
    }
}

#[derive(Clone)]
pub struct CaptureSource {
    pub(crate) sys_handle: imp_dc::CaptureSource,
}

impl CaptureSource {
    pub fn id(&self) -> u64 {
        self.sys_handle.id()
    }
    pub fn title(&self) -> String {
        self.sys_handle.title()
    }
    pub fn display_id(&self) -> i64 {
        self.sys_handle.display_id()
    }
}

impl std::fmt::Display for CaptureSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptureSource")
            .field("id", &self.id())
            .field("title", &self.title())
            .field("display_id", &self.display_id())
            .finish()
    }
}

pub enum CaptureResult {
    Success,
    ErrorTemporary,
    ErrorPermanent,
    ErrorUserStopped,
}

fn capture_result_from_sys(result: imp_dc::CaptureResult) -> CaptureResult {
    match result {
        imp_dc::CaptureResult::Success => CaptureResult::Success,
        imp_dc::CaptureResult::ErrorTemporary => CaptureResult::ErrorTemporary,
        imp_dc::CaptureResult::ErrorPermanent => CaptureResult::ErrorPermanent,
        imp_dc::CaptureResult::ErrorUserStopped => CaptureResult::ErrorUserStopped,
    }
}
