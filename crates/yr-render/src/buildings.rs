//! Building layers resolved from retail art data. Playback is presentation
//! state only; no animation clock changes the simulation.

use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, IniSection, ShpFile};

/// State sampled from the simulation when a building is drawn.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingAnimation {
    pub elapsed: u64,
    /// Build progress in whole ticks. Absent when this factory is idle.
    /// Stalled production keeps the same progress and therefore frame.
    pub production: Option<u64>,
    pub damaged: bool,
    pub powered: bool,
}

#[derive(Clone)]
pub(crate) struct Layer {
    pub file: String,
    pub offset: (i32, i32),
    pub shadow: bool,
    pub frames: usize,
    pub clip: Clip,
    pub powered: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Clip {
    start: usize,
    end: usize,
    rate: u64,
}

impl Clip {
    fn parse(section: &IniSection, frames: usize) -> Option<Self> {
        let number = |key, default| match section.get(key) {
            Some(value) => value.parse::<usize>().ok(),
            None => Some(default),
        };
        let start = number("LoopStart", number("Start", 0)?)?;
        let end = number("LoopEnd", frames)?;
        let rate = number("Rate", 200)?;
        (start < end && end <= frames && rate > 0).then_some(Self {
            start,
            end,
            rate: rate as u64,
        })
    }

    pub fn frame(self, ticks: u64) -> usize {
        // Prototype interpretation of Rate: rate/900 frames per 60 Hz
        // logic tick, or rate/225 at our 15 Hz. This remains provisional
        // until the Wine comparison. Widen before multiplying clocks.
        let phase =
            (u128::from(ticks) * u128::from(self.rate) / 225) % (self.end - self.start) as u128;
        self.start + phase as usize
    }
}

pub(crate) struct Overlay {
    normal: Option<Layer>,
    damaged: Option<Layer>,
}

impl Overlay {
    pub fn layer(&self, damaged: bool) -> Option<&Layer> {
        if damaged {
            self.damaged.as_ref().or(self.normal.as_ref())
        } else {
            self.normal.as_ref()
        }
    }
}

pub(crate) struct BuildingArt {
    pub base: Layer,
    pub bib: Option<Layer>,
    pub active: Vec<Overlay>,
    pub production: Overlay,
}

impl BuildingArt {
    pub fn load(
        vfs: &AssetFs,
        ini: &IniFile,
        image: &str,
        section: &str,
        theater: Theater,
        foundation: (u8, u8),
    ) -> Option<Self> {
        let section = ini.section(section)?;
        let (fx, fy) = (i32::from(foundation.0), i32::from(foundation.1));
        let offset = ((fx - fy) * 15, (fx + fy - 2) * 15 / 2);
        let base = load_layer(vfs, ini, image, theater, offset, true)?;
        let bib = section
            .get("BibShape")
            .and_then(|name| load_layer(vfs, ini, name, theater, offset, true));
        let overlay = |key: &str| {
            let load = |name: &str| {
                let coord = |axis| match section.get(&format!("{key}{axis}")) {
                    Some(v) => v.parse::<i32>().ok(),
                    None => Some(0),
                };
                let offset = (
                    offset.0.checked_add(coord("X")?)?,
                    offset.1.checked_add(coord("Y")?)?,
                );
                let shadow = ini
                    .section(name)
                    .and_then(|s| s.get_bool("Shadow"))
                    .unwrap_or(false);
                let mut layer = load_layer(vfs, ini, name, theater, offset, shadow)?;
                layer.powered = section.get_bool(&format!("{key}Powered")).unwrap_or(false);
                Some(layer)
            };
            Overlay {
                normal: section.get(key).and_then(load),
                damaged: section.get(&format!("{key}Damaged")).and_then(load),
            }
        };
        Some(Self {
            base,
            bib,
            active: [
                "ActiveAnim",
                "ActiveAnimTwo",
                "ActiveAnimThree",
                "ActiveAnimFour",
            ]
            .iter()
            .map(|key| overlay(key))
            .collect(),
            production: overlay("ProductionAnim"),
        })
    }
}

fn load_layer(
    vfs: &AssetFs,
    ini: &IniFile,
    name: &str,
    theater: Theater,
    offset: (i32, i32),
    shadow: bool,
) -> Option<Layer> {
    let section = ini.section(name);
    let image = section.and_then(|s| s.get("Image")).unwrap_or(name);
    let file = crate::building_shp_candidates(&image.to_ascii_uppercase(), theater)
        .into_iter()
        .find(|file| vfs.read(file).is_some())?;
    let shp = ShpFile::parse(vfs.read(&file)?).ok()?;
    // All probed building layers store an equal shadow half, including
    // overlays that do not request shadow drawing in artmd.ini.
    let frames = shp.frames.len() / 2;
    if frames == 0 {
        return None;
    }
    let clip = match section {
        Some(section) => Clip::parse(section, frames)?,
        None => Clip {
            start: 0,
            end: frames,
            rate: 200,
        },
    };
    Some(Layer {
        file,
        offset,
        shadow,
        frames,
        clip,
        powered: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_uses_exclusive_end_and_bounded_integer_clock() {
        let ini = IniFile::parse("[A]\nStart=3\nLoopStart=3\nLoopEnd=6\nRate=200\n");
        let clip = Clip::parse(ini.section("A").unwrap(), 6).unwrap();
        assert_eq!(
            (0..5).map(|t| clip.frame(t)).collect::<Vec<_>>(),
            [3, 3, 4, 5, 3]
        );
        assert!((3..6).contains(&clip.frame(u64::MAX)));
        let ini = IniFile::parse(&format!("[A]\nRate={}\n", u64::MAX));
        let clip = Clip::parse(ini.section("A").unwrap(), 6).unwrap();
        assert!(clip.frame(u64::MAX) < 6);
    }

    #[test]
    fn hostile_clips_are_rejected() {
        for value in [
            "LoopStart=-1",
            "LoopStart=6",
            "LoopEnd=7",
            "LoopEnd=0",
            "Rate=0",
            "Rate=-1",
            "LoopStart=oops",
            "Start=oops",
            "LoopEnd=18446744073709551616",
        ] {
            let ini = IniFile::parse(&format!("[A]\n{value}\n"));
            assert!(
                Clip::parse(ini.section("A").unwrap(), 6).is_none(),
                "{value}"
            );
        }
    }
}
