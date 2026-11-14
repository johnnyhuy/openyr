//! Bounded, one-shot retail effect canvases. Playback never advances on draw.

use crate::sprites::Sprite;
use std::collections::{HashMap, VecDeque};
use yr_assets::AssetFs;
use yr_formats::{IniFile, IniSection, Palette, ShpFile};

const MAX_CACHED_FRAMES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageSmokeSpec {
    pub particle: String,
    pub interval: u64,
    pub cap: usize,
}

pub(crate) fn damage_smoke_spec(
    rules: &IniFile,
    kind: &str,
    hp: i64,
    max_hp: i64,
) -> Option<DamageSmokeSpec> {
    let value = rules.value("AudioVisual", "ConditionYellow")?;
    // Bound the whole part before the shared exact-decimal conversion.
    let number = value
        .trim()
        .strip_suffix('%')
        .unwrap_or(value.trim())
        .trim();
    let whole = number
        .strip_prefix('+')
        .unwrap_or(number)
        .split('.')
        .next()?;
    if !whole.is_empty() && whole.parse::<u64>().ok()? > 100 {
        return None;
    }
    let threshold = yr_rules::Decimal::parse(value)?;
    if threshold <= yr_rules::Decimal::ZERO
        || threshold > yr_rules::Decimal::ONE
        || hp <= 0
        || max_hp <= 0
        || i128::from(hp) * 1_000_000 > i128::from(max_hp) * i128::from(threshold.units())
    {
        return None;
    }
    let systems: Vec<_> = rules
        .value(kind, "DamageParticleSystems")?
        .split(',')
        .map(str::trim)
        .take(33)
        .collect();
    if systems.len() > 32 {
        return None;
    }
    systems.into_iter().find_map(|name| {
        if !safe_name(name) {
            return None;
        }
        let system = rules.section(name)?;
        if !system.get("BehavesLike")?.eq_ignore_ascii_case("Smoke")
            || system.get_bool("Spawns") != Some(true)
        {
            return None;
        }
        let particle = system.get("HoldsWhat")?;
        let frames: u64 = system.get("SpawnFrames")?.parse().ok()?;
        let cap: usize = system.get("ParticleCap")?.parse().ok()?;
        if !safe_name(particle) || !(1..=240).contains(&frames) || !(1..=64).contains(&cap) {
            return None;
        }
        Some(DamageSmokeSpec {
            particle: particle.to_ascii_uppercase(),
            interval: frames.div_ceil(4),
            cap,
        })
    })
}

pub(crate) fn impact_name(section: &IniSection, damage: i64) -> Option<String> {
    if damage <= 0 || section.get_bool("EMEffect") == Some(true) {
        return None;
    }
    let names: Vec<_> = section
        .get("AnimList")?
        .split(',')
        .map(str::trim)
        .take(129)
        .collect();
    if names.is_empty() || names.len() > 128 {
        return None;
    }
    // Retail lists increase with delivered damage. The documented 25-point
    // bands ignore target armour and clamp to the final available entry.
    let index = usize::try_from(damage / 25).ok()?.min(names.len() - 1);
    names
        .get(index)
        .filter(|name| safe_name(name))
        .map(|name| name.to_string())
}

struct Clip {
    file: String,
    start: usize,
    end: usize,
    loop_start: usize,
    loop_end: usize,
    loop_count: u64,
    rate: u64,
    divisor: u64,
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
        // Flaming infantry have eight running groups before their collapse,
        // followed by a separate half-sheet. Only present the collapse here;
        // original running motion and its transition still need observation.
        let (first, limit) = if section.and_then(|s| s.get_bool("IsFlamingGuy")) == Some(true) {
            let running = usize::try_from(number("RunningFrames", 0)?).ok()?;
            let first = running.checked_mul(8)?;
            let limit = shp.frames.len() / 2;
            if running == 0 || shp.frames.len() % 2 != 0 || first >= limit {
                return None;
            }
            (first, limit)
        } else {
            (0, shp.frames.len())
        };
        let start = usize::try_from(number("Start", first as u64)?).ok()?;
        let end = usize::try_from(number("End", limit as u64)?).ok()?;
        let loop_start = usize::try_from(number("LoopStart", start as u64)?).ok()?;
        let loop_end = usize::try_from(number("LoopEnd", end as u64)?).ok()?;
        let loop_count = number("LoopCount", 1)?;
        let rate = number("Rate", 200)?;
        let translucent = section
            .and_then(|s| s.get_bool("Translucent"))
            .unwrap_or(false);
        let transparency = number("Translucency", if translucent { 50 } else { 0 })?;
        if start < first
            || start >= end
            || end > limit
            || loop_start < start
            || loop_start >= loop_end
            || loop_end > end
            || !(1..=1024).contains(&loop_count)
            || rate == 0
            || transparency > 100
        {
            return None;
        }
        Some(Self {
            file,
            start,
            end,
            loop_start,
            loop_end,
            loop_count,
            rate,
            divisor: 225,
            alpha: (255 * (100 - transparency) / 100) as u8,
        })
    }

    fn frame(&self, ticks: u64) -> Option<usize> {
        let phase = u128::from(ticks) * u128::from(self.rate) / u128::from(self.divisor);
        let prefix = (self.loop_start - self.start) as u128;
        if phase < prefix {
            return Some(self.start + phase as usize);
        }
        let phase = phase - prefix;
        let span = (self.loop_end - self.loop_start) as u128;
        let repeated = span * u128::from(self.loop_count);
        if phase < repeated {
            return Some(self.loop_start + (phase % span) as usize);
        }
        let tail = phase - repeated;
        (tail < (self.end - self.loop_end) as u128).then(|| self.loop_end + tail as usize)
    }

    fn load_smoke(fs: &AssetFs, rules: &IniFile, name: &str) -> Option<Self> {
        if !safe_name(name) {
            return None;
        }
        let section = rules.section(name)?;
        if !section.get("BehavesLike")?.eq_ignore_ascii_case("Smoke")
            || section.get_bool("DeleteOnStateLimit") != Some(true)
        {
            return None;
        }
        let image = section.get("Image")?;
        if !safe_name(image) {
            return None;
        }
        let file = format!("{image}.SHP");
        let shp = ShpFile::parse(fs.read(&file)?).ok()?;
        let number = |key, default| match section.get(key) {
            Some(v) => v.parse::<u64>().ok(),
            None => Some(default),
        };
        let start = usize::try_from(number("StartFrame", 0)?).ok()?;
        let end = usize::try_from(number("EndStateAI", 0)?).ok()?;
        let advance = number("StateAIAdvance", 0)?;
        let transparency = number("Translucency", 0)?;
        if shp.width == 0
            || shp.height == 0
            || shp.width > 512
            || shp.height > 512
            || shp.frames.len() > 256
            || start >= end
            || end > shp.frames.len()
            || !(1..=240).contains(&advance)
            || transparency > 100
        {
            return None;
        }
        Some(Self {
            file,
            start,
            end,
            loop_start: start,
            loop_end: end,
            loop_count: 1,
            // Provisional 60 Hz particle clocks sampled at 15 sim ticks/s.
            rate: 4,
            divisor: advance,
            alpha: (255 * (100 - transparency) / 100) as u8,
        })
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
        self.sprite(fs, name, frame, &file, alpha)
    }

    fn smoke_clip(&mut self, fs: &AssetFs, rules: &IniFile, name: &str) -> Option<&Clip> {
        if !safe_name(name) {
            return None;
        }
        // Particle names cannot alias animation names with different clocks.
        let key = format!("smoke:{name}");
        if self.clips.len() >= 128 && !self.clips.contains_key(&key) {
            return None;
        }
        self.clips
            .entry(key)
            .or_insert_with(|| Clip::load_smoke(fs, rules, name))
            .as_ref()
    }

    pub fn smoke_alive(&mut self, fs: &AssetFs, rules: &IniFile, name: &str, age: u64) -> bool {
        self.smoke_clip(fs, rules, name)
            .and_then(|c| c.frame(age))
            .is_some()
    }

    pub fn smoke(
        &mut self,
        fs: &AssetFs,
        rules: &IniFile,
        name: &str,
        age: u64,
    ) -> Option<&Sprite> {
        let clip = self.smoke_clip(fs, rules, name)?;
        let frame = clip.frame(age)?;
        let file = clip.file.clone();
        let alpha = clip.alpha;
        self.sprite(fs, &format!("smoke:{name}"), frame, &file, alpha)
    }

    fn sprite(
        &mut self,
        fs: &AssetFs,
        name: &str,
        frame: usize,
        file: &str,
        alpha: u8,
    ) -> Option<&Sprite> {
        let key = (name.to_string(), frame);
        if !self.frames.contains_key(&key) {
            let sprite = (|| {
                let pal = Palette::parse(fs.read("ANIM.PAL")?)?;
                let shp = ShpFile::parse(fs.read(file)?).ok()?;
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
