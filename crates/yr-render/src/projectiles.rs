//! Retail projectile bodies at authoritative simulation positions.

use crate::sprites::Sprite;
use std::collections::{HashMap, VecDeque};
use yr_assets::AssetFs;
use yr_formats::{IniFile, Palette, ShpFile};
use yr_rules::Rules;

const MAX_SOURCES: usize = 128;
const MAX_FRAMES: usize = 64;

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
        let valid = if rotating {
            shp.frames.len() == 32
                || shp.frames.len() == 64 && shp.frames[32..].iter().all(|f| f.is_empty())
        } else {
            shp.frames.len() == 1
        };
        valid.then_some(Self { file, rotating })
    }

    fn frame(&self, motion: Option<(i32, i32)>) -> Option<usize> {
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
}

impl Projectiles {
    pub fn get(
        &mut self,
        fs: &AssetFs,
        rules: &Rules,
        art: &IniFile,
        pal: &Palette,
        source: u64,
        motion: Option<(i32, i32)>,
    ) -> Option<&Sprite> {
        if self.bodies.len() >= MAX_SOURCES && !self.bodies.contains_key(&source) {
            return None;
        }
        let body = self
            .bodies
            .entry(source)
            .or_insert_with(|| Body::load(fs, rules, art, source))
            .as_ref()?;
        let frame = body.frame(motion)?;
        let key = (source, frame);
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
        };
        for (motion, frame) in [
            ((-1, -1), 0),
            ((-1, 1), 8),
            ((1, 1), 16),
            ((1, -1), 24),
            ((i32::MIN, i32::MIN), 0),
            ((i32::MAX, i32::MAX), 16),
        ] {
            assert_eq!(body.frame(Some(motion)), Some(frame));
        }
        for frame in 0..32 {
            assert_eq!(body.frame(Some(motion(frame))), Some(frame));
        }
        assert_eq!(body.frame(None), None);
        assert_eq!(body.frame(Some((0, 0))), None);
        assert_eq!(body.frame(Some((-100_001, -100_000))), Some(0));
        assert_eq!(body.frame(Some((-100_000, -100_001))), Some(0));
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
