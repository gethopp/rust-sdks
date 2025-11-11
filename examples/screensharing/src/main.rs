use clap::Parser;
use livekit::options::{TrackPublishOptions, VideoCodec};
use livekit::prelude::*;
use livekit::track::{LocalTrack, LocalVideoTrack, TrackSource};
use livekit::webrtc::desktop_capturer::{
    CaptureResult, DesktopCaptureSourceType, DesktopCapturer, DesktopCapturerOptions, DesktopFrame,
};
use livekit::webrtc::native::yuv_helper;
use livekit::webrtc::prelude::{
    I420Buffer, RtcVideoSource, VideoFrame, VideoResolution, VideoRotation,
};
use livekit::webrtc::video_source::native::NativeVideoSource;
use livekit_api::access_token;
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::pin::pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::task::{Context, Waker};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Capture the mouse cursor
    #[arg(long)]
    capture_cursor: bool,

    /// Capture a window rather than a screen
    #[arg(long)]
    capture_window: bool,

    /// Use system screen picker (macOS only)
    #[cfg(target_os = "macos")]
    #[arg(long, default_value_t = true)]
    use_system_picker: bool,
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter(Some(env!("CARGO_CRATE_NAME")), log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let args = Args::parse();

    #[cfg(target_os = "linux")]
    {
        /* This is needed for getting the system picker for screen sharing. */
        use glib::MainLoop;
        let main_loop = MainLoop::new(None, false);
        let _handle = std::thread::spawn(move || {
            main_loop.run();
        });
    }

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

    let (room, _) = Room::connect(&url, &token, RoomOptions::default()).await.unwrap();
    log::info!("Connected to room: {} - {}", room.name(), String::from(room.sid().await));

    let (video_source_sender, mut video_source_receiver) = tokio::sync::mpsc::channel(1);

    let callback = {
        // These dimensions are arbitrary initial values.
        // libwebrtc only exposes the resolution of the source in the DesktopFrame
        // passed to the callback, so wait to publish the video track until
        // the callback is called the first time.
        let mut stream_width = 1920;
        let mut stream_height = 1080;

        let mut video_frame = VideoFrame {
            rotation: VideoRotation::VideoRotation0,
            buffer: I420Buffer::new(stream_width, stream_height),
            timestamp_us: 0,
        };
        let mut video_source: Option<NativeVideoSource> = None;
        move |result: CaptureResult, frame: DesktopFrame| {
            match result {
                CaptureResult::ErrorTemporary => {
                    log::debug!("Error temporary");
                    return;
                }
                CaptureResult::ErrorPermanent => {
                    log::debug!("Error permanent");
                    return;
                }
                _ => {}
            }
            let height = frame.height().try_into().unwrap();
            let width = frame.width().try_into().unwrap();

            if width != stream_width || height != stream_height {
                stream_width = width;
                stream_height = height;
                video_frame.buffer = I420Buffer::new(width, height);
            }

            let stride = frame.stride();
            let data = frame.data();

            let (s_y, s_u, s_v) = video_frame.buffer.strides();
            let (y, u, v) = video_frame.buffer.data_mut();
            yuv_helper::argb_to_i420(
                data,
                stride,
                y,
                s_y,
                u,
                s_u,
                v,
                s_v,
                frame.width(),
                frame.height(),
            );

            if let Some(video_source) = &video_source {
                video_source.capture_frame(&video_frame);
            } else {
                // This is the first time the callback has been called.
                // Use the resolution from the DesktopFrame to create a video source
                // and push it over a channel to be published from the Tokio context.
                let video_source_inner = NativeVideoSource::new(VideoResolution {
                    width: stream_width,
                    height: stream_height,
                });

                // This callback is synchronous, however, it gets called via
                // `capturer.capture_frame()` from the Tokio context. Thus, calling
                // the channel sender's send_blocking method panics. To work around this,
                // use the async send method and manually poll the future once, which is
                // all that is needed.
                let future = pin!(video_source_sender.send(video_source_inner.clone()));
                let waker = Waker::noop();
                let mut context = Context::from_waker(waker);
                let _ = future.poll(&mut context);

                video_source = Some(video_source_inner);
            }
        }
    };
    let mut options = DesktopCapturerOptions::new();
    if args.capture_window {
        options.set_source_type(DesktopCaptureSourceType::Window);
    }
    #[cfg(target_os = "macos")]
    {
        options.set_sck_system_picker(args.use_system_picker);
    }
    options.set_include_cursor(args.capture_cursor);

    let mut capturer =
        DesktopCapturer::new(callback, options).expect("Failed to create desktop capturer");
    let sources = capturer.get_source_list();
    let selected_source = if sources.len() == 0 {
        None
    // On Wayland, the XDG Desktop Portal presents a UI for the user
    // to select the source and libwebrtc only returns that one source,
    // so do not present a redundant UI here.
    } else if sources.len() == 1 {
        Some(sources.first().unwrap().clone())
    } else {
        let options: Vec<_> = sources.clone().into_iter().map(|s| s.to_string()).collect();
        let map: HashMap<_, _> = sources.clone().into_iter().map(|s| (s.to_string(), s)).collect();
        match inquire::Select::new("Select desktop capture source:", options).prompt() {
            Ok(s) => Some(map.get(&s).unwrap().clone()),
            Err(e) => panic!("{e:?}"),
        }
    };

    log::info!("Starting desktop capture. Press Ctrl + C to quit.");
    capturer.start_capture(selected_source);

    let ctrl_c_received = Arc::new(AtomicBool::new(false));
    tokio::spawn({
        let ctrl_c_received = ctrl_c_received.clone();
        async move {
            tokio::signal::ctrl_c().await.unwrap();
            ctrl_c_received.store(true, Ordering::Release);
        }
    });

    loop {
        if ctrl_c_received.load(Ordering::Acquire) == true {
            log::info!("Ctrl + C received, stopping desktop capture.");
            break;
        }

        capturer.capture_frame();
        if let Ok(video_source) = video_source_receiver.try_recv() {
            let track = LocalVideoTrack::create_video_track(
                "screen_share",
                RtcVideoSource::Native(video_source),
            );

            room.local_participant()
                .publish_track(
                    LocalTrack::Video(track),
                    TrackPublishOptions {
                        source: TrackSource::Screenshare,
                        video_codec: VideoCodec::VP9,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
    }
}
