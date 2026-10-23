//! Bounded PCM mixer shared by the device callback and headless proof runs.

use std::collections::VecDeque;
use std::sync::Arc;
use yr_formats::audio::Pcm;

const MAX_VOICES: usize = 32;
const MAX_EVA: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    Music,
    Response,
    Eva,
    Effect,
}

#[derive(Clone)]
pub struct Playback {
    pub pcm: Arc<Pcm>,
    pub bus: Bus,
    pub gain: f32,
}

struct Voice {
    sound: Playback,
    /// Source frame position multiplied by the output sample rate.
    phase: u64,
}

pub struct Mixer {
    rate: u32,
    voices: Vec<Voice>,
    eva: VecDeque<Playback>,
}

impl Mixer {
    pub fn new(rate: u32) -> Self {
        assert!(rate > 0);
        Self {
            rate,
            voices: Vec::with_capacity(MAX_VOICES),
            eva: VecDeque::with_capacity(MAX_EVA),
        }
    }

    pub fn play(&mut self, sound: Playback) {
        if !matches!(sound.pcm.channels, 1 | 2)
            || sound.pcm.sample_rate == 0
            || sound.pcm.samples.is_empty()
            || !sound
                .pcm
                .samples
                .len()
                .is_multiple_of(usize::from(sound.pcm.channels))
            || !sound.gain.is_finite()
        {
            return;
        }
        match sound.bus {
            Bus::Music | Bus::Response => self.voices.retain(|v| v.sound.bus != sound.bus),
            Bus::Eva if self.voices.iter().any(|v| v.sound.bus == Bus::Eva) => {
                if self.eva.len() < MAX_EVA {
                    self.eva.push_back(sound);
                }
                return;
            }
            _ => {}
        }
        if self.voices.len() == MAX_VOICES {
            if let Some(i) = self.voices.iter().position(|v| v.sound.bus == Bus::Effect) {
                self.voices.remove(i);
            } else {
                return;
            }
        }
        self.voices.push(Voice { sound, phase: 0 });
    }

    /// One stereo output frame, with linear sample-rate conversion. A mono
    /// source feeds both speakers; music loops and EVA lines play serially.
    pub fn next_frame(&mut self) -> [f32; 2] {
        if !self.voices.iter().any(|v| v.sound.bus == Bus::Eva) {
            if let Some(eva) = self.eva.pop_front() {
                self.play(eva);
            }
        }
        let mut out = [0.0; 2];
        for v in &mut self.voices {
            let pcm = &v.sound.pcm;
            let channels = usize::from(pcm.channels);
            let frames = pcm.samples.len() / channels;
            let end = frames as u64 * u64::from(self.rate);
            if v.phase >= end {
                continue;
            }
            let at = (v.phase / u64::from(self.rate)) as usize;
            let next = if at + 1 < frames {
                at + 1
            } else if v.sound.bus == Bus::Music {
                0
            } else {
                at
            };
            let fraction = (v.phase % u64::from(self.rate)) as f32 / self.rate as f32;
            let level = match v.sound.bus {
                Bus::Music => 0.18,
                Bus::Response | Bus::Eva => 0.8,
                Bus::Effect => 0.5,
            } * v.sound.gain.clamp(0.0, 1.0);
            for (channel, output) in out.iter_mut().enumerate() {
                let c = channel.min(channels - 1);
                let a = f32::from(pcm.samples[at * channels + c]);
                let b = f32::from(pcm.samples[next * channels + c]);
                *output += (a + (b - a) * fraction) / 32768.0 * level;
            }
            v.phase += u64::from(pcm.sample_rate);
            if v.sound.bus == Bus::Music {
                v.phase %= end;
            }
        }
        self.voices.retain(|v| {
            v.phase
                < (v.sound.pcm.samples.len() / usize::from(v.sound.pcm.channels)) as u64
                    * u64::from(self.rate)
        });
        out.map(|s| s.clamp(-1.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(bus: Bus, rate: u32, channels: u16, samples: Vec<i16>) -> Playback {
        Playback {
            pcm: Arc::new(Pcm {
                sample_rate: rate,
                channels,
                samples,
            }),
            bus,
            gain: 1.0,
        }
    }

    #[test]
    fn resamples_and_preserves_stereo_then_ends() {
        let mut m = Mixer::new(4);
        m.play(sound(Bus::Effect, 2, 2, vec![0, 16000, 16000, 0]));
        assert_eq!(m.next_frame(), [0.0, 8000.0 / 32768.0]);
        assert_eq!(m.next_frame(), [4000.0 / 32768.0; 2]);
        assert_eq!(m.next_frame(), [8000.0 / 32768.0, 0.0]);
        assert_eq!(m.next_frame(), [8000.0 / 32768.0, 0.0]);
        assert_eq!(m.next_frame(), [0.0; 2]);
    }

    #[test]
    fn music_wraps_and_mono_fills_both_channels() {
        let mut m = Mixer::new(2);
        m.play(sound(Bus::Music, 2, 1, vec![10000, -10000]));
        let a = m.next_frame();
        let b = m.next_frame();
        assert_eq!(a[0], a[1]);
        assert!(a[0] > 0.0 && b[0] < 0.0);
        assert_eq!(m.next_frame(), a);
        assert_eq!(m.next_frame(), b);
    }

    #[test]
    fn voices_are_bounded_responses_replace_and_eva_queues() {
        let mut m = Mixer::new(1);
        for _ in 0..1000 {
            m.play(sound(Bus::Effect, 1, 1, vec![32767]));
        }
        assert_eq!(m.voices.len(), MAX_VOICES);
        assert_eq!(m.next_frame(), [1.0; 2]);
        for _ in 0..2 {
            m.play(sound(Bus::Response, 1, 1, vec![1000]));
        }
        assert_eq!(m.voices.len(), 1);
        for _ in 0..100 {
            m.play(sound(Bus::Eva, 1, 1, vec![2000]));
        }
        assert_eq!(m.eva.len(), MAX_EVA);
        assert_eq!(
            m.voices.iter().filter(|v| v.sound.bus == Bus::Eva).count(),
            1
        );
        for _ in 0..9 {
            assert!(m.next_frame()[0] > 0.0);
        }
        assert_eq!(m.next_frame(), [0.0; 2]);
    }

    #[test]
    fn invalid_pcm_is_silent() {
        let mut m = Mixer::new(44100);
        for (rate, channels, data) in [
            (0, 1, vec![1]),
            (1, 0, vec![1]),
            (1, 2, vec![1]),
            (1, 1, vec![]),
        ] {
            m.play(sound(Bus::Effect, rate, channels, data));
        }
        assert_eq!(m.next_frame(), [0.0; 2]);
    }
}
