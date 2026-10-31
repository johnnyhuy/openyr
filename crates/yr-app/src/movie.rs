//! Bounded native Bink playback on a presentation clock, outside the sim.
use infinitier_bik_decoder::{BikPixels, BikStreamingDecoder};
use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use yr_formats::BinkFile;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub index: u32,
    pub sequence: u64,
    pub rgba: Vec<u8>,
}
struct Shared {
    latest: Mutex<Option<Result<Frame, String>>>,
    cancelled: AtomicBool,
    wake: Condvar,
    sleep: Mutex<()>,
}
pub struct Movie {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    pub current: Option<Frame>,
    pub error: Option<String>,
}
#[derive(Clone, Copy)]
pub struct Timing {
    frames: u32,
    numerator: u32,
    denominator: u32,
}
impl Timing {
    pub fn sequence(self, elapsed: Duration) -> u64 {
        (elapsed.as_nanos() * u128::from(self.numerator)
            / (1_000_000_000 * u128::from(self.denominator)))
        .min(u128::from(u64::MAX)) as u64
    }
    pub fn deadline(self, sequence: u64) -> Duration {
        Duration::from_nanos(
            (u128::from(sequence) * 1_000_000_000 * u128::from(self.denominator)
                / u128::from(self.numerator))
            .min(u128::from(u64::MAX)) as u64,
        )
    }
}
fn validate(bytes: &[u8]) -> Result<Timing, String> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Menu movie exceeds 16 MiB".into());
    }
    let meta = BinkFile::parse(bytes).map_err(|e| e.to_string())?;
    if meta.revision != b'i'
        || meta.width > 1024
        || meta.height > 1024
        || meta.frames.len() > 30_000
        || !meta.tracks.is_empty()
        || meta.flags != 0
        || u64::from(meta.fps_num) > 60 * u64::from(meta.fps_den)
        || meta.fps_num < meta.fps_den
    {
        return Err("Unsupported menu movie geometry, audio or frame clock".into());
    }
    Ok(Timing {
        frames: meta.frames.len() as u32,
        numerator: meta.fps_num,
        denominator: meta.fps_den,
    })
}
/// Sequential decoding is necessary for dependent Bink frames. Only the latest
/// visible result is retained, so a stalled UI cannot accumulate frame buffers.
impl Movie {
    pub fn new(bytes: Vec<u8>) -> Result<Self, String> {
        let timing = validate(&bytes)?;
        let shared = Arc::new(Shared {
            latest: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            wake: Condvar::new(),
            sleep: Mutex::new(()),
        });
        let worker = shared.clone();
        let data: Arc<[u8]> = bytes.into();
        let thread = std::thread::Builder::new()
            .name("openyr-menu-movie".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    play(data, timing, &worker)
                }));
                let error = match result {
                    Ok(Err(e)) => Some(e),
                    Err(_) => Some("Menu movie decoder stopped".into()),
                    _ => None,
                };
                if let Some(e) = error {
                    if let Ok(mut slot) = worker.latest.lock() {
                        *slot = Some(Err(e));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            shared,
            thread: Some(thread),
            current: None,
            error: None,
        })
    }
    pub fn poll(&mut self) -> bool {
        let next = self.shared.latest.lock().ok().and_then(|mut v| v.take());
        match next {
            Some(Ok(frame)) => {
                self.current = Some(frame);
                true
            }
            Some(Err(e)) => {
                self.error = Some(e);
                true
            }
            None => false,
        }
    }
}
impl Drop for Movie {
    fn drop(&mut self) {
        {
            let _guard = self.shared.sleep.lock();
            self.shared.cancelled.store(true, Ordering::Relaxed);
            self.shared.wake.notify_all();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn play(data: Arc<[u8]>, timing: Timing, shared: &Shared) -> Result<(), String> {
    let epoch = Instant::now();
    let mut sequence = 0u64;
    loop {
        let mut decoder = BikStreamingDecoder::new(Cursor::new(data.clone()), "RA2TS_L.BIK")
            .map_err(|e| e.to_string())?;
        for index in 0..timing.frames {
            if shared.cancelled.load(Ordering::Relaxed) {
                return Ok(());
            }
            let frame = decoder
                .next_frame()
                .map_err(|e| e.to_string())?
                .ok_or("Menu movie ended early")?;
            {
                if let BikPixels::Yuv(planes) = frame.video.pixels {
                    let rgba = limited_rgba(&planes)?;
                    let frame = Frame {
                        width: decoder.width(),
                        height: decoder.height(),
                        index,
                        sequence,
                        rgba,
                    };
                    *shared
                        .latest
                        .lock()
                        .map_err(|_| "Menu movie result unavailable")? = Some(Ok(frame));
                }
            }
            sequence = sequence.checked_add(1).ok_or("Menu movie clock overflow")?;
            let delay = timing.deadline(sequence).saturating_sub(epoch.elapsed());
            if !delay.is_zero() && !shared.cancelled.load(Ordering::Relaxed) {
                let guard = shared
                    .sleep
                    .lock()
                    .map_err(|_| "Menu movie clock unavailable")?;
                let _wait = shared
                    .wake
                    .wait_timeout_while(guard, delay, |_| !shared.cancelled.load(Ordering::Relaxed))
                    .map_err(|_| "Menu movie clock unavailable")?;
            }
        }
    }
}
/// Limited-range BT.601. The menu's Y=16 black and Y=235 white must expand
/// to display black/white; treating these planes as full-range lifts shadows.
pub fn limited_rgba(planes: &infinitier_bik_decoder::VideoFrame) -> Result<Vec<u8>, String> {
    let (w, h) = (planes.y.width as usize, planes.y.height as usize);
    if w == 0 || h == 0 || w > 1024 || h > 1024 {
        return Err("Invalid menu movie plane dimensions".into());
    }
    for (p, width, height) in [
        (&planes.y, w, h),
        (&planes.u, w.div_ceil(2), h.div_ceil(2)),
        (&planes.v, w.div_ceil(2), h.div_ceil(2)),
    ] {
        if (p.width as usize) < width
            || (p.height as usize) < height
            || p.stride < width
            || p.stride
                .checked_mul(height)
                .is_none_or(|len| len > p.data.len())
        {
            return Err("Truncated menu movie plane".into());
        }
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let c = i32::from(planes.y.data[y * planes.y.stride + x]) - 16;
            let d = i32::from(planes.u.data[(y / 2) * planes.u.stride + x / 2]) - 128;
            let e = i32::from(planes.v.data[(y / 2) * planes.v.stride + x / 2]) - 128;
            let value = |n: i32| ((n + 128) >> 8).clamp(0, 255) as u8;
            rgba.extend([
                value(298 * c + 409 * e),
                value(298 * c - 100 * d - 208 * e),
                value(298 * c + 516 * d),
                255,
            ]);
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limited_range_black_white_and_truncated_planes() {
        let mut frame = infinitier_bik_decoder::VideoFrame::new(2, 2, false);
        frame.y.data.fill(16);
        frame.u.data.fill(128);
        frame.v.data.fill(128);
        assert!(limited_rgba(&frame)
            .unwrap()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [0, 0, 0, 255]));
        frame.y.data.fill(235);
        assert!(limited_rgba(&frame)
            .unwrap()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [255, 255, 255, 255]));
        frame.u.data.clear();
        assert!(limited_rgba(&frame).is_err());
    }
    #[test]
    fn rational_clock_has_no_per_frame_rounding_drift() {
        let clock = Timing {
            frames: 431,
            numerator: 15,
            denominator: 1,
        };
        assert_eq!(clock.sequence(Duration::from_secs(3600)), 54000);
        assert_eq!(clock.deadline(54000), Duration::from_secs(3600));
        assert_eq!(clock.sequence(Duration::from_secs(1)), 15);
        assert!(Movie::new(vec![]).is_err());
    }
}
#[cfg(test)]
mod retail_tests;
