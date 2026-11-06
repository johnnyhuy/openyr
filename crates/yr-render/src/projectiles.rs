//! Retail projectile bodies at authoritative simulation positions.

use crate::sprites::Sprite;
use std::collections::{HashMap, VecDeque};
use yr_assets::AssetFs;
use yr_formats::{IniFile, Palette, ShpFile};
use yr_rules::Rules;

const MAX_SOURCES: usize = 128;
const MAX_FRAMES: usize = 64;

/// Explicit particle art with a bounded native emission interval in sim ticks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectileTrailer {
    pub name: String,
    pub interval: u16,
}

impl ProjectileTrailer {
    pub(crate) fn load(section: &yr_formats::IniSection) -> Option<Self> {
        let name = section.get("Trailer")?;
        if !safe_image(name) {
            return None;
        }
        // The retail art comment documents a default of three frames.
        // Mapping those frames to sim ticks remains a provisional policy.
        let interval = match section.get("SpawnDelay") {
            Some(value) => value.parse::<u16>().ok()?,
            None => 3,
        };
        (interval > 0).then(|| Self {
            name: name.to_string(),
            interval,
        })
    }
}

/// Explicit retail line-trail settings. Fade cadence is a presentation policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineTrail {
    pub color: [u8; 3],
    pub decrement: u8,
}

impl LineTrail {
    pub(crate) fn load(section: &yr_formats::IniSection) -> Option<Self> {
        if section.get_bool("UseLineTrail") != Some(true) {
            return None;
        }
        let mut components = section.get("LineTrailColor")?.split(',');
        let mut color = [0; 3];
        for value in &mut color {
            *value = components.next()?.trim().parse::<u8>().ok()?;
        }
        if components.next().is_some() {
            return None;
        }
        let decrement = section.get("LineTrailColorDecrement")?.parse::<u8>().ok()?;
        (decrement > 0 && color != [0; 3]).then_some(Self { color, decrement })
    }

    pub fn color_at(self, age: u64) -> Option<[u8; 3]> {
        let fade = age.saturating_mul(u64::from(self.decrement)).min(255) as u8;
        let color = self.color.map(|c| c.saturating_sub(fade));
        (color != [0; 3]).then_some(color)
    }
}

fn safe_image(image: &str) -> bool {
    !image.is_empty()
        && image.len() <= 64
        && !image.eq_ignore_ascii_case("none")
        && image
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

struct Body {
    file: String,
    rotating: bool,
    // Zero for static and rotating bodies. Otherwise the count of body
    // frames in the first half of the sheet; the second half is the
    // empty shadow slot observed in retail animated projectile sheets.
    animated: usize,
    anim_palette: bool,
}

impl Body {
    fn load(fs: &AssetFs, rules: &Rules, art: &IniFile, source: u64) -> Option<Self> {
        let mut matches = rules.weapons.iter().filter(|w| w.source_id() == source);
        let weapon = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let projectile = rules.projectiles.get(weapon.projectile.as_deref()?)?;
        if !projectile.defined || projectile.inviso == Some(true) {
            return None;
        }
        let image = projectile.image.as_deref().unwrap_or(&projectile.id);
        if !safe_image(image) {
            return None;
        }
        let section = art.section(image);
        // Animated and voxel bodies require separate frame/pose policies.
        if ["Animates", "Voxel"]
            .iter()
            .any(|key| section.and_then(|s| s.get_bool(key)) == Some(true))
        {
            return None;
        }
        let rotating = section.and_then(|s| s.get_bool("Rotates")) == Some(true);
        let image = section.and_then(|s| s.get("Image")).unwrap_or(image);
        if !safe_image(image) {
            return None;
        }
        let file = format!("{}.SHP", image.to_ascii_uppercase());
        let shp = ShpFile::parse(fs.read(&file)?).ok()?;
        if shp.width == 0 || shp.height == 0 || shp.width > 128 || shp.height > 128 {
            return None;
        }
        let anim_palette = section.and_then(|s| s.get_bool("AnimPalette")) == Some(true);
        let (valid, animated) = if rotating {
            (
                shp.frames.len() == 32
                    || shp.frames.len() == 64 && shp.frames[32..].iter().all(|f| f.is_empty()),
                0,
            )
        } else if shp.frames.len() == 1 {
            (true, 0)
        } else {
            // Animated bodies cycle the first half of the sheet. The
            // second half is an empty shadow slot, as on infantry sheets.
            let half = shp.frames.len() / 2;
            (
                shp.frames.len() % 2 == 0
                    && half >= 2
                    && shp.frames[..half].iter().all(|f| !f.is_empty())
                    && shp.frames[half..].iter().all(|f| f.is_empty()),
                half,
            )
        };
        valid.then_some(Self {
            file,
            rotating,
            animated,
            anim_palette,
        })
    }

    fn frame(&self, motion: Option<(i32, i32)>, tick: u64) -> Option<usize> {
        if self.animated > 0 {
            // Absolute tick phase reconstructs the same frame after a
            // load, with no saved presentation state. One frame per
            // simulation tick is a provisional cadence.
            return Some((tick % self.animated as u64) as usize);
        }
        if !self.rotating {
            return Some(0);
        }
        let (dx, dy) = motion?;
        if dx == 0 && dy == 0 {
            return None;
        }
        // Project the actual axis-clamped sim step, not the shooter's facing
        // or an unclamped target vector. Sheets run counterclockwise from up.
        let sx = f64::from(dx) - f64::from(dy);
        let sy = (f64::from(dx) + f64::from(dy)) / 2.0;
        let phase = (-sx).atan2(-sy).rem_euclid(std::f64::consts::TAU);
        Some((phase * 32.0 / std::f64::consts::TAU).round() as usize % 32)
    }
}

#[derive(Default)]
pub(crate) struct Projectiles {
    bodies: HashMap<u64, Option<Body>>,
    frames: HashMap<(u64, usize), Option<Sprite>>,
    order: VecDeque<(u64, usize)>,
    anim_pal: Option<Palette>,
    anim_pal_tried: bool,
}

impl Projectiles {
    #[cfg(test)]
    pub fn get(
        &mut self,
        fs: &AssetFs,
        rules: &Rules,
        art: &IniFile,
        pal: &Palette,
        source: u64,
        motion: Option<(i32, i32)>,
    ) -> Option<&Sprite> {
        self.get_at(fs, rules, art, pal, source, motion, 0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_at(
        &mut self,
        fs: &AssetFs,
        rules: &Rules,
        art: &IniFile,
        pal: &Palette,
        source: u64,
        motion: Option<(i32, i32)>,
        tick: u64,
    ) -> Option<&Sprite> {
        if self.bodies.len() >= MAX_SOURCES && !self.bodies.contains_key(&source) {
            return None;
        }
        let body = self
            .bodies
            .entry(source)
            .or_insert_with(|| Body::load(fs, rules, art, source))
            .as_ref()?;
        let frame = body.frame(motion, tick)?;
        let key = (source, frame);
        if body.anim_palette && !self.anim_pal_tried {
            self.anim_pal_tried = true;
            self.anim_pal = fs.read("ANIM.PAL").and_then(Palette::parse);
        }
        let pal = if body.anim_palette {
            self.anim_pal.as_ref()?
        } else {
            pal
        };
        if !self.frames.contains_key(&key) {
            let sprite = (|| {
                let shp = ShpFile::parse(fs.read(&body.file)?).ok()?;
                let rgba = shp
                    .decode_frame_canvas(frame)
                    .ok()?
                    .into_iter()
                    .flat_map(|index| if index == 0 { [0; 4] } else { pal.rgba(index) })
                    .collect();
                Some(Sprite {
                    w: u32::from(shp.width),
                    h: u32::from(shp.height),
                    ax: i32::from(shp.width) / 2,
                    ay: i32::from(shp.height) / 2,
                    rgba,
                })
            })();
            if self.frames.len() >= MAX_FRAMES {
                if let Some(old) = self.order.pop_front() {
                    self.frames.remove(&old);
                }
            }
            self.order.push_back(key);
            self.frames.insert(key, sprite);
        }
        self.frames.get(&key)?.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn motion(frame: usize) -> (i32, i32) {
        let a = frame as f64 * std::f64::consts::TAU / 32.0;
        let (sx, sy) = (-a.sin(), -a.cos());
        (
            ((sy + sx / 2.0) * 100_000.0).round() as i32,
            ((sy - sx / 2.0) * 100_000.0).round() as i32,
        )
    }

    #[test]
    fn directional_frames_follow_projected_motion_and_wrap_at_north() {
        let body = Body {
            file: String::new(),
            rotating: true,
            animated: 0,
            anim_palette: false,
        };
        for (motion, frame) in [
            ((-1, -1), 0),
            ((-1, 1), 8),
            ((1, 1), 16),
            ((1, -1), 24),
            ((i32::MIN, i32::MIN), 0),
            ((i32::MAX, i32::MAX), 16),
        ] {
            assert_eq!(body.frame(Some(motion), 0), Some(frame));
        }
        for frame in 0..32 {
            assert_eq!(body.frame(Some(motion(frame)), 0), Some(frame));
        }
        assert_eq!(body.frame(None, 0), None);
        assert_eq!(body.frame(Some((0, 0)), 0), None);
        assert_eq!(body.frame(Some((-100_001, -100_000)), 0), Some(0));
        assert_eq!(body.frame(Some((-100_000, -100_001)), 0), Some(0));
        let animated = Body {
            file: String::new(),
            rotating: false,
            animated: 7,
            anim_palette: false,
        };
        for tick in 0..14u64 {
            assert_eq!(animated.frame(None, tick), Some((tick % 7) as usize));
        }
        assert_eq!(
            animated.frame(None, u64::MAX),
            Some((u64::MAX % 7) as usize)
        );
    }

    #[test]
    fn hostile_and_ambiguous_directional_layouts_are_optional() {
        let dir =
            std::env::temp_dir().join(format!("openyr-projectile-bounds-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rules = Rules::load(
            &IniFile::parse(
                "[VehicleTypes]\n0=T\n[T]\nPrimary=G\n[G]\nProjectile=P\n[P]\nImage=SAFE\n",
            ),
            &IniFile::parse(""),
        );
        let source = rules.weapons.get("G").unwrap().source_id();
        let fixture = |w: u16, h: u16, count: u16, tail: bool| {
            let mut data = Vec::new();
            for n in [0, w, h, count] {
                data.extend(n.to_le_bytes());
            }
            for i in 0..count.min(65) {
                for n in [0u16, 0, 1, 1] {
                    data.extend(n.to_le_bytes());
                }
                for n in [
                    1u32,
                    0,
                    0,
                    if i < 32 || tail {
                        8 + u32::from(count) * 24
                    } else {
                        0
                    },
                ] {
                    data.extend(n.to_le_bytes());
                }
            }
            data.push(1);
            data
        };
        for (w, h, count, tail, ini, valid) in [
            (2, 2, 32, false, "Rotates=yes", true),
            (2, 2, 64, false, "Rotates=yes", true),
            (2, 2, 64, true, "Rotates=yes", false),
            (2, 2, 31, false, "Rotates=yes", false),
            (2, 2, 33, false, "Rotates=yes", false),
            (129, 2, 32, false, "Rotates=yes", false),
            (2, 129, 32, false, "Rotates=yes", false),
            (u16::MAX, u16::MAX, 32, false, "Rotates=yes", false),
            (2, 2, u16::MAX, false, "Rotates=yes", false),
            (2, 2, 32, false, "Rotates=yes\nAnimates=yes", false),
            (2, 2, 32, false, "Rotates=yes\nVoxel=yes", false),
            (2, 2, 32, false, "Rotates=yes\nImage=../unsafe", false),
            (2, 2, 32, false, "", false),
        ] {
            std::fs::write(dir.join("SAFE.SHP"), fixture(w, h, count, tail)).unwrap();
            let fs = AssetFs::open(&dir).unwrap();
            assert_eq!(
                Body::load(
                    &fs,
                    &rules,
                    &IniFile::parse(&format!("[SAFE]\n{ini}")),
                    source
                )
                .is_some(),
                valid,
                "{w}x{h} count {count}, {ini}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn retail_directional_bodies_match_all_source_frames_with_bounded_caches() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let ini = |name| IniFile::parse(&String::from_utf8_lossy(fs.read(name).unwrap()));
        let art = ini("ARTMD.INI");
        let rules = Rules::load(&ini("RULESMD.INI"), &art);
        let pal = Palette::parse(fs.read("UNITTEM.PAL").unwrap()).unwrap();
        let mut cache = Projectiles::default();
        for (weapon, file, size, count) in [
            ("HoverMissile", "DRAGON.SHP", (24, 16), 32),
            ("Medusa", "MEDUSA.SHP", (19, 19), 64),
            ("MissileLauncher", "DRAGON.SHP", (24, 16), 32),
        ] {
            let source = rules.weapons.get(weapon).unwrap().source_id();
            let shp = ShpFile::parse(fs.read(file).unwrap()).unwrap();
            assert_eq!((shp.width, shp.height), size);
            assert_eq!(shp.frames.len(), count);
            assert!(cache.get(&fs, &rules, &art, &pal, source, None).is_none());
            assert!(cache
                .get(&fs, &rules, &art, &pal, source, Some((0, 0)))
                .is_none());
            for frame in 0..32 {
                let expected: Vec<_> = shp
                    .decode_frame_canvas(frame)
                    .unwrap()
                    .into_iter()
                    .flat_map(|i| if i == 0 { [0; 4] } else { pal.rgba(i) })
                    .collect();
                let sprite = cache
                    .get(&fs, &rules, &art, &pal, source, Some(motion(frame)))
                    .unwrap();
                assert_eq!(sprite.rgba, expected, "{weapon} frame {frame}");
                assert_eq!(
                    (sprite.ax, sprite.ay),
                    (i32::from(size.0) / 2, i32::from(size.1) / 2)
                );
            }
        }
        assert_eq!(cache.frames.len(), MAX_FRAMES);
        assert_eq!(cache.order.len(), MAX_FRAMES);
        for source in 0..200 {
            cache.get(&fs, &rules, &art, &pal, source, Some((1, 1)));
        }
        assert_eq!(cache.bodies.len(), MAX_SOURCES);
        assert_eq!(cache.frames.len(), MAX_FRAMES);
        // Evicted frames reconstruct without relying on previous draws.
        let source = rules.weapons.get("HoverMissile").unwrap().source_id();
        assert!(cache
            .get(&fs, &rules, &art, &pal, source, Some(motion(0)))
            .is_some());
        println!("DRAGON: 32x24x16, MEDUSA: 32 body frames on 19x19 plus empty half; all source pixels match");
    }

    #[test]
    fn retail_cannon_body_matches_every_source_pixel_and_does_not_guess_invisible_art() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let ini = |name| IniFile::parse(&String::from_utf8_lossy(fs.read(name).unwrap()));
        let art = ini("ARTMD.INI");
        let rules = Rules::load(&ini("RULESMD.INI"), &art);
        let pal = Palette::parse(fs.read("UNITTEM.PAL").unwrap()).unwrap();
        let mut cache = Projectiles::default();
        let shp = ShpFile::parse(fs.read("120MM.SHP").unwrap()).unwrap();
        assert_eq!((shp.width, shp.height, shp.frames.len()), (24, 24, 1));
        let expected: Vec<_> = shp
            .decode_frame_canvas(0)
            .unwrap()
            .into_iter()
            .flat_map(|index| if index == 0 { [0; 4] } else { pal.rgba(index) })
            .collect();
        assert!(expected.as_chunks::<4>().0.iter().any(|p| p[3] != 0));
        for id in ["105mm", "120mm"] {
            let source = rules.weapons.get(id).unwrap().source_id();
            let sprite = cache.get(&fs, &rules, &art, &pal, source, None).unwrap();
            assert_eq!((sprite.w, sprite.h, sprite.ax, sprite.ay), (24, 24, 12, 12));
            assert_eq!(sprite.rgba, expected);
        }
        let map = yr_formats::MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap()))
            .unwrap();
        let theater = yr_assets::Theater::from_map_name(&map.theater).unwrap();
        let actual_pal = Palette::parse(fs.read(theater.unit_palette()).unwrap()).unwrap();
        let actual: Vec<_> = shp
            .decode_frame_canvas(0)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    actual_pal.rgba(index)
                }
            })
            .collect();
        let mut bank = crate::sprites::SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        assert_eq!(
            bank.get_projectile(rules.weapons.get("105mm").unwrap().source_id())
                .unwrap()
                .rgba,
            actual
        );
        assert!(cache
            .get(
                &fs,
                &rules,
                &art,
                &pal,
                rules.weapons.get("M60").unwrap().source_id(),
                None
            )
            .is_none());
        assert!(cache.get(&fs, &rules, &art, &pal, u64::MAX, None).is_none());
        for source in 0..200 {
            cache.get(&fs, &rules, &art, &pal, source, None);
        }
        assert_eq!(cache.bodies.len(), 128);
        println!("120MM.SHP: 24x24, one frame, exact UNITTEM.PAL body, no shadow-half assumption");
    }

    #[test]
    fn retail_animated_bodies_match_every_source_frame_and_cycle_by_tick() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let ini = |name| IniFile::parse(&String::from_utf8_lossy(fs.read(name).unwrap()));
        let art = ini("ARTMD.INI");
        // These weapons are not referenced by installed technos. Register
        // their unchanged retail chains on existing technos for the test.
        let mut rules_ini = ini("RULESMD.INI");
        rules_ini.merge_from(&IniFile::parse(
            "[FV]\nPrimary=CyCannon\n[MTNK]\nPrimary=Grenade\n[HTNK]\nPrimary=RPGTower\n[GGI]\nPrimary=EMPulseWeapon\n",
        ));
        let rules = Rules::load(&rules_ini, &art);
        let unit_pal = Palette::parse(fs.read("UNITTEM.PAL").unwrap()).unwrap();
        let anim_pal = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
        let mut cache = Projectiles::default();
        for (weapon, file, size, animated, use_anim_pal) in [
            ("CyCannon", "TORPEDO.SHP", (9, 9), 3usize, true),
            ("Grenade", "DISCUS.SHP", (8, 5), 7, false),
            ("RPGTower", "CANISTER.SHP", (9, 8), 15, false),
            ("EMPulseWeapon", "PULSBALL.SHP", (16, 18), 23, true),
        ] {
            let source = rules.weapons.get(weapon).unwrap().source_id();
            let shp = ShpFile::parse(fs.read(file).unwrap()).unwrap();
            assert_eq!((shp.width, shp.height), size, "{weapon}");
            assert_eq!(shp.frames.len(), animated * 2, "{weapon}");
            assert!(
                shp.frames[animated..].iter().all(|f| f.is_empty()),
                "{weapon}"
            );
            let pal = if use_anim_pal { &anim_pal } else { &unit_pal };
            for tick in 0..(animated as u64 * 2) {
                let frame = (tick % animated as u64) as usize;
                let expected: Vec<_> = shp
                    .decode_frame_canvas(frame)
                    .unwrap()
                    .into_iter()
                    .flat_map(|i| if i == 0 { [0; 4] } else { pal.rgba(i) })
                    .collect();
                let sprite = cache
                    .get_at(&fs, &rules, &art, &unit_pal, source, None, tick)
                    .unwrap_or_else(|| panic!("{weapon} tick {tick}"));
                assert_eq!(sprite.rgba, expected, "{weapon} frame {frame}");
                assert_eq!(
                    (sprite.ax, sprite.ay),
                    (i32::from(size.0) / 2, i32::from(size.1) / 2),
                    "{weapon}"
                );
            }
            println!("{weapon}: {file} {size:?} {animated} animated frames, anim_palette={use_anim_pal}, all source pixels match");
        }
        assert!(cache.frames.len() <= MAX_FRAMES);
    }

    #[test]
    fn animated_layouts_reject_mixed_or_nonempty_shadow_halves() {
        let dir =
            std::env::temp_dir().join(format!("openyr-animated-bounds-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rules = Rules::load(
            &IniFile::parse(
                "[VehicleTypes]\n0=T\n[T]\nPrimary=G\n[G]\nProjectile=P\n[P]\nImage=SAFE\n",
            ),
            &IniFile::parse(""),
        );
        let source = rules.weapons.get("G").unwrap().source_id();
        // Frame headers mark a frame empty through a zero data offset.
        let fixture = |count: u16, empty: &[usize]| {
            let mut data = Vec::new();
            for n in [0u16, 2, 2, count] {
                data.extend(n.to_le_bytes());
            }
            for i in 0..count {
                for n in [0u16, 0, 1, 1] {
                    data.extend(n.to_le_bytes());
                }
                for n in [1u32, 0, 0] {
                    data.extend(n.to_le_bytes());
                }
                data.extend(
                    if empty.contains(&(i as usize)) {
                        0u32
                    } else {
                        8 + u32::from(count) * 24
                    }
                    .to_le_bytes(),
                );
            }
            data.push(1);
            data
        };
        for (count, empty, valid) in [
            (6u16, vec![3usize, 4, 5], true),
            (4, vec![2, 3], true),
            (6, vec![3, 4], false),
            (6, vec![0, 3, 4, 5], false),
            (5, vec![3, 4], false),
            (2, vec![1], false),
            (6, vec![], false),
        ] {
            std::fs::write(dir.join("SAFE.SHP"), fixture(count, &empty)).unwrap();
            let fs = AssetFs::open(&dir).unwrap();
            assert_eq!(
                Body::load(&fs, &rules, &IniFile::parse("[SAFE]\n"), source).is_some(),
                valid,
                "count {count} empty {empty:?}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod trail_tests {
    use super::*;

    #[test]
    fn explicit_trail_settings_are_bounded_and_fade_without_overflow() {
        let parse = |text: &str| {
            LineTrail::load(
                IniFile::parse(&format!("[X]\n{text}"))
                    .section("X")
                    .unwrap(),
            )
        };
        let style =
            parse("UseLineTrail=yes\nLineTrailColor=216,216,255\nLineTrailColorDecrement=16")
                .unwrap();
        assert_eq!(style.color_at(0), Some([216, 216, 255]));
        assert_eq!(style.color_at(13), Some([8, 8, 47]));
        assert_eq!(style.color_at(15), Some([0, 0, 15]));
        assert_eq!(style.color_at(16), None);
        assert_eq!(style.color_at(u64::MAX), None);
        for text in [
            "UseLineTrail=no\nLineTrailColor=1,2,3\nLineTrailColorDecrement=1",
            "UseLineTrail=yes\nLineTrailColor=0,0,0\nLineTrailColorDecrement=1",
            "UseLineTrail=yes\nLineTrailColor=1,2\nLineTrailColorDecrement=1",
            "UseLineTrail=yes\nLineTrailColor=1,2,3,4\nLineTrailColorDecrement=1",
            "UseLineTrail=yes\nLineTrailColor=-1,2,3\nLineTrailColorDecrement=1",
            "UseLineTrail=yes\nLineTrailColor=256,2,3\nLineTrailColorDecrement=1",
            "UseLineTrail=yes\nLineTrailColor=1,2,3\nLineTrailColorDecrement=0",
            "UseLineTrail=yes\nLineTrailColor=1,2,3\nLineTrailColorDecrement=256",
            "UseLineTrail=yes\nLineTrailColor=1,2,3\nLineTrailColorDecrement=-1",
            "UseLineTrail=yes\nLineTrailColor=1,2,3\nLineTrailColorDecrement=18446744073709551615",
            "UseLineTrail=yes\nLineTrailColor=1,2,3",
            "UseLineTrail=yes\nLineTrailColorDecrement=16",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
    }

    #[test]
    fn retail_missile_trail_colors_come_from_the_fired_weapon() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let ini = |name| IniFile::parse(&String::from_utf8_lossy(fs.read(name).unwrap()));
        let art = ini("ARTMD.INI");
        let rules = Rules::load(&ini("RULESMD.INI"), &art);
        let bank = crate::sprites::SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        for (id, color, decrement) in [
            ("HoverMissile", [216, 216, 255], 16),
            ("Medusa", [208; 3], 12),
        ] {
            assert_eq!(
                bank.projectile_line_trail(rules.weapons.get(id).unwrap().source_id()),
                Some(LineTrail { color, decrement })
            );
        }
        for id in ["105mm", "M60", "ElectricBolt"] {
            assert_eq!(
                bank.projectile_line_trail(rules.weapons.get(id).unwrap().source_id()),
                None
            );
        }
        assert_eq!(bank.projectile_line_trail(u64::MAX), None);
    }
}

#[cfg(test)]
mod trailer_tests {
    use super::*;

    #[test]
    fn projectile_trailer_settings_reject_hostile_names_and_intervals() {
        let parse = |text: &str| {
            ProjectileTrailer::load(
                IniFile::parse(&format!("[P]\n{text}"))
                    .section("P")
                    .unwrap(),
            )
        };
        assert_eq!(parse("Trailer=SMOKEY2").unwrap().interval, 3);
        assert_eq!(
            parse("Trailer=DURASMOKE\nSpawnDelay=2").unwrap().interval,
            2
        );
        for text in [
            "",
            "Trailer=",
            "Trailer=none",
            "Trailer=../BAD",
            "Trailer=A,B",
            "Trailer=A\nSpawnDelay=0",
            "Trailer=A\nSpawnDelay=-1",
            "Trailer=A\nSpawnDelay=nope",
            "Trailer=A\nSpawnDelay=65536",
            "Trailer=A\nSpawnDelay=18446744073709551615",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
        assert!(parse(&format!("Trailer={}", "A".repeat(65))).is_none());
    }

    #[test]
    fn retail_projectile_trailers_resolve_explicit_art_and_match_every_source_frame() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let ini = |name| IniFile::parse(&String::from_utf8_lossy(fs.read(name).unwrap()));
        let art = ini("ARTMD.INI");
        let mut rules_ini = ini("RULESMD.INI");
        let overrides = "\n[FV]\nPrimary=ChemLauncher\n[SHAD]\nPrimary=DredCollision\n";
        rules_ini.merge_from(&IniFile::parse(overrides));
        let rules = Rules::load(&rules_ini, &art);
        let map_path =
            std::env::temp_dir().join(format!("openyr-trailer-source-{}.mmx", std::process::id()));
        let mut map_bytes = fs.read("EB1.MMX").unwrap().to_vec();
        map_bytes.extend(overrides.as_bytes());
        std::fs::write(&map_path, map_bytes).unwrap();
        let pal = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
        let mut bank =
            crate::sprites::SpriteBank::open(dir.to_str().unwrap(), map_path.to_str().unwrap())
                .unwrap();
        std::fs::remove_file(&map_path).unwrap();
        for (weapon, name, interval, rate) in [
            ("DredCollision", "DURASMOKE", 2, 450),
            ("ChemLauncher", "SMOKEY2", 3, 200),
            ("SubTorpedo", "BBBLELRG", 3, 450),
        ] {
            assert_eq!(
                bank.projectile_trailer(rules.weapons.get(weapon).unwrap().source_id()),
                Some(ProjectileTrailer {
                    name: name.into(),
                    interval
                })
            );
            let shp = ShpFile::parse(fs.read(&format!("{name}.SHP")).unwrap()).unwrap();
            let mut seen = std::collections::BTreeSet::new();
            for age in 0..300u64 {
                let frame = age * rate / 225;
                let sprite = bank.get_effect(name, age);
                if frame >= shp.frames.len() as u64 {
                    assert!(sprite.is_none());
                    continue;
                }
                let sprite = sprite.unwrap();
                seen.insert(frame);
                let expected: Vec<_> = shp
                    .decode_frame_canvas(frame as usize)
                    .unwrap()
                    .into_iter()
                    .flat_map(|index| {
                        if index == 0 {
                            [0; 4]
                        } else {
                            let [r, g, b, _] = pal.rgba(index);
                            [r, g, b, if name == "BBBLELRG" { 255 } else { 127 }]
                        }
                    })
                    .collect();
                assert_eq!(sprite.rgba, expected, "{name} age {age}, frame {frame}");
                assert_eq!(
                    (sprite.ax, sprite.ay),
                    (i32::from(shp.width) / 2, i32::from(shp.height) / 2)
                );
            }
            // DURASMOKE advances two source frames per native tick. Check the
            // skipped canvases too using an isolated rate override in Effects.
            if rate == 200 {
                assert_eq!(seen.len(), shp.frames.len());
            }
            assert!(!bank.effect_alive(name, u64::MAX));
            println!(
                "{weapon}: {name}, {}x{}, {} source frames, interval {interval}, rate {rate}",
                shp.width,
                shp.height,
                shp.frames.len()
            );
        }
        for weapon in ["HoverMissile", "Medusa", "105mm", "M60", "ElectricBolt"] {
            assert!(bank
                .projectile_trailer(rules.weapons.get(weapon).unwrap().source_id())
                .is_none());
        }
        assert!(bank.projectile_trailer(u64::MAX).is_none());
        assert_eq!(
            ProjectileTrailer::load(art.section("NKMSLDN").unwrap()),
            None
        );
    }
}
