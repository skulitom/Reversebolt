use reversebolt_core::{
    Result,
    live::{NativeTarget, ProcessInfo, Region, Screen, Word},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{Diagnostics::Debug::ReadProcessMemory, Memory::*, Threading::*},
    UI::WindowsAndMessaging::*,
};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(window: HWND, dc: HDC, flags: u32) -> i32;
}

struct ProcessHandle(HANDLE);
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn created(handle: HANDLE) -> Result<u64> {
    let mut times = [FILETIME::default(); 4];
    if unsafe {
        GetProcessTimes(
            handle,
            &mut times[0],
            &mut times[1],
            &mut times[2],
            &mut times[3],
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok((u64::from(times[0].dwHighDateTime) << 32) | u64::from(times[0].dwLowDateTime))
}

fn process_info(pid: u32, window: HWND) -> Result<ProcessInfo> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let handle = ProcessHandle(handle);
    let mut path = vec![0u16; 32768];
    let mut size = path.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(handle.0, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut size)
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let path = String::from_utf16_lossy(&path[..size as usize]);
    let name = Path::new(&path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut title = vec![0u16; 1024];
    let count = if window.is_null() {
        0
    } else {
        unsafe { GetWindowTextW(window, title.as_mut_ptr(), 1024) }.max(0) as usize
    };
    let mut session = 0;
    unsafe {
        ProcessIdToSessionId(pid, &mut session);
    }
    Ok(ProcessInfo {
        pid,
        name,
        title: String::from_utf16_lossy(&title[..count]),
        window: window as u64,
        session,
        created: format!("{:016X}", created(handle.0)?),
    })
}

pub fn processes() -> Result<Vec<ProcessInfo>> {
    unsafe extern "system" fn window(hwnd: HWND, param: LPARAM) -> i32 {
        if unsafe { IsWindowVisible(hwnd) } == 0 || unsafe { GetWindowTextLengthW(hwnd) } == 0 {
            return 1;
        }
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        if pid == std::process::id() {
            return 1;
        }
        if let Ok(info) = process_info(pid, hwnd) {
            let result = unsafe { &mut *(param as *mut Vec<ProcessInfo>) };
            result.push(info);
        }
        1
    }
    let mut result = Vec::<ProcessInfo>::new();
    if unsafe { EnumWindows(Some(window), &mut result as *mut _ as LPARAM) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    result.sort_by_key(|a| (a.name.to_lowercase(), a.title.to_lowercase(), a.pid));
    Ok(result)
}

pub struct Process {
    handle: ProcessHandle,
    info: ProcessInfo,
}
impl Process {
    pub fn open(pid: u32, expected_created: &str, expected_window: Option<u64>) -> Result<Self> {
        let info = processes()?
            .into_iter()
            .find(|p| p.pid == pid && expected_window.is_none_or(|w| p.window == w))
            .ok_or("That window is no longer available. Refresh the program list.")?;
        if info.created != expected_created {
            return Err("The process identity changed. Refresh the list and reconnect.".into());
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
        if handle.is_null() {
            return Err(format!(
                "Could not read this process: {}",
                std::io::Error::last_os_error()
            ));
        }
        let handle = ProcessHandle(handle);
        if format!("{:016X}", created(handle.0)?) != expected_created {
            return Err("The process restarted while connecting.".into());
        }
        Ok(Self { handle, info })
    }
}

impl NativeTarget for Process {
    fn info(&self) -> &ProcessInfo {
        &self.info
    }
    fn alive(&self) -> bool {
        let mut exit = 0;
        unsafe { GetExitCodeProcess(self.handle.0, &mut exit) != 0 && exit == 259 }
    }
    fn regions(&self) -> Result<Vec<Region>> {
        let mut address = 0usize;
        let mut result = vec![];
        let mut region = MEMORY_BASIC_INFORMATION::default();
        while unsafe {
            VirtualQueryEx(
                self.handle.0,
                address as *const _,
                &mut region,
                std::mem::size_of_val(&region),
            )
        } != 0
        {
            let base = region.BaseAddress as usize;
            if region.State == MEM_COMMIT
                && region.Protect & PAGE_GUARD == 0
                && matches!(
                    region.Protect & 0xff,
                    PAGE_READWRITE
                        | PAGE_WRITECOPY
                        | PAGE_EXECUTE_READWRITE
                        | PAGE_EXECUTE_WRITECOPY
                )
            {
                result.push(Region {
                    base: base as u64,
                    length: region.RegionSize,
                    image: region.Type == MEM_IMAGE,
                });
            }
            let Some(next) = base.checked_add(region.RegionSize).filter(|n| *n > address) else {
                break;
            };
            address = next;
        }
        Ok(result)
    }
    fn read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        if length > 64 * 1024 {
            return Err("Read chunk exceeds the bounded capture size.".into());
        }
        let mut bytes = vec![0u8; length];
        let mut count = 0;
        let ok = unsafe {
            ReadProcessMemory(
                self.handle.0,
                address as *const _,
                bytes.as_mut_ptr().cast(),
                length,
                &mut count,
            )
        };
        if ok == 0 || count != length {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(bytes)
    }
    fn screen(&self, path: &Path) -> Result<Screen> {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(self.info.window as HWND, &mut pid);
        }
        if pid != self.info.pid || !self.alive() {
            return Err("The original target window is no longer available.".into());
        }
        capture_bounded(self.info.window, path.to_owned())
    }
}

static CAPTURING: AtomicBool = AtomicBool::new(false);
fn capture_bounded(window: u64, path: PathBuf) -> Result<Screen> {
    if CAPTURING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("The previous window capture is still waiting for its renderer.".into());
    }
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                CAPTURING.store(false, Ordering::SeqCst);
            }
        }
        let _guard = Guard;
        let _ = tx.send(capture(window as HWND, &path));
    });
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|_| "Window capture timed out. Memory evidence is still available.".to_string())?
}

fn capture(window: HWND, path: &Path) -> Result<Screen> {
    if unsafe { IsIconic(window) } != 0 {
        return Err("Restore the selected window before capturing it.".into());
    }
    let mut rect = RECT::default();
    if unsafe { GetClientRect(window, &mut rect) } == 0 {
        return Err("Could not read the window dimensions.".into());
    }
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    if width < 1 || height < 1 || width > 7680 || height > 4320 {
        return Err("Window dimensions are outside the capture budget.".into());
    }
    struct Surface {
        window: HWND,
        source: HDC,
        dc: HDC,
        bitmap: HBITMAP,
        old: HGDIOBJ,
    }
    impl Drop for Surface {
        fn drop(&mut self) {
            unsafe {
                if !self.old.is_null() {
                    SelectObject(self.dc, self.old);
                }
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap);
                }
                if !self.dc.is_null() {
                    DeleteDC(self.dc);
                }
                if !self.source.is_null() {
                    ReleaseDC(self.window, self.source);
                }
            }
        }
    }
    let mut surface = Surface {
        window,
        source: std::ptr::null_mut(),
        dc: std::ptr::null_mut(),
        bitmap: std::ptr::null_mut(),
        old: std::ptr::null_mut(),
    };
    surface.source = unsafe { GetDC(window) };
    if surface.source.is_null() {
        return Err("Window drawing surface is unavailable.".into());
    }
    surface.dc = unsafe { CreateCompatibleDC(surface.source) };
    surface.bitmap = unsafe { CreateCompatibleBitmap(surface.source, width, height) };
    if surface.dc.is_null() || surface.bitmap.is_null() {
        return Err("Could not allocate a window capture surface.".into());
    }
    surface.old = unsafe { SelectObject(surface.dc, surface.bitmap) };
    // PW_CLIENTONLY. No desktop fallback: failed game renderers must be explicit.
    if unsafe { PrintWindow(window, surface.dc, 1) } == 0 {
        return Err("This renderer did not provide a window image.".into());
    }
    unsafe {
        SelectObject(surface.dc, surface.old);
    }
    surface.old = std::ptr::null_mut();
    let mut bitmap = BITMAPINFO::default();
    bitmap.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bitmap.bmiHeader.biWidth = width;
    bitmap.bmiHeader.biHeight = -height;
    bitmap.bmiHeader.biPlanes = 1;
    bitmap.bmiHeader.biBitCount = 32;
    bitmap.bmiHeader.biCompression = BI_RGB;
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    if unsafe {
        GetDIBits(
            surface.dc,
            surface.bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr().cast(),
            &mut bitmap,
            DIB_RGB_COLORS,
        )
    } != height
    {
        return Err("The window image could not be read.".into());
    }
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    let rgba = image::RgbaImage::from_raw(width as u32, height as u32, pixels)
        .ok_or("Invalid capture buffer.")?;
    rgba.save(path).map_err(|e| e.to_string())?;
    let range = rgba
        .pixels()
        .map(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]))
        .fold((u32::MAX, 0u32), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let mut warning = (range.1 - range.0 < 12).then(|| {
        "The image is nearly uniform; the renderer may not support this capture method.".to_string()
    });
    let (text, words) = match ocr(&rgba) {
        Ok(result) => result,
        Err(error) => {
            warning = Some(format!(
                "Image saved; text recognition unavailable: {error}"
            ));
            (String::new(), vec![])
        }
    };
    Ok(Screen {
        path: path.to_string_lossy().into_owned(),
        width: width as u32,
        height: height as u32,
        text,
        words,
        warning,
    })
}

fn ocr(image: &image::RgbaImage) -> Result<(String, Vec<Word>)> {
    use windows::{
        Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::DataWriter,
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    };
    let run = || -> windows::core::Result<(String, Vec<Word>)> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        let maximum = OcrEngine::MaxImageDimension()?;
        let scaled = image::imageops::thumbnail(
            image,
            maximum.min(image.width()),
            maximum.min(image.height()),
        );
        let factor_x = image.width() as f32 / scaled.width() as f32;
        let factor_y = image.height() as f32 / scaled.height() as f32;
        let mut bytes = scaled.as_raw().clone();
        for pixel in bytes.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bytes)?;
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &writer.DetachBuffer()?,
            BitmapPixelFormat::Bgra8,
            scaled.width() as i32,
            scaled.height() as i32,
            BitmapAlphaMode::Ignore,
        )?;
        let recognized = engine.RecognizeAsync(&bitmap)?.join()?;
        let mut words = vec![];
        for (index, line) in recognized.Lines()?.into_iter().enumerate() {
            for word in line.Words()? {
                let r = word.BoundingRect()?;
                words.push(Word {
                    text: word.Text()?.to_string(),
                    bounds: [
                        r.X * factor_x,
                        r.Y * factor_y,
                        r.Width * factor_x,
                        r.Height * factor_y,
                    ],
                    line: index,
                });
            }
        }
        Ok((recognized.Text()?.to_string(), words))
    };
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|e| e.to_string())?;
    let result = run().map_err(|e| e.to_string());
    unsafe {
        RoUninitialize();
    }
    result
}
