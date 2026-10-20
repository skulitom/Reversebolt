//! Capture only the selected HWND using Windows Graphics Capture.
//! A single frame is returned; no monitor or desktop capture is used.
use reversebolt_core::Result;
use std::{sync::mpsc, time::Duration};
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

struct OneFrame {
    sender: Option<mpsc::SyncSender<Result<image::RgbaImage>>>,
    window: u64,
}

impl GraphicsCaptureApiHandler for OneFrame {
    type Flags = (mpsc::SyncSender<Result<image::RgbaImage>>, u64);
    type Error = String;

    fn new(ctx: Context<Self::Flags>) -> Result<Self> {
        Ok(Self {
            sender: Some(ctx.flags.0),
            window: ctx.flags.1,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        control: InternalCaptureControl,
    ) -> Result<()> {
        let result = (|| {
            let (width, height) = (frame.width(), frame.height());
            if width == 0 || height == 0 || width > 7680 || height > 4320 {
                return Err("Window dimensions are outside the capture budget.".into());
            }
            let buffer = frame.buffer().map_err(|e| e.to_string())?;
            let mut staging = vec![];
            let pixels = buffer.as_nopadding_buffer(&mut staging).to_vec();
            let image = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or("Invalid graphics frame.")?;
            crop_client(self.window, image)
        })();
        if let Some(sender) = self.sender.take() {
            let _ = sender.try_send(result);
        }
        control.stop();
        Ok(())
    }

    fn on_closed(&mut self) -> Result<()> {
        if let Some(sender) = self.sender.take() {
            let _ = sender.try_send(Err("The selected window closed during capture.".into()));
        }
        Ok(())
    }
}

/// Called inside native's single bounded capture worker. If an OS stop/join
/// hangs, that worker remains occupied instead of spawning more capture threads.
pub fn capture(window: u64) -> Result<image::RgbaImage> {
    let (tx, rx) = mpsc::sync_channel(1);
    let settings = Settings::new(
        Window::from_raw_hwnd(window as *mut _),
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Exclude,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        (tx, window),
    );
    let control = OneFrame::start_free_threaded(settings).map_err(|e| e.to_string())?;
    let result = rx
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| "No graphics frame arrived before the deadline.".to_string());
    control.stop().map_err(|e| e.to_string())?;
    result?
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
