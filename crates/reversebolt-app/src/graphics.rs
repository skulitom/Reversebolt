//! Capture only the selected HWND using Windows Graphics Capture.
//! Select the last completed frame in a short, content-neutral burst.
use crate::capture_sampling::{Burst, MAX_FRAMES, detail_fraction};
use reversebolt_core::Result;
use reversebolt_core::visual::{CaptureSampling, SampledFrame};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
    window::Window,
};

#[derive(Default)]
struct State {
    burst: Burst<image::RgbaImage>,
    done: bool,
    closed: bool,
    error: Option<String>,
}
type Shared = Arc<(Mutex<State>, Condvar)>;
struct WindowFrames {
    shared: Shared,
    window: u64,
    origin: Instant,
}

pub struct Capture {
    pub image: image::RgbaImage,
    pub sampling: CaptureSampling,
}

impl GraphicsCaptureApiHandler for WindowFrames {
    type Flags = (Shared, u64, Instant);
    type Error = String;

    fn new(ctx: Context<Self::Flags>) -> Result<Self> {
        Ok(Self {
            shared: ctx.flags.0,
            window: ctx.flags.1,
            origin: ctx.flags.2,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        control: InternalCaptureControl,
    ) -> Result<()> {
        {
            let mut state = self.shared.0.lock().unwrap();
            if state.done
                || state
                    .burst
                    .finished(self.origin.elapsed().as_micros() as u64)
            {
                state.done = true;
                self.shared.1.notify_all();
                drop(state);
                control.stop();
                return Ok(());
            }
        }
        let result = (|| {
            let compositor_100ns = frame.timestamp().ok().map(|t| t.Duration);
            let (width, height) = (frame.width(), frame.height());
            if width == 0 || height == 0 || width > 7680 || height > 4320 {
                return Err("Window dimensions are outside the capture budget.".into());
            }
            let buffer = frame.buffer().map_err(|e| e.to_string())?;
            let mut staging = vec![];
            let pixels = buffer.as_nopadding_buffer(&mut staging).to_vec();
            let image = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or("Invalid graphics frame.")?;
            let image = crop_client(self.window, image)?;
            let metadata = SampledFrame {
                completed_us: self.origin.elapsed().as_micros() as u64,
                compositor_100ns,
                width: image.width(),
                height: image.height(),
                rgba_sha256: reversebolt_core::visual::digest(image.as_raw()),
                detail_fraction: detail_fraction(&image),
            };
            Ok((image, metadata))
        })();
        let mut state = self.shared.0.lock().unwrap();
        if !state.done {
            match result {
                Ok((image, mut metadata)) => {
                    // Policy time includes conversion/diagnostic work. No frame
                    // that completes after the fixed deadline replaces one.
                    metadata.completed_us = self.origin.elapsed().as_micros() as u64;
                    if !state.burst.push(metadata, image) || state.burst.frames.len() == MAX_FRAMES
                    {
                        state.done = true;
                    }
                }
                Err(error) => {
                    state.error = Some(error);
                    state.done = true;
                }
            }
        }
        let stop = state.done;
        self.shared.1.notify_all();
        drop(state);
        if stop {
            control.stop();
        }
        Ok(())
    }

    fn on_closed(&mut self) -> Result<()> {
        let mut state = self.shared.0.lock().unwrap();
        state.closed = true;
        state.done = true;
        self.shared.1.notify_all();
        Ok(())
    }
}

/// Called inside native's single bounded capture worker. If an OS stop/join
/// hangs, that worker remains occupied instead of spawning more capture threads.
pub fn capture(window: u64) -> Result<Capture> {
    let origin = Instant::now();
    let shared: Shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
    let settings = Settings::new(
        Window::from_raw_hwnd(window as *mut _),
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Exclude,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        (shared.clone(), window, origin),
    );
    let control = WindowFrames::start_free_threaded(settings).map_err(|e| e.to_string())?;
    let mut state = shared.0.lock().unwrap();
    loop {
        let now = origin.elapsed().as_micros() as u64;
        let deadline = state.burst.deadline_us().unwrap_or(3_000_000);
        if state.done || now >= deadline {
            break;
        }
        state = shared
            .1
            .wait_timeout(state, Duration::from_micros(deadline - now))
            .unwrap()
            .0;
    }
    state.done = true;
    let result = if let Some(error) = state.error.take() {
        Err(error)
    } else if state.closed {
        Err("The selected window closed during capture.".into())
    } else if let Some(sampling) = state.burst.receipt() {
        Ok(Capture {
            image: state.burst.latest.take().unwrap(),
            sampling,
        })
    } else {
        Err("No graphics frame arrived before the deadline.".into())
    };
    drop(state);
    control.stop().map_err(|e| e.to_string())?;
    result
}

fn crop_client(window: u64, image: image::RgbaImage) -> Result<image::RgbaImage> {
    use windows::Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::{
            Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
            Gdi::ClientToScreen,
        },
        UI::WindowsAndMessaging::GetClientRect,
    };
    let hwnd = HWND(window as *mut _);
    let (mut outer, mut client, mut origin) = (RECT::default(), RECT::default(), POINT::default());
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut outer as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
        .map_err(|e| e.to_string())?;
        GetClientRect(hwnd, &mut client).map_err(|e| e.to_string())?;
        if !ClientToScreen(hwnd, &mut origin).as_bool() {
            return Err("Could not locate the client area.".into());
        }
    }
    let (x, y, w, h) = (
        origin.x - outer.left,
        origin.y - outer.top,
        client.right - client.left,
        client.bottom - client.top,
    );
    if outer.right - outer.left != image.width() as i32
        || outer.bottom - outer.top != image.height() as i32
        || x < 0
        || y < 0
        || w <= 0
        || h <= 0
        || x + w > image.width() as i32
        || y + h > image.height() as i32
    {
        return Err("The window geometry changed during graphics capture.".into());
    }
    Ok(image::imageops::crop_imm(&image, x as u32, y as u32, w as u32, h as u32).to_image())
}
