//! Bounded, one-shot retail effect canvases. Playback never advances on draw.

use crate::sprites::Sprite;
use std::collections::{HashMap, VecDeque};
use yr_assets::AssetFs;
use yr_formats::{IniFile, Palette, ShpFile};

const MAX_CACHED_FRAMES: usize = 64;

struct Clip {
    file: String,
    start: usize,
    end: usize,
    rate: u64,
    alpha: u8,
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Clip {
    fn load(fs: &AssetFs, art: &IniFile, name: &str) -> Option<Self> {
        if !safe_name(name) {
            return None;
        }
        let section = art.section(name);
        let image = section.and_then(|s| s.get("Image")).unwrap_or(name);
        if !safe_name(image) {
            return None;
        }
        let file = format!("{image}.SHP");
        let shp = ShpFile::parse(fs.read(&file)?).ok()?;
        // Optional effects cannot allocate arbitrary canvases or populate
        // a cache from hostile header counts. Parsing also bounds by bytes.
        if shp.width == 0
            || shp.height == 0
            || shp.width > 512
            || shp.height > 512
            || shp.frames.len() > 256
        {
            return None;
        }
        let number = |key, default| match section.and_then(|s| s.get(key)) {
            Some(v) => v.parse::<u64>().ok(),
            None => Some(default),
        };
        let start = usize::try_from(number("Start", 0)?).ok()?;
        let end = usize::try_from(number("End", shp.frames.len() as u64)?).ok()?;
        let rate = number("Rate", 200)?;
        let translucent = section
            .and_then(|s| s.get_bool("Translucent"))
            .unwrap_or(false);
        let transparency = number("Translucency", if translucent { 50 } else { 0 })?;
        if start >= end || end > shp.frames.len() || rate == 0 || transparency > 100 {
            return None;
        }
        Some(Self {
            file,
            start,
            end,
            rate,
            alpha: (255 * (100 - transparency) / 100) as u8,
        })
    }

    fn frame(&self, ticks: u64) -> Option<usize> {
        let phase = u128::from(ticks) * u128::from(self.rate) / 225;
        (phase < (self.end - self.start) as u128).then(|| self.start + phase as usize)
    }
}

#[derive(Default)]
pub(crate) struct Effects {
    clips: HashMap<String, Option<Clip>>,
    frames: HashMap<(String, usize), Option<Sprite>>,
    order: VecDeque<(String, usize)>,
}

impl Effects {
    fn clip(&mut self, fs: &AssetFs, art: &IniFile, name: &str) -> Option<&Clip> {
        // Names originate in the bounded rules registry, never in clocks.
        if !safe_name(name) || self.clips.len() >= 128 && !self.clips.contains_key(name) {
            return None;
        }
        self.clips
            .entry(name.into())
            .or_insert_with(|| Clip::load(fs, art, name))
            .as_ref()
    }

    pub fn alive(&mut self, fs: &AssetFs, art: &IniFile, name: &str, ticks: u64) -> bool {
        self.clip(fs, art, name)
            .and_then(|c| c.frame(ticks))
            .is_some()
    }

    pub fn get(&mut self, fs: &AssetFs, art: &IniFile, name: &str, ticks: u64) -> Option<&Sprite> {
        let clip = self.clip(fs, art, name)?;
        let frame = clip.frame(ticks)?;
        let file = clip.file.clone();
        let alpha = clip.alpha;
        let key = (name.to_string(), frame);
        if !self.frames.contains_key(&key) {
            let sprite = (|| {
                let pal = Palette::parse(fs.read("ANIM.PAL")?)?;
                let shp = ShpFile::parse(fs.read(&file)?).ok()?;
                let rgba = shp
                    .decode_frame_canvas(frame)
                    .ok()?
                    .into_iter()
                    .flat_map(|index| {
                        if index == 0 {
                            [0; 4]
                        } else {
                            let [r, g, b, _] = pal.rgba(index);
                            [r, g, b, alpha]
                        }
                    })
                    .collect();
                Some(Sprite {
                    w: u32::from(shp.width),
                    h: u32::from(shp.height),
                    ax: i32::from(shp.width) / 2,
                    ay: i32::from(shp.height) / 2,
                    rgba,
                })
            })();
            if self.frames.len() >= MAX_CACHED_FRAMES {
                if let Some(old) = self.order.pop_front() {
                    self.frames.remove(&old);
                }
            }
            self.order.push_back(key.clone());
            self.frames.insert(key.clone(), sprite);
        }
        self.frames.get(&key)?.as_ref()
    }
}

#[cfg(test)]
mod tests;
