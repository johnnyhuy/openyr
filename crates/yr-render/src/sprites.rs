//! Live-object sprites for the window: retail art for sim units and
//! buildings, decoded once per (type, frame, house colour) into RGBA
//! with straight alpha. Shadows are kept as half-alpha black so the
//! blitter halves whatever terrain lies underneath, matching the static
//! map renderer's shadow pass.

pub use crate::buildings::BuildingAnimation;
use crate::buildings::{BuildingArt, Layer};
use std::collections::HashMap;
use std::fs;
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, MapFile, Palette, ShpFile};
use yr_rules::{Rules, TechnoKind};

/// Alpha value marking a shadow pixel (darken the destination by half).
pub const SHADOW_ALPHA: u8 = 128;

/// Presentation state sampled after a simulation tick. These clocks do
/// not change the simulation or advance when the window redraws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InfantryAnimation {
    pub moving: bool,
    /// Ticks since movement started or stopped.
    pub elapsed: u64,
    /// Ticks since the most recent shot, if one has been observed.
    pub since_shot: Option<u64>,
}

/// Prototype playback cadence at 15 simulation ticks per second. Retail
/// sequence triples describe frame layout, not a verified playback rate.
const INFANTRY_FRAME_TICKS: u64 = 2;
const IDLE_WAIT_TICKS: u64 = 90;

#[derive(Debug, Clone, Copy)]
struct Sequence {
    start: usize,
    count: usize,
    stride: usize,
}

impl Sequence {
    fn parse(value: &str) -> Option<Self> {
        let mut fields = value.split(',').map(str::trim);
        let start = fields.next()?.parse().ok()?;
        let count = fields.next()?.parse().ok()?;
        let stride = fields.next()?.parse().ok()?;
        (count > 0).then_some(Self {
            start,
            count,
            stride,
        })
    }

    fn frame(self, facing: usize, elapsed: u64, body_frames: usize) -> Option<usize> {
        // Check the whole sequence, including every facing, before using
        // any of it. Malformed INI values must never select a shadow.
        let end = self
            .start
            .checked_add(self.stride.checked_mul(7)?)?
            .checked_add(self.count)?;
        if end > body_frames {
            return None;
        }
        let phase = (elapsed / INFANTRY_FRAME_TICKS) % self.count as u64;
        self.start
            .checked_add(self.stride.checked_mul(facing)?)?
            .checked_add(phase as usize)
    }

    fn duration(self) -> u64 {
        (self.count as u64).saturating_mul(INFANTRY_FRAME_TICKS)
    }
}

struct InfantrySequences {
    body_frames: usize,
    ready: Option<Sequence>,
    walk: Option<Sequence>,
    fire: Option<Sequence>,
    idle: [Option<Sequence>; 2],
}

impl InfantrySequences {
    fn frame(&self, octant: u8, animation: InfantryAnimation) -> usize {
        let facing = infantry_stand_frame(octant);
        let ready = self
            .ready
            .and_then(|s| s.frame(facing, 0, self.body_frames))
            .unwrap_or(if facing < self.body_frames { facing } else { 0 });
        let (sequence, elapsed) = if animation.moving {
            (self.walk, animation.elapsed)
        } else if let Some(age) = animation
            .since_shot
            .filter(|age| self.fire.is_some_and(|s| *age < s.duration()))
        {
            (self.fire, age)
        } else {
            // Play the two non-directional idle gestures occasionally,
            // returning to the current facing between gestures.
            let cycle = animation.elapsed / IDLE_WAIT_TICKS;
            let age = animation.elapsed % IDLE_WAIT_TICKS;
            let idle = self.idle[(cycle % 2) as usize];
            if cycle == 0 || !idle.is_some_and(|s| age < s.duration()) {
                return ready;
            }
            (idle, age)
        };
        sequence
            .and_then(|s| s.frame(facing, elapsed, self.body_frames))
            .unwrap_or(ready)
    }
}

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

/// Ground art is drawn before every object's body. Body and active
/// overlays remain together in the building's footprint order.
pub struct BuildingSprites<'a> {
    pub bib: Option<&'a Sprite>,
    pub layers: Vec<&'a Sprite>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct LayerKey {
    file: String,
    frame: usize,
    shadow: bool,
    remap: [u8; 3],
    offset: (i32, i32),
}

/// Lazily decoded sprites for one map's theater.
pub struct SpriteBank {
    vfs: AssetFs,
    rules: Rules,
    rules_ini: IniFile,
    art_ini: IniFile,
    theater: Theater,
    unit_pal: Palette,
    infantry: HashMap<String, InfantrySequences>,
    cache: HashMap<(String, usize, [u8; 3]), Option<Sprite>>,
    buildings: HashMap<String, Option<BuildingArt>>,
    layers: HashMap<LayerKey, Option<Sprite>>,
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
        let mut infantry = HashMap::new();
        for t in rules.infantry.iter() {
            let Some(section) = t.art.as_ref().and_then(|a| art_ini.section(&a.section)) else {
                continue;
            };
            let Some(sequence) = section.get("Sequence").and_then(|s| art_ini.section(s)) else {
                continue;
            };
            let Some(shp) = vfs
                .read(&format!("{}.SHP", t.image))
                .and_then(|b| ShpFile::parse(b).ok())
            else {
                continue;
            };
            let read = |key| sequence.get(key).and_then(Sequence::parse);
            infantry.insert(
                t.id.to_ascii_uppercase(),
                InfantrySequences {
                    body_frames: shp.frames.len() / 2,
                    ready: read("Ready"),
                    walk: read("Walk"),
                    fire: read("FireUp"),
                    idle: [read("Idle1"), read("Idle2")],
                },
            );
        }
        Ok(SpriteBank {
            vfs,
            rules,
            rules_ini,
            art_ini,
            theater,
            unit_pal,
            infantry,
            cache: HashMap::new(),
            buildings: HashMap::new(),
            layers: HashMap::new(),
        })
    }

    /// RGB house colour of a rules house/country section (`Color=`).
    pub fn house_color(&self, house: &str) -> [u8; 3] {
        crate::house_color(&self.rules_ini, house)
    }

    /// Retail building body, bib and active/production overlays. Cache
    /// individual frames so unrelated overlay periods cannot multiply
    /// into an unbounded cache of composite images.
    pub fn get_building(
        &mut self,
        id: &str,
        remap: [u8; 3],
        animation: BuildingAnimation,
    ) -> Option<BuildingSprites<'_>> {
        let id = id.to_ascii_uppercase();
        if !self.buildings.contains_key(&id) {
            let art = self
                .rules
                .techno(&id)
                .filter(|t| t.kind == TechnoKind::Building)
                .and_then(|t| {
                    let art = t.art.as_ref()?;
                    BuildingArt::load(
                        &self.vfs,
                        &self.art_ini,
                        &t.image,
                        &art.section,
                        self.theater,
                        art.foundation.unwrap_or((1, 1)),
                    )
                });
            self.buildings.insert(id.clone(), art);
        }
        let art = self.buildings.get(&id)?.as_ref()?;
        let key = |layer: &Layer, frame| LayerKey {
            file: layer.file.clone(),
            frame,
            shadow: layer.shadow,
            remap,
            offset: layer.offset,
        };
        let state_frame = |layer: &Layer| usize::from(animation.damaged).min(layer.frames - 1);
        let bib = art.bib.as_ref().map(|layer| key(layer, state_frame(layer)));
        let mut keys = vec![key(&art.base, state_frame(&art.base))];
        let phase = |layer: &Layer, ticks| {
            layer.clip.frame(if layer.powered && !animation.powered {
                0
            } else {
                ticks
            })
        };
        for overlay in &art.active {
            if let Some(layer) = overlay.layer(animation.damaged) {
                keys.push(key(layer, phase(layer, animation.elapsed)));
            }
        }
        if let Some(progress) = animation.production {
            if let Some(layer) = art.production.layer(animation.damaged) {
                keys.push(key(layer, phase(layer, progress)));
            }
        }
        if let Some(elapsed) = animation.unloading {
            if let Some(layer) = art.special.layer(animation.damaged) {
                if let Some(frame) = layer.clip.frame_once(elapsed) {
                    keys.push(key(layer, frame));
                }
            }
        }
        for key in bib.iter().chain(&keys) {
            if !self.layers.contains_key(key) {
                let sprite = self.decode_shp(&key.file, key.frame, key.shadow, remap, key.offset);
                self.layers.insert(key.clone(), sprite);
            }
        }
        // A missing base must still trigger the caller's fallback marker.
        self.layers.get(&keys[0])?.as_ref()?;
        Some(BuildingSprites {
            bib: bib.as_ref().and_then(|key| self.layers.get(key)?.as_ref()),
            layers: keys
                .iter()
                .filter_map(|key| self.layers.get(key)?.as_ref())
                .collect(),
        })
    }

    /// Sprite for rules type `id` facing `facing` (sim convention: 0 is
    /// map north / cell -y, clockwise in 256ths), remapped to `remap`.
    /// Facing is quantised to the 8 screen octants. `None` when the art
    /// does not resolve (callers fall back to a marker).
    pub fn get(&mut self, id: &str, facing: u8, remap: [u8; 3]) -> Option<&Sprite> {
        self.get_animated(id, facing, remap, InfantryAnimation::default())
    }

    /// Draw the retail infantry sequence for this tick's observed action.
    /// Vehicles and buildings keep their existing static-art behaviour.
    pub fn get_animated(
        &mut self,
        id: &str,
        facing: u8,
        remap: [u8; 3],
        animation: InfantryAnimation,
    ) -> Option<&Sprite> {
        let octant = screen_octant(facing);
        let id = id.to_ascii_uppercase();
        let frame = if self
            .rules
            .techno(&id)
            .is_some_and(|t| t.art.as_ref().is_some_and(|a| a.voxel))
        {
            usize::from(octant)
        } else if self
            .rules
            .techno(&id)
            .is_some_and(|t| t.kind == TechnoKind::Building)
        {
            0
        } else {
            self.infantry.get(&id).map_or_else(
                || infantry_stand_frame(octant),
                |s| s.frame(octant, animation),
            )
        };
        let key = (id.clone(), frame, remap);
        if !self.cache.contains_key(&key) {
            let sprite = self.decode(&id, frame, remap);
            self.cache.insert(key.clone(), sprite);
        }
        self.cache.get(&key).and_then(Option::as_ref)
    }

    fn decode(&self, id: &str, frame: usize, remap: [u8; 3]) -> Option<Sprite> {
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
                voxel_facing(frame as u8),
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
            (file, frame, (0, 0))
        };
        self.decode_shp(&file, frame, true, remap, offset)
    }

    fn decode_shp(
        &self,
        file: &str,
        frame: usize,
        shadow: bool,
        remap: [u8; 3],
        offset: (i32, i32),
    ) -> Option<Sprite> {
        let shp = ShpFile::parse(self.vfs.read(file)?).ok()?;
        let frame = if frame < shp.frames.len() / 2 {
            frame
        } else {
            0
        };
        let canvas = shp.decode_frame_canvas(frame).ok()?;
        let (w, h) = (usize::from(shp.width), usize::from(shp.height));
        let mut rgba = vec![0u8; w * h * 4];
        if shadow && shp.frames.len() >= 2 {
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
            ax: (w as i32 / 2).checked_sub(offset.0)?,
            ay: (h as i32 / 2).checked_sub(offset.1)?,
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

    fn sequences() -> InfantrySequences {
        InfantrySequences {
            body_frames: 400,
            ready: Sequence::parse("0,1,1"),
            walk: Sequence::parse("8,6,6"),
            fire: Sequence::parse("164,6,6"),
            idle: [Sequence::parse("56,15,0,S"), Sequence::parse("71,15,0,E")],
        }
    }

    #[test]
    fn sequences_select_action_facing_and_phase() {
        let s = sequences();
        for octant in 0..8 {
            let facing = infantry_stand_frame(octant);
            assert_eq!(s.frame(octant, InfantryAnimation::default()), facing);
            for phase in 0..6 {
                let animation = InfantryAnimation {
                    moving: true,
                    elapsed: phase * 2,
                    since_shot: Some(0),
                };
                assert_eq!(s.frame(octant, animation), 8 + facing * 6 + phase as usize);
                let animation = InfantryAnimation {
                    moving: false,
                    elapsed: 0,
                    since_shot: Some(phase * 2),
                };
                assert_eq!(
                    s.frame(octant, animation),
                    164 + facing * 6 + phase as usize
                );
            }
            // A completed fire clip returns to ready even if the target
            // is still in range; walking takes priority over firing.
            assert_eq!(
                s.frame(
                    octant,
                    InfantryAnimation {
                        since_shot: Some(12),
                        ..Default::default()
                    }
                ),
                facing
            );
            assert_eq!(
                s.frame(
                    octant,
                    InfantryAnimation {
                        elapsed: 90,
                        ..Default::default()
                    }
                ),
                71
            );
            assert_eq!(
                s.frame(
                    octant,
                    InfantryAnimation {
                        elapsed: 180,
                        ..Default::default()
                    }
                ),
                56
            );
            assert_eq!(
                s.frame(
                    octant,
                    InfantryAnimation {
                        elapsed: 210,
                        ..Default::default()
                    }
                ),
                facing
            );
        }
    }

    #[test]
    fn hostile_sequences_fall_back_without_addressing_shadows() {
        for value in ["0,0,0", "-1,6,6", "8,no,6", "8,6", "8,6,-1"] {
            assert!(Sequence::parse(value).is_none(), "{value}");
        }
        for value in [
            format!("{},6,6", usize::MAX),
            format!("8,{},6", usize::MAX),
            format!("8,6,{}", usize::MAX),
            "399,2,0".to_string(),
            "360,6,6".to_string(),
        ] {
            let mut s = sequences();
            s.walk = Sequence::parse(&value);
            s.fire = s.walk;
            for octant in 0..8 {
                let ready = infantry_stand_frame(octant);
                assert_eq!(
                    s.frame(
                        octant,
                        InfantryAnimation {
                            moving: true,
                            elapsed: u64::MAX,
                            since_shot: None
                        }
                    ),
                    ready,
                    "{value}"
                );
                assert_eq!(
                    s.frame(
                        octant,
                        InfantryAnimation {
                            since_shot: Some(0),
                            ..Default::default()
                        }
                    ),
                    ready,
                    "{value}"
                );
            }
        }
        let s = sequences();
        let frame = s.frame(
            7,
            InfantryAnimation {
                moving: true,
                elapsed: u64::MAX,
                since_shot: None,
            },
        );
        assert!(
            (14..20).contains(&frame),
            "large clocks wrap within the walk sequence"
        );
    }

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
