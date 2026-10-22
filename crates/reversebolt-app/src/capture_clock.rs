//! Read only the local performance counter, bracketed in the investigation clock.
use reversebolt_core::{
    Result,
    image_clock::{POLICY, Receipt, Sample},
    live::Screen,
};
use std::time::Instant;
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

pub fn sample(origin: Instant) -> Result<Sample> {
    let (mut frequency, mut counter) = (0i64, 0i64);
    if unsafe { QueryPerformanceFrequency(&mut frequency) } == 0 || frequency <= 0 {
        return Err("Performance counter frequency unavailable.".into());
    }
    let begin = origin.elapsed().as_micros();
    let success = unsafe { QueryPerformanceCounter(&mut counter) };
    let end = origin.elapsed().as_micros();
    if success == 0 || counter < 0 {
        return Err("Performance counter unavailable.".into());
    }
    Ok(Sample {
        frequency_hz: frequency as u64,
        counter_ticks: counter,
        local_before_us: begin.try_into().map_err(|_| "Capture clock overflow")?,
        local_after_us: end.try_into().map_err(|_| "Capture clock overflow")?,
    })
}

pub fn capture(
    clock_id: &str,
    origin: Instant,
    action: impl FnOnce() -> Result<Screen>,
) -> Result<Screen> {
    let before = sample(origin);
    let mut screen = action()?;
    let after = sample(origin);
    if let Some(sampling) = screen.sampling.as_mut() {
        sampling.clock = None;
        sampling.clock_error = None;
        match (before, after) {
            (Ok(before), Ok(after)) => {
                sampling.clock = Some(Receipt {
                    policy: POLICY.into(),
                    clock_id: clock_id.into(),
                    before,
                    after,
                })
            }
            (Err(error), _) | (_, Err(error)) => sampling.clock_error = Some(error),
        }
    }
    Ok(screen)
}
