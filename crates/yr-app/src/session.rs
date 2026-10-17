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
use yr_sim::{Command, Fix, Role, Sim};

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
    extra: Option<&'a str>,
}

struct PlaceCtx<'a> {
    next_id: &'a mut u32,
    taken_cells: &'a mut Vec<(u16, u16)>,
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
    if let Some(e) = spec.extra {
        placed.push((e.to_string(), Role::Combat));
    }
    for (id, role) in placed {
        let Some(t) = rules.techno(&id) else { continue };
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
    if let (Some(y), Some(p), Some(r)) = (&yard, &plant, &refinery) {
        let mut ctx = PlaceCtx {
            next_id: &mut next_id,
            taken_cells: &mut taken_cells,
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
                extra: None,
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
        let uid = place_unit(&mut next_id, rules, id, cell, PLAYER, &mut extra);
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
            let weap = first_of(rules, &["GAWEAP", "NAWEAP", "YAWEAP"]);
            let mut ctx = PlaceCtx {
                next_id: &mut next_id,
                taken_cells: &mut taken_cells,
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
                    extra: weap.as_deref(),
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
                place_unit(&mut next_id, rules, t, cell, ENEMY, &mut extra);
                cmds.extend(extra);
            }
        }
    }
    sim.step(&cmds);
    Ok(Session {
        sim,
        outcome: Outcome::Ongoing,
        had_enemy,
    })
}

impl Session {
    /// Advance the match and re-judge win/lose on owned entities.
    pub fn step(&mut self, commands: &[Command]) {
        self.sim.step(commands);
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
