//! Live-object sprites for the window: retail art for sim units and
//! buildings, decoded once per (type, frame, house colour) into RGBA
//! with straight alpha. Shadows are kept as half-alpha black so the
//! blitter halves whatever terrain lies underneath, matching the static
//! map renderer's shadow pass.

pub use crate::buildings::BuildingAnimation;
use crate::buildings::{BuildingArt, Layer};
use std::collections::{HashMap, HashSet, VecDeque};
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

    fn once(self, facing: usize, elapsed: u64, body_frames: usize) -> Option<usize> {
        (elapsed < self.duration())
            .then(|| self.frame(facing, elapsed, body_frames))
            .flatten()
    }
}

pub(crate) struct InfantrySequences {
    body_frames: usize,
    ready: Option<Sequence>,
    walk: Option<Sequence>,
    fire: Option<Sequence>,
    idle: [Option<Sequence>; 2],
    death: [Option<Sequence>; 2],
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

pub(crate) fn load_infantry_sequences(
    vfs: &AssetFs,
    rules: &Rules,
    art_ini: &IniFile,
) -> HashMap<String, InfantrySequences> {
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
        if shp.width == 0
            || shp.height == 0
            || shp.width > 256
            || shp.height > 256
            || shp.frames.len() > 4096
            || shp.frames.len() % 2 != 0
        {
            continue;
        }
        let read = |key| sequence.get(key).and_then(Sequence::parse);
        infantry.insert(
            t.id.to_ascii_uppercase(),
            InfantrySequences {
                body_frames: shp.frames.len() / 2,
                ready: read("Ready"),
                walk: read("Walk"),
                fire: read("FireUp"),
                idle: [read("Idle1"), read("Idle2")],
                death: [read("Die1"), read("Die2")],
            },
        );
    }
    infantry
}

pub(crate) fn standing_frame(sequences: Option<&InfantrySequences>, facing: u8) -> usize {
    let octant = screen_octant(facing);
    sequences.map_or_else(
        || infantry_stand_frame(octant),
        |s| s.frame(octant, InfantryAnimation::default()),
    )
}

/// Match/background eligibility uses the same native unit files and direction
/// frames as live decoding. Check once per map type, before omitting baked art.
pub(crate) fn map_unit_kinds(
    vfs: &AssetFs,
    rules: &Rules,
    infantry: &HashMap<String, InfantrySequences>,
    pal: &Palette,
    map: &MapFile,
) -> HashSet<String> {
    let mut kinds: HashSet<String> = map
        .units
        .iter()
        .chain(&map.infantry)
        .chain(&map.aircraft)
        .map(|obj| obj.kind.to_ascii_uppercase())
        .collect();
    kinds.retain(|id| {
        let Some(t) = rules.techno(id) else {
            return false;
        };
        if t.kind == TechnoKind::Building {
            return false;
        }
        if t.art.as_ref().is_some_and(|a| a.voxel) {
            let Some(model) = crate::voxels::VoxelModel::load(vfs, &t.image.to_ascii_uppercase())
            else {
                return false;
            };
            let poses = model.frames_at(0);
            return (0..8).all(|octant| {
                crate::voxels::animated_sprite(&model, voxel_facing(octant), &poses, pal, [170; 3])
                    .is_some()
            });
        }
        // Theater-only unit files are not yet resolved by the live SHP path.
        if t.art.as_ref().is_some_and(|a| a.theater) {
            return false;
        }
        let Some(shp) = vfs
            .read(&format!("{}.SHP", t.image))
            .and_then(|data| ShpFile::parse(data).ok())
        else {
            return false;
        };
        if shp.width == 0
            || shp.height == 0
            || shp.width > 512
            || shp.height > 512
            || shp.frames.is_empty()
            || shp.frames.len() > 4096
        {
            return false;
        }
        (0..8).all(|octant| {
            let requested = standing_frame(infantry.get(id), octant * 32);
            let frame = if requested < shp.frames.len() / 2 {
                requested
            } else {
                0
            };
            shp.decode_frame_canvas(frame).is_ok()
                && (shp.frames.len() < 2
                    || shp
                        .decode_frame_canvas(frame + shp.frames.len() / 2)
                        .is_ok())
        })
    });
    kinds
}

/// One decoded object image in native (scale 1) pixels.
pub struct Sprite {
    pub w: u32,
    pub h: u32,
    /// RGBA8; alpha 0 is clear, [`SHADOW_ALPHA`] is a ground shadow.
    /// Other alpha values are body coverage; voxel antialiasing reserves 128.
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

#[derive(Clone, PartialEq, Eq, Hash)]
enum SpriteFrame {
    Shp(usize),
    Voxel { octant: u8, poses: Vec<usize> },
}

type SpriteKey = (String, SpriteFrame, [u8; 3]);

/// Limit decoded unit canvases even when modded parts have coprime
/// animation lengths. Keys contain resolved poses, never elapsed ticks.
const MAX_UNIT_SPRITES: usize = 512;

/// Lazily decoded sprites for one map's theater.
pub struct SpriteBank {
    vfs: AssetFs,
    rules: Rules,
    rules_ini: IniFile,
    art_ini: IniFile,
    theater: Theater,
    unit_pal: Palette,
    infantry: HashMap<String, InfantrySequences>,
    cache: HashMap<SpriteKey, Option<Sprite>>,
    cache_order: VecDeque<SpriteKey>,
    voxels: HashMap<String, Option<crate::voxels::VoxelModel>>,
    buildings: HashMap<String, Option<BuildingArt>>,
    map_structure_colors: HashMap<(String, u16, u16), [u8; 3]>,
    map_unit_kinds: HashSet<String>,
    layers: HashMap<LayerKey, Option<Sprite>>,
    effects: crate::effects::Effects,
    projectiles: crate::projectiles::Projectiles,
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
        // Map-house identity is presentation metadata. Session intentionally
        // groups pre-placed structures under the neutral simulation owner.
        let map_structure_colors = map
            .structures
            .iter()
            .filter(|obj| {
                crate::live_building_art(&vfs, &rules, &art_ini, &obj.kind, theater).is_some()
            })
            .map(|obj| {
                (
                    (obj.kind.to_ascii_uppercase(), obj.x, obj.y),
                    crate::house_color(&rules_ini, &obj.owner),
                )
            })
            .collect();
        let infantry = load_infantry_sequences(&vfs, &rules, &art_ini);
        let map_unit_kinds = map_unit_kinds(&vfs, &rules, &infantry, &unit_pal, &map);
        Ok(SpriteBank {
            vfs,
            rules,
            rules_ini,
            art_ini,
            theater,
            unit_pal,
            infantry,
            cache: HashMap::new(),
            cache_order: VecDeque::new(),
            voxels: HashMap::new(),
            buildings: HashMap::new(),
            map_structure_colors,
            map_unit_kinds,
            layers: HashMap::new(),
            effects: Default::default(),
            projectiles: Default::default(),
        })
    }

    /// This map type passed the shared live-art preflight.
    pub fn supports_map_unit(&self, kind: &str) -> bool {
        self.map_unit_kinds.contains(&kind.to_ascii_uppercase())
    }

    /// RGB house colour of a rules house/country section (`Color=`).
    pub fn house_color(&self, house: &str) -> [u8; 3] {
        crate::house_color(&self.rules_ini, house)
    }

    /// Original map-house colour for a pre-placed structure, without adding
    /// house names or rendering state to the simulation/save format.
    pub fn map_structure_color(&self, kind: &str, cell: (u16, u16)) -> Option<[u8; 3]> {
        self.map_structure_colors
            .get(&(kind.to_ascii_uppercase(), cell.0, cell.1))
            .copied()
    }

    pub fn muzzle_effect(&self, kind: &str, facing: u8) -> Option<String> {
        let weapon = self.rules_ini.value(kind, "Primary")?;
        let names: Vec<_> = self
            .rules_ini
            .value(weapon, "Anim")?
            .split(',')
            .map(str::trim)
            .take(9)
            .collect();
        let index = if names.len() == 8 {
            usize::from(screen_octant(facing))
        } else {
            0
        };
        names
            .get(index)
            .filter(|n| !n.is_empty())
            .map(|n| n.to_string())
    }

    pub fn destruction_effect(&self, kind: &str) -> Option<String> {
        self.rules_ini
            .value(kind, "Explosion")?
            .split(',')
            .next()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
    }

    /// Resolve the actual fired weapon, never the victim's type or current owner.
    pub fn impact_effect(&self, source: u64, damage: i64) -> Option<String> {
        crate::effects::impact_name(self.fired_warhead(source)?, damage)
    }

    fn fired_warhead(&self, source: u64) -> Option<&yr_formats::IniSection> {
        let mut matches = self
            .rules
            .weapons
            .iter()
            .filter(|weapon| weapon.source_id() == source);
        let weapon = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let warhead = weapon.warhead.as_deref()?;
        self.rules_ini.section(warhead)
    }

    /// Resolve supported death selectors from the actual lethal weapon.
    pub fn infantry_death_sequence(&mut self, kind: &str, source: u64) -> Option<u8> {
        let sequence = self.fired_warhead(source)?.get("InfDeath")?.parse().ok()?;
        self.infantry_death_alive(kind, sequence, 0)
            .then_some(sequence)
    }

    fn special_infantry_death(&self, kind: &str, sequence: u8) -> Option<&str> {
        // Explicit retail selectors choose generic human death art.
        // Non-human death behavior is separate.
        if self.rules.techno(kind)?.kind != TechnoKind::Infantry
            || self.rules_ini.section(kind)?.get_bool("NotHuman") == Some(true)
        {
            return None;
        }
        match sequence {
            3 => self.rules_ini.section("General")?.get("InfantryExplode"),
            4 => self.rules_ini.section("General")?.get("FlamingInfantry"),
            5 => Some("ELECTRO"),
            6 => self.rules_ini.section("General")?.get("InfantryHeadPop"),
            7 => Some("NUKEDIE"),
            8 => self.rules_ini.section("General")?.get("InfantryVirus"),
            9 => self.rules_ini.section("General")?.get("InfantryMutate"),
            10 => self.rules_ini.section("General")?.get("InfantryBrute"),
            _ => None,
        }
    }

    fn infantry_death_frame(
        &self,
        kind: &str,
        sequence: u8,
        facing: u8,
        age: u64,
    ) -> Option<usize> {
        let index = usize::from(sequence.checked_sub(1)?);
        let data = self.infantry.get(&kind.to_ascii_uppercase())?;
        data.death.get(index).copied().flatten()?.once(
            infantry_stand_frame(screen_octant(facing)),
            age,
            data.body_frames,
        )
    }

    pub fn infantry_death_alive(&mut self, kind: &str, sequence: u8, age: u64) -> bool {
        if let Some(name) = self
            .special_infantry_death(kind, sequence)
            .map(str::to_owned)
        {
            self.effect_alive(&name, age)
        } else {
            self.infantry_death_frame(kind, sequence, 0, age).is_some()
        }
    }

    /// Ordinary body/shadow art uses house color; special art uses ANIM.PAL.
    pub fn get_infantry_death(
        &mut self,
        kind: &str,
        sequence: u8,
        facing: u8,
        remap: [u8; 3],
        age: u64,
    ) -> Option<&Sprite> {
        if let Some(name) = self
            .special_infantry_death(kind, sequence)
            .map(str::to_owned)
        {
            return self.get_effect(&name, age);
        }
        let frame = self.infantry_death_frame(kind, sequence, facing, age)?;
        self.cached_sprite(kind.to_ascii_uppercase(), SpriteFrame::Shp(frame), remap)
    }

    pub fn effect_alive(&mut self, name: &str, ticks: u64) -> bool {
        self.effects.alive(&self.vfs, &self.art_ini, name, ticks)
    }

    pub fn get_effect(&mut self, name: &str, ticks: u64) -> Option<&Sprite> {
        self.effects.get(&self.vfs, &self.art_ini, name, ticks)
    }

    /// Static projectile art from the retained fired weapon identity.
    pub fn get_projectile(&mut self, source: u64) -> Option<&Sprite> {
        self.get_projectile_directed(source, None)
    }

    /// Optional explicit trail data follows the saved fired weapon identity.
    pub fn projectile_line_trail(&self, source: u64) -> Option<crate::LineTrail> {
        crate::LineTrail::load(self.projectile_art(source)?)
    }

    /// Optional particle art resolves independently of supported body classes.
    pub fn projectile_trailer(&self, source: u64) -> Option<crate::ProjectileTrailer> {
        crate::ProjectileTrailer::load(self.projectile_art(source)?)
    }

    fn projectile_art(&self, source: u64) -> Option<&yr_formats::IniSection> {
        let mut matches = self
            .rules
            .weapons
            .iter()
            .filter(|w| w.source_id() == source);
        let weapon = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let projectile = self.rules.projectiles.get(weapon.projectile.as_deref()?)?;
        if !projectile.defined || projectile.inviso == Some(true) {
            return None;
        }
        self.art_ini
            .section(projectile.image.as_deref().unwrap_or(&projectile.id))
    }

    /// Optional directional art uses the authoritative next flight step in leptons.
    pub fn get_projectile_directed(
        &mut self,
        source: u64,
        motion: Option<(i32, i32)>,
    ) -> Option<&Sprite> {
        self.get_projectile_directed_at(source, motion, 0)
    }

    pub fn get_projectile_directed_at(
        &mut self,
        source: u64,
        motion: Option<(i32, i32)>,
        tick: u64,
    ) -> Option<&Sprite> {
        self.projectiles.get_at(
            &self.vfs,
            &self.rules,
            &self.art_ini,
            &self.unit_pal,
            source,
            motion,
            tick,
        )
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
            let art =
                crate::live_building_art(&self.vfs, &self.rules, &self.art_ini, &id, self.theater);
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
        // Factory shells are health states, not looping clips. Retail
        // Allied roof art has two body frames despite a LoopEnd of 9.
        for overlay in &art.shell {
            if let Some(layer) = overlay.layer(animation.damaged) {
                keys.push(key(layer, state_frame(layer)));
            }
        }
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
    /// Voxel models use frame zero when no absolute tick is supplied.
    pub fn get_animated(
        &mut self,
        id: &str,
        facing: u8,
        remap: [u8; 3],
        animation: InfantryAnimation,
    ) -> Option<&Sprite> {
        self.get_animated_at(id, facing, remap, animation, 0)
    }

    /// Draw infantry actions and authored voxel poses at a completed sim
    /// tick. Repeated redraws select the same pose; movement does not
    /// restart rotor clocks. The clock does not change simulation state.
    pub fn get_animated_at(
        &mut self,
        id: &str,
        facing: u8,
        remap: [u8; 3],
        animation: InfantryAnimation,
        tick: u64,
    ) -> Option<&Sprite> {
        let octant = screen_octant(facing);
        let id = id.to_ascii_uppercase();
        let frame = if self
            .rules
            .techno(&id)
            .is_some_and(|t| t.art.as_ref().is_some_and(|a| a.voxel))
        {
            if !self.voxels.contains_key(&id) {
                let image = &self.rules.techno(&id)?.image;
                let model = crate::voxels::VoxelModel::load(&self.vfs, &image.to_ascii_uppercase());
                self.voxels.insert(id.clone(), model);
            }
            let model = self.voxels.get(&id)?.as_ref()?;
            SpriteFrame::Voxel {
                octant,
                poses: model.frames_at(tick),
            }
        } else if self
            .rules
            .techno(&id)
            .is_some_and(|t| t.kind == TechnoKind::Building)
        {
            SpriteFrame::Shp(0)
        } else {
            SpriteFrame::Shp(self.infantry.get(&id).map_or_else(
                || infantry_stand_frame(octant),
                |s| s.frame(octant, animation),
            ))
        };
        self.cached_sprite(id, frame, remap)
    }

    fn cached_sprite(&mut self, id: String, frame: SpriteFrame, remap: [u8; 3]) -> Option<&Sprite> {
        let key = (id.clone(), frame.clone(), remap);
        if !self.cache.contains_key(&key) {
            let sprite = self.decode(&id, &frame, remap);
            if self.cache.len() == MAX_UNIT_SPRITES {
                if let Some(oldest) = self.cache_order.pop_front() {
                    self.cache.remove(&oldest);
                }
            }
            self.cache_order.push_back(key.clone());
            self.cache.insert(key.clone(), sprite);
        }
        self.cache.get(&key).and_then(Option::as_ref)
    }

    fn decode(&self, id: &str, frame: &SpriteFrame, remap: [u8; 3]) -> Option<Sprite> {
        let t = self.rules.techno(id)?;
        let art = t.art.as_ref();
        let image = t.image.to_ascii_uppercase();
        let frame = match frame {
            SpriteFrame::Voxel { octant, poses } => {
                let model = self.voxels.get(id)?.as_ref()?;
                return crate::voxels::animated_sprite(
                    model,
                    voxel_facing(*octant),
                    poses,
                    &self.unit_pal,
                    remap,
                );
            }
            SpriteFrame::Shp(frame) => *frame,
        };
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
pub(crate) fn voxel_facing(octant: u8) -> u16 {
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
    fn retail_electrocution_requires_a_human_victim_and_explicit_lethal_selector() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        let source = bank.rules.weapons.get("ElectricBolt").unwrap().source_id();
        for kind in ["E1", "GGI", "SHK"] {
            assert_eq!(bank.infantry_death_sequence(kind, source), Some(5));
        }
        for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
            assert_eq!(bank.infantry_death_sequence(kind, source), None, "{kind}");
            assert!(bank
                .get_infantry_death(kind, 5, 0, [255, 0, 0], 0)
                .is_none());
        }
        let first = bank
            .get_infantry_death("E1", 5, 0, [255, 0, 0], 0)
            .unwrap()
            .rgba
            .clone();
        let other = bank
            .get_infantry_death("GGI", 5, 128, [0, 0, 255], 0)
            .unwrap();
        assert!(
            first == other.rgba,
            "generic death ignores facing and house remap"
        );
        assert!(bank.infantry_death_alive("E1", 5, 30));
        assert!(!bank.infantry_death_alive("E1", 5, 31));
        assert!(!bank.infantry_death_alive("E1", 5, u64::MAX));
        assert_eq!(bank.infantry_death_sequence("E1", 0), None);
        for selector in [0, 11, u8::MAX] {
            assert!(bank
                .get_infantry_death("E1", selector, 0, [0; 3], 0)
                .is_none());
        }
        // Optional art failure suppresses only the special death, even if
        // its source is known. It never substitutes a standing infantry.
        bank.art_ini = IniFile::parse("[ELECTRO]\nImage=../../unsafe");
        bank.effects = Default::default();
        assert_eq!(bank.infantry_death_sequence("E1", source), None);
        assert!(bank.get_infantry_death("E1", 5, 0, [0; 3], 0).is_none());
        assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
    }

    #[test]
    fn retail_virus_deaths_use_general_art_for_human_lethal_hits() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        for weapon in ["Virusgun", "VirusgunE"] {
            let source = bank.rules.weapons.get(weapon).unwrap().source_id();
            for kind in ["E1", "GGI", "SHK"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), Some(8));
            }
            for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), None, "{kind}");
                assert!(bank.get_infantry_death(kind, 8, 0, [0; 3], 0).is_none());
            }
        }
        let first = bank
            .get_infantry_death("E1", 8, 0, [255, 0, 0], 8)
            .unwrap()
            .rgba
            .clone();
        assert!(
            first
                == bank
                    .get_infantry_death("GGI", 8, 128, [0, 0, 255], 8)
                    .unwrap()
                    .rgba,
            "generic Virus death ignores facing and house remap"
        );
        assert!(bank.infantry_death_alive("E1", 8, 39));
        assert!(!bank.infantry_death_alive("E1", 8, 40));
        assert!(!bank.infantry_death_alive("E1", 8, u64::MAX));
        assert_eq!(bank.infantry_death_sequence("E1", 0), None);
        let source = bank.rules.weapons.get("Virusgun").unwrap().source_id();
        let original = bank.rules_ini.clone();
        bank.rules_ini
            .merge_from(&IniFile::parse("[General]\nInfantryVirus=ELECTRO"));
        let expected = bank.get_effect("ELECTRO", 8).unwrap().rgba.clone();
        assert!(bank.get_infantry_death("E1", 8, 0, [0; 3], 8).unwrap().rgba == expected);
        for invalid in ["", "MISSING", "../../unsafe"] {
            bank.rules_ini = original.clone();
            bank.rules_ini.merge_from(&IniFile::parse(&format!(
                "[General]\nInfantryVirus={invalid}"
            )));
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
        }
        bank.rules_ini = original;
        for malformed in ["[VIRUSD]\nImage=../../unsafe", "[VIRUSD]\nRate=0"] {
            bank.art_ini = IniFile::parse(malformed);
            bank.effects = Default::default();
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
            assert!(bank.get_infantry_death("E1", 8, 0, [0; 3], 0).is_none());
            assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
        }
    }

    #[test]
    fn retail_head_pop_uses_general_art_for_human_lethal_hits() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        for weapon in ["PsiWave", "SuperPsiWave", "CRSuperMindBlast"] {
            let source = bank.rules.weapons.get(weapon).expect(weapon).source_id();
            for kind in ["E1", "GGI", "SHK"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), Some(6));
            }
            for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), None, "{kind}");
                assert!(bank.get_infantry_death(kind, 6, 0, [0; 3], 0).is_none());
            }
        }
        let first = bank
            .get_infantry_death("E1", 6, 0, [255, 0, 0], 8)
            .unwrap()
            .rgba
            .clone();
        assert!(
            first
                == bank
                    .get_infantry_death("GGI", 6, 128, [0, 0, 255], 8)
                    .unwrap()
                    .rgba,
            "generic head pop ignores facing and remap"
        );
        assert!(bank.infantry_death_alive("E1", 6, 33));
        assert!(!bank.infantry_death_alive("E1", 6, 34));
        assert!(!bank.infantry_death_alive("E1", 6, u64::MAX));
        let source = bank.rules.weapons.get("PsiWave").unwrap().source_id();
        let original = bank.rules_ini.clone();
        // The general setting is authoritative, including optional overrides.
        bank.rules_ini
            .merge_from(&IniFile::parse("[General]\nInfantryHeadPop=ELECTRO"));
        let expected = bank.get_effect("ELECTRO", 8).unwrap().rgba.clone();
        assert!(bank.get_infantry_death("E1", 6, 0, [0; 3], 8).unwrap().rgba == expected);
        for invalid in ["", "MISSING", "../../unsafe"] {
            bank.rules_ini = original.clone();
            bank.rules_ini.merge_from(&IniFile::parse(&format!(
                "[General]\nInfantryHeadPop={invalid}"
            )));
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
        }
        bank.rules_ini = original;
        for malformed in ["[YURIDIE]\nImage=../../unsafe", "[YURIDIE]\nRate=0"] {
            bank.art_ini = IniFile::parse(malformed);
            bank.effects = Default::default();
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
            assert!(bank.get_infantry_death("E1", 6, 0, [0; 3], 0).is_none());
            assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
        }
    }

    #[test]
    fn retail_explosion_deaths_use_general_art_for_human_lethal_hits() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        for weapon in ["105mm", "120mm", "120mmx", "SonicZap"] {
            let source = bank.rules.weapons.get(weapon).expect(weapon).source_id();
            for kind in ["E1", "GGI", "SHK"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), Some(3));
            }
            for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), None, "{kind}");
                assert!(bank.get_infantry_death(kind, 3, 0, [0; 3], 0).is_none());
            }
        }
        let first = bank
            .get_infantry_death("E1", 3, 0, [255, 0, 0], 8)
            .unwrap()
            .rgba
            .clone();
        assert!(
            first
                == bank
                    .get_infantry_death("GGI", 3, 128, [0, 0, 255], 8)
                    .unwrap()
                    .rgba,
            "generic explosion ignores facing and remap"
        );
        assert!(bank.infantry_death_alive("E1", 3, 14));
        assert!(!bank.infantry_death_alive("E1", 3, 15));
        assert!(!bank.infantry_death_alive("E1", 3, u64::MAX));
        let source = bank.rules.weapons.get("105mm").unwrap().source_id();
        let original = bank.rules_ini.clone();
        // The general setting is authoritative, including optional overrides.
        bank.rules_ini
            .merge_from(&IniFile::parse("[General]\nInfantryExplode=ELECTRO"));
        let expected = bank.get_effect("ELECTRO", 8).unwrap().rgba.clone();
        assert!(bank.get_infantry_death("E1", 3, 0, [0; 3], 8).unwrap().rgba == expected);
        for invalid in ["", "MISSING", "../../unsafe"] {
            bank.rules_ini = original.clone();
            bank.rules_ini.merge_from(&IniFile::parse(&format!(
                "[General]\nInfantryExplode={invalid}"
            )));
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
        }
        bank.rules_ini = original.clone();
        bank.rules_ini
            .sections
            .iter_mut()
            .find(|section| section.name.eq_ignore_ascii_case("General"))
            .unwrap()
            .entries
            .retain(|(key, _)| !key.eq_ignore_ascii_case("InfantryExplode"));
        assert_eq!(bank.infantry_death_sequence("E1", source), None);
        assert!(bank.get_infantry_death("E1", 3, 0, [0; 3], 0).is_none());
        bank.rules_ini = original;
        for malformed in ["[S_BANG34]\nImage=../../unsafe", "[S_BANG34]\nRate=0"] {
            bank.art_ini = IniFile::parse(malformed);
            bank.effects = Default::default();
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
            assert!(bank.get_infantry_death("E1", 3, 0, [0; 3], 0).is_none());
            assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
        }
    }

    #[test]
    fn retail_burn_deaths_use_general_art_for_human_lethal_hits() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        for weapon in ["PsychicJab", "PsychicJabE", "MirageGun", "MirageGunE"] {
            let source = bank.rules.weapons.get(weapon).expect(weapon).source_id();
            for kind in ["E1", "GGI", "SHK"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), Some(4));
            }
            for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), None, "{kind}");
                assert!(bank.get_infantry_death(kind, 4, 0, [0; 3], 0).is_none());
            }
        }
        let first = bank
            .get_infantry_death("E1", 4, 0, [255, 0, 0], 8)
            .unwrap()
            .rgba
            .clone();
        assert!(
            first
                == bank
                    .get_infantry_death("GGI", 4, 128, [0, 0, 255], 8)
                    .unwrap()
                    .rgba,
            "generic burn collapse ignores facing and remap"
        );
        assert!(bank.infantry_death_alive("E1", 4, 26));
        assert!(!bank.infantry_death_alive("E1", 4, 27));
        assert!(!bank.infantry_death_alive("E1", 4, u64::MAX));
        let source = bank.rules.weapons.get("PsychicJab").unwrap().source_id();
        let original = bank.rules_ini.clone();
        // The general setting is authoritative, including optional overrides.
        bank.rules_ini
            .merge_from(&IniFile::parse("[General]\nFlamingInfantry=ELECTRO"));
        let expected = bank.get_effect("ELECTRO", 8).unwrap().rgba.clone();
        assert!(bank.get_infantry_death("E1", 4, 0, [0; 3], 8).unwrap().rgba == expected);
        for invalid in ["", "MISSING", "../../unsafe"] {
            bank.rules_ini = original.clone();
            bank.rules_ini.merge_from(&IniFile::parse(&format!(
                "[General]\nFlamingInfantry={invalid}"
            )));
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
        }
        bank.rules_ini = original.clone();
        bank.rules_ini
            .sections
            .iter_mut()
            .find(|section| section.name.eq_ignore_ascii_case("General"))
            .unwrap()
            .entries
            .retain(|(key, _)| !key.eq_ignore_ascii_case("FlamingInfantry"));
        assert_eq!(bank.infantry_death_sequence("E1", source), None);
        assert!(bank.get_infantry_death("E1", 4, 0, [0; 3], 0).is_none());
        bank.rules_ini = original;
        for malformed in ["[FLAMEGUY]\nImage=../../unsafe", "[FLAMEGUY]\nRate=0"] {
            bank.art_ini = IniFile::parse(malformed);
            bank.effects = Default::default();
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
            assert!(bank.get_infantry_death("E1", 4, 0, [0; 3], 0).is_none());
            assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
        }
    }

    #[test]
    fn retail_brute_smash_uses_general_art_for_human_lethal_hits() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        for weapon in ["Punch", "PunchE", "Smash", "TRexInfBite"] {
            let source = bank.rules.weapons.get(weapon).expect(weapon).source_id();
            for kind in ["E1", "GGI", "SHK"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), Some(10));
            }
            for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), None, "{kind}");
                assert!(bank.get_infantry_death(kind, 10, 0, [0; 3], 0).is_none());
            }
        }
        let first = bank
            .get_infantry_death("E1", 10, 0, [255, 0, 0], 8)
            .unwrap()
            .rgba
            .clone();
        assert!(
            first
                == bank
                    .get_infantry_death("GGI", 10, 128, [0, 0, 255], 8)
                    .unwrap()
                    .rgba,
            "generic Brute smash ignores facing and remap"
        );
        assert!(bank.infantry_death_alive("E1", 10, 67));
        assert!(!bank.infantry_death_alive("E1", 10, 68));
        assert!(!bank.infantry_death_alive("E1", 10, u64::MAX));
        let source = bank.rules.weapons.get("Punch").unwrap().source_id();
        let original = bank.rules_ini.clone();
        // The general setting is authoritative, including optional overrides.
        bank.rules_ini
            .merge_from(&IniFile::parse("[General]\nInfantryBrute=ELECTRO"));
        let expected = bank.get_effect("ELECTRO", 8).unwrap().rgba.clone();
        assert!(
            bank.get_infantry_death("E1", 10, 0, [0; 3], 8)
                .unwrap()
                .rgba
                == expected
        );
        for invalid in ["", "MISSING", "../../unsafe"] {
            bank.rules_ini = original.clone();
            bank.rules_ini.merge_from(&IniFile::parse(&format!(
                "[General]\nInfantryBrute={invalid}"
            )));
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
        }
        bank.rules_ini = original.clone();
        bank.rules_ini
            .sections
            .iter_mut()
            .find(|section| section.name.eq_ignore_ascii_case("General"))
            .unwrap()
            .entries
            .retain(|(key, _)| !key.eq_ignore_ascii_case("InfantryBrute"));
        assert_eq!(bank.infantry_death_sequence("E1", source), None);
        assert!(bank.get_infantry_death("E1", 10, 0, [0; 3], 0).is_none());
        bank.rules_ini = original;
        for malformed in ["[BRUTDIE]\nImage=../../unsafe", "[BRUTDIE]\nRate=0"] {
            bank.art_ini = IniFile::parse(malformed);
            bank.effects = Default::default();
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
            assert!(bank.get_infantry_death("E1", 10, 0, [0; 3], 0).is_none());
            assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
        }
    }

    #[test]
    fn retail_mutate_deaths_use_general_art_for_human_victims() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        // No retail weapon section carries the Mutate warheads; only the
        // Genetic Mutator superweapon uses them, and it is not simulated.
        // Selector resolution is exercised directly here.
        for kind in ["E1", "GGI", "SHK"] {
            assert!(
                bank.get_infantry_death(kind, 9, 0, [0; 3], 0).is_some(),
                "{kind}"
            );
            assert!(bank.infantry_death_alive(kind, 9, 0), "{kind}");
        }
        for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
            assert_eq!(bank.infantry_death_sequence(kind, 0), None, "{kind}");
            assert!(
                bank.get_infantry_death(kind, 9, 0, [0; 3], 0).is_none(),
                "{kind}"
            );
            assert!(!bank.infantry_death_alive(kind, 9, 0), "{kind}");
        }
        let first = bank
            .get_infantry_death("E1", 9, 0, [255, 0, 0], 8)
            .unwrap()
            .rgba
            .clone();
        assert!(
            first
                == bank
                    .get_infantry_death("GGI", 9, 128, [0, 0, 255], 8)
                    .unwrap()
                    .rgba,
            "generic Mutate death ignores facing and remap"
        );
        assert!(bank.infantry_death_alive("E1", 9, 67));
        assert!(!bank.infantry_death_alive("E1", 9, 68));
        assert!(!bank.infantry_death_alive("E1", 9, u64::MAX));
        let original = bank.rules_ini.clone();
        // The general setting is authoritative, including optional overrides.
        bank.rules_ini
            .merge_from(&IniFile::parse("[General]\nInfantryMutate=ELECTRO"));
        let expected = bank.get_effect("ELECTRO", 8).unwrap().rgba.clone();
        assert!(bank.get_infantry_death("E1", 9, 0, [0; 3], 8).unwrap().rgba == expected);
        for invalid in ["", "MISSING", "../../unsafe"] {
            bank.rules_ini = original.clone();
            bank.rules_ini.merge_from(&IniFile::parse(&format!(
                "[General]\nInfantryMutate={invalid}"
            )));
            assert!(bank.get_infantry_death("E1", 9, 0, [0; 3], 0).is_none());
            assert!(!bank.infantry_death_alive("E1", 9, 0));
        }
        bank.rules_ini = original.clone();
        bank.rules_ini
            .sections
            .iter_mut()
            .find(|section| section.name.eq_ignore_ascii_case("General"))
            .unwrap()
            .entries
            .retain(|(key, _)| !key.eq_ignore_ascii_case("InfantryMutate"));
        assert!(bank.get_infantry_death("E1", 9, 0, [0; 3], 0).is_none());
        assert!(!bank.infantry_death_alive("E1", 9, 0));
        bank.rules_ini = original;
        for malformed in ["[GENDEATH]\nImage=../../unsafe", "[GENDEATH]\nRate=0"] {
            bank.art_ini = IniFile::parse(malformed);
            bank.effects = Default::default();
            assert!(bank.get_infantry_death("E1", 9, 0, [0; 3], 0).is_none());
            assert!(!bank.infantry_death_alive("E1", 9, 0));
            assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
        }
    }

    #[test]
    fn retail_nuke_melt_requires_a_human_victim_and_explicit_lethal_selector() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        for weapon in ["RadBeamWeapon", "RadEruptionWeapon", "DiskLaser"] {
            let source = bank.rules.weapons.get(weapon).expect(weapon).source_id();
            for kind in ["E1", "GGI", "SHK"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), Some(7));
            }
            for kind in ["DOG", "COW", "MTNK", "SHAD", "GAPOWR", "MISSING"] {
                assert_eq!(bank.infantry_death_sequence(kind, source), None, "{kind}");
                assert!(bank.get_infantry_death(kind, 7, 0, [0; 3], 0).is_none());
            }
        }
        let first = bank
            .get_infantry_death("E1", 7, 0, [255, 0, 0], 0)
            .unwrap()
            .rgba
            .clone();
        let other = bank
            .get_infantry_death("GGI", 7, 128, [0, 0, 255], 0)
            .unwrap();
        assert!(first == other.rgba, "generic melt ignores facing and remap");
        assert!(bank.infantry_death_alive("E1", 7, 67));
        assert!(!bank.infantry_death_alive("E1", 7, 68));
        assert!(!bank.infantry_death_alive("E1", 7, u64::MAX));
        let source = bank.rules.weapons.get("RadBeamWeapon").unwrap().source_id();
        for malformed in ["[NUKEDIE]\nImage=../../unsafe", "[NUKEDIE]\nRate=0"] {
            bank.art_ini = IniFile::parse(malformed);
            bank.effects = Default::default();
            assert_eq!(bank.infantry_death_sequence("E1", source), None);
            assert!(bank.get_infantry_death("E1", 7, 0, [0; 3], 0).is_none());
            assert!(bank.get_infantry_death("E1", 1, 0, [0; 3], 0).is_some());
        }
    }

    #[test]
    fn retail_voxel_cache_uses_resolved_poses_and_stays_bounded() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let dir = std::path::PathBuf::from(home).join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        let remap = [220, 30, 30];
        let idle = InfantryAnimation::default();
        let signature = |s: &Sprite| (s.w, s.h, s.ax, s.ay, s.rgba.clone());
        let first = signature(bank.get_animated_at("SHAD", 0, remap, idle, 0).unwrap());
        assert_eq!(
            signature(bank.get_animated_at("SHAD", 0, remap, idle, 1).unwrap()),
            first
        );
        let second = signature(bank.get_animated_at("SHAD", 0, remap, idle, 2).unwrap());
        assert_ne!(first, second);
        let moving = InfantryAnimation {
            moving: true,
            elapsed: 123,
            since_shot: Some(0),
        };
        assert_eq!(
            signature(bank.get_animated_at("SHAD", 0, remap, moving, 2).unwrap()),
            second
        );
        for tick in [4, 100, 100_000] {
            assert_eq!(
                signature(bank.get_animated_at("SHAD", 0, remap, idle, tick).unwrap()),
                first
            );
        }
        assert_eq!(
            signature(
                bank.get_animated_at("SHAD", 0, remap, idle, u64::MAX)
                    .unwrap()
            ),
            second
        );
        assert_eq!(
            bank.cache.len(),
            2,
            "ticks and actions do not grow pose cache"
        );
        assert_eq!(bank.voxels.len(), 1, "model is parsed once per type");
        let mut rebuilt = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
        assert_eq!(
            signature(rebuilt.get_animated_at("SHAD", 0, remap, idle, 2).unwrap()),
            second,
            "rebuilding presentation at a saved tick keeps the phase"
        );
        let tank = signature(bank.get_animated_at("MTNK", 0, remap, idle, 0).unwrap());
        assert_eq!(
            signature(
                bank.get_animated_at("MTNK", 0, remap, idle, u64::MAX)
                    .unwrap()
            ),
            tank
        );
        assert_eq!(bank.cache.len(), 3, "static tank has one pose");
        for color in 0..MAX_UNIT_SPRITES + 1 {
            let remap = [color as u8, (color / 256) as u8, 0];
            assert!(bank.get("E1", 0, remap).is_some());
        }
        assert_eq!(bank.cache.len(), MAX_UNIT_SPRITES);
        assert_eq!(bank.cache_order.len(), MAX_UNIT_SPRITES);
        assert_eq!(
            signature(bank.get_animated_at("SHAD", 0, remap, idle, 0).unwrap()),
            first,
            "evicted pose rebuilds identically"
        );
        assert_eq!(bank.cache.len(), MAX_UNIT_SPRITES);
    }

    fn sequences() -> InfantrySequences {
        InfantrySequences {
            body_frames: 400,
            ready: Sequence::parse("0,1,1"),
            walk: Sequence::parse("8,6,6"),
            fire: Sequence::parse("164,6,6"),
            idle: [Sequence::parse("56,15,0,S"), Sequence::parse("71,15,0,E")],
            death: [Sequence::parse("134,15,0"), Sequence::parse("149,15,0")],
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
    fn death_sequences_play_once_and_reject_entire_invalid_ranges() {
        for value in ["134,15,0", "164,6,6"] {
            let sequence = Sequence::parse(value).unwrap();
            for facing in 0..8 {
                assert!(sequence
                    .once(facing, sequence.duration() - 1, 400)
                    .is_some());
                assert!(sequence.once(facing, sequence.duration(), 400).is_none());
                assert!(sequence.once(facing, u64::MAX, 400).is_none());
            }
        }
        for value in [
            format!("{},15,0", usize::MAX),
            format!("134,{},0", usize::MAX),
            format!("134,15,{}", usize::MAX),
            "399,2,0".into(),
            "360,6,6".into(),
        ] {
            let sequence = Sequence::parse(&value).unwrap();
            assert!(sequence.once(0, 0, 400).is_none(), "{value}");
        }
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
