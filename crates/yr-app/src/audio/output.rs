//! Native device output. Device discovery and recovery run on an audio worker.
//! The real-time callback only consumes decoded PCM and sends bounded notices.

use super::mixer::{Bus, Mixer, Playback};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Error, ErrorKind, FromSample, SampleFormat, SizedSample};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const RETRY_MIN: Duration = Duration::from_millis(500);
const RETRY_MAX: Duration = Duration::from_secs(8);
const POLL: Duration = Duration::from_millis(20);
const NOTICE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Stats {
    frames: AtomicU64,
    signal_frames: AtomicU64,
}

pub struct LiveOutput {
    tx: mpsc::SyncSender<Playback>,
    muted: Arc<AtomicBool>,
    stats: Arc<Stats>,
}

impl LiveOutput {
    pub fn open() -> Result<Self, String> {
        let (tx, rx) = mpsc::sync_channel(64);
        let muted = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Stats::default());
        let worker_mute = muted.clone();
        let worker_stats = stats.clone();
        std::thread::Builder::new()
            .name("openyr-audio".into())
            .spawn(move || {
                run(rx, || Connection::open(&worker_mute, &worker_stats));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { tx, muted, stats })
    }

    pub fn play(&self, sound: Playback) {
        let _ = self.tx.try_send(sound);
    }

    pub fn toggle_mute(&self) -> bool {
        !self.muted.fetch_xor(true, Ordering::Relaxed)
    }

    pub fn signal_frames(&self) -> u64 {
        self.stats.signal_frames.load(Ordering::Relaxed)
    }

    pub fn frames_played(&self) -> u64 {
        self.stats.frames.load(Ordering::Relaxed)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Continue,
    Retry,
    Disable,
}

fn action(kind: ErrorKind) -> Action {
    match kind {
        // CPAL keeps these streams active, including automatic CoreAudio reroutes.
        ErrorKind::Xrun | ErrorKind::DeviceChanged | ErrorKind::RealtimeDenied => Action::Continue,
        ErrorKind::DeviceBusy
        | ErrorKind::DeviceNotAvailable
        | ErrorKind::HostUnavailable
        | ErrorKind::ResourceExhausted
        | ErrorKind::StreamInvalidated
        | ErrorKind::BackendError => Action::Retry,
        // Permission/configuration failures need a user or code change.
        _ => Action::Disable,
    }
}

struct Errors {
    faults: mpsc::Receiver<Error>,
    notices: mpsc::Receiver<Error>,
    #[cfg(test)]
    inject: mpsc::SyncSender<Error>,
}

fn error_channel() -> (impl FnMut(Error) + Send + 'static, Errors) {
    // Separate slots prevent a flood of xruns from hiding a disconnected device.
    let (fault_tx, faults) = mpsc::sync_channel(1);
    let (notice_tx, notices) = mpsc::sync_channel(1);
    #[cfg(test)]
    let inject = fault_tx.clone();
    let report = move |error: Error| {
        let tx = if action(error.kind()) == Action::Continue {
            &notice_tx
        } else {
            &fault_tx
        };
        let _ = tx.try_send(error);
    };
    (
        report,
        Errors {
            faults,
            notices,
            #[cfg(test)]
            inject,
        },
    )
}

// Only the audio worker owns the stream. The generic handle permits device-free
// regression tests of the same queue, callback-error and recovery paths.
struct Connection<S> {
    _stream: S,
    tx: mpsc::SyncSender<Playback>,
    errors: Errors,
}

impl Connection<cpal::Stream> {
    fn open(muted: &Arc<AtomicBool>, stats: &Arc<Stats>) -> Result<Self, Error> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| {
                Error::with_message(ErrorKind::DeviceNotAvailable, "no output device")
            })?;
        let supported = device.default_output_config()?;
        let format = supported.sample_format();
        let config = supported.config();
        let (tx, rx) = mpsc::sync_channel(64);
        let (report, errors) = error_channel();
        macro_rules! build {
            ($t:ty) => {
                build::<$t>(
                    &device,
                    config.clone(),
                    rx,
                    muted.clone(),
                    stats.clone(),
                    report,
                )
            };
        }
        let stream = match format {
            SampleFormat::F32 => build!(f32),
            SampleFormat::F64 => build!(f64),
            SampleFormat::I16 => build!(i16),
            SampleFormat::U16 => build!(u16),
            SampleFormat::I32 => build!(i32),
            SampleFormat::U32 => build!(u32),
            _ => {
                return Err(Error::with_message(
                    ErrorKind::UnsupportedConfig,
                    format!("unsupported audio output format {format:?}"),
                ))
            }
        }?;
        stream.play()?;
        println!(
            "OpenYR: audio output {} Hz, {} channels, {format:?}",
            config.sample_rate, config.channels
        );
        Ok(Self {
            _stream: stream,
            tx,
            errors,
        })
    }
}

struct Recovery<S> {
    connection: Option<Connection<S>>,
    music: Option<Playback>,
    next_attempt: Option<Instant>,
    delay: Duration,
    notice_after: Instant,
    opened: bool,
}

impl<S> Recovery<S> {
    fn new(now: Instant) -> Self {
        Self {
            connection: None,
            music: None,
            next_attempt: Some(now),
            delay: RETRY_MIN,
            notice_after: now,
            opened: false,
        }
    }

    fn play(&mut self, sound: Playback) {
        if sound.bus == Bus::Music {
            self.music = Some(sound.clone());
        }
        if let Some(connection) = &self.connection {
            let _ = connection.tx.try_send(sound);
        }
        // Expired effects and speech are discarded during an outage.
    }

    fn failed(&mut self, now: Instant, error: Error) {
        if action(error.kind()) == Action::Disable {
            self.next_attempt = None;
            eprintln!("OpenYR: audio disabled ({:?}: {error}); restart after correcting the device configuration or permissions", error.kind());
        } else {
            eprintln!(
                "OpenYR: audio interrupted ({:?}: {error}); retrying in {} ms",
                error.kind(),
                self.delay.as_millis()
            );
            self.next_attempt = Some(now + self.delay);
            self.delay = (self.delay * 2).min(RETRY_MAX);
        }
    }

    fn poll(&mut self, now: Instant, open: impl FnOnce() -> Result<Connection<S>, Error>) {
        if let Some(connection) = &self.connection {
            let fault = connection.errors.faults.try_recv().ok();
            if let Ok(notice) = connection.errors.notices.try_recv() {
                if now >= self.notice_after {
                    eprintln!(
                        "OpenYR: audio notice ({:?}: {notice}); stream remains active",
                        notice.kind()
                    );
                    self.notice_after = now + NOTICE_INTERVAL;
                }
            }
            if let Some(error) = fault {
                // Drop the old device and its callback before opening a replacement.
                self.connection = None;
                self.failed(now, error);
            }
        }
        if self.next_attempt.is_some_and(|at| now >= at) {
            self.next_attempt = None;
            match open() {
                Ok(connection) => {
                    if let Some(music) = &self.music {
                        let _ = connection.tx.try_send(music.clone());
                    }
                    self.connection = Some(connection);
                    self.delay = RETRY_MIN;
                    if self.opened {
                        println!("OpenYR: audio restored; music restarted, mute preserved");
                    }
                    self.opened = true;
                }
                Err(error) => self.failed(now, error),
            }
        }
    }
}

fn run<S>(rx: mpsc::Receiver<Playback>, mut open: impl FnMut() -> Result<Connection<S>, Error>) {
    let mut recovery = Recovery::new(Instant::now());
    loop {
        let disconnected = recovery.connection.is_none();
        recovery.poll(Instant::now(), &mut open);
        if disconnected && recovery.connection.is_some() {
            // Opening a device can be slow. Do not replay effects queued while
            // discovery was blocked, but retain a newly submitted theme.
            for _ in 0..64 {
                match rx.try_recv() {
                    Ok(sound) if sound.bus == Bus::Music => recovery.play(sound),
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
        match rx.recv_timeout(POLL) {
            Ok(sound) => recovery.play(sound),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            // No join on the app thread: dropping a slow native device cannot stall sim ticks.
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    rx: mpsc::Receiver<Playback>,
    muted: Arc<AtomicBool>,
    stats: Arc<Stats>,
    report: impl FnMut(Error) + Send + 'static,
) -> Result<cpal::Stream, Error> {
    let mut mixer = Mixer::new(config.sample_rate);
    let channels = usize::from(config.channels);
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for _ in 0..64 {
                match rx.try_recv() {
                    Ok(sound) => mixer.play(sound),
                    Err(_) => break,
                }
            }
            let mute = muted.load(Ordering::Relaxed);
            let mut signal_frames = 0;
            for frame in data.chunks_exact_mut(channels) {
                let stereo = mixer.next_frame();
                if !mute && stereo.iter().any(|s| *s != 0.0) {
                    signal_frames += 1;
                }
                for (channel, sample) in frame.iter_mut().enumerate() {
                    let value = if mute {
                        0.0
                    } else if channels == 1 {
                        (stereo[0] + stereo[1]) * 0.5
                    } else {
                        stereo.get(channel).copied().unwrap_or(0.0)
                    };
                    *sample = T::from_sample(value);
                }
            }
            stats
                .frames
                .fetch_add((data.len() / channels) as u64, Ordering::Relaxed);
            stats
                .signal_frames
                .fetch_add(signal_frames, Ordering::Relaxed);
        },
        report,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_formats::audio::Pcm;

    fn sound(bus: Bus) -> Playback {
        Playback {
            bus,
            pcm: Arc::new(Pcm {
                sample_rate: 44100,
                channels: 1,
                samples: vec![1000, -1000],
            }),
            gain: 1.0,
        }
    }

    fn connection() -> (Connection<()>, mpsc::Receiver<Playback>, impl FnMut(Error)) {
        let (tx, rx) = mpsc::sync_channel(64);
        let (report, errors) = error_channel();
        (
            Connection {
                _stream: (),
                tx,
                errors,
            },
            rx,
            report,
        )
    }

    #[test]
    fn recoverable_callback_notification_keeps_playback() {
        let now = Instant::now();
        let mut recovery = Recovery::new(now);
        let (stream, rx, mut report) = connection();
        recovery.poll(now, || Ok(stream));
        for kind in [
            ErrorKind::Xrun,
            ErrorKind::DeviceChanged,
            ErrorKind::RealtimeDenied,
        ] {
            report(kind.into());
            recovery.poll(now, || panic!("{kind:?} rebuilt a healthy stream"));
            recovery.play(sound(Bus::Effect));
            assert_eq!(
                rx.try_recv().unwrap().bus,
                Bus::Effect,
                "{kind:?} silenced playback"
            );
        }
    }

    #[test]
    fn disconnect_retries_then_restarts_music_without_stale_speech() {
        let now = Instant::now();
        let mut recovery = Recovery::new(now);
        let (stream, rx, mut report) = connection();
        recovery.poll(now, || Ok(stream));
        recovery.play(sound(Bus::Music));
        rx.try_recv().unwrap();
        // Even a full notice slot cannot mask device loss or lose its diagnostic.
        for _ in 0..1000 {
            report(ErrorKind::Xrun.into());
        }
        report(Error::with_message(
            ErrorKind::DeviceNotAvailable,
            "test unplug",
        ));
        recovery.poll(now, || panic!("retried without delay"));
        assert!(recovery.connection.is_none());
        recovery.play(sound(Bus::Eva));
        recovery.play(sound(Bus::Effect));
        recovery.poll(now + RETRY_MIN - Duration::from_millis(1), || {
            panic!("retried early")
        });
        recovery.poll(now + RETRY_MIN, || Err(ErrorKind::DeviceBusy.into()));
        assert_eq!(recovery.next_attempt, Some(now + RETRY_MIN * 3));
        let (replacement, rx, _report) = connection();
        recovery.poll(now + RETRY_MIN * 3, || Ok(replacement));
        assert!(recovery.connection.is_some());
        let resumed = rx.try_recv().unwrap();
        assert_eq!(resumed.bus, Bus::Music);
        assert!(Arc::ptr_eq(
            &resumed.pcm,
            &recovery.music.as_ref().unwrap().pcm
        ));
        assert!(rx.try_recv().is_err());
        recovery.play(sound(Bus::Response));
        assert_eq!(rx.try_recv().unwrap().bus, Bus::Response);
    }

    #[test]
    fn startup_outage_uses_capped_backoff_and_permanent_errors_stop_retries() {
        let mut now = Instant::now();
        let mut recovery = Recovery::<()>::new(now);
        recovery.play(sound(Bus::Music));
        for ms in [500, 1000, 2000, 4000, 8000, 8000] {
            recovery.poll(now, || Err(ErrorKind::DeviceNotAvailable.into()));
            assert_eq!(recovery.next_attempt, Some(now + Duration::from_millis(ms)));
            now = recovery.next_attempt.unwrap();
        }
        let (stream, rx, mut report) = connection();
        recovery.poll(now, || Ok(stream));
        assert_eq!(rx.try_recv().unwrap().bus, Bus::Music);
        report(ErrorKind::PermissionDenied.into());
        recovery.poll(now, || panic!("retried a permission failure"));
        assert!(recovery.connection.is_none());
        assert!(recovery.next_attempt.is_none());
        recovery.poll(now + Duration::from_secs(3600), || {
            panic!("retried a permanent failure")
        });
    }

    #[test]
    fn callback_preserves_error_kind_and_backend_message() {
        let (mut report, errors) = error_channel();
        let error = Error::with_message(ErrorKind::StreamInvalidated, "backend status -50");
        report(error.clone());
        assert_eq!(errors.faults.try_recv().unwrap(), error);
        assert!(errors.notices.try_recv().is_err());
    }

    #[test]
    fn slow_device_open_does_not_block_app_or_mute() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (tx, rx) = mpsc::sync_channel(64);
        let output = LiveOutput {
            tx,
            muted: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(Stats::default()),
        };
        let muted = output.muted.clone();
        let stats = output.stats.clone();
        let (stream, plays, _report) = connection();
        let mut stream = Some(stream);
        let worker = std::thread::spawn(move || {
            run(rx, || {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                assert!(muted.load(Ordering::Relaxed));
                stats.frames.fetch_add(10, Ordering::Relaxed);
                Ok(stream.take().expect("unexpected second open"))
            })
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(output.toggle_mute());
        output.play(sound(Bus::Music));
        for _ in 0..1000 {
            output.play(sound(Bus::Effect));
        }
        assert_eq!(output.frames_played(), 0);
        release_tx.send(()).unwrap();
        assert_eq!(
            plays.recv_timeout(Duration::from_secs(2)).unwrap().bus,
            Bus::Music
        );
        assert!(
            plays.try_recv().is_err(),
            "replayed expired effects after slow open"
        );
        assert_eq!(output.frames_played(), 10);
        drop(output);
        worker.join().unwrap();
    }

    #[test]
    #[ignore = "opens the native output device and plays the retail theme"]
    fn native_output_recovers_and_preserves_mute() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        let mut bank =
            super::super::SoundBank::new(yr_assets::AssetFs::open(&dir).unwrap(), "EB1.MMX");
        let theme = bank.theme().expect("retail theme");
        let muted = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Stats::default());
        let mut recovery = Recovery::new(Instant::now());
        let open = || Connection::open(&muted, &stats);
        recovery.poll(Instant::now(), open);
        assert!(recovery.connection.is_some(), "native output unavailable");
        recovery.play(theme.playback);
        let wait_signal = |recovery: &mut Recovery<cpal::Stream>, baseline| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while stats.signal_frames.load(Ordering::Relaxed) < baseline + 4800 {
                assert!(
                    Instant::now() < deadline,
                    "native callback stopped delivering signal"
                );
                recovery.poll(Instant::now(), open);
                std::thread::sleep(POLL);
            }
        };
        wait_signal(&mut recovery, 0);
        println!(
            "native callback before interruption: {} signal frames",
            stats.signal_frames.load(Ordering::Relaxed)
        );
        muted.store(true, Ordering::Relaxed);
        recovery
            .connection
            .as_ref()
            .unwrap()
            .errors
            .inject
            .try_send(Error::with_message(
                ErrorKind::DeviceNotAvailable,
                "injected disconnect for recovery proof",
            ))
            .unwrap();
        recovery.poll(Instant::now(), || panic!("retried before deadline"));
        assert!(recovery.connection.is_none());
        std::thread::sleep(RETRY_MIN);
        // The production opener supplies a new native stream with the same mute/stats.
        recovery.poll(Instant::now(), || Connection::open(&muted, &stats));
        assert!(recovery.connection.is_some());
        std::thread::sleep(Duration::from_millis(100));
        let silent_signal = stats.signal_frames.load(Ordering::Relaxed);
        let silent_frames = stats.frames.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(100));
        assert!(stats.frames.load(Ordering::Relaxed) > silent_frames);
        assert_eq!(stats.signal_frames.load(Ordering::Relaxed), silent_signal);
        assert!(muted.load(Ordering::Relaxed));
        println!("native replacement delivered frames while muted; signal counter unchanged at {silent_signal}");
        muted.store(false, Ordering::Relaxed);
        wait_signal(&mut recovery, silent_signal);
        println!(
            "native callback after recovery/unmute: {} signal frames",
            stats.signal_frames.load(Ordering::Relaxed)
        );
    }
}
