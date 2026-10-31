//! Skirmish session setup (issue #14): turn a parsed retail map into a
//! running deterministic sim.
//!
//! Contents: the sim terrain grid for the map, the map's own pre-placed
//! structures/units as sim entities, a player starting force at the
//! first start waypoint, a computer base at the second, and the
//! rules' starting credits. Starting composition is the sandbox's
//! documented pick (an MCV stand-in unit, one harvester, four tanks,
//! four infantry = the retail `UnitCount=10`); win/lose is judged on
//! owned entities with no buildings and no units left.

use yr_assets::{AssetFs, Theater};
use yr_formats::MapFile;
use yr_rules::{Armor, Rules, TechnoKind};
use yr_sim::terrain::{Loco, TerrainGrid};
use yr_sim::{BuildingSpec, Command, Factory, Fix, Product, Role, Sim, UnitSpec};

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
    /// What the player's sidebar can queue, in slot order.
    pub buildables: Vec<Buildable>,
    pub sidebar_art: crate::sidebar::SidebarArt,
    pub radar: crate::sidebar::radar::Radar,
    pub cursor_art: crate::cursor::Art,
    pub(crate) services: crate::service::Services,
    /// Rules ids by sim `kind` tag (units and buildings); the first
    /// entries are the sidebar slots, so a `Produce` kind indexes here too.
    pub kinds: Vec<String>,
    /// Generic prerequisite tokens (`POWER`, ...) and their buildings.
    prerequisite_groups: Vec<(String, Vec<String>)>,
    pub(crate) seed: u64,
    pub(crate) setup_hash: u64,
}

/// One sidebar slot: a rules unit or structure the player can build.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Buildable {
    pub id: String,
    #[serde(skip)]
    pub name: String,
    /// Presentation only; filtering never changes the production kind tag.
    #[serde(skip)]
    pub category: BuildCategory,
    pub factory: Factory,
    pub cost: i64,
    /// Build time at full power in ticks (rules `BuildSpeed`).
    pub ticks: u32,
    /// Cameo as RGBA (w, h, pixels) when the art resolved.
    #[serde(skip)]
    pub cameo: Option<(u32, u32, Vec<u8>)>,
    /// Rules `Prerequisite=` tokens (type ids or generic groups).
    prerequisite: Vec<String>,
    #[serde(serialize_with = "serialize_setup_product")]
    product: Product,
}

fn serialize_setup_product<S: serde::Serializer>(
    product: &Product,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&product.without_weapon_source(), serializer)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BuildCategory {
    #[default]
    Structures,
    Defences,
    Infantry,
    Vehicles,
}

impl BuildCategory {
    fn from_techno(t: &yr_rules::TechnoType) -> Self {
        match t.kind {
            TechnoKind::Building
                if t.build_category
                    .as_deref()
                    .is_some_and(|c| c.eq_ignore_ascii_case("Combat")) =>
            {
                Self::Defences
            }
            TechnoKind::Building => Self::Structures,
            TechnoKind::Infantry => Self::Infantry,
            _ => Self::Vehicles,
        }
    }
}

impl Buildable {
    /// Footprint in cells for structures, `None` for units.
    pub fn footprint(&self) -> Option<(u16, u16)> {
        match self.product {
            Product::Building(b) => Some((b.w, b.h)),
            Product::Unit(_) => None,
        }
    }
}

/// Sandbox sidebar picks (allied tech, YR ids), kept when present in rules.
const VEHICLE_SLOTS: &[&str] = &["MTNK", "FV", "SREF", "CMIN"];
const INFANTRY_SLOTS: &[&str] = &["E1", "GGI", "SNIPE"];

/// The player's country. `[Countries]` lists Americans first, the
/// skirmish dialog's default; it decides `RequiredHouses=` and
/// `ForbiddenHouses=` (no sniper, Airforce Command instead of the
/// generic air pad).
pub const PLAYER_COUNTRY: &str = "Americans";

/// Ticks per second the session runs at (the window steps at 15 Hz).
pub const TICKS_PER_SEC: u64 = 15;

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

/// `Factory=` decides which production queue a building hosts;
/// `Refinery=yes` takes harvester deliveries.
fn building_role(t: &yr_rules::TechnoType) -> Role {
    match t.factory.as_deref() {
        Some(f) if f.eq_ignore_ascii_case("BuildingType") => Role::ConstructionYard,
        Some(f) if f.eq_ignore_ascii_case("UnitType") => Role::WarFactory,
        Some(f) if f.eq_ignore_ascii_case("InfantryType") => Role::Barracks,
        _ if t.refinery == Some(true) => Role::Refinery,
        _ => Role::Combat,
    }
}

/// What the sim needs to spawn a unit of rules type `t`.
fn unit_spec(rules: &Rules, t: &yr_rules::TechnoType) -> UnitSpec {
    UnitSpec {
        loco: techno_loco(t.kind),
        speed: techno_speed(t),
        hp: t.strength.unwrap_or(100),
        armor: armor_idx(t),
        weapon: t
            .primary
            .as_deref()
            .and_then(|w| yr_sim::WeaponSpec::from_rules(rules, w)),
        role: if t.harvester == Some(true) {
            Role::Harvester
        } else {
            Role::Combat
        },
    }
}

/// Whether `t` is open to `country` under `RequiredHouses=`/`ForbiddenHouses=`.
fn house_allows(t: &yr_rules::TechnoType, country: &str) -> bool {
    let named = |list: &[String]| list.iter().any(|h| h.eq_ignore_ascii_case(country));
    (t.required_houses.is_empty() || named(&t.required_houses)) && !named(&t.forbidden_houses)
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
        let role = match building_role(t) {
            Role::Combat => role,
            derived => derived,
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

    let yard = first_of(rules, &["GACNST", "NACNST", "YACNST"]);
    let (buildables, mut kinds) = buildables(fs, rules, yard.as_deref());
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
    // Computer base and guards at the second waypoint.
    let had_enemy = waypoints.len() > 1;
    if had_enemy {
        let enemy_at = waypoints[1];
        if let (Some(y), Some(p), Some(r)) = (&yard, &plant, &refinery) {
            let extra: Vec<&str> = weap.iter().chain(&barracks).map(String::as_str).collect();
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
    if had_enemy {
        // Both sides use Allied factory types in this first skirmish. Resolve
        // costs, specs and prerequisite groups once; decisions live in yr-sim.
        let mut choices = Vec::new();
        for id in ["MTNK", "E1", "GAPOWR", "GAREFN", "GAWEAP", "GAPILE"] {
            let Some((slot, b)) = buildables.iter().enumerate().find(|(_, b)| b.id == id) else {
                continue;
            };
            let prerequisites = b
                .prerequisite
                .iter()
                .map(|token| {
                    let group = rules
                        .globals
                        .prerequisite_groups
                        .iter()
                        .find(|(t, _)| t.eq_ignore_ascii_case(token))
                        .map(|(_, ids)| ids);
                    kinds
                        .iter()
                        .enumerate()
                        .filter(|(_, id)| {
                            group.map_or_else(
                                || token.eq_ignore_ascii_case(id),
                                |ids| ids.iter().any(|g| g.eq_ignore_ascii_case(id)),
                            )
                        })
                        .map(|(i, _)| i as u16)
                        .collect()
                })
                .collect();
            let mut product = b.product;
            // Economy-lite supplies income; do not also grant free harvesters.
            if let Product::Building(spec) = &mut product {
                spec.free_unit = None;
            }
            choices.push(yr_sim::ai::AiBuild {
                factory: b.factory,
                kind: slot as u16,
                product,
                cost: b.cost,
                ticks: b.ticks,
                prerequisites,
            });
        }
        sim.set_credits_of(ENEMY, rules.skirmish.money.unwrap_or(10_000));
        sim.set_ai(Some(yr_sim::ai::SkirmishAi::new(ENEMY, PLAYER, choices)));
    }
    let setup_hash = setup_fingerprint(
        &sim,
        &buildables,
        &kinds,
        &rules.globals.prerequisite_groups,
    )?;
    let mut radar = crate::sidebar::radar::Radar::load(fs, map, rules);
    radar.reset(&sim, &kinds);
    Ok(Session {
        radar,
        services: crate::service::Services::load(rules),
        sim,
        outcome: Outcome::Ongoing,
        had_enemy,
        buildables,
        sidebar_art: crate::sidebar::SidebarArt::load(fs),
        cursor_art: crate::cursor::Art::load(fs),
        kinds,
        prerequisite_groups: rules.globals.prerequisite_groups.clone(),
        seed,
        setup_hash,
    })
}

fn setup_fingerprint(
    sim: &Sim,
    buildables: &[Buildable],
    kinds: &[String],
    groups: &[(String, Vec<String>)],
) -> Result<u64, String> {
    let description =
        serde_json::to_vec(&(buildables, kinds, groups)).map_err(|e| e.to_string())?;
    Ok(crate::snapshot::fingerprint(
        sim.setup_fingerprint_bytes()
            .map_err(|e| e.to_string())?
            .iter()
            .chain(description.iter())
            .copied(),
    ))
}

/// Build time in ticks: `Cost * BuildSpeed` minutes per 1000 credits at
/// 15 ticks/s (900 ticks per minute); exact integer maths on the decimal.
pub fn build_ticks(cost: i64, build_speed_units: i64) -> u32 {
    let t = cost.max(0) * build_speed_units * 900 / (1000 * 1_000_000);
    t.clamp(1, i64::from(u32::MAX)) as u32
}

/// Sidebar slots and the session kind table they start. Units are the
/// sandbox picks above; structures are every `[BuildingTypes]` entry the
/// player's side can build: `Prerequisite=` names the side's construction
/// yard (how YR gates side ownership), `TechLevel=` within the skirmish
/// level, open to [`PLAYER_COUNTRY`], with a cost and a `Foundation=`.
/// Walls, water-bound shipyards and the yard itself (deployed from an
/// MCV) are left out. Slot `i` is kind tag `i`; free units (refinery
/// harvesters) are interned after the slots.
fn buildables(fs: &AssetFs, rules: &Rules, yard: Option<&str>) -> (Vec<Buildable>, Vec<String>) {
    let names = crate::localization::SidebarNames::load(fs);
    let speed = rules.globals.build_speed.map_or(700_000, |d| d.units());
    let tech_level = rules.skirmish.tech_level.unwrap_or(10);
    let palette = ["CAMEO.PAL", "CAMEOMD.PAL"]
        .iter()
        .find_map(|n| fs.read(n).and_then(yr_formats::Palette::parse));
    let mut out = Vec::new();
    let mut free_units: Vec<(usize, String)> = Vec::new();
    let units = VEHICLE_SLOTS
        .iter()
        .map(|id| (id.to_string(), Factory::Vehicle))
        .chain(
            INFANTRY_SLOTS
                .iter()
                .map(|id| (id.to_string(), Factory::Infantry)),
        );
    let structures = rules
        .buildings
        .iter()
        .filter(|t| {
            let gated =
                yard.is_some_and(|y| t.prerequisite.iter().any(|p| p.eq_ignore_ascii_case(y)));
            gated
                && t.tech_level.is_some_and(|l| (1..=tech_level).contains(&l))
                && t.wall != Some(true)
                && t.water_bound != Some(true)
                && t.construction_yard != Some(true)
        })
        .map(|t| (t.id.clone(), Factory::Building));
    for (id, factory) in units.chain(structures) {
        let Some(t) = rules.techno(&id) else { continue };
        let Some(cost) = t.cost else { continue };
        if !house_allows(t, PLAYER_COUNTRY) {
            continue;
        }
        let product = if factory == Factory::Building {
            let Some((w, h)) = t.art.as_ref().and_then(|a| a.foundation) else {
                continue;
            };
            if let Some(free) = &t.free_unit {
                free_units.push((out.len(), free.clone()));
            }
            Product::Building(BuildingSpec {
                w: u16::from(w),
                h: u16::from(h),
                hp: t.strength.unwrap_or(500),
                armor: armor_idx(t),
                power: t.power.unwrap_or(0),
                role: building_role(t),
                adjacent: t.adjacent.unwrap_or(2).clamp(0, 255) as u8,
                base_normal: t.base_normal.unwrap_or(true),
                free_unit: None,
            })
        } else {
            Product::Unit(unit_spec(rules, t))
        };
        let cameo = t
            .art
            .as_ref()
            .and_then(|a| a.cameo.as_deref())
            .zip(palette.as_ref())
            .and_then(|(c, pal)| decode_cameo(fs, c, pal));
        out.push(Buildable {
            id: id.clone(),
            name: names.name(t),
            category: BuildCategory::from_techno(t),
            factory,
            cost,
            ticks: build_ticks(cost, speed),
            cameo,
            prerequisite: t.prerequisite.clone(),
            product,
        });
    }
    let mut kinds: Vec<String> = out.iter().map(|b| b.id.clone()).collect();
    for (slot, id) in free_units {
        let Some(t) = rules.techno(&id) else { continue };
        let tag = intern(&mut kinds, &id);
        if let Product::Building(spec) = &mut out[slot].product {
            spec.free_unit = Some((tag, unit_spec(rules, t)));
        }
    }
    (out, kinds)
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
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Sidebar emphasis only. Payment remains gradual, so a dim slot
    /// can still be queued with insufficient credits. Paid structures
    /// waiting for placement remain bright even if their factory is lost.
    pub fn slot_available(&self, slot: usize) -> bool {
        let Some(b) = self.buildables.get(slot) else {
            return false;
        };
        self.ready_structure() == Some(slot)
            || (self.sim.credits() >= b.cost
                && self.prerequisites_met(slot)
                && self
                    .sim
                    .buildings()
                    .iter()
                    .any(|h| h.owner == PLAYER && h.role == b.factory.role()))
    }

    /// Queue sidebar slot `slot`. `None` when out of range, when its
    /// prerequisites are missing, or for a structure while another one is
    /// building or waiting to be placed (one at a time, as in RA2).
    pub fn produce(&self, slot: usize) -> Option<Command> {
        let b = self.buildables.get(slot)?;
        if !self.prerequisites_met(slot) {
            return None;
        }
        let busy = self
            .sim
            .production()
            .iter()
            .any(|o| o.factory == Factory::Building);
        if b.factory == Factory::Building && busy {
            return None;
        }
        Some(Command::Produce {
            factory: b.factory,
            kind: u16::try_from(slot).ok()?,
            product: b.product,
            cost: b.cost,
            ticks: b.ticks,
        })
    }

    /// Whether the player owns what slot `slot`'s `Prerequisite=` asks
    /// for: each token is a type id or a generic group (`POWER` is any
    /// of `PrerequisitePower=`), matched against the player's buildings.
    pub fn prerequisites_met(&self, slot: usize) -> bool {
        let Some(b) = self.buildables.get(slot) else {
            return false;
        };
        let owned: Vec<&str> = self
            .sim
            .buildings()
            .iter()
            .filter(|h| h.owner == PLAYER)
            .filter_map(|h| self.kinds.get(usize::from(h.kind)).map(String::as_str))
            .collect();
        b.prerequisite.iter().all(|token| {
            let group = self
                .prerequisite_groups
                .iter()
                .find(|(t, _)| t.eq_ignore_ascii_case(token))
                .map(|(_, ids)| ids.as_slice());
            owned.iter().any(|id| match group {
                Some(ids) => ids.iter().any(|g| g.eq_ignore_ascii_case(id)),
                None => token.eq_ignore_ascii_case(id),
            })
        })
    }

    /// Slot of the finished structure waiting to be placed, if any.
    pub fn ready_structure(&self) -> Option<usize> {
        self.sim
            .production()
            .iter()
            .find(|o| o.factory == Factory::Building)
            .filter(|o| o.done())
            .map(|o| usize::from(o.kind))
    }

    /// Top-left footprint cell for slot `slot` with the cursor over map
    /// cell `cell`: the footprint centres on the cursor (the cursor cell
    /// is the top-left one for 1- and 2-cell sides).
    pub fn placement_origin(&self, slot: usize, cell: (i32, i32)) -> Option<(u16, u16)> {
        let (w, h) = self.buildables.get(slot)?.footprint()?;
        let x = cell.0 - (i32::from(w) - 1) / 2;
        let y = cell.1 - (i32::from(h) - 1) / 2;
        Some((u16::try_from(x).ok()?, u16::try_from(y).ok()?))
    }

    /// Whether slot `slot`'s structure may stand with its top-left cell at
    /// `origin` (the sim's own placement rules, for the ghost preview).
    pub fn can_place(&self, slot: usize, origin: (u16, u16)) -> bool {
        match self.buildables.get(slot).map(|b| b.product) {
            Some(Product::Building(spec)) => self.sim.can_place(PLAYER, &spec, origin.0, origin.1),
            _ => false,
        }
    }

    /// Place the finished structure of slot `slot` at `origin`; `None`
    /// unless it is the ready one and the spot is valid.
    pub fn place(&self, slot: usize, origin: (u16, u16)) -> Option<Command> {
        if self.ready_structure() != Some(slot) || !self.can_place(slot, origin) {
            return None;
        }
        Some(Command::PlaceProduced {
            kind: u16::try_from(slot).ok()?,
            x: origin.0,
            y: origin.1,
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
        self.sim.step(commands);
        self.radar.update(&self.sim, &self.kinds);
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
}

#[cfg(test)]
mod save_tests {
    use super::*;

    fn fresh(name: &str, cost: i64, cameo: Option<(u32, u32, Vec<u8>)>) -> Session {
        let sim = Sim::new(42);
        let buildables = vec![Buildable {
            id: "E1".into(),
            name: name.into(),
            category: BuildCategory::Infantry,
            factory: Factory::Infantry,
            cost,
            ticks: 10,
            cameo,
            prerequisite: Vec::new(),
            product: Product::Unit(UnitSpec {
                loco: Loco::Foot,
                speed: Fix::ONE,
                hp: 100,
                armor: 0,
                weapon: None,
                role: Role::Combat,
            }),
        }];
        let kinds = vec!["E1".into()];
        let setup_hash = setup_fingerprint(&sim, &buildables, &kinds, &[]).unwrap();
        Session {
            sim,
            outcome: Outcome::Ongoing,
            had_enemy: false,
            buildables,
            sidebar_art: Default::default(),
            radar: Default::default(),
            cursor_art: Default::default(),
            services: Default::default(),
            kinds,
            prerequisite_groups: Vec::new(),
            seed: 42,
            setup_hash,
        }
    }

    #[test]
    fn localized_labels_and_new_cameos_load_but_changed_gameplay_rejects() {
        let original = fresh("GI", 200, None);
        let bytes = crate::snapshot::encode(&original).unwrap();
        let mut localized = fresh("Localized infantry", 200, Some((1, 1, vec![255; 4])));
        localized.buildables[0].category = BuildCategory::Vehicles;
        localized.setup_hash =
            setup_fingerprint(&localized.sim, &localized.buildables, &localized.kinds, &[])
                .unwrap();
        crate::snapshot::restore(&mut localized, &bytes)
            .expect("presentation changes are compatible");
        assert_eq!(original.sim.state_hash(), localized.sim.state_hash());
        assert_eq!(localized.buildables[0].name, "Localized infantry");
        let mut rebalanced = fresh("GI", 250, None);
        assert!(crate::snapshot::restore(&mut rebalanced, &bytes)
            .unwrap_err()
            .contains("map or rules"));
    }

    #[test]
    fn building_tab_uses_buildcat_instead_of_weapon_or_power() {
        let ini = yr_formats::IniFile::parse("[BuildingTypes]\n0=TOWER\n1=POWER\n2=UNKNOWN\n[TOWER]\nBuildCat=cOmBaT\n[POWER]\nBuildCat=Power\n[UNKNOWN]\nPrimary=Gun\n");
        let rules = Rules::load(&ini, &yr_formats::IniFile::default());
        assert_eq!(
            BuildCategory::from_techno(rules.techno("TOWER").unwrap()),
            BuildCategory::Defences
        );
        assert_eq!(
            BuildCategory::from_techno(rules.techno("POWER").unwrap()),
            BuildCategory::Structures
        );
        assert_eq!(
            BuildCategory::from_techno(rules.techno("UNKNOWN").unwrap()),
            BuildCategory::Structures
        );
    }
}
