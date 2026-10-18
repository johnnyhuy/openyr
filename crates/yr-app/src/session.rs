//! Skirmish session setup (issue #14): turn a parsed retail map into a
//! running deterministic sim.
//!
//! Contents: the sim terrain grid for the map, the map's own pre-placed
//! structures/units as sim entities, a player starting force at the
//! first start waypoint, a scripted enemy base at the second, and the
//! rules' starting credits. Starting composition is the sandbox's
//! documented pick (an MCV stand-in unit, one harvester, four tanks,
//! four infantry = the retail `UnitCount=10`); win/lose is judged on
//! owned entities with no buildings and no units left.

use yr_assets::{AssetFs, Theater};
use yr_formats::MapFile;
use yr_rules::{Armor, Rules, TechnoKind};
use yr_sim::terrain::{Loco, TerrainGrid};
use yr_sim::{Command, Factory, Fix, Role, Sim, UnitSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Won,
    Lost,
}

pub struct Session {
    pub sim: Sim,
    pub outcome: Outcome,
    had_enemy: bool,
    waves: Vec<Wave>,
    next_wave: usize,
    /// Enemy rally (spawn) and player base cells for scripted waves.
    enemy_at: (u16, u16),
    player_at: (u16, u16),
    /// Rules ids + specs needed to spawn wave units mid-match.
    wave_specs: Vec<(String, WaveUnit)>,
    /// What the player's sidebar can queue, in slot order.
    pub buildables: Vec<Buildable>,
    /// Ids of units spawned by waves (they hunt once idle).
    wave_units: Vec<u32>,
    /// Rules ids by sim `kind` tag (units and buildings); the first
    /// entries are the sidebar slots, so a `Produce` kind indexes here too.
    pub kinds: Vec<String>,
}

/// Idle wave units re-target every this many ticks.
const HUNT_TICKS: u64 = 30;

/// One sidebar slot: a rules unit the player's factories can build.
#[derive(Debug, Clone)]
pub struct Buildable {
    pub id: String,
    pub name: String,
    pub factory: Factory,
    pub cost: i64,
    /// Build time at full power in ticks (rules `BuildSpeed`).
    pub ticks: u32,
    /// Cameo as RGBA (w, h, pixels) when the art resolved.
    pub cameo: Option<(u32, u32, Vec<u8>)>,
    spec: UnitSpec,
}

/// Sandbox sidebar picks (allied tech, YR ids), kept when present in rules.
const VEHICLE_SLOTS: &[&str] = &["MTNK", "FV", "SREF", "CMIN"];
const INFANTRY_SLOTS: &[&str] = &["E1", "GGI", "SNIPE"];

/// One scripted attack wave: at `tick`, `count` of each unit kind spawn
/// beside the enemy base and attack-move to the player's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wave {
    pub tick: u64,
    pub units: Vec<(String, u8)>,
}

/// Spawn parameters resolved from rules once at setup.
#[derive(Debug, Clone, Copy)]
struct WaveUnit {
    loco: Loco,
    speed: Fix,
    hp: i64,
    armor: u8,
    weapon: Option<yr_sim::WeaponSpec>,
}

/// Ticks per second the session runs at (the window steps at 15 Hz).
pub const TICKS_PER_SEC: u64 = 15;

/// The sandbox's scripted wave plan: first wave after 90 s, then every
/// 60 s, escalating. Waves stop once the enemy base has no buildings.
pub fn default_waves(tank: Option<&str>, inf: Option<&str>) -> Vec<Wave> {
    let plan: [(u64, u8, u8); 5] = [
        (90, 2, 0),
        (150, 2, 3),
        (210, 3, 4),
        (270, 4, 4),
        (330, 6, 6),
    ];
    plan.iter()
        .map(|&(secs, tanks, infs)| {
            let mut units = Vec::new();
            if let Some(t) = tank {
                units.push((t.to_string(), tanks));
            }
            if let (Some(i), true) = (inf, infs > 0) {
                units.push((i.to_string(), infs));
            }
            Wave {
                tick: secs * TICKS_PER_SEC,
                units,
            }
        })
        .collect()
}

const PLAYER: u8 = yr_sim::OWNER_PLAYER;
const ENEMY: u8 = yr_sim::OWNER_ENEMY;
const NEUTRAL: u8 = yr_sim::OWNER_NEUTRAL;

/// Start waypoints from `[Waypoints]`: keys 0..=7 sorted, value is
/// `y * 1000 + x` (the same packing as `[Terrain]` keys).
pub fn start_waypoints(map: &MapFile) -> Vec<(u16, u16)> {
    let Some(sec) = map.ini.section("Waypoints") else {
        return Vec::new();
    };
    let mut keys: Vec<(u32, i64)> = sec
        .entries
        .iter()
        .filter_map(|(k, v)| Some((k.trim().parse::<u32>().ok()?, v.trim().parse::<i64>().ok()?)))
        .filter(|(k, _)| *k <= 7)
        .collect();
    keys.sort_by_key(|k| k.0);
    keys.iter()
        .map(|(_, v)| ((v % 1000) as u16, (v / 1000) as u16))
        .collect()
}

fn armor_idx(rules_techno: &yr_rules::TechnoType) -> u8 {
    rules_techno
        .armor
        .and_then(|a| Armor::ALL.iter().position(|x| *x == a))
        .unwrap_or(0) as u8
}

fn techno_loco(kind: TechnoKind) -> Loco {
    match kind {
        TechnoKind::Infantry => Loco::Foot,
        TechnoKind::Aircraft => Loco::Fly,
        _ => Loco::Track,
    }
}

/// Speed leptons/tick at 15 ticks/sec from rules `Speed=` (approximation
/// documented in the issue: Speed 6 tank ~= 154 leptons/tick).
fn techno_speed(t: &yr_rules::TechnoType) -> Fix {
    Fix::from_raw((t.speed.unwrap_or(4) * 256 / 10) as i32)
}

fn place_unit(
    kinds: &mut Vec<String>,
    next_id: &mut u32,
    rules: &Rules,
    id: &str,
    cell: (u16, u16),
    owner: u8,
    commands: &mut Vec<Command>,
) -> Option<u32> {
    let t = rules.techno(id)?;
    let weapon = t
        .primary
        .as_deref()
        .and_then(|w| yr_sim::WeaponSpec::from_rules(rules, w));
    let before = *next_id;
    *next_id += 1;
    commands.push(Command::Spawn {
        kind: intern(kinds, id),
        x: Fix::from_int(i32::from(cell.0)) + Fix::from_raw(128),
        y: Fix::from_int(i32::from(cell.1)) + Fix::from_raw(128),
        loco: techno_loco(t.kind),
        speed: techno_speed(t),
        hp: t.strength.unwrap_or(100),
        armor: armor_idx(t),
        weapon,
    });
    commands.push(Command::AssignOwner { id: before, owner });
    Some(before)
}

/// Index of rules id `id` in the session kind table, appending it once.
fn intern(kinds: &mut Vec<String>, id: &str) -> u16 {
    let i = kinds.iter().position(|k| k == id).unwrap_or_else(|| {
        kinds.push(id.to_string());
        kinds.len() - 1
    });
    u16::try_from(i).unwrap_or(u16::MAX)
}

fn first_of(rules: &Rules, ids: &[&str]) -> Option<String> {
    ids.iter()
        .find(|id| rules.techno(id).is_some())
        .map(|s| s.to_string())
}

/// First passable cell at or after `start`, scanning expanding offsets.
fn free_cell(
    grid: &TerrainGrid,
    loco: Loco,
    start: (u16, u16),
    sim: &Sim,
    taken_cells: &[(u16, u16)],
) -> Option<(u16, u16)> {
    for r in 0..12i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let (x, y) = (i32::from(start.0) + dx, i32::from(start.1) + dy);
                if x < 0 || y < 0 || x > u16::MAX as i32 || y > u16::MAX as i32 {
                    continue;
                }
                let (x, y) = (x as u16, y as u16);
                if !grid.passable(loco, x, y) {
                    continue;
                }
                let taken = taken_cells.contains(&(x, y))
                    || sim
                        .units()
                        .iter()
                        .any(|u| u.x.to_int_floor() as u16 == x && u.y.to_int_floor() as u16 == y);
                if !taken {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

struct BaseSpec<'a> {
    yard: &'a str,
    power_plant: &'a str,
    refinery: &'a str,
    extra: &'a [&'a str],
}

struct PlaceCtx<'a> {
    next_id: &'a mut u32,
    taken_cells: &'a mut Vec<(u16, u16)>,
    kinds: &'a mut Vec<String>,
    rules: &'a Rules,
    grid: &'a TerrainGrid,
}

fn place_base(
    ctx: &mut PlaceCtx,
    near: (u16, u16),
    owner: u8,
    spec: &BaseSpec,
    commands: &mut Vec<Command>,
) {
    let (rules, grid) = (ctx.rules, ctx.grid);
    let (next_id, taken_cells) = (&mut *ctx.next_id, &mut *ctx.taken_cells);
    let defs = [
        (spec.yard, Role::Combat),
        (spec.power_plant, Role::Combat),
        (spec.refinery, Role::Refinery),
    ];
    let mut placed: Vec<(String, Role)> = defs.iter().map(|(a, b)| (a.to_string(), *b)).collect();
    placed.extend(spec.extra.iter().map(|e| (e.to_string(), Role::Combat)));
    for (id, role) in placed {
        let Some(t) = rules.techno(&id) else { continue };
        // `Factory=` decides which production queue a building hosts.
        let role = match t.factory.as_deref() {
            Some(f) if f.eq_ignore_ascii_case("UnitType") => Role::WarFactory,
            Some(f) if f.eq_ignore_ascii_case("InfantryType") => Role::Barracks,
            _ => role,
        };
        let (fw, fh) = t
            .art
            .as_ref()
            .and_then(|a| a.foundation)
            .unwrap_or((2u8, 2u8));
        let (fw, fh) = (u16::from(fw), u16::from(fh));
        // Find a free footprint whose cells are all passable now.
        let mut spot = None;
        'scan: for r in 0..16i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (x, y) = (i32::from(near.0) + dx, i32::from(near.1) + dy);
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let (x, y) = (x as u16, y as u16);
                    let ok = (0..fh).all(|oy| {
                        (0..fw).all(|ox| {
                            grid.cell(x + ox, y + oy).is_some_and(|c| !c.building)
                                && !taken_cells.contains(&(x + ox, y + oy))
                        })
                    });
                    if ok {
                        spot = Some((x, y));
                        break 'scan;
                    }
                }
            }
        }
        let Some((x, y)) = spot else { continue };
        for oy in 0..fh {
            for ox in 0..fw {
                taken_cells.push((x + ox, y + oy));
            }
        }
        *next_id += 1;
        commands.push(Command::PlaceBuilding {
            kind: intern(ctx.kinds, &id),
            x,
            y,
            w: fw,
            h: fh,
            hp: t.strength.unwrap_or(500),
            armor: armor_idx(t),
            power: t.power.unwrap_or(0),
            owner,
            role,
        });
    }
}

/// Load a session straight from a game dir + map name (window entry point).
/// Mirrors the retail test: map INI overrides merge into the rules.
pub fn load(gamedir: &str, map_name: &str, seed: u64) -> Result<Session, String> {
    let mut fs = AssetFs::open(std::path::Path::new(gamedir)).map_err(|e| e.to_string())?;
    let raw = match fs.read(map_name) {
        Some(d) => d.to_vec(),
        None => std::fs::read(map_name).map_err(|e| format!("{map_name}: {e}"))?,
    };
    let map = MapFile::parse(&String::from_utf8_lossy(&raw)).map_err(|e| e.to_string())?;
    let mut rules_ini = yr_formats::IniFile::parse(&String::from_utf8_lossy(
        fs.read("RULESMD.INI").ok_or("rulesmd.ini not found")?,
    ));
    rules_ini.merge_from(&map.ini);
    let art_ini = yr_formats::IniFile::parse(&String::from_utf8_lossy(
        fs.read("ARTMD.INI").ok_or("artmd.ini not found")?,
    ));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    setup(&mut fs, &map, &rules, &overlay_types, seed)
}

/// Build a session for `map` on the opened install.
pub fn setup(
    fs: &mut AssetFs,
    map: &MapFile,
    rules: &Rules,
    overlay_types: &[String],
    seed: u64,
) -> Result<Session, String> {
    let theater = Theater::from_map_name(&map.theater).ok_or("unknown theater")?;
    fs.set_theater(theater);
    let grid = yr_render::build_terrain(fs, map, overlay_types, theater)?;
    let mut sim = Sim::new(seed);
    if let (Some(min), Some(max)) = (
        rules.globals.min_low_power_production_speed,
        rules.globals.max_low_power_production_speed,
    ) {
        let to_fix = |d: yr_rules::Decimal| Fix::from_raw((d.units() * 256 / 1_000_000) as i32);
        sim.set_low_power_speeds(to_fix(min), to_fix(max));
    }
    sim.set_terrain(grid.clone());
    sim.set_credits(rules.skirmish.money.unwrap_or(10_000));

    let buildables = buildables(fs, rules);
    let mut kinds: Vec<String> = buildables.iter().map(|b| b.id.clone()).collect();
    let mut cmds: Vec<Command> = Vec::new();
    let mut next_id = sim.next_id_peek();
    let mut taken_cells: Vec<(u16, u16)> = Vec::new();
    // Map's own pre-placed objects (neutral unless owned otherwise).
    for obj in &map.structures {
        let Some(t) = rules.techno(&obj.kind) else {
            continue;
        };
        next_id += 1;
        let (fw, fh) = t
            .art
            .as_ref()
            .and_then(|a| a.foundation)
            .unwrap_or((1u8, 1u8));
        let (fw, fh) = (u16::from(fw), u16::from(fh));
        cmds.push(Command::PlaceBuilding {
            kind: intern(&mut kinds, &obj.kind),
            x: obj.x,
            y: obj.y,
            w: fw,
            h: fh,
            hp: t.strength.unwrap_or(500) * i64::from(obj.health.max(1)) / 256,
            armor: armor_idx(t),
            power: t.power.unwrap_or(0),
            owner: NEUTRAL,
            role: Role::Combat,
        });
    }
    for obj in map.units.iter().chain(&map.infantry).chain(&map.aircraft) {
        let mut extra = Vec::new();
        place_unit(
            &mut kinds,
            &mut next_id,
            rules,
            &obj.kind,
            (obj.x, obj.y),
            NEUTRAL,
            &mut extra,
        );
        cmds.extend(extra);
        taken_cells.push((obj.x, obj.y));
    }
    // Neutral spawns queued above use predicted ids; owner assignment is
    // inside place_unit via next_id_peek. Starting forces:
    let waypoints = start_waypoints(map);
    let player_at = waypoints.first().copied().unwrap_or((10, 10));
    let yard = first_of(rules, &["GACNST", "NACNST", "YACNST"]);
    let plant = first_of(rules, &["GAPOWR", "NAPOWR", "YAPOWR"]);
    let refinery = first_of(rules, &["GAREFN", "NAREFN", "YAREFN"]);
    let weap = first_of(rules, &["GAWEAP", "NAWEAP", "YAWEAP"]);
    let barracks = first_of(rules, &["GAPILE", "NAHAND", "YABRCK"]);
    if let (Some(y), Some(p), Some(r)) = (&yard, &plant, &refinery) {
        let factories: Vec<&str> = weap.iter().chain(&barracks).map(String::as_str).collect();
        let mut ctx = PlaceCtx {
            next_id: &mut next_id,
            taken_cells: &mut taken_cells,
            kinds: &mut kinds,
            rules,
            grid: &grid,
        };
        place_base(
            &mut ctx,
            player_at,
            PLAYER,
            &BaseSpec {
                yard: y,
                power_plant: p,
                refinery: r,
                extra: &factories,
            },
            &mut cmds,
        );
    }
    let mcv = first_of(rules, &["MCV", "SMCV", "PCV"]);
    let harv = first_of(rules, &["HARV", "CMON", "AMCV"]);
    let tank = first_of(rules, &["GTNK", "HTNK", "MTNK", "LTNK"]);
    let inf = first_of(rules, &["E1", "E2", "CONS", "SHK"]);
    let mut force: Vec<String> = Vec::new();
    if let Some(m) = &mcv {
        force.push(m.clone());
    }
    if let Some(h) = &harv {
        force.push(h.clone());
    }
    if let Some(t) = &tank {
        force.extend(std::iter::repeat_n(t.clone(), 4));
    }
    if let Some(i) = &inf {
        force.extend(std::iter::repeat_n(i.clone(), 4));
    }
    let target_count = rules.skirmish.unit_count.unwrap_or(10).max(0) as usize;
    if force.len() > target_count {
        force.truncate(target_count);
    }
    for id in &force {
        let loco = rules
            .techno(id)
            .map(|t| techno_loco(t.kind))
            .unwrap_or(Loco::Track);
        let Some(cell) = free_cell(&grid, loco, player_at, &sim, &taken_cells) else {
            continue;
        };
        taken_cells.push(cell);
        let mut extra = Vec::new();
        let uid = place_unit(
            &mut kinds,
            &mut next_id,
            rules,
            id,
            cell,
            PLAYER,
            &mut extra,
        );
        cmds.extend(extra);
        if let (Some(uid), Some(h)) = (uid, &harv) {
            if id == h {
                cmds.push(Command::AssignRole {
                    id: uid,
                    role: Role::Harvester,
                });
            }
        }
    }
    // Scripted enemy base + guards at the second waypoint.
    let had_enemy = waypoints.len() > 1;
    if had_enemy {
        let enemy_at = waypoints[1];
        if let (Some(y), Some(p), Some(r)) = (&yard, &plant, &refinery) {
            let extra: Vec<&str> = weap.iter().map(String::as_str).collect();
            let mut ctx = PlaceCtx {
                next_id: &mut next_id,
                taken_cells: &mut taken_cells,
                kinds: &mut kinds,
                rules,
                grid: &grid,
            };
            place_base(
                &mut ctx,
                enemy_at,
                ENEMY,
                &BaseSpec {
                    yard: y,
                    power_plant: p,
                    refinery: r,
                    extra: &extra,
                },
                &mut cmds,
            );
        }
        if let Some(t) = &tank {
            for _ in 0..3 {
                let Some(cell) = free_cell(&grid, Loco::Track, enemy_at, &sim, &taken_cells) else {
                    break;
                };
                taken_cells.push(cell);
                let mut extra = Vec::new();
                place_unit(&mut kinds, &mut next_id, rules, t, cell, ENEMY, &mut extra);
                cmds.extend(extra);
            }
        }
    }
    sim.step(&cmds);
    let waves = if had_enemy {
        default_waves(tank.as_deref(), inf.as_deref())
    } else {
        Vec::new()
    };
    let mut wave_specs: Vec<(String, WaveUnit)> = Vec::new();
    for (id, _) in waves.iter().flat_map(|w| &w.units) {
        if wave_specs.iter().any(|(k, _)| k == id) {
            continue;
        }
        let Some(t) = rules.techno(id) else { continue };
        wave_specs.push((
            id.clone(),
            WaveUnit {
                loco: techno_loco(t.kind),
                speed: techno_speed(t),
                hp: t.strength.unwrap_or(100),
                armor: armor_idx(t),
                weapon: t
                    .primary
                    .as_deref()
                    .and_then(|w| yr_sim::WeaponSpec::from_rules(rules, w)),
            },
        ));
    }
    let enemy_at = waypoints.get(1).copied().unwrap_or(player_at);
    Ok(Session {
        sim,
        outcome: Outcome::Ongoing,
        had_enemy,
        waves,
        next_wave: 0,
        enemy_at,
        player_at,
        wave_specs,
        buildables,
        kinds,
        wave_units: Vec::new(),
    })
}

/// Build time in ticks: `Cost * BuildSpeed` minutes per 1000 credits at
/// 15 ticks/s (900 ticks per minute); exact integer maths on the decimal.
pub fn build_ticks(cost: i64, build_speed_units: i64) -> u32 {
    let t = cost.max(0) * build_speed_units * 900 / (1000 * 1_000_000);
    t.clamp(1, i64::from(u32::MAX)) as u32
}

fn buildables(fs: &AssetFs, rules: &Rules) -> Vec<Buildable> {
    let speed = rules.globals.build_speed.map_or(700_000, |d| d.units());
    let palette = ["CAMEO.PAL", "CAMEOMD.PAL"]
        .iter()
        .find_map(|n| fs.read(n).and_then(yr_formats::Palette::parse));
    let mut out = Vec::new();
    let slots = VEHICLE_SLOTS
        .iter()
        .map(|id| (*id, Factory::Vehicle))
        .chain(INFANTRY_SLOTS.iter().map(|id| (*id, Factory::Infantry)));
    for (id, factory) in slots {
        let Some(t) = rules.techno(id) else { continue };
        let Some(cost) = t.cost else { continue };
        let role = if t.harvester == Some(true) {
            Role::Harvester
        } else {
            Role::Combat
        };
        let cameo = t
            .art
            .as_ref()
            .and_then(|a| a.cameo.as_deref())
            .zip(palette.as_ref())
            .and_then(|(c, pal)| decode_cameo(fs, c, pal));
        out.push(Buildable {
            id: id.to_string(),
            name: t
                .ui_name
                .clone()
                .or(t.name.clone())
                .unwrap_or_else(|| id.to_string()),
            factory,
            cost,
            ticks: build_ticks(cost, speed),
            cameo,
            spec: UnitSpec {
                loco: techno_loco(t.kind),
                speed: techno_speed(t),
                hp: t.strength.unwrap_or(100),
                armor: armor_idx(t),
                weapon: t
                    .primary
                    .as_deref()
                    .and_then(|w| yr_sim::WeaponSpec::from_rules(rules, w)),
                role,
            },
        });
    }
    out
}

/// First frame of a cameo SHP through the cameo palette (index 0 clear).
fn decode_cameo(
    fs: &AssetFs,
    name: &str,
    pal: &yr_formats::Palette,
) -> Option<(u32, u32, Vec<u8>)> {
    let data = fs.read(&format!("{name}.SHP"))?;
    let shp = yr_formats::ShpFile::parse(data).ok()?;
    let idx = shp.decode_frame_canvas(0).ok()?;
    let rgba = idx
        .iter()
        .flat_map(|&i| if i == 0 { [0, 0, 0, 0] } else { pal.rgba(i) })
        .collect();
    Some((u32::from(shp.width), u32::from(shp.height), rgba))
}

impl Session {
    /// Queue sidebar slot `slot` (None when out of range).
    pub fn produce(&self, slot: usize) -> Option<Command> {
        let b = self.buildables.get(slot)?;
        Some(Command::Produce {
            factory: b.factory,
            kind: u16::try_from(slot).ok()?,
            spec: b.spec,
            cost: b.cost,
            ticks: b.ticks,
        })
    }

    /// Cancel the newest queued build of sidebar slot `slot`.
    pub fn cancel(&self, slot: usize) -> Option<Command> {
        let b = self.buildables.get(slot)?;
        Some(Command::CancelProduce {
            factory: b.factory,
            kind: u16::try_from(slot).ok()?,
        })
    }

    /// Queued count of `slot` and, when it heads its factory queue, the
    /// build progress as (done, total).
    pub fn slot_status(&self, slot: usize) -> (usize, Option<(i64, i64)>) {
        let queue = self.sim.production();
        let count = queue.iter().filter(|o| usize::from(o.kind) == slot).count();
        let head = self.buildables.get(slot).and_then(|b| {
            queue
                .iter()
                .find(|o| o.factory == b.factory)
                .filter(|o| usize::from(o.kind) == slot)
                .map(|o| o.fraction())
        });
        (count, head)
    }

    /// Advance the match and re-judge win/lose on owned entities.
    pub fn step(&mut self, commands: &[Command]) {
        let mut cmds = self.wave_commands();
        cmds.extend(self.hunt_commands());
        cmds.extend_from_slice(commands);
        self.sim.step(&cmds);
        if self.outcome != Outcome::Ongoing {
            return;
        }
        let player_alive = self.sim.units().iter().any(|u| u.owner == PLAYER)
            || self.sim.buildings().iter().any(|b| b.owner == PLAYER);
        let enemy_alive = self.sim.units().iter().any(|u| u.owner == ENEMY)
            || self.sim.buildings().iter().any(|b| b.owner == ENEMY);
        if !player_alive {
            self.outcome = Outcome::Lost;
        } else if self.had_enemy && !enemy_alive {
            self.outcome = Outcome::Won;
        }
    }

    /// Scripted waves press on: a wave unit left idle (its attack-move
    /// done, target dead) attacks the nearest player building or unit,
    /// ties broken by entity order. Without this, waves that reach the
    /// base outside weapon range of a building stall the match. Once the
    /// enemy base has fallen its guards join the hunt (last stand).
    fn hunt_commands(&mut self) -> Vec<Command> {
        let sim = &self.sim;
        self.wave_units
            .retain(|id| sim.units().iter().any(|u| u.id == *id));
        if !sim.tick().is_multiple_of(HUNT_TICKS) || self.outcome != Outcome::Ongoing {
            return Vec::new();
        }
        let targets: Vec<(u32, (Fix, Fix))> = sim
            .buildings()
            .iter()
            .filter(|b| b.owner == PLAYER)
            .map(|b| (b.id, b.centre()))
            .chain(
                sim.units()
                    .iter()
                    .filter(|u| u.owner == PLAYER)
                    .map(|u| (u.id, (u.x, u.y))),
            )
            .collect();
        let last_stand = !sim.buildings().iter().any(|b| b.owner == ENEMY);
        let mut cmds = Vec::new();
        for u in sim.units() {
            let hunter = u.owner == ENEMY && (last_stand || self.wave_units.contains(&u.id));
            if !hunter || u.weapon.is_none() || u.target.is_some() || u.attack.is_some() {
                continue;
            }
            let d2 = |p: (Fix, Fix)| {
                let (dx, dy) = (i64::from((p.0 - u.x).raw()), i64::from((p.1 - u.y).raw()));
                dx * dx + dy * dy
            };
            if let Some(&(target, _)) = targets.iter().min_by_key(|(_, p)| d2(*p)) {
                cmds.push(Command::Attack { id: u.id, target });
            }
        }
        cmds
    }

    /// Waves launched so far and the total planned.
    pub fn waves_progress(&self) -> (usize, usize) {
        (self.next_wave, self.waves.len())
    }

    /// Seconds until the next wave, if one is still pending.
    pub fn next_wave_in_secs(&self) -> Option<u64> {
        let w = self.waves.get(self.next_wave)?;
        Some(
            w.tick
                .saturating_sub(self.sim.tick())
                .div_ceil(TICKS_PER_SEC),
        )
    }

    /// Commands for a wave due this tick: spawn beside the enemy base,
    /// hand to the enemy, attack-move on the player's start. Ids are
    /// predicted from `next_id_peek` (spawns run first in the slice).
    /// A wave only launches while the enemy still owns a building.
    fn wave_commands(&mut self) -> Vec<Command> {
        let mut cmds = Vec::new();
        let Some(wave) = self.waves.get(self.next_wave) else {
            return cmds;
        };
        if self.sim.tick() < wave.tick || self.outcome != Outcome::Ongoing {
            return cmds;
        }
        self.next_wave += 1;
        if !self.sim.buildings().iter().any(|b| b.owner == ENEMY) {
            return cmds;
        }
        let Some(grid) = self.sim.terrain().cloned() else {
            return cmds;
        };
        let mut next_id = self.sim.next_id_peek();
        let mut taken: Vec<(u16, u16)> = Vec::new();
        for (kind, count) in &wave.units {
            let Some((_, spec)) = self.wave_specs.iter().find(|(k, _)| k == kind) else {
                continue;
            };
            // March on the nearest passable cell to the player's start
            // (the start itself is usually under the construction yard).
            let Some(goal) = free_cell(&grid, spec.loco, self.player_at, &self.sim, &[]) else {
                continue;
            };
            let dest = (
                Fix::from_int(i32::from(goal.0)) + Fix::from_raw(128),
                Fix::from_int(i32::from(goal.1)) + Fix::from_raw(128),
            );
            for _ in 0..*count {
                let Some(cell) = free_cell(&grid, spec.loco, self.enemy_at, &self.sim, &taken)
                else {
                    break;
                };
                taken.push(cell);
                let id = next_id;
                next_id += 1;
                cmds.push(Command::Spawn {
                    kind: intern(&mut self.kinds, kind),
                    x: Fix::from_int(i32::from(cell.0)) + Fix::from_raw(128),
                    y: Fix::from_int(i32::from(cell.1)) + Fix::from_raw(128),
                    loco: spec.loco,
                    speed: spec.speed,
                    hp: spec.hp,
                    armor: spec.armor,
                    weapon: spec.weapon,
                });
                cmds.push(Command::AssignOwner { id, owner: ENEMY });
                self.wave_units.push(id);
                cmds.push(Command::AttackMove {
                    id,
                    x: dest.0,
                    y: dest.1,
                });
            }
        }
        cmds
    }
}
