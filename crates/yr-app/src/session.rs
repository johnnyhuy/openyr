//! Skirmish session setup (issue #14): turn a parsed retail map into a
//! running deterministic sim.
//!
//! Contents: the sim terrain grid for the map, the map's own pre-placed
//! structures/units as sim entities, a player starting force at the
//! first start waypoint, a computer base at the second, and the
//! selected starting credits. Each participant uses their selected side's
//! starting composition. Legacy saves retain three computer guards. Short Game
//! judges surviving structures; other matches also count surviving units.

use crate::match_config::{Country, MatchConfig, Side, COLOR_NAMES};
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

/// Explicit front-menu overrides, outside the deterministic simulation payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartOptions {
    /// New sessions share the count; envelope-1/2 reconstruction keeps old guards.
    pub shared_forces: bool,
    pub credits: Option<i64>,
    pub units: Option<u16>,
}
impl Default for StartOptions {
    fn default() -> Self {
        Self {
            shared_forces: true,
            credits: None,
            units: None,
        }
    }
}
impl StartOptions {
    pub fn validate(self) -> Result<(), String> {
        if (!self.shared_forces && (self.credits.is_some() || self.units.is_some()))
            || self
                .credits
                .is_some_and(|v| !(0..=1_000_000_000).contains(&v))
            || self.units.is_some_and(|v| v > 4096)
        {
            return Err("Starting credits or unit count exceeds supported bounds".into());
        }
        Ok(())
    }
}

pub struct Session {
    pub match_config: MatchConfig,
    pub owner_colors: [[u8; 3]; 3],
    color_palette: [[u8; 3]; 8],
    pub start_options: StartOptions,
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
    /// Named installed map for menu loads; absent for unlabelled setup or paths.
    pub(crate) source_map: Option<String>,
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

/// Validate the selected country's generated force.
/// Map-authored objects do not pass through this generated-force filter.
fn country_type<'a>(
    rules: &'a Rules,
    id: &str,
    kind: TechnoKind,
    country: Country,
) -> Result<&'a yr_rules::TechnoType, String> {
    let country = country.id();
    let t = rules
        .techno(id)
        .filter(|t| t.defined && t.kind == kind)
        .ok_or_else(|| format!("Skirmish setup requires the defined {kind:?} type {id}"))?;
    if !t.owner.iter().any(|h| h.eq_ignore_ascii_case(country)) || !house_allows(t, country) {
        return Err(format!("Skirmish setup requires {id} to allow {country}"));
    }
    Ok(t)
}

fn validate_country_setup(
    fs: &AssetFs,
    rules: &Rules,
    theater: Theater,
    country: Country,
) -> Result<(), String> {
    let side = country.side();
    for id in side.buildings() {
        let t = country_type(rules, id, TechnoKind::Building, country)?;
        let name = yr_render::building_shp_candidates(&t.image.to_ascii_uppercase(), theater)
            .into_iter()
            .find(|name| fs.read(name).is_some())
            .ok_or_else(|| format!("Skirmish setup requires {id} building art"))?;
        if !fs
            .read(&name)
            .and_then(|data| yr_formats::ShpFile::parse(data).ok())
            .is_some_and(|s| !s.frames.is_empty())
        {
            return Err(format!("Skirmish setup cannot decode {id} art {name}"));
        }
    }
    for id in side.force() {
        let t = country_type(
            rules,
            id,
            if id == side.force()[3] {
                TechnoKind::Infantry
            } else {
                TechnoKind::Vehicle
            },
            country,
        )?;
        let image = t.image.to_ascii_uppercase();
        let voxel = t.art.as_ref().is_some_and(|a| a.voxel);
        let name = format!("{image}.{}", if voxel { "VXL" } else { "SHP" });
        let data = fs
            .read(&name)
            .ok_or_else(|| format!("Skirmish setup requires {id} art {name}"))?;
        let valid = if voxel {
            yr_formats::VxlFile::parse(data).is_ok()
        } else {
            yr_formats::ShpFile::parse(data).is_ok_and(|s| !s.frames.is_empty())
        };
        if !valid {
            return Err(format!("Skirmish setup cannot decode {id} art {name}"));
        }
        if id == side.force()[1] && t.harvester != Some(true) {
            return Err(format!("Skirmish setup requires {id} Harvester=yes"));
        }
    }
    Ok(())
}

/// Retain one MCV and miner, then repeat four tanks and four infantry
/// when a map requests more than the retail ten-unit force.
fn starting_kind_for(side: Side, index: usize) -> &'static str {
    if side == Side::Allied {
        return starting_kind(index);
    }
    let force = side.force();
    match index {
        0 => force[0],
        1 => force[1],
        i if (i - 2) % 8 < 4 => force[2],
        _ => force[3],
    }
}

fn starting_kind(index: usize) -> &'static str {
    match index {
        0 => "AMCV",
        1 => "CMIN",
        i if (i - 2) % 8 < 4 => "MTNK",
        _ => "E1",
    }
}

fn starting_count(rules: &Rules, grid: &TerrainGrid) -> Result<usize, String> {
    let count = rules.skirmish.unit_count.unwrap_or(10);
    let cells = grid.width() * grid.height();
    usize::try_from(count)
        .ok()
        .filter(|&n| n <= cells)
        .ok_or_else(|| format!("Skirmish setup UnitCount={count} is outside this map's capacity"))
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
    load_with_options(gamedir, map_name, seed, StartOptions::default())
}

pub fn load_with_options(
    gamedir: &str,
    map_name: &str,
    seed: u64,
    options: StartOptions,
) -> Result<Session, String> {
    load_with_config(gamedir, map_name, seed, options, MatchConfig::default())
}

pub fn load_with_config(
    gamedir: &str,
    map_name: &str,
    seed: u64,
    options: StartOptions,
    config: MatchConfig,
) -> Result<Session, String> {
    options.validate()?;
    config.validate()?;
    if !options.shared_forces && config != MatchConfig::default() {
        return Err("Legacy starting forces cannot use participant overrides".into());
    }
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
    let mut rules = Rules::load(&rules_ini, &art_ini);
    if let Some(credits) = options.credits {
        rules.skirmish.money = Some(credits);
    }
    if let Some(units) = options.units {
        rules.skirmish.unit_count = Some(i64::from(units));
    }
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    let mut session = setup_profile(
        &mut fs,
        &map,
        &rules,
        &overlay_types,
        seed,
        options.shared_forces,
        &config,
    )?;
    session.start_options = options;
    if crate::snapshot::valid_map_reference(map_name) && fs.read(map_name).is_some() {
        session.source_map = Some(map_name.to_owned());
    }
    Ok(session)
}

/// Build a session for `map` on the opened install.
pub fn setup(
    fs: &mut AssetFs,
    map: &MapFile,
    rules: &Rules,
    overlay_types: &[String],
    seed: u64,
) -> Result<Session, String> {
    setup_profile(
        fs,
        map,
        rules,
        overlay_types,
        seed,
        true,
        &MatchConfig::default(),
    )
}

fn setup_profile(
    fs: &mut AssetFs,
    map: &MapFile,
    rules: &Rules,
    overlay_types: &[String],
    seed: u64,
    shared_forces: bool,
    config: &MatchConfig,
) -> Result<Session, String> {
    let theater = Theater::from_map_name(&map.theater).ok_or("unknown theater")?;
    fs.set_theater(theater);
    config.validate()?;
    for player in &config.players {
        validate_country_setup(fs, rules, theater, player.country)?;
    }
    let grid = yr_render::build_terrain(fs, map, overlay_types, theater)?;
    let target_count = starting_count(rules, &grid)?;
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

    let player_side = config.players[0].country.side();
    let enemy_side = config.players[1].country.side();
    let [yard, power_plant, refinery, war_factory, barracks] = player_side.buildings();
    let enemy_base = enemy_side.buildings();
    let (buildables, mut kinds) =
        catalog_buildables(fs, rules, Some(yard), config.players[0].country);
    let (enemy_buildables, enemy_kinds) =
        catalog_buildables(fs, rules, Some(enemy_base[0]), config.players[1].country);
    for id in &enemy_kinds {
        intern(&mut kinds, id);
    }
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
        cmds.push(Command::PlaceMapBuilding {
            kind: intern(&mut kinds, &obj.kind),
            x: obj.x,
            y: obj.y,
            spec: BuildingSpec {
                w: fw,
                h: fh,
                hp: t.strength.unwrap_or(500),
                armor: armor_idx(t),
                power: t.power.unwrap_or(0),
                role: Role::Combat,
                adjacent: 0,
                base_normal: true,
                free_unit: None,
            },
            health: obj.health,
            owner: NEUTRAL,
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
    let authored = crate::frontend::start_points(&map.ini);
    let chosen =
        |owner: usize, fallback: Option<(u16, u16)>| -> Result<Option<(u16, u16)>, String> {
            match config.players[owner].start {
                Some(index) => authored[usize::from(index)].map(Some).ok_or_else(|| {
                    "The selected starting position is missing from this map".into()
                }),
                None => Ok(fallback),
            }
        };
    let enemy_explicit = config.players[1]
        .start
        .and_then(|i| authored[usize::from(i)]);
    let player_default = if enemy_explicit.is_some() {
        waypoints
            .iter()
            .copied()
            .find(|p| Some(*p) != enemy_explicit)
    } else {
        waypoints.first().copied()
    };
    let player_at = chosen(0, player_default)?.unwrap_or((10, 10));
    let explicit = config.players.iter().any(|p| p.start.is_some());
    let enemy_default = if explicit {
        waypoints.iter().copied().find(|p| *p != player_at)
    } else {
        waypoints.get(1).copied()
    };
    let enemy_at = chosen(1, enemy_default)?;
    if explicit && enemy_at == Some(player_at) {
        return Err("Choose different starting positions".into());
    }
    {
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
                yard,
                power_plant,
                refinery,
                extra: &[war_factory, barracks],
            },
            &mut cmds,
        );
    }
    for index in 0..target_count {
        let id = starting_kind_for(player_side, index);
        let loco = rules
            .techno(id)
            .map(|t| techno_loco(t.kind))
            .unwrap_or(Loco::Track);
        let Some(cell) = free_cell(&grid, loco, player_at, &sim, &taken_cells) else {
            return Err(format!(
                "Skirmish setup cannot place all {target_count} starting units near waypoint 0"
            ));
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
        if let Some(uid) = uid.filter(|_| id == player_side.force()[1]) {
            cmds.push(Command::AssignRole {
                id: uid,
                role: Role::Harvester,
            });
        }
    }
    // Computer base and guards at the second waypoint.
    let had_enemy = enemy_at.is_some();
    if let Some(enemy_at) = enemy_at {
        {
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
                    yard: enemy_base[0],
                    power_plant: enemy_base[1],
                    refinery: enemy_base[2],
                    extra: &enemy_base[3..],
                },
                &mut cmds,
            );
        }
        {
            let count = if shared_forces { target_count } else { 3 };
            for index in 0..count {
                let id = if shared_forces {
                    starting_kind_for(enemy_side, index)
                } else {
                    "MTNK"
                };
                let loco = rules
                    .techno(id)
                    .map(|t| techno_loco(t.kind))
                    .unwrap_or(Loco::Track);
                let Some(cell) = free_cell(&grid, loco, enemy_at, &sim, &taken_cells) else {
                    if !shared_forces {
                        break;
                    }
                    return Err(format!("Skirmish setup cannot place all {count} computer starting units near waypoint 1"));
                };
                taken_cells.push(cell);
                let mut extra = Vec::new();
                let uid = place_unit(&mut kinds, &mut next_id, rules, id, cell, ENEMY, &mut extra);
                cmds.extend(extra);
                if let Some(id) = uid.filter(|_| id == enemy_side.force()[1]) {
                    cmds.push(Command::AssignRole {
                        id,
                        role: Role::Harvester,
                    });
                }
            }
        }
    }
    sim.step(&cmds);
    if had_enemy {
        // Resolve the computer country's factory types and prerequisite groups
        // once; decisions live in yr-sim.
        let mut choices = Vec::new();
        for id in [
            enemy_side.force()[2],
            enemy_side.force()[3],
            enemy_base[1],
            enemy_base[2],
            enemy_base[3],
            enemy_base[4],
        ] {
            let Some(b) = enemy_buildables.iter().find(|b| b.id == id) else {
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
                kind: intern(&mut kinds, &b.id),
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
    let mut color_rules = yr_formats::IniFile::parse(&String::from_utf8_lossy(
        fs.read("RULESMD.INI").unwrap_or_default(),
    ));
    color_rules.merge_from(&map.ini);
    let color_palette =
        std::array::from_fn(|i| yr_render::named_color(&color_rules, COLOR_NAMES[i]));
    let owner_colors = [
        color_palette[usize::from(config.players[0].color)],
        color_palette[usize::from(config.players[1].color)],
        [170; 3],
    ];
    radar.set_colors(owner_colors);
    Ok(Session {
        match_config: config.clone(),
        owner_colors,
        color_palette,
        start_options: StartOptions::default(),
        radar,
        services: crate::service::Services::load(rules),
        sim,
        outcome: Outcome::Ongoing,
        had_enemy,
        buildables,
        sidebar_art: crate::sidebar::SidebarArt::load_for_side(fs, player_side),
        cursor_art: crate::cursor::Art::load(fs),
        kinds,
        prerequisite_groups: rules.globals.prerequisite_groups.clone(),
        seed,
        setup_hash,
        source_map: None,
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
/// level, open to the selected country, with a cost and a `Foundation=`.
/// Walls, water-bound shipyards and the yard itself (deployed from an
/// MCV) are left out. Slot `i` is kind tag `i`; free units (refinery
/// harvesters) are interned after the slots.
fn catalog_buildables(
    fs: &AssetFs,
    rules: &Rules,
    yard: Option<&str>,
    country: Country,
) -> (Vec<Buildable>, Vec<String>) {
    let side = country.side();
    let country = country.id();
    let names = crate::localization::SidebarNames::load(fs);
    let speed = rules.globals.build_speed.map_or(700_000, |d| d.units());
    let tech_level = rules.skirmish.tech_level.unwrap_or(10);
    let palette = ["CAMEO.PAL", "CAMEOMD.PAL"]
        .iter()
        .find_map(|n| fs.read(n).and_then(yr_formats::Palette::parse));
    let mut out = Vec::new();
    let mut free_units: Vec<(usize, String)> = Vec::new();
    let units = side
        .vehicles()
        .iter()
        .map(|id| (id.to_string(), Factory::Vehicle))
        .chain(
            side.infantry()
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
        if !house_allows(t, country) {
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
    pub fn apply_match_config(&mut self, config: MatchConfig) -> Result<(), String> {
        config.validate()?;
        self.owner_colors[0] = self.color_palette[usize::from(config.players[0].color)];
        self.owner_colors[1] = self.color_palette[usize::from(config.players[1].color)];
        self.radar.set_colors(self.owner_colors);
        self.match_config = config;
        Ok(())
    }
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

    /// The current factory head for this slot, if any.
    pub fn slot_head(&self, slot: usize) -> Option<&yr_sim::BuildOrder> {
        let b = self.buildables.get(slot)?;
        self.sim
            .production()
            .iter()
            .find(|o| o.factory == b.factory)
            .filter(|o| usize::from(o.kind) == slot)
    }

    /// Manual production controls: left resumes held work before queuing;
    /// right suspends the unfinished head, then cancels on a second click.
    /// Waiting types and ready structures can be cancelled immediately.
    pub fn production_click(
        &self,
        slot: usize,
        right: bool,
        pending: &[Command],
    ) -> Option<Command> {
        let b = self.buildables.get(slot)?;
        let queue = self.sim.production_with_pending(pending);
        if let Some(head) = queue
            .iter()
            .find(|o| o.factory == b.factory)
            .filter(|o| usize::from(o.kind) == slot && !o.done())
        {
            if head.held != right {
                return Some(Command::HoldProduce {
                    factory: head.factory,
                    kind: head.kind,
                    held: right,
                });
            }
        }
        if right {
            self.cancel(slot)
        } else {
            if b.factory == Factory::Building && queue.iter().any(|o| o.factory == b.factory) {
                return None;
            }
            // The pending preview already checked construction occupancy.
            if !self.prerequisites_met(slot) {
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
    }

    /// Cancel a held head, otherwise the newest matching queued build.
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
        let player_alive = (!self.match_config.short_game
            && self.sim.units().iter().any(|u| u.owner == PLAYER))
            || self.sim.buildings().iter().any(|b| b.owner == PLAYER);
        let enemy_alive = (!self.match_config.short_game
            && self.sim.units().iter().any(|u| u.owner == ENEMY))
            || self.sim.buildings().iter().any(|b| b.owner == ENEMY);
        if !player_alive {
            self.outcome = Outcome::Lost;
        } else if self.had_enemy && !enemy_alive {
            self.outcome = Outcome::Won;
        }
    }
}

#[cfg(test)]
mod starting_force_tests {
    use super::*;
    use yr_formats::IniFile;

    #[test]
    fn generated_country_checks_owner_required_forbidden_and_kind() {
        let make = |fields: &str| {
            Rules::load(
                &IniFile::parse(&format!("[VehicleTypes]\n0=AMCV\n[AMCV]\n{fields}")),
                &IniFile::parse(""),
            )
        };
        for fields in [
            "Owner=Russians",
            "Owner=Americans\nRequiredHouses=British",
            "Owner=Americans\nForbiddenHouses=Americans",
            "",
        ] {
            assert!(
                country_type(&make(fields), "AMCV", TechnoKind::Vehicle, Country::America).is_err(),
                "{fields}"
            );
        }
        let allowed = make("Owner=americans\nRequiredHouses=AMERICANS\nForbiddenHouses=British");
        assert!(country_type(&allowed, "AMCV", TechnoKind::Vehicle, Country::America).is_ok());
        assert!(country_type(&allowed, "AMCV", TechnoKind::Infantry, Country::America).is_err());
        assert!(country_type(&allowed, "SMCV", TechnoKind::Vehicle, Country::America).is_err());
        let undefined = Rules::load(
            &IniFile::parse("[VehicleTypes]\n0=AMCV"),
            &IniFile::parse(""),
        );
        assert!(country_type(&undefined, "AMCV", TechnoKind::Vehicle, Country::America).is_err());
    }

    #[test]
    fn configured_force_counts_include_zero_and_more_than_ten() {
        let grid = TerrainGrid::new(0, 0, 10, 10);
        for count in [0, 1, 2, 6, 10, 11, 18, 100] {
            let mut rules = Rules::load(&IniFile::parse(""), &IniFile::parse(""));
            rules.skirmish.unit_count = Some(count);
            let count = starting_count(&rules, &grid).unwrap();
            let force: Vec<_> = (0..count).map(starting_kind).collect();
            assert_eq!(force.len(), count);
            assert_eq!(
                force.iter().filter(|&&id| id == "AMCV").count(),
                usize::from(count > 0)
            );
            assert_eq!(
                force.iter().filter(|&&id| id == "CMIN").count(),
                usize::from(count > 1)
            );
            if count == 10 {
                assert_eq!(
                    force,
                    ["AMCV", "CMIN", "MTNK", "MTNK", "MTNK", "MTNK", "E1", "E1", "E1", "E1"]
                );
            }
        }
        for count in [-1, 101, i64::MAX] {
            let mut rules = Rules::load(&IniFile::parse(""), &IniFile::parse(""));
            rules.skirmish.unit_count = Some(count);
            assert!(starting_count(&rules, &grid).is_err());
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
            match_config: MatchConfig::default(),
            owner_colors: [[170; 3]; 3],
            color_palette: [[170; 3]; 8],
            start_options: StartOptions::default(),
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
            source_map: None,
        }
    }

    #[test]
    fn map_health_setup_change_rejects_affected_matches_without_rewriting() {
        let make = |corrected, health: u16| {
            let mut session = fresh("GI", 200, None);
            let hp = 100 * i64::from(health.max(1)) / 256;
            let spec = BuildingSpec {
                w: 1,
                h: 1,
                hp: 100,
                armor: 0,
                power: 0,
                role: Role::Combat,
                adjacent: 0,
                base_normal: true,
                free_unit: None,
            };
            session.sim.step(&[if corrected {
                Command::PlaceMapBuilding {
                    kind: 0,
                    x: 2,
                    y: 3,
                    spec,
                    health,
                    owner: NEUTRAL,
                }
            } else {
                Command::PlaceBuilding {
                    kind: 0,
                    x: 2,
                    y: 3,
                    w: 1,
                    h: 1,
                    hp,
                    armor: 0,
                    power: 0,
                    owner: NEUTRAL,
                    role: Role::Combat,
                }
            }]);
            session.setup_hash =
                setup_fingerprint(&session.sim, &session.buildables, &session.kinds, &[]).unwrap();
            session
        };
        let prior_full = crate::snapshot::encode(&make(false, 256u16)).unwrap();
        let mut current_full = make(true, 256);
        crate::snapshot::restore(&mut current_full, &prior_full).unwrap();
        assert_eq!(crate::snapshot::encode(&current_full).unwrap(), prior_full);

        let prior_half = make(false, 128);
        let bytes = crate::snapshot::encode(&prior_half).unwrap();
        let mut current_half = make(true, 128);
        let before = crate::snapshot::encode(&current_half).unwrap();
        assert!(crate::snapshot::restore(&mut current_half, &bytes)
            .unwrap_err()
            .contains("map, rules or engine setup"));
        assert_eq!(crate::snapshot::encode(&current_half).unwrap(), before);
        // Raw snapshots retain exactly what was saved, including the old maximum.
        let old_sim = prior_half.sim.save_bytes().unwrap();
        let restored = Sim::from_save_bytes(&old_sim).unwrap();
        assert_eq!(
            (restored.buildings()[0].hp, restored.buildings()[0].max_hp),
            (50, 50)
        );
        assert_eq!(restored.save_bytes().unwrap(), old_sim);
        println!("Map-health compatibility: full-health old match accepted byte-for-byte; affected old match rejected without replacement; raw old sim retains 50/50 without migration.");
    }

    #[test]
    fn saved_map_metadata_preserves_legacy_payload_and_gameplay_hashes() {
        let mut original = fresh("GI", 200, None);
        original.start_options.shared_forces = false;
        let legacy = crate::snapshot::encode(&original).unwrap();
        let state_hash = original.sim.state_hash();
        let setup_hash = original.setup_hash;
        original.source_map = Some("XMP29U2.MAP".into());
        let named = crate::snapshot::encode(&original).unwrap();
        assert_eq!(original.sim.state_hash(), state_hash);
        assert_eq!(original.setup_hash, setup_hash);
        assert_eq!(&legacy[12..37], &named[12..37]);
        assert_eq!(&legacy[37..], &named[39 + "XMP29U2.MAP".len()..]);
        let mut restored = fresh("GI", 200, None);
        crate::snapshot::restore(&mut restored, &legacy).unwrap();
        assert_eq!(crate::snapshot::encode(&restored).unwrap(), legacy);
        crate::snapshot::restore(&mut restored, &named).unwrap();
        assert_eq!(crate::snapshot::encode(&restored).unwrap(), named);
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
            .contains("map, rules or engine setup"));
    }

    #[test]
    fn sparse_terrain_changes_setup_keys_and_rejects_old_rectangular_matches() {
        let make = |sparse| {
            let mut session = fresh("GI", 200, None);
            session.sim.set_terrain(if sparse {
                TerrainGrid::from_tiles(0, 0, 2, 2, [(1, 1, 0, 13, 0)].into_iter())
            } else {
                TerrainGrid::new(0, 0, 2, 2)
            });
            session.setup_hash =
                setup_fingerprint(&session.sim, &session.buildables, &session.kinds, &[]).unwrap();
            session
        };
        let permissive = make(false);
        let mut current = make(true);
        assert_ne!(current.setup_hash, permissive.setup_hash);
        let before = current.sim.state_hash();
        assert!(crate::snapshot::restore(
            &mut current,
            &crate::snapshot::encode(&permissive).unwrap()
        )
        .is_err());
        assert_eq!(current.sim.state_hash(), before);
        let mut restored = make(true);
        crate::snapshot::restore(&mut restored, &crate::snapshot::encode(&current).unwrap())
            .unwrap();
        assert_eq!(restored.sim.state_hash(), before);
        assert!(restored.sim.terrain().unwrap().cell(0, 0).is_none());
    }

    #[test]
    fn setup_free_cells_skip_sparse_holes() {
        let grid = TerrainGrid::from_tiles(0, 0, 4, 4, [(2, 2, 0, 13, 0)].into_iter());
        assert_eq!(
            free_cell(&grid, Loco::Track, (1, 1), &Sim::new(42), &[]),
            Some((2, 2))
        );
        assert_eq!(
            free_cell(&grid, Loco::Fly, (1, 1), &Sim::new(42), &[(2, 2)]),
            None
        );
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
