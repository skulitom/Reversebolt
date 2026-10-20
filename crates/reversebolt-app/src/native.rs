use reversebolt_core::{
    Result,
    live::{NativeTarget, ProcessInfo, Region, Screen, TextSource, Word},
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
    observer: std::cell::RefCell<Option<crate::ocr::Observer>>,
    models: PathBuf,
}
impl Process {
    pub fn open(
        pid: u32,
        expected_created: &str,
        expected_window: Option<u64>,
        models: PathBuf,
    ) -> Result<Self> {
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
        Ok(Self {
            handle,
            info,
            observer: std::cell::RefCell::new(None),
            models,
        })
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
        let mut queries = 0;
        loop {
            if queries >= 100_000 || result.len() >= 32768 {
                return Err("Memory inventory exceeded its bounded query budget.".into());
            }
            queries += 1;
            let count = unsafe {
                VirtualQueryEx(
                    self.handle.0,
                    address as *const _,
                    &mut region,
                    std::mem::size_of_val(&region),
                )
            };
            if count == 0 {
                let error = unsafe { GetLastError() };
                if error == ERROR_INVALID_PARAMETER && self.alive() {
                    break;
                }
                return Err(format!(
                    "Memory inventory stopped at 0x{address:X}: {}",
                    std::io::Error::from_raw_os_error(error as i32)
                ));
            }
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
                    allocation_base: region.AllocationBase as u64,
                    protection: region.Protect,
                    kind: region.Type,
                });
            }
            let Some(next) = base.checked_add(region.RegionSize).filter(|n| *n > address) else {
                return Err("Memory inventory returned a non-advancing region.".into());
            };
            address = next;
        }
        Ok(result)
    }
    fn query(&self, address: u64) -> Result<Option<Region>> {
        let mut region = MEMORY_BASIC_INFORMATION::default();
        if unsafe {
            VirtualQueryEx(
                self.handle.0,
                address as *const _,
                &mut region,
                std::mem::size_of_val(&region),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok((region.State == MEM_COMMIT
            && region.Protect & PAGE_GUARD == 0
            && matches!(
                region.Protect & 0xff,
                PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY
            ))
        .then_some(Region {
            base: region.BaseAddress as u64,
            length: region.RegionSize,
            image: region.Type == MEM_IMAGE,
            allocation_base: region.AllocationBase as u64,
            protection: region.Protect,
            kind: region.Type,
        }))
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
        if unsafe { IsIconic(self.info.window as HWND) } != 0 {
            return Err("Restore the selected window before capturing it.".into());
        }
        let result = capture_bounded(self.info.window, path.to_owned());
        unsafe {
            GetWindowThreadProcessId(self.info.window as HWND, &mut pid);
        }
        if pid != self.info.pid || !self.alive() {
            return Err("The original target changed while capturing.".into());
        }
        result
    }
    fn recognize(&self, screen: &mut Screen) -> Result<()> {
        let mut observer = self.observer.borrow_mut();
        observer
            .get_or_insert_with(|| crate::ocr::Observer::load(&self.models))
            .recognize(screen)
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
        let result = match crate::graphics::capture(window) {
            Ok(image) => save_screen(image, &path, "windows_graphics_capture", None),
            Err(error) => capture(window as HWND, &path).map(|mut screen| {
                let note =
                    format!("Using compatibility capture. Graphics capture unavailable: {error}");
                screen.warning = Some(match screen.warning.take() {
                    Some(previous) => format!("{previous} {note}"),
                    None => note,
                });
                screen
            }),
        };
        let _ = tx.send(result);
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
    save_screen(rgba, path, "print_window", None)
}

struct PngBudget(Vec<u8>);
impl std::io::Write for PngBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > reversebolt_core::visual::MAX_IMAGE_BYTES - self.0.len() {
            return Err(std::io::Error::other(
                "PNG exceeded the 10 MiB image budget.",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn save_screen(
    rgba: image::RgbaImage,
    path: &Path,
    method: &str,
    previous_warning: Option<String>,
) -> Result<Screen> {
    use image::ImageEncoder as _;
    let mut png = PngBudget(Vec::new());
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    std::fs::write(path, &png.0).map_err(|e| e.to_string())?;
    let range = rgba
        .pixels()
        .map(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]))
        .fold((u32::MAX, 0u32), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let warning = (range.1 - range.0 < 12)
        .then(|| {
            "The image is nearly uniform; the renderer may not support this capture method."
                .to_string()
        })
        .or(previous_warning);
    Ok(Screen {
        path: path.to_string_lossy().into_owned(),
        width: rgba.width(),
        height: rgba.height(),
        text: String::new(),
        words: vec![],
        warning,
        method: method.into(),
    })
}

pub(crate) fn ocr(image: &image::RgbaImage) -> Result<(String, Vec<Word>)> {
    use windows::{
        Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::DataWriter,
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    };
    // windows-rs caches agile activation factories for the process lifetime.
    // Letting the last MTA apartment disappear between observations can unload
    // Windows.Media.Ocr.dll while that cache still points into it. Blue Prince's
    // second capture after an idle interval exposed this as an access violation.
    // Retain one MTA usage cookie for the same lifetime as those static factories.
    // It is intentionally released only by process teardown, including when
    // capture workers are still unwinding during application shutdown.
    static MTA: std::sync::OnceLock<Result<usize>> = std::sync::OnceLock::new();
    MTA.get_or_init(|| {
        unsafe { windows::Win32::System::Com::CoIncrementMTAUsage() }
            .map(|cookie| cookie.0 as usize)
            .map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(Clone::clone)?;
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
                    source: TextSource::WindowsOcr,
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

/// Offline comparison of generic OCR preprocessing on an already captured image.
/// This path has no game identity, memory access, expected values or known regions.
pub fn evaluate_ocr(path: &Path, output: &Path) -> Result<()> {
    let image = image::open(path).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let original = image.to_rgba8();
    let gray = image.to_luma8();
    let mut inverted = gray.clone();
    image::imageops::invert(&mut inverted);
    let mut binary = gray.clone();
    // Otsu's threshold maximizes between-class intensity variance.
    let mut histogram = [0u64; 256];
    for pixel in gray.pixels() {
        histogram[pixel[0] as usize] += 1;
    }
    let total = (gray.width() as u64 * gray.height() as u64) as f64;
    let sum: f64 = histogram
        .iter()
        .enumerate()
        .map(|(i, n)| i as f64 * *n as f64)
        .sum();
    let (mut count, mut partial, mut best, mut threshold) = (0.0, 0.0, 0.0, 0u8);
    for (i, n) in histogram.iter().enumerate() {
        count += *n as f64;
        partial += i as f64 * *n as f64;
        if count == 0.0 || count == total {
            continue;
        }
        let difference = partial / count - (sum - partial) / (total - count);
        let score = count * (total - count) * difference * difference;
        if score > best {
            best = score;
            threshold = i as u8;
        }
    }
    for pixel in binary.pixels_mut() {
        pixel[0] = if pixel[0] > threshold { 0 } else { 255 };
    }
    let mut report = vec![];
    for (name, source) in [
        ("original", original),
        ("gray", image::DynamicImage::ImageLuma8(gray).into_rgba8()),
        (
            "inverted",
            image::DynamicImage::ImageLuma8(inverted).into_rgba8(),
        ),
        ("otsu", image::DynamicImage::ImageLuma8(binary).into_rgba8()),
    ] {
        for scale in [1, 2] {
            let pixels = image::imageops::resize(
                &source,
                source.width() * scale,
                source.height() * scale,
                image::imageops::FilterType::CatmullRom,
            );
            let file = output.join(format!("{name}-{scale}x.png"));
            pixels.save(&file).map_err(|e| e.to_string())?;
            let start = std::time::Instant::now();
            match ocr(&pixels) {
                Ok((text, words)) => report.push(serde_json::json!({
                    "variant":name, "scale":scale, "text":text, "words":words,
                    "elapsed_ms":start.elapsed().as_millis(), "image":file,
                })),
                Err(error) => report.push(serde_json::json!({
                    "variant":name, "scale":scale, "error":error,
                })),
            }
        }
    }
    std::fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn png_budget_rejects_overflow_and_capture_pixels_round_trip() {
        use std::io::Write;
        let mut buffer = super::PngBudget(vec![0; reversebolt_core::visual::MAX_IMAGE_BYTES - 2]);
        buffer.write_all(&[1, 2]).unwrap();
        assert!(buffer.write_all(&[3]).is_err());
        assert_eq!(buffer.0.len(), reversebolt_core::visual::MAX_IMAGE_BYTES);
        let path =
            std::env::temp_dir().join(format!("reversebolt-png-{}.png", uuid::Uuid::new_v4()));
        let pixels =
            image::RgbaImage::from_fn(64, 32, |x, y| image::Rgba([x as u8, y as u8, 42, 255]));
        super::save_screen(pixels.clone(), &path, "fixture", None).unwrap();
        assert_eq!(image::open(&path).unwrap().into_rgba8(), pixels);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn ocr_factories_survive_idle_worker_threads() {
        // Actual Windows OCR across caller-thread teardown, plus an explicit
        // request to unload unused COM libraries. Also test longer idle periods
        // against a live window: this short test alone did not reproduce the bug.
        for _ in 0..4 {
            std::thread::spawn(|| {
                let image = image::RgbaImage::from_pixel(320, 160, image::Rgba([255; 4]));
                super::ocr(&image).expect("Windows OCR must remain usable");
            })
            .join()
            .unwrap();
            unsafe {
                windows::Win32::System::Com::CoFreeUnusedLibrariesEx(0, None);
            }
        }
    }
}
