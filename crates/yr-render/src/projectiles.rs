//! Static retail projectile bodies at authoritative simulation positions.

use crate::sprites::Sprite;
use std::collections::HashMap;
use yr_assets::AssetFs;
use yr_formats::{IniFile, Palette, ShpFile};
use yr_rules::Rules;

fn safe_image(image: &str) -> bool {
    !image.is_empty()
        && image.len() <= 64
        && !image.eq_ignore_ascii_case("none")
        && image
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[derive(Default)]
pub(crate) struct Projectiles {
    bodies: HashMap<u64, Option<Sprite>>,
}

impl Projectiles {
    pub fn get(
        &mut self,
        fs: &AssetFs,
        rules: &Rules,
        art: &IniFile,
        pal: &Palette,
        source: u64,
    ) -> Option<&Sprite> {
        if self.bodies.len() >= 128 && !self.bodies.contains_key(&source) {
            return None;
        }
        self.bodies
            .entry(source)
            .or_insert_with(|| {
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
                // Directional, animated and voxel projectiles need their own
                // authored frame/pose selection; do not guess a first frame.
                if ["Rotates", "Animates", "Voxel"]
                    .iter()
                    .any(|key| section.and_then(|s| s.get_bool(key)) == Some(true))
                {
                    return None;
                }
                let image = section.and_then(|s| s.get("Image")).unwrap_or(image);
                if !safe_image(image) {
                    return None;
                }
                let file = format!("{}.SHP", image.to_ascii_uppercase());
                let shp = ShpFile::parse(fs.read(&file)?).ok()?;
                if shp.width == 0
                    || shp.height == 0
                    || shp.width > 128
                    || shp.height > 128
                    || shp.frames.len() != 1
                {
                    return None;
                }
                let rgba = shp
                    .decode_frame_canvas(0)
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
            })
            .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            let sprite = cache.get(&fs, &rules, &art, &pal, source).unwrap();
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
                rules.weapons.get("M60").unwrap().source_id()
            )
            .is_none());
        assert!(cache.get(&fs, &rules, &art, &pal, u64::MAX).is_none());
        for source in 0..200 {
            cache.get(&fs, &rules, &art, &pal, source);
        }
        assert_eq!(cache.bodies.len(), 128);
        println!("120MM.SHP: 24x24, one frame, exact UNITTEM.PAL body, no shadow-half assumption");
    }
}
