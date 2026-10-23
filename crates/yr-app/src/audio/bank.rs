//! Retail sound definitions and lazily decoded PCM, owned by the app thread.

use super::mixer::{Bus, Playback};
use std::collections::BTreeMap;
use std::sync::Arc;
use yr_assets::AssetFs;
use yr_formats::audio::{decode_wav, parse_bag_index, AudFile, BagEntry, Pcm};
use yr_formats::{CsfFile, IniFile, MixArchive};

struct Bank {
    container: Option<String>,
    bag_range: std::ops::Range<usize>,
    entries: Vec<BagEntry>,
}

pub struct Cue {
    pub event: String,
    pub sample: String,
    pub playback: Playback,
}

pub struct SoundBank {
    fs: AssetFs,
    pub rules: IniFile,
    sounds: IniFile,
    eva: IniFile,
    themes: IniFile,
    csf: Option<CsfFile>,
    banks: Vec<Bank>,
    cache: BTreeMap<String, Option<Arc<Pcm>>>,
    choices: BTreeMap<String, usize>,
}

pub fn read_ini(fs: &AssetFs, name: &str) -> IniFile {
    fs.read(name)
        .map(|b| IniFile::parse(&String::from_utf8_lossy(b)))
        .unwrap_or_default()
}

fn tokens(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
}

impl SoundBank {
    pub fn new(fs: AssetFs, map: &str) -> Self {
        let mut rules = read_ini(&fs, "RULESMD.INI");
        rules.merge_from(&read_ini(&fs, map));
        let mut banks = Vec::new();
        // The VFS winner covers loose overrides and expansion archives.
        // Only pair files from the same source, never an IDX with another BAG.
        if fs
            .locate("AUDIO.IDX")
            .zip(fs.locate("AUDIO.BAG"))
            .is_some_and(|(a, b)| a.source == b.source)
        {
            if let (Some(idx), Some(bag)) = (fs.read("AUDIO.IDX"), fs.read("AUDIO.BAG")) {
                if let Ok(entries) = parse_bag_index(idx) {
                    banks.push(Bank {
                        container: None,
                        bag_range: 0..bag.len(),
                        entries,
                    });
                }
            }
        }
        // Both banks are needed: YR's bank does not contain all RA2 voices.
        for container in ["AUDIOMD.MIX", "AUDIO.MIX"] {
            let Some(bytes) = fs.read(container) else {
                continue;
            };
            let Ok(mix) = MixArchive::parse(bytes) else {
                continue;
            };
            let (Some(idx), Some(bag)) = (mix.get("AUDIO.IDX"), mix.get("AUDIO.BAG")) else {
                continue;
            };
            let Ok(entries) = parse_bag_index(idx) else {
                continue;
            };
            let start = bag.as_ptr() as usize - bytes.as_ptr() as usize;
            banks.push(Bank {
                container: Some(container.into()),
                bag_range: start..start + bag.len(),
                entries,
            });
        }
        Self {
            sounds: read_ini(&fs, "SOUNDMD.INI"),
            eva: read_ini(&fs, "EVAMD.INI"),
            themes: read_ini(&fs, "THEMEMD.INI"),
            csf: fs.read("RA2MD.CSF").and_then(|b| CsfFile::parse(b).ok()),
            fs,
            rules,
            banks,
            cache: BTreeMap::new(),
            choices: BTreeMap::new(),
        }
    }

    /// File WAV/AUD first, then YR and RA2 sound banks. Failed lookups are
    /// cached too, so absent samples stay silent without repeated decoding.
    pub fn sample(&mut self, name: &str) -> Option<Arc<Pcm>> {
        let name = name.trim_start_matches('$');
        let key = name.to_ascii_uppercase();
        if let Some(hit) = self.cache.get(&key) {
            return hit.clone();
        }
        let decoded = self
            .decode(name)
            .filter(|p| {
                matches!(p.channels, 1 | 2)
                    && (1..=192_000).contains(&p.sample_rate)
                    && !p.samples.is_empty()
                    && p.samples.len().is_multiple_of(usize::from(p.channels))
            })
            .map(Arc::new);
        self.cache.insert(key, decoded.clone());
        decoded
    }

    fn decode(&self, name: &str) -> Option<Pcm> {
        let has_ext = name.contains('.');
        for file in if has_ext {
            vec![name.to_string()]
        } else {
            vec![format!("{name}.WAV"), format!("{name}.AUD")]
        } {
            if let Some(data) = self.fs.read(&file) {
                let decoded = if data.starts_with(b"RIFF") {
                    decode_wav(data)
                } else {
                    AudFile::decode(data)
                };
                if let Ok(pcm) = decoded {
                    return Some(pcm);
                }
            }
        }
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        for bank in &self.banks {
            let Some(entry) = bank
                .entries
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(stem))
            else {
                continue;
            };
            let data = self
                .fs
                .read(bank.container.as_deref().unwrap_or("AUDIO.BAG"))?;
            let Some(bag) = data.get(bank.bag_range.clone()) else {
                continue;
            };
            if let Ok(pcm) = entry.decode(bag) {
                return Some(pcm);
            }
        }
        None
    }

    pub fn cue(&mut self, event: &str, bus: Bus) -> Option<Cue> {
        let (samples, gain) = if bus == Bus::Eva {
            let name = self
                .eva
                .value(event, "Allied")
                .map(str::to_string)
                .or_else(|| {
                    self.csf
                        .as_ref()?
                        .entries
                        .iter()
                        .find(|e| e.label.eq_ignore_ascii_case(event))?
                        .sound
                        .clone()
                })?;
            (vec![name], 1.0)
        } else {
            let section = self.sounds.section(event)?;
            let samples: Vec<String> = tokens(section.get("Sounds")?)
                .map(|s| s.trim_start_matches('$').to_string())
                .collect();
            let gain = section
                .get("Volume")
                .and_then(yr_rules::Decimal::parse)
                .map_or(1.0, |d| {
                    d.units().clamp(0, 100_000_000) as f32 / 100_000_000.0
                });
            (samples, gain)
        };
        if samples.is_empty() {
            return None;
        }
        let choice = self.choices.entry(event.to_ascii_uppercase()).or_default();
        let start = *choice % samples.len();
        *choice = choice.wrapping_add(1);
        for i in 0..samples.len() {
            let sample = &samples[(start + i) % samples.len()];
            if let Some(pcm) = self.sample(sample) {
                return Some(Cue {
                    event: event.into(),
                    sample: sample.clone(),
                    playback: Playback { pcm, bus, gain },
                });
            }
        }
        None
    }

    /// Loop the first available normal theme in retail list order.
    pub fn theme(&mut self) -> Option<Cue> {
        let candidates: Vec<String> = self
            .themes
            .section("Themes")?
            .entries
            .iter()
            .filter_map(|(_, id)| {
                let s = self.themes.section(id)?;
                (s.get_bool("Normal") != Some(false))
                    .then(|| s.get("Sound").unwrap_or(id).to_string())
            })
            .collect();
        for sample in candidates {
            if let Some(pcm) = self.sample(&sample) {
                return Some(Cue {
                    event: "Theme".into(),
                    sample,
                    playback: Playback {
                        pcm,
                        bus: Bus::Music,
                        gain: 1.0,
                    },
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_formats::audio::encode_wav;

    #[test]
    fn definitions_strip_dollar_cycle_samples_and_apply_volume() {
        let wav = encode_wav(&Pcm {
            sample_rate: 22050,
            channels: 1,
            samples: vec![100, -100],
        });
        let fs = AssetFs::from_files(vec![
            (
                "SOUNDMD.INI".into(),
                b"[Select]\nSounds=$a, $b missing\nVolume=85\n".to_vec(),
            ),
            ("EVAMD.INI".into(), b"[EVA_UnitReady]\nAllied=a\n".to_vec()),
            ("a.wav".into(), wav.clone()),
            ("b.wav".into(), wav),
        ]);
        let mut bank = SoundBank::new(fs, "map");
        let a = bank.cue("Select", Bus::Response).unwrap();
        assert_eq!(a.sample, "a");
        assert_eq!(a.playback.gain, 0.85);
        assert_eq!(bank.cue("Select", Bus::Response).unwrap().sample, "b");
        assert_eq!(bank.cue("Select", Bus::Response).unwrap().sample, "a");
        assert!(Arc::ptr_eq(&a.playback.pcm, &bank.sample("A").unwrap()));
        assert!(bank.cue("absent", Bus::Effect).is_none());
        assert!(bank.sample("missing").is_none());
        assert_eq!(bank.cue("EVA_UnitReady", Bus::Eva).unwrap().sample, "a");
    }

    fn mix(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((files.iter().map(|(_, b)| b.len()).sum::<usize>() as u32).to_le_bytes());
        let mut offset = 0u32;
        for (name, bytes) in files {
            out.extend(yr_formats::mix_id(name).to_le_bytes());
            out.extend(offset.to_le_bytes());
            out.extend((bytes.len() as u32).to_le_bytes());
            offset += bytes.len() as u32;
        }
        for (_, bytes) in files {
            out.extend(bytes);
        }
        out
    }

    fn bank_archive(name: &str, sample: i16) -> Vec<u8> {
        let mut idx = b"GABA".to_vec();
        idx.extend(2u32.to_le_bytes());
        idx.extend(1u32.to_le_bytes());
        let mut stem = [0; 16];
        stem[..name.len()].copy_from_slice(name.as_bytes());
        idx.extend(stem);
        for field in [0u32, 2, 22050, 6, 0] {
            idx.extend(field.to_le_bytes());
        }
        mix(&[
            ("AUDIO.IDX", idx),
            ("AUDIO.BAG", sample.to_le_bytes().to_vec()),
        ])
    }

    #[test]
    fn bank_indices_stay_paired_and_incomplete_loose_override_does_not_hide_ra2() {
        let fs = AssetFs::from_files(vec![
            (
                "LANGMD.MIX".into(),
                mix(&[("AUDIOMD.MIX", bank_archive("shared", 200))]),
            ),
            (
                "LANGUAGE.MIX".into(),
                mix(&[("AUDIO.MIX", bank_archive("legacy", 100))]),
            ),
            // This loose index must not be paired with the archive's bag.
            ("AUDIO.IDX".into(), b"corrupt loose override".to_vec()),
        ]);
        let mut bank = SoundBank::new(fs, "map");
        assert_eq!(bank.sample("shared").unwrap().samples, [200]);
        assert_eq!(bank.sample("legacy").unwrap().samples, [100]);
        assert!(bank.sample("absent").is_none());
    }
}
