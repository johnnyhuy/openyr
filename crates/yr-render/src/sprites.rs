//! Live-object sprites for the window: retail art for sim units and
//! buildings, decoded once per (type, facing, house colour) into RGBA
//! with straight alpha. Shadows are kept as half-alpha black so the
//! blitter halves whatever terrain lies underneath, matching the static
//! map renderer's shadow pass.

use std::collections::HashMap;
use std::fs;
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, MapFile, Palette, ShpFile};
use yr_rules::{Rules, TechnoKind};

/// Alpha value marking a shadow pixel (darken the destination by half).
pub const SHADOW_ALPHA: u8 = 128;

/// One decoded object image in native (scale 1) pixels.
pub struct Sprite {
    pub w: u32,
    pub h: u32,
    /// RGBA8; alpha is 0 (clear), [`SHADOW_ALPHA`] (shadow) or 255.
    pub rgba: Vec<u8>,
    /// Sprite pixel that sits on the object's anchor: the cell centre for
    /// units, the centre of the top-left footprint cell for buildings.
    pub ax: i32,
    pub ay: i32,
}

/// Lazily decoded sprites for one map's theater.
pub struct SpriteBank {
    vfs: AssetFs,
    rules: Rules,
    rules_ini: IniFile,
    theater: Theater,
    unit_pal: Palette,
    cache: HashMap<(String, u8, [u8; 3]), Option<Sprite>>,
}

impl SpriteBank {
    /// Open the install and resolve the map's theater and rules (map INI
    /// overrides merged, as in game).
    pub fn open(gamedir: &str, map_name: &str) -> Result<SpriteBank, String> {
        let mut vfs = AssetFs::open(std::path::Path::new(gamedir)).map_err(|e| e.to_string())?;
        let raw = match vfs.read(map_name) {
            Some(d) => d.to_vec(),
            None => fs::read(map_name).map_err(|e| format!("{map_name}: {e}"))?,
        };
        let map = MapFile::parse(&String::from_utf8_lossy(&raw)).map_err(|e| e.to_string())?;
        let theater = Theater::from_map_name(&map.theater)
            .ok_or_else(|| format!("unknown theater {}", map.theater))?;
        vfs.set_theater(theater);
        let pal = vfs
            .read(theater.palette())
            .and_then(Palette::parse)
            .ok_or("theater palette not found")?;
        let unit_pal = vfs
            .read(theater.unit_palette())
            .and_then(Palette::parse)
            .unwrap_or(pal);
        let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(
            vfs.read("RULESMD.INI").ok_or("rulesmd.ini not found")?,
        ));
        rules_ini.merge_from(&map.ini);
        let art_ini = IniFile::parse(&String::from_utf8_lossy(
            vfs.read("ARTMD.INI").ok_or("artmd.ini not found")?,
        ));
        let rules = Rules::load(&rules_ini, &art_ini);
        Ok(SpriteBank {
            vfs,
            rules,
            rules_ini,
            theater,
            unit_pal,
            cache: HashMap::new(),
        })
    }

    /// RGB house colour of a rules house/country section (`Color=`).
    pub fn house_color(&self, house: &str) -> [u8; 3] {
        crate::house_color(&self.rules_ini, house)
    }

    /// Sprite for rules type `id` facing `facing` (sim convention: 0 is
    /// map north / cell -y, clockwise in 256ths), remapped to `remap`.
    /// Facing is quantised to the 8 screen octants. `None` when the art
    /// does not resolve (callers fall back to a marker).
    pub fn get(&mut self, id: &str, facing: u8, remap: [u8; 3]) -> Option<&Sprite> {
        let octant = screen_octant(facing);
        let key = (id.to_string(), octant, remap);
        if !self.cache.contains_key(&key) {
            let sprite = self.decode(id, octant, remap);
            self.cache.insert(key.clone(), sprite);
        }
        self.cache.get(&key).and_then(Option::as_ref)
    }

    fn decode(&self, id: &str, octant: u8, remap: [u8; 3]) -> Option<Sprite> {
        let t = self.rules.techno(id)?;
        let art = t.art.as_ref();
        let image = t.image.to_ascii_uppercase();
        if art.is_some_and(|a| a.voxel) {
            let body = self.vfs.read(&format!("{image}.VXL"))?;
            let tur = self.vfs.read(&format!("{image}TUR.VXL"));
            let barl = self.vfs.read(&format!("{image}BARL.VXL"));
            let (w, h) = (128usize, 128usize);
            let mut rgba = vec![0u8; w * h * 4];
            crate::draw_voxel(
                &mut rgba,
                w,
                h,
                64,
                64,
                body,
                tur,
                barl,
                voxel_facing(octant),
                &self.unit_pal,
                Some(remap),
            );
            return (rgba.as_chunks::<4>().0.iter().any(|p| p[3] != 0)).then_some(Sprite {
                w: w as u32,
                h: h as u32,
                rgba,
                ax: 64,
                ay: 64,
            });
        }
        let (file, frame, offset) = if t.kind == TechnoKind::Building {
            let file = crate::building_shp_candidates(&image, self.theater)
                .into_iter()
                .find(|f| self.vfs.read(f).is_some())?;
            let (fx, fy) = art.and_then(|a| a.foundation).unwrap_or((1, 1));
            let (fx, fy) = (i32::from(fx), i32::from(fy));
            // Art centre sits on the footprint centre (see render_map).
            let offset = ((fx - fy) * 15, (fx + fy - 2) * 15 / 2);
            (file, 0, offset)
        } else {
            let file = format!("{image}.SHP");
            self.vfs.read(&file)?;
            (file, infantry_stand_frame(octant), (0, 0))
        };
        let shp = ShpFile::parse(self.vfs.read(&file)?).ok()?;
        let frame = if frame < shp.frames.len() { frame } else { 0 };
        let canvas = shp.decode_frame_canvas(frame).ok()?;
        let (w, h) = (usize::from(shp.width), usize::from(shp.height));
        let mut rgba = vec![0u8; w * h * 4];
        if shp.frames.len() >= 2 {
            let shadow_frame = frame + shp.frames.len() / 2;
            if let Some(shadow) = (shadow_frame < shp.frames.len())
                .then(|| shp.decode_frame_canvas(shadow_frame).ok())
                .flatten()
            {
                for (p, &i) in shadow.iter().enumerate() {
                    if i != 0 {
                        rgba[p * 4..p * 4 + 4].copy_from_slice(&[0, 0, 0, SHADOW_ALPHA]);
                    }
                }
            }
        }
        for (p, &i) in canvas.iter().enumerate() {
            if i != 0 {
                rgba[p * 4..p * 4 + 4].copy_from_slice(&crate::remap_rgba(
                    &self.unit_pal,
                    i,
                    Some(remap),
                ));
            }
        }
        Some(Sprite {
            w: w as u32,
            h: h as u32,
            rgba,
            ax: w as i32 / 2 - offset.0,
            ay: h as i32 / 2 - offset.1,
        })
    }
}

/// Screen octant (0 = up, clockwise) of a sim facing. Sim facing 0 is
/// cell -y, which projects to screen up-right; the cell direction is
/// projected through the iso transform before quantising.
pub fn screen_octant(facing: u8) -> u8 {
    let ang = (f32::from(facing) - 64.0) * std::f32::consts::TAU / 256.0;
    let (s, c) = ang.sin_cos();
    let (sx, sy) = ((c - s) * 30.0, (c + s) * 15.0);
    let a = sx.atan2(-sy).rem_euclid(std::f32::consts::TAU);
    ((a / (std::f32::consts::TAU / 8.0)).round() as u8) % 8
}

/// Standing frame for a screen octant. Infantry SHPs store their "Ready"
/// sequence as 8 facings, counter-clockwise from screen up
/// (observed on E1/GGI; see docs/formats/shp.md).
fn infantry_stand_frame(octant: u8) -> usize {
    usize::from((8 - octant) % 8)
}

/// `draw_voxel` facing for a screen octant: back-project the octant's
/// screen direction to a cell-space angle and offset it to the
/// renderer's model rotation (model +x faces `(facing + 32)` in 256ths).
fn voxel_facing(octant: u8) -> u16 {
    let a = f32::from(octant) * std::f32::consts::TAU / 8.0;
    // Screen direction (sx, sy) = (sin a, -cos a); invert the iso map
    // sx = (c - s) * 30, sy = (c + s) * 15.
    let (sx, sy) = (a.sin(), -a.cos());
    let (c, s) = (sx / 60.0 + sy / 30.0, sy / 30.0 - sx / 60.0);
    let cell = s.atan2(c).rem_euclid(std::f32::consts::TAU);
    let units = (cell * 256.0 / std::f32::consts::TAU).round() as i32;
    ((units - 32).rem_euclid(256)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octants_follow_the_iso_projection() {
        // Arrange / Act / Assert: cell -y (sim 0) is screen up-right,
        // cell +x (64) down-right, +y (128) down-left, -x (192) up-left.
        assert_eq!(screen_octant(0), 1);
        assert_eq!(screen_octant(64), 3);
        assert_eq!(screen_octant(128), 5);
        assert_eq!(screen_octant(192), 7);
        // Diagonal (+x, -y) is screen right.
        assert_eq!(screen_octant(32), 2);
    }

    #[test]
    fn voxel_facing_points_model_x_along_the_octant() {
        // Model +x faces (facing + 32); screen right (octant 2) is the
        // cell direction (+x, -y), i.e. sim facing 32 -> 32 - 32 + ... .
        for octant in 0..8u8 {
            let f = voxel_facing(octant);
            let sim = (f + 32) % 256 + 64;
            assert_eq!(screen_octant((sim % 256) as u8), octant, "octant {octant}");
        }
    }
}
