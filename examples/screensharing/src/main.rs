use livekit::options::{TrackPublishOptions, VideoCodec, VideoEncoding};
use livekit::prelude::*;
use livekit::track::{LocalTrack, LocalVideoTrack, TrackSource};
use livekit::webrtc::desktop_capturer::{DesktopCapturer, DesktopFrame, CaptureResult};
use livekit::webrtc::native::yuv_helper;
use livekit::webrtc::prelude::{
    NV12Buffer, I420Buffer, RtcVideoSource, VideoFrame, VideoResolution, VideoRotation,
};
use livekit::webrtc::video_source::native::NativeVideoSource;
use livekit_api::access_token;
use std::env;
use std::sync::Arc;
use std::sync::Mutex;

// Connect to a room using the specified env variables
// and print all incoming events

#[tokio::main]
async fn main() {
    env_logger::init();

    let url = env::var("LIVEKIT_URL").expect("LIVEKIT_URL is not set");
    let api_key = env::var("LIVEKIT_API_KEY").expect("LIVEKIT_API_KEY is not set");
    let api_secret = env::var("LIVEKIT_API_SECRET").expect("LIVEKIT_API_SECRET is not set");

    let token = access_token::AccessToken::with_api_key(&api_key, &api_secret)
        .with_identity("rust-bot")
        .with_name("Rust Bot")
        .with_grants(access_token::VideoGrants {
            room_join: true,
            room: "dev_room".to_string(),
            ..Default::default()
        })
        .to_jwt()
        .unwrap();

    let (room, mut rx) = Room::connect(&url, &token, RoomOptions::default()).await.unwrap();
    log::info!("Connected to room: {} - {}", room.name(), String::from(room.sid().await));

    let width = 3440;
    let height = 1440;
    let buffer_source = NativeVideoSource::new(VideoResolution { width, height });
    let track = LocalVideoTrack::create_video_track(
        "screen_share",
        RtcVideoSource::Native(buffer_source.clone()),
    );

    let res = room
        .local_participant()
        .publish_track(
            LocalTrack::Video(track),
            TrackPublishOptions {
                source: TrackSource::Screenshare,
                video_codec: VideoCodec::VP9,
                video_encoding: Some(VideoEncoding {
                    max_bitrate: 4000 * 1000,
                    max_framerate: 30.,
                }),
                simulcast: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let buffer_source_clone = buffer_source.clone();
    let video_frame = Arc::new(Mutex::new(VideoFrame {
        rotation: VideoRotation::VideoRotation0,
        buffer: I420Buffer::new(width, height),
        timestamp_us: 0,
    }));
    let callback = move |result: CaptureResult, frame: DesktopFrame| {
        let video_frame = video_frame.clone();
        let height = frame.height();
        let width = frame.width();
        let stride = frame.stride();
        let data = frame.data();

        let mut framebuffer = video_frame.lock().unwrap();
        let buffer = &mut framebuffer.buffer;
        let (s_y, s_u, s_v) = buffer.strides();
        let (y, u, v) = buffer.data_mut();
        yuv_helper::abgr_to_i420(data, stride, y, s_y, u, s_u, v, s_v, width, height);

        buffer_source_clone.capture_frame(&*framebuffer);
    };
    let mut capturer = DesktopCapturer::new(callback, false);
    if capturer.is_none() {
        return;
    }
    let mut capturer = capturer.unwrap();
    let sources = capturer.get_source_list();
    for source in sources.iter() {
        println!("Source: {}", source);
    }
    let source = sources[0].clone();
    capturer.start_capture(source);
    /* let mut capturer = DesktopCapturer::new(callback, true);
    let sources = capturer.get_source_list();
    for source in sources {
        println!("Source: {}", source);
    } */
    //capturer.start_capture();
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    //capturer.stop_capture();
}
