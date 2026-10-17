//! Deterministic simulation core for OpenYR.
//!
//! Rules (docs/architecture.md): fixed-point maths only, no floats, no
//! `HashMap`/`HashSet` iteration in sim paths, fixed tick order, seeded
//! RNG carried in the state, and a state hash so identical command
//! streams provably produce identical games. No I/O, no wall clock.
//!
//! This is the M4 skeleton: units with positions and move targets on a
//! flat plane. Terrain, combat and economy arrive in later issues and
//! plug into the same tick phases.

pub mod building;
pub mod combat;
mod fix;
mod hash;
pub mod path;
mod rng;
pub mod terrain;

pub use fix::Fix;
pub use rng::Rng;
pub use building::Building;
pub use combat::WeaponSpec;
pub use terrain::TerrainGrid;

/// One cell in leptons: positions are in leptons (256 per cell), the
/// original game's sub-cell unit.
pub const LEPTONS_PER_CELL: i32 = 256;

/// Commands are the only way to mutate the sim from outside. Applied in
/// slice order at the start of a tick, so replays are command streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Spawn {
        x: Fix,
        y: Fix,
        loco: terrain::Loco,
        speed: Fix,
        hp: i64,
        armor: u8,
        weapon: Option<WeaponSpec>,
    },
    Move { id: u32, x: Fix, y: Fix },
    Attack { id: u32, target: u32 },
    /// Send a harvester into its auto-loop (ore -> refinery -> ore).
    Harvest { id: u32 },
    /// Mark a unit's economy role (scenario setup).
    AssignRole { id: u32, role: Role },
    /// Place a building: footprint cells, health, armour, power
    /// (positive produces, negative drains).
    PlaceBuilding { x: u16, y: u16, w: u16, h: u16, hp: i64, armor: u8, power: i64 },
    Stop { id: u32 },
    Remove { id: u32 },
}

/// Things that happened during a tick, for the renderer/audio to react
/// to. Deterministic, drained by the caller after each step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Fired { shooter: u32, target: u32 },
    Hit { target: u32, damage: i64 },
    Died { id: u32 },
    /// A harvester finished unloading; `total` is the sim credit pool.
    Unloaded { id: u32, value: i64, total: i64 },
}

/// What a unit is for; drives the economy state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Combat,
    Harvester,
    Refinery,
}

/// Credits per harvested bail (RA2 bail values).
pub const BAIL_ORE: i64 = 25;
pub const BAIL_GEM: i64 = 50;
/// Bails a harvester carries before returning (RA2 capacity).
pub const HARVESTER_CAPACITY: u8 = 20;
/// Ticks spent loading one bail / unloading the cargo.
pub const HARVEST_TICKS: u32 = 20;
pub const UNLOAD_TICKS: u32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Harvest {
    Idle,
    ToOre(u16, u16),
    Loading(u32),
    ToRefinery(u32),
    Unloading(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub id: u32,
    pub x: Fix,
    pub y: Fix,
    pub target: Option<(Fix, Fix)>,
    /// Leptons per tick at 100% terrain speed.
    pub speed: Fix,
    pub loco: terrain::Loco,
    /// 0 = north, clockwise in 32nds (renderer-facing direction).
    pub facing: u8,
    /// Remaining waypoints (map cells) from the last A* plan.
    pub path: std::collections::VecDeque<(u16, u16)>,
    pub hp: i64,
    pub max_hp: i64,
    /// Armour class index (`Armor::ALL` order).
    pub armor: u8,
    pub weapon: Option<WeaponSpec>,
    pub cooldown: u32,
    pub attack: Option<u32>,
    pub shots: u32,
    pub role: Role,
    /// Credits worth of ore/gems currently carried.
    pub cargo: i64,
    pub bails: u8,
    harvest: Harvest,
}

/// A homing projectile in flight (internal; observed via events).
#[derive(Debug, Clone, Copy)]
struct Projectile {
    x: Fix,
    y: Fix,
    owner: u32,
    target: u32,
    damage: i64,
    speed: Fix,
}

#[derive(Debug, Clone)]
pub struct Sim {
    tick: u64,
    rng: Rng,
    next_id: u32,
    /// Kept in id order; all iteration is over this Vec.
    units: Vec<Unit>,
    terrain: Option<TerrainGrid>,
    projectiles: Vec<Projectile>,
    events: Vec<Event>,
    credits: i64,
    buildings: Vec<Building>,
    /// (min, max) low-power production speed factors from the rules.
    low_power_speeds: (Fix, Fix),
}

impl Sim {
    pub fn new(seed: u64) -> Sim {
        Sim { tick: 0, rng: Rng::new(seed), next_id: 1, units: Vec::new(), terrain: None, projectiles: Vec::new(), events: Vec::new(), credits: 0, buildings: Vec::new(), low_power_speeds: (Fix::from_raw(128), Fix::ONE) }
    }

    /// Attach the scenario terrain (static; fingerprinted into the hash).
    pub fn set_terrain(&mut self, terrain: TerrainGrid) {
        self.terrain = Some(terrain);
    }

    pub fn terrain(&self) -> Option<&TerrainGrid> {
        self.terrain.as_ref()
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// Advance one tick: apply commands (in order), then move units (in
    /// id order). Phase order is part of the determinism contract.
    pub fn step(&mut self, commands: &[Command]) {
        for cmd in commands {
            match *cmd {
                Command::Spawn { x, y, loco, speed, hp, armor, weapon } => {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.units.push(Unit {
                        id,
                        x,
                        y,
                        target: None,
                        speed,
                        loco,
                        facing: 128,
                        path: std::collections::VecDeque::new(),
                        hp,
                        max_hp: hp,
                        armor,
                        weapon,
                        cooldown: 0,
                        attack: None,
                        shots: 0,
                        role: Role::Combat,
                        cargo: 0,
                        bails: 0,
                        harvest: Harvest::Idle,
                    });
                }
                Command::PlaceBuilding { x, y, w, h, hp, armor, power } => {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.buildings.push(Building { id, cell: (x, y), size: (w, h), hp, max_hp: hp, armor, power });
                    if let Some(g) = self.terrain.as_mut() {
                        g.set_building(x, y, w, h, true);
                    }
                }
                Command::AssignRole { id, role } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.role = role;
                    }
                }
                Command::Harvest { id } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        if u.role == Role::Harvester && u.harvest == Harvest::Idle {
                            u.harvest = Harvest::ToOre(u16::MAX, u16::MAX);
                        }
                    }
                }
                Command::Attack { id, target } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.attack = Some(target);
                    }
                }
                Command::Stop { id } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.attack = None;
                        u.target = None;
                        u.path.clear();
                    }
                }
                Command::Move { id, x, y } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.target = Some((x, y));
                    }
                }
                Command::Remove { id } => {
                    self.units.retain(|u| u.id != id);
                }
            }
        }
        self.harvest_phase();
        // Attackers close to range; in-range attackers hold position.
        for i in 0..self.units.len() {
            let Some(enemy) = self.units[i].attack else { continue };
            let (Some(w), Some(e)) = (self.units[i].weapon, self.target_pos(enemy)) else { continue };
            let (ex, ey) = (e.0, e.1);
            let (dx, dy) = (ex - self.units[i].x, ey - self.units[i].y);
            if dist2(dx, dy) <= i64::from(w.range.raw()) * i64::from(w.range.raw()) {
                self.units[i].path.clear();
                let u = &mut self.units[i];
                u.target = None;
                u.facing = octant(dx, dy);
            } else {
                self.units[i].target = Some((ex, ey));
            }
        }
        // Movement phase (id order). With terrain, units follow A*
        // waypoints at terrain-adjusted speed; without, direct steps.
        let terrain = self.terrain.clone();
        for i in 0..self.units.len() {
            let Some((tx, ty)) = self.units[i].target else { continue };
            match &terrain {
                Some(grid) => self.move_on_grid(i, grid, tx, ty),
                None => {
                    let u = &mut self.units[i];
                    let (dx, dy) = (tx - u.x, ty - u.y);
                    let step = u.speed;
                    u.x += dx.clamp(-step, step);
                    u.y += dy.clamp(-step, step);
                    if u.x == tx && u.y == ty {
                        u.target = None;
                    }
                }
            }
        }
        self.fire_phase();
        self.projectile_phase();
        self.tick += 1;
    }

    /// Harvester state machine (runs before movement): pick ore, drive,
    /// load bails, return to the first refinery, unload into credits.
    fn harvest_phase(&mut self) {
        let Some(mut grid) = self.terrain.clone() else { return };
        let mut grid_dirty = false;
        for i in 0..self.units.len() {
            if self.units[i].role != Role::Harvester {
                continue;
            }
            let cell = (
                self.units[i].x.to_int_floor().max(0) as u16,
                self.units[i].y.to_int_floor().max(0) as u16,
            );
            match self.units[i].harvest {
                Harvest::Idle => {}
                Harvest::ToOre(tx, ty) => {
                    let target = if (tx, ty) == (u16::MAX, u16::MAX) || grid.cell(tx, ty).is_none_or(|c| c.resource == 0) {
                        grid.nearest_resource(cell)
                    } else {
                        grid.cell(tx, ty).map(|c| (tx, ty, c.overlay))
                    };
                    match target {
                        Some((ox, oy, _)) => {
                            self.units[i].harvest = Harvest::ToOre(ox, oy);
                            if (ox, oy) == cell {
                                self.units[i].harvest = Harvest::Loading(HARVEST_TICKS);
                                self.units[i].target = None;
                                self.units[i].path.clear();
                            } else {
                                self.units[i].target = Some((Fix::from_int(i32::from(ox)) + Fix::from_raw(128), Fix::from_int(i32::from(oy)) + Fix::from_raw(128)));
                            }
                        }
                        None => {
                            // No ore left: unload whatever we carry.
                            if self.units[i].bails > 0 {
                                if let Some(r) = self.units.iter().find(|u| u.role == Role::Refinery) {
                                    self.units[i].harvest = Harvest::ToRefinery(r.id);
                                } else {
                                    self.units[i].harvest = Harvest::Idle;
                                }
                            } else {
                                self.units[i].harvest = Harvest::Idle;
                            }
                        }
                    }
                }
                Harvest::Loading(mut left) => {
                    if left > 0 {
                        left -= 1;
                        self.units[i].harvest = Harvest::Loading(left);
                    } else if let Some(kind) = grid.take_bail(cell.0, cell.1) {
                        grid_dirty = true;
                        self.units[i].bails += 1;
                        self.units[i].cargo += match kind {
                            terrain::Overlay::Gem => BAIL_GEM,
                            _ => BAIL_ORE,
                        };
                        if self.units[i].bails >= HARVESTER_CAPACITY {
                            if let Some(r) = self.units.iter().find(|u| u.role == Role::Refinery) {
                                self.units[i].harvest = Harvest::ToRefinery(r.id);
                            } else {
                                self.units[i].harvest = Harvest::Idle;
                            }
                        } else {
                            self.units[i].harvest = Harvest::Loading(HARVEST_TICKS);
                        }
                    } else {
                        self.units[i].harvest = Harvest::ToOre(u16::MAX, u16::MAX);
                    }
                }
                Harvest::ToRefinery(rid) => {
                    let Some(r) = self.units.iter().find(|u| u.id == rid) else {
                        self.units[i].harvest = Harvest::Idle;
                        continue;
                    };
                    let rcell = (r.x.to_int_floor().max(0) as u16, r.y.to_int_floor().max(0) as u16);
                    // Unload from an adjacent cell: the refinery's own
                    // cell is occupied by the refinery itself.
                    let adjacent = (i32::from(cell.0) - i32::from(rcell.0)).abs() <= 1
                        && (i32::from(cell.1) - i32::from(rcell.1)).abs() <= 1;
                    if adjacent {
                        self.units[i].harvest = Harvest::Unloading(UNLOAD_TICKS);
                        self.units[i].target = None;
                        self.units[i].path.clear();
                    } else {
                        self.units[i].target = Some((r.x, r.y));
                    }
                }
                Harvest::Unloading(mut left) => {
                    if left > 0 {
                        left -= 1;
                        self.units[i].harvest = Harvest::Unloading(left);
                    } else {
                        let value = self.units[i].cargo;
                        self.credits += value;
                        self.units[i].cargo = 0;
                        self.units[i].bails = 0;
                        self.events.push(Event::Unloaded { id: self.units[i].id, value, total: self.credits });
                        self.units[i].harvest = Harvest::ToOre(u16::MAX, u16::MAX);
                    }
                }
            }
        }
        if grid_dirty {
            self.terrain = Some(grid);
        }
    }

    fn fire_phase(&mut self) {
        let mut spawned: Vec<Projectile> = Vec::new();
        let mut instant: Vec<(u32, u32, i64)> = Vec::new();
        for i in 0..self.units.len() {
            if self.units[i].cooldown > 0 {
                self.units[i].cooldown -= 1;
            }
            let u = &self.units[i];
            let (Some(w), Some(enemy_id)) = (u.weapon, u.attack) else { continue };
            if u.cooldown > 0 {
                continue;
            }
            let Some(((ex, ey), earmor)) = self.target_pos(enemy_id).zip(self.target_armor(enemy_id)) else { continue };
            let (dx, dy) = (ex - u.x, ey - u.y);
            if dist2(dx, dy) > i64::from(w.range.raw()) * i64::from(w.range.raw()) {
                continue;
            }
            self.events.push(Event::Fired { shooter: u.id, target: enemy_id });
            let damage = w.damage_against(earmor);
            if w.projectile_speed == Fix::ZERO {
                instant.push((u.id, enemy_id, damage));
            } else {
                spawned.push(Projectile { x: u.x, y: u.y, owner: u.id, target: enemy_id, damage, speed: w.projectile_speed });
            }
            let u = &mut self.units[i];
            u.shots += 1;
            u.cooldown = w.rof_ticks;
        }
        self.projectiles.extend(spawned);
        for (_, target, damage) in instant {
            self.apply_damage(target, damage);
        }
        self.reap_dead();
    }

    fn projectile_phase(&mut self) {
        let mut hits: Vec<(u32, i64)> = Vec::new();
        let mut alive: Vec<Projectile> = Vec::new();
        for mut p in std::mem::take(&mut self.projectiles) {
            let Some((tx, ty)) = self.target_pos(p.target) else { continue };
            let (dx, dy) = (tx - p.x, ty - p.y);
            if dist2(dx, dy) <= i64::from(p.speed.raw()) * i64::from(p.speed.raw()) {
                hits.push((p.target, p.damage));
            } else {
                // Axis-clamped homing step (deterministic, no floats).
                p.x += dx.clamp(-p.speed, p.speed);
                p.y += dy.clamp(-p.speed, p.speed);
                alive.push(p);
            }
        }
        self.projectiles = alive;
        for (target, damage) in hits {
            self.apply_damage(target, damage);
        }
        self.reap_dead();
    }

    fn apply_damage(&mut self, target: u32, damage: i64) {
        if let Some(u) = self.units.iter_mut().find(|x| x.id == target) {
            if u.hp > 0 && damage > 0 {
                u.hp -= damage;
                self.events.push(Event::Hit { target, damage });
            }
            return;
        }
        if let Some(b) = self.buildings.iter_mut().find(|b| b.id == target) {
            if b.hp > 0 && damage > 0 {
                b.hp -= damage;
                self.events.push(Event::Hit { target, damage });
            }
        }
    }

    /// Aim point (leptons) for a unit or building id.
    pub fn target_pos(&self, id: u32) -> Option<(Fix, Fix)> {
        if let Some(u) = self.units.iter().find(|u| u.id == id) {
            return Some((u.x, u.y));
        }
        self.buildings.iter().find(|b| b.id == id).map(Building::centre)
    }

    fn target_armor(&self, id: u32) -> Option<u8> {
        if let Some(u) = self.units.iter().find(|u| u.id == id) {
            return Some(u.armor);
        }
        self.buildings.iter().find(|b| b.id == id).map(|b| b.armor)
    }

    pub fn buildings(&self) -> &[Building] {
        &self.buildings
    }

    /// (produced, drained) power totals.
    pub fn power(&self) -> (i64, i64) {
        let mut produced = 0;
        let mut drained = 0;
        for b in &self.buildings {
            if b.power >= 0 {
                produced += b.power;
            } else {
                drained -= b.power;
            }
        }
        (produced, drained)
    }

    pub fn low_power(&self) -> bool {
        let (produced, drained) = self.power();
        drained > produced
    }

    /// Configure the rules' low-power production speeds
    /// (`MinLowPowerProductionSpeed`, `MaxLowPowerProductionSpeed`).
    pub fn set_low_power_speeds(&mut self, min: Fix, max: Fix) {
        self.low_power_speeds = (min, max);
    }

    /// Production speed multiplier: 1.0 on full power; on low power it
    /// interpolates min..max by produced/drain (production issue uses it).
    pub fn production_factor(&self) -> Fix {
        let (produced, drained) = self.power();
        if drained <= produced || drained == 0 {
            return Fix::ONE;
        }
        let (min, max) = self.low_power_speeds;
        // min + (max-min) * produced/drained, exact fixed-point.
        let ratio = Fix::from_raw((produced * 256 / drained) as i32);
        min + (max - min) * ratio
    }

    fn reap_dead(&mut self) {
        let dead_buildings: Vec<Building> =
            self.buildings.iter().filter(|b| b.hp <= 0).cloned().collect();
        if !dead_buildings.is_empty() {
            self.buildings.retain(|b| b.hp > 0);
            for b in &dead_buildings {
                if let Some(g) = self.terrain.as_mut() {
                    g.set_building(b.cell.0, b.cell.1, b.size.0, b.size.1, false);
                }
                self.events.push(Event::Died { id: b.id });
                for u in &mut self.units {
                    if u.attack == Some(b.id) {
                        u.attack = None;
                        u.target = None;
                        u.path.clear();
                    }
                }
            }
        }
        let dead: Vec<u32> = self.units.iter().filter(|u| u.hp <= 0).map(|u| u.id).collect();
        if dead.is_empty() {
            return;
        }
        self.units.retain(|u| u.hp > 0);
        for id in dead {
            self.events.push(Event::Died { id });
            for u in &mut self.units {
                if u.attack == Some(id) {
                    u.attack = None;
                    u.target = None;
                    u.path.clear();
                }
            }
            self.projectiles.retain(|p| p.owner != id);
        }
    }

    pub fn credits(&self) -> i64 {
        self.credits
    }

    /// Events since the last drain (renderer/audio consume these).
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn move_on_grid(&mut self, i: usize, grid: &TerrainGrid, tx: Fix, ty: Fix) {
        let (ux, uy, loco) = {
            let u = &self.units[i];
            (u.x.to_int_floor(), u.y.to_int_floor(), u.loco)
        };
        let (start, goal) = ((ux as u16, uy as u16), (tx.to_int_floor() as u16, ty.to_int_floor() as u16));
        if self.units[i].path.is_empty() && start != goal {
            match path::find_path(grid, loco, start, goal) {
                Some(p) => self.units[i].path = p.into(),
                None => {
                    self.units[i].target = None;
                    return;
                }
            }
        }
        let Some(&(wx, wy)) = self.units[i].path.front() else {
            self.units[i].target = None;
            return;
        };
        // Ground units may not enter a cell another ground unit holds.
        if loco != terrain::Loco::Fly {
            let occupied = self.units.iter().enumerate().any(|(j, o)| {
                j != i
                    && o.loco != terrain::Loco::Fly
                    && o.x.to_int_floor() as u16 == wx
                    && o.y.to_int_floor() as u16 == wy
            });
            if occupied {
                return;
            }
        }
        let centre = |c: u16| Fix::from_int(i32::from(c)) + Fix::from_raw(128);
        let (px, py) = (centre(wx), centre(wy));
        let factor = grid.speed_factor(loco, start.0, start.1).unwrap_or(Fix::ONE);
        let u = &mut self.units[i];
        let step = u.speed * factor;
        let (dx, dy) = (px - u.x, py - u.y);
        if dx != Fix::ZERO || dy != Fix::ZERO {
            u.facing = octant(dx, dy);
        }
        u.x += dx.clamp(-step, step);
        u.y += dy.clamp(-step, step);
        if u.x == px && u.y == py {
            u.path.pop_front();
            if u.path.is_empty() {
                // Final approach to the exact target leptons.
                let (dx, dy) = (tx - u.x, ty - u.y);
                u.x += dx.clamp(-step, step);
                u.y += dy.clamp(-step, step);
                if u.x == tx && u.y == ty {
                    u.target = None;
                }
            }
        }
    }

    /// Roll the sim RNG (for combat scatter etc.). Deterministic: the
    /// state lives here, never in a global.
    pub fn next_random(&mut self) -> u64 {
        self.rng.next_u64()
    }

    /// Hash of everything that defines the game state.
    pub fn state_hash(&self) -> u64 {
        let mut h = hash::Hasher::new();
        h.write_u64(self.tick);
        h.write_u64(self.rng.state());
        h.write_u64(u64::from(self.next_id));
        h.write_u64(self.terrain.as_ref().map_or(0, TerrainGrid::hash));
        h.write_u64(self.units.len() as u64);
        for u in &self.units {
            h.write_u64(u64::from(u.id));
            h.write_i64(i64::from(u.x.raw()));
            h.write_i64(i64::from(u.y.raw()));
            h.write_i64(i64::from(u.speed.raw()));
            h.write_u64(u.loco as u64);
            h.write_u64(u64::from(u.facing));
            h.write_u64(u.path.len() as u64);
            h.write_i64(u.hp);
            h.write_u64(u64::from(u.cooldown));
            h.write_u64(u.attack.map_or(0, u64::from));
            h.write_i64(u.cargo);
            h.write_u64(u64::from(u.bails));
            if let Some(&(wx, wy)) = u.path.front() {
                h.write_u64(u64::from(wx));
                h.write_u64(u64::from(wy));
            }
            match u.target {
                Some((tx, ty)) => {
                    h.write_u64(1);
                    h.write_i64(i64::from(tx.raw()));
                    h.write_i64(i64::from(ty.raw()));
                }
                None => h.write_u64(0),
            }
        }
        h.write_i64(self.credits);
        h.write_u64(self.buildings.len() as u64);
        for b in &self.buildings {
            h.write_u64(u64::from(b.id));
            h.write_i64(b.hp);
            h.write_i64(b.power);
        }
        h.write_u64(self.projectiles.len() as u64);
        for p in &self.projectiles {
            h.write_i64(i64::from(p.x.raw()));
            h.write_i64(i64::from(p.y.raw()));
        }
        h.finish()
    }
}

/// Squared lepton distance (no floats, no sqrt).
fn dist2(dx: Fix, dy: Fix) -> i64 {
    let (a, b) = (i64::from(dx.raw()), i64::from(dy.raw()));
    a * a + b * b
}

/// Facing (0=north, clockwise, 256 = full circle) from a step delta.
fn octant(dx: Fix, dy: Fix) -> u8 {
    let (sx, sy) = (dx.raw().signum(), dy.raw().signum());
    match (sx, sy) {
        (0, -1) => 0,
        (1, -1) => 32,
        (1, 0) => 64,
        (1, 1) => 96,
        (0, 1) => 128,
        (-1, 1) => 160,
        (-1, 0) => 192,
        (-1, -1) => 224,
        _ => 128,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(sim: &mut Sim, ticks: u64) -> u64 {
        sim.step(&[Command::Spawn { x: Fix::from_int(0), y: Fix::from_int(0), loco: terrain::Loco::Track, speed: Fix::from_int(4), hp: 100, armor: 0, weapon: None }]);
        sim.step(&[Command::Move { id: 1, x: Fix::from_int(2560), y: Fix::from_int(1280) }]);
        for _ in 2..ticks {
            sim.step(&[]);
        }
        sim.state_hash()
    }

    #[test]
    fn identical_streams_hash_identically_over_10k_ticks() {
        let (mut a, mut b) = (Sim::new(42), Sim::new(42));
        assert_eq!(script(&mut a, 10_000), script(&mut b, 10_000));
        // Unit actually arrived (2560 leptons at 4/tick = 640 ticks).
        assert_eq!(a.units()[0].x, Fix::from_int(2560));
        assert_eq!(a.units()[0].target, None);
    }

    #[test]
    fn different_seed_or_commands_diverge() {
        let (mut a, b, mut c) = (Sim::new(1), Sim::new(2), Sim::new(1));
        a.next_random();
        assert_ne!(a.state_hash(), b.state_hash());
        c.step(&[Command::Spawn { x: Fix::ZERO, y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2), hp: 100, armor: 0, weapon: None }]);
        assert_ne!(a.state_hash(), c.state_hash());
    }

    #[test]
    fn removal_and_spawn_ids_are_stable() {
        let mut sim = Sim::new(7);
        sim.step(&[
            Command::Spawn { x: Fix::ZERO, y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2), hp: 100, armor: 0, weapon: None },
            Command::Spawn { x: Fix::from_int(256), y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2), hp: 100, armor: 0, weapon: None },
            Command::Remove { id: 1 },
            Command::Spawn { x: Fix::from_int(512), y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2), hp: 100, armor: 0, weapon: None },
        ]);
        let ids: Vec<u32> = sim.units().iter().map(|u| u.id).collect();
        assert_eq!(ids, [2, 3], "ids never reused, order stable");
    }

    fn small_grid() -> TerrainGrid {
        // 6x3: cliff wall at x=3 except gap at y=1.
        let mut tiles = Vec::new();
        for y in 0..3u16 {
            for x in 0..6u16 {
                tiles.push((x, y, 0, if x == 3 && y != 1 { 15 } else { 13 }, 0));
            }
        }
        TerrainGrid::from_tiles(0, 0, 6, 3, tiles.into_iter())
    }

    #[test]
    fn unit_paths_around_obstacles_and_arrives() {
        let mut sim = Sim::new(3);
        sim.set_terrain(small_grid());
        let centre = |c: i32| Fix::from_int(c) + Fix::from_raw(128);
        sim.step(&[Command::Spawn { x: centre(0), y: centre(0), loco: terrain::Loco::Track, speed: Fix::from_int(64), hp: 100, armor: 0, weapon: None }]);
        sim.step(&[Command::Move { id: 1, x: centre(5), y: centre(0) }]);
        for _ in 0..500 {
            sim.step(&[]);
            if sim.units()[0].target.is_none() {
                break;
            }
        }
        let u = &sim.units()[0];
        assert_eq!((u.x, u.y), (centre(5), centre(0)), "arrived through the gap");
        assert!(sim.tick() < 500, "did not time out");
    }

    #[test]
    fn ground_units_wait_for_occupied_cells() {
        let mut sim = Sim::new(3);
        sim.set_terrain(small_grid());
        let centre = |c: i32| Fix::from_int(c) + Fix::from_raw(128);
        // Leader parked in the gap cell; follower ordered through it.
        sim.step(&[
            Command::Spawn { x: centre(3), y: centre(1), loco: terrain::Loco::Track, speed: Fix::from_int(64), hp: 100, armor: 0, weapon: None },
            Command::Spawn { x: centre(0), y: centre(1), loco: terrain::Loco::Track, speed: Fix::from_int(64), hp: 100, armor: 0, weapon: None },
        ]);
        sim.step(&[Command::Move { id: 2, x: centre(5), y: centre(1) }]);
        for _ in 0..40 {
            sim.step(&[]);
        }
        let follower = &sim.units()[1];
        assert_ne!(follower.x, centre(5), "gap blocked: follower must wait, at {:?}", follower.x);
        // Free the gap; follower proceeds.
        sim.step(&[Command::Remove { id: 1 }]);
        for _ in 0..200 {
            sim.step(&[]);
            if sim.units()[0].target.is_none() {
                break;
            }
        }
        assert_eq!(sim.units()[0].x, centre(5), "arrived after the gap freed");
    }

    fn gun(damage: i64, rof: u32, range_cells: i32, speed: Fix) -> WeaponSpec {
        let mut verses = [yr_rules::Decimal::ZERO; crate::combat::ARMOR_COUNT];
        verses[0] = yr_rules::Decimal::ONE;
        WeaponSpec { damage, rof_ticks: rof, range: Fix::from_int(range_cells), projectile_speed: speed, verses }
    }

    #[test]
    fn duel_kills_in_predicted_shots_with_events() {
        let mut sim = Sim::new(5);
        let w = gun(50, 10, 10, Fix::ZERO);
        sim.step(&[
            Command::Spawn { x: Fix::ZERO, y: Fix::ZERO, loco: terrain::Loco::Track, speed: Fix::from_int(4), hp: 200, armor: 0, weapon: Some(w) },
            Command::Spawn { x: Fix::from_int(2), y: Fix::ZERO, loco: terrain::Loco::Track, speed: Fix::from_int(4), hp: 120, armor: 0, weapon: None },
            Command::Attack { id: 1, target: 2 },
        ]);
        let mut events = Vec::new();
        for _ in 0..60 {
            sim.step(&[]);
            events.extend(sim.drain_events());
        }
        assert_eq!(sim.units().len(), 1, "target died");
        assert_eq!(sim.units()[0].shots, 3, "120hp / 50dmg = 3 shots");
        assert_eq!(events.iter().filter(|e| matches!(e, Event::Fired { .. })).count(), 3);
        assert_eq!(events.last(), Some(&Event::Died { id: 2 }));
    }

    #[test]
    fn harvester_loop_earns_credits_and_depletes_ore() {
        let mut grid = TerrainGrid::new(0, 0, 8, 2);
        grid.set_resource(6, 0, terrain::Overlay::Ore, 40);
        let mut sim = Sim::new(4);
        sim.set_terrain(grid);
        let centre = |c: i32| Fix::from_int(c) + Fix::from_raw(128);
        sim.step(&[
            Command::Spawn { x: centre(0), y: centre(0), loco: terrain::Loco::Track, speed: Fix::from_int(256), hp: 100, armor: 0, weapon: None },
            Command::Spawn { x: centre(1), y: centre(0), loco: terrain::Loco::Track, speed: Fix::from_int(256), hp: 100, armor: 0, weapon: None },
            Command::AssignRole { id: 1, role: Role::Refinery },
            Command::AssignRole { id: 2, role: Role::Harvester },
            Command::Harvest { id: 2 },
        ]);
        // Move the refinery marker onto the spawn area cell (0,0) is fine;
        // the harvester starts at (1,0). Run until two unloads happened.
        let mut unloads = 0;
        for _ in 0..4000 {
            sim.step(&[]);
            unloads += sim.drain_events().iter().filter(|e| matches!(e, Event::Unloaded { .. })).count();
            if unloads >= 2 {
                break;
            }
        }
        assert_eq!(unloads, 2, "harvester completed two loops");
        assert_eq!(sim.credits(), 2 * i64::from(HARVESTER_CAPACITY) * BAIL_ORE, "20 bails x 25cr per loop");
        let g = sim.terrain().unwrap();
        assert_eq!(g.cell(6, 0).map(|c| c.resource), Some(0), "ore cell depleted");
        assert_eq!(g.cell(6, 0).map(|c| c.overlay), Some(terrain::Overlay::None));
    }

    #[test]
    fn buildings_power_and_footprint_lifecycle() {
        let mut sim = Sim::new(6);
        sim.set_terrain(TerrainGrid::new(0, 0, 8, 8));
        // Power plant (+200) and two consumers (-150 each via two placements).
        sim.step(&[
            Command::PlaceBuilding { x: 0, y: 0, w: 2, h: 2, hp: 1000, armor: 0, power: 200 },
            Command::PlaceBuilding { x: 4, y: 0, w: 2, h: 2, hp: 500, armor: 0, power: -150 },
        ]);
        assert_eq!(sim.power(), (200, 150));
        assert!(!sim.low_power());
        assert_eq!(sim.production_factor(), Fix::ONE);
        let g = sim.terrain().unwrap();
        assert!(!g.passable(terrain::Loco::Foot, 0, 0), "footprint blocks");
        // Add drain past production: low power slows production.
        sim.step(&[Command::PlaceBuilding { x: 4, y: 4, w: 2, h: 2, hp: 500, armor: 0, power: -150 }]);
        assert!(sim.low_power());
        let f = sim.production_factor();
        assert!(f < Fix::ONE && f >= Fix::from_raw(128), "low-power factor interpolates: {f:?}");
        // A tank destroys the consumer at (4,4); footprint frees.
        let mut verses = [yr_rules::Decimal::ZERO; crate::combat::ARMOR_COUNT];
        verses[0] = yr_rules::Decimal::ONE;
        let w = WeaponSpec { damage: 250, rof_ticks: 1, range: Fix::from_int(20), projectile_speed: Fix::ZERO, verses };
        sim.step(&[
            Command::Spawn { x: Fix::from_raw(128), y: Fix::from_int(6), loco: terrain::Loco::Track, speed: Fix::from_int(4), hp: 500, armor: 0, weapon: Some(w) },
            Command::Attack { id: 4, target: 3 },
        ]);
        let mut died = false;
        for _ in 0..50 {
            sim.step(&[]);
            died |= sim.drain_events().iter().any(|e| matches!(e, Event::Died { id: 3 }));
            if died {
                break;
            }
        }
        assert!(died, "building destroyed");
        assert!(sim.terrain().unwrap().passable(terrain::Loco::Foot, 4, 4), "footprint freed");
        assert_eq!(sim.power(), (200, 150), "drain removed with the building");
    }

    #[test]
    fn projectile_travel_delays_the_hit() {
        let mut sim = Sim::new(5);
        // 256 leptons/tick over 4 cells (1024 leptons): hits land ~4 ticks after firing.
        let w = gun(50, 1, 10, Fix::from_int(1));
        sim.step(&[
            Command::Spawn { x: Fix::ZERO, y: Fix::ZERO, loco: terrain::Loco::Track, speed: Fix::from_int(4), hp: 200, armor: 0, weapon: Some(w) },
            Command::Spawn { x: Fix::from_int(4), y: Fix::ZERO, loco: terrain::Loco::Track, speed: Fix::from_int(4), hp: 40, armor: 0, weapon: None },
            Command::Attack { id: 1, target: 2 },
        ]);
        sim.step(&[]);
        let first = sim.drain_events();
        assert!(first.iter().any(|e| matches!(e, Event::Fired { .. })), "fired at once: {first:?}");
        assert!(!first.iter().any(|e| matches!(e, Event::Hit { .. })), "but not yet hit: {first:?}");
        let mut died_at = None;
        for t in 0..30 {
            sim.step(&[]);
            if sim.drain_events().iter().any(|e| matches!(e, Event::Died { .. })) {
                died_at = Some(t);
                break;
            }
        }
        assert!(died_at.is_some(), "projectile eventually kills");
    }
}
