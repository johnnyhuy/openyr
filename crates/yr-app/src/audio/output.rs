//! Native device output. The callback only consumes decoded PCM commands.

use super::mixer::{Mixer, Playback};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};

#[derive(Default)]
struct Stats {
    frames: AtomicU64,
    signal_frames: AtomicU64,
}

pub struct LiveOutput {
    _stream: cpal::Stream,
    tx: mpsc::SyncSender<Playback>,
    muted: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    stats: Arc<Stats>,
}

impl LiveOutput {
    pub fn open() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let format = supported.sample_format();
        let config = supported.config();
        let (tx, rx) = mpsc::sync_channel(64);
        let muted = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Stats::default());
        macro_rules! build {
            ($t:ty) => {
                build::<$t>(
                    &device,
                    config.clone(),
                    rx,
                    muted.clone(),
                    failed.clone(),
                    stats.clone(),
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
            _ => return Err(format!("unsupported audio output format {format:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        println!(
            "OpenYR: audio output {} Hz, {} channels, {format:?}",
            config.sample_rate, config.channels
        );
        Ok(Self {
            _stream: stream,
            tx,
            muted,
            failed,
            stats,
        })
    }

    pub fn play(&self, sound: Playback) {
        let _ = self.tx.try_send(sound);
    }

    pub fn toggle_mute(&self) -> bool {
        !self.muted.fetch_xor(true, Ordering::Relaxed)
    }

    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    pub fn signal_frames(&self) -> u64 {
        self.stats.signal_frames.load(Ordering::Relaxed)
    }

    pub fn frames_played(&self) -> u64 {
        self.stats.frames.load(Ordering::Relaxed)
    }
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    rx: mpsc::Receiver<Playback>,
    muted: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    stats: Arc<Stats>,
) -> Result<cpal::Stream, cpal::Error> {
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
        move |_| {
            failed.store(true, Ordering::Relaxed);
        },
        None,
    )
}
