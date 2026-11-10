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

pub mod ai;
pub mod building;
pub mod combat;
mod fix;
mod hash;
#[cfg(test)]
mod impact_tests;
pub mod path;
pub mod production;
mod rng;
pub mod save;
pub mod service;
pub mod terrain;

pub use building::Building;
pub use combat::WeaponSpec;
pub use fix::Fix;
pub use production::{BuildOrder, BuildingSpec, Factory, Product, UnitSpec};
pub use rng::Rng;
pub use service::RepairSpec;
pub use terrain::TerrainGrid;

/// One cell in leptons: positions are in leptons (256 per cell), the
/// original game's sub-cell unit.
pub const LEPTONS_PER_CELL: i32 = 256;

#[cfg(test)]
mod map_health_tests;

/// Commands are the only way to mutate the sim from outside. Applied in
/// slice order at the start of a tick, so replays are command streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Spawn {
        /// Caller type tag (e.g. a rules-id index), kept on the unit for
        /// rendering; opaque to the sim.
        kind: u16,
        x: Fix,
        y: Fix,
        loco: terrain::Loco,
        speed: Fix,
        hp: i64,
        armor: u8,
        weapon: Option<WeaponSpec>,
    },
    Move {
        id: u32,
        x: Fix,
        y: Fix,
    },
    /// Move to a point, engaging any hostile that comes in weapon range.
    AttackMove {
        id: u32,
        x: Fix,
        y: Fix,
    },
    Attack {
        id: u32,
        target: u32,
    },
    /// Send a harvester into its auto-loop (ore -> refinery -> ore).
    Harvest {
        id: u32,
    },
    /// Mark a unit's economy role (scenario setup).
    AssignRole {
        id: u32,
        role: Role,
    },
    /// Set a unit's owner (scenario setup).
    AssignOwner {
        id: u32,
        owner: u8,
    },
    /// Place a building: footprint cells, health, armour, power
    /// (positive produces, negative drains).
    PlaceBuilding {
        /// Caller type tag, kept on the building (see `Spawn`).
        kind: u16,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        hp: i64,
        armor: u8,
        power: i64,
        owner: u8,
        role: Role,
    },
    /// Scenario setup: place a map structure at its authored 0..256 health
    /// fraction. Keep the full type maximum; zero/rounded-down HP starts at one.
    /// Fractions above 256 are full health; type HP uses the snapshot's
    /// supported 1..=i32::MAX range.
    PlaceMapBuilding {
        kind: u16,
        x: u16,
        y: u16,
        spec: BuildingSpec,
        health: u16,
        owner: u8,
    },
    /// Scenario/debug: destroy a unit or building outright.
    Destroy {
        id: u32,
    },
    Stop {
        id: u32,
    },
    /// Queue a build at the player's factory of `factory` kind; `kind` is
    /// the caller's type index, echoed in [`Event::Produced`] /
    /// [`Event::Placed`].
    Produce {
        factory: Factory,
        kind: u16,
        product: Product,
        cost: i64,
        ticks: u32,
    },
    /// Put the finished structure of `kind` (head of the building queue)
    /// on the map with its top-left footprint cell at `(x, y)`. Ignored
    /// unless it is done, a construction yard still stands, and
    /// [`Sim::can_place`] allows the spot.
    PlaceProduced {
        kind: u16,
        x: u16,
        y: u16,
    },
    /// Toggle repair of a player-owned structure using a rules recipe.
    Repair {
        id: u32,
        spec: Option<RepairSpec>,
    },
    /// Sell an owned structure. The session supplies its rules value.
    Sell {
        id: u32,
        cost: i64,
        percent: yr_rules::Decimal,
    },
    /// Set suspension on the player's unfinished head of this factory queue.
    /// A mismatched kind or already completed order is ignored.
    HoldProduce {
        factory: Factory,
        kind: u16,
        held: bool,
    },
    /// Cancel a held head of this kind, otherwise the newest matching order.
    /// Refund exactly what that order had drawn.
    CancelProduce {
        factory: Factory,
        kind: u16,
    },
    Remove {
        id: u32,
    },
}

/// Things that happened during a tick, for the renderer/audio to react
/// to. Deterministic, drained by the caller after each step.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    RepairStarted {
        id: u32,
    },
    Sold {
        id: u32,
        value: i64,
        total: i64,
    },
    Fired {
        shooter: u32,
        target: u32,
    },
    /// Actual flight segment, including the final step to an impact. This
    /// observation carries no presentation clock and consumes no RNG.
    ProjectileMoved {
        from: (Fix, Fix),
        to: (Fix, Fix),
        origin: Option<ShotOrigin>,
    },
    Hit {
        target: u32,
        damage: i64,
    },
    /// A shot reached its target, including hits that inflict zero damage.
    Impact {
        target: u32,
        x: Fix,
        y: Fix,
        origin: Option<ShotOrigin>,
    },
    /// The positive-health target was killed by this hit. Later arrivals in
    /// the same phase cannot replace the lethal shot's provenance.
    FatalHit {
        target: u32,
        kind: u16,
        owner: u8,
        facing: u8,
        x: Fix,
        y: Fix,
        origin: Option<ShotOrigin>,
    },
    Died {
        id: u32,
    },
    /// A harvester finished unloading; `total` is the sim credit pool.
    Unloaded {
        id: u32,
        value: i64,
        total: i64,
    },
    /// A factory finished a build and the unit `id` rolled out.
    Produced {
        id: u32,
        kind: u16,
    },
    /// A structure finished building and waits to be placed.
    Ready {
        kind: u16,
    },
    /// A produced structure `id` was placed on the map.
    Placed {
        id: u32,
        kind: u16,
    },
}

/// What a unit is for; drives the economy state machine.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Combat,
    Harvester,
    Refinery,
    /// Building that hosts the vehicle production queue.
    WarFactory,
    /// Building that hosts the infantry production queue.
    Barracks,
    /// Building that hosts the structure queue.
    ConstructionYard,
}

/// Owner ids used by units and buildings.
pub const OWNER_PLAYER: u8 = 0;
pub const OWNER_ENEMY: u8 = 1;
pub const OWNER_NEUTRAL: u8 = 2;

/// Credits per harvested bail (RA2 bail values).
pub const BAIL_ORE: i64 = 25;
pub const BAIL_GEM: i64 = 50;
/// Bails a harvester carries before returning (RA2 capacity).
pub const HARVESTER_CAPACITY: u8 = 20;
/// Ticks spent loading one bail / unloading the cargo.
pub const HARVEST_TICKS: u32 = 20;
pub const UNLOAD_TICKS: u32 = 30;
/// Attack-move ends once a unit stalls within this many cells of its
/// destination (a crowd holds the exact cell).
pub const GUARD_ARRIVE_CELLS: i32 = 3;
/// Ticks a unit waits on an occupied cell before replanning around units.
pub const BLOCKED_REPLAN_TICKS: u16 = 10;
/// How far (cells) a blocked goal is substituted by a standable one.
pub const GOAL_SEARCH_CELLS: i32 = 6;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
enum Harvest {
    Idle,
    ToOre(u16, u16),
    Loading(u32),
    ToRefinery(u16, u16),
    Unloading(u32),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub id: u32,
    /// Caller type tag from `Spawn`/`Produce` (renderer art lookup).
    pub kind: u16,
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
    /// Attack-move destination: resume towards it once no hostile is in range.
    guard: Option<(Fix, Fix)>,
    pub shots: u32,
    pub role: Role,
    pub owner: u8,
    /// Credits worth of ore/gems currently carried.
    pub cargo: i64,
    pub bails: u8,
    harvest: Harvest,
    /// Consecutive ticks spent waiting on a cell another unit holds.
    blocked: u16,
}

/// Immutable presentation provenance captured when the weapon fires.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShotOrigin {
    pub shooter: u32,
    pub weapon: Option<u64>,
    /// Delivered damage before target armour, used for impact art selection.
    pub damage: i64,
}

/// A homing projectile in flight. Presentation observes the saved position;
/// it cannot change the flight or advance a separate movement clock.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct Projectile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<ShotOrigin>,
    pub x: Fix,
    pub y: Fix,
    owner: u32,
    target: u32,
    damage: i64,
    speed: Fix,
}

impl Projectile {
    fn motion_to(&self, tx: Fix, ty: Fix) -> (Fix, Fix) {
        (
            (tx - self.x).clamp(-self.speed, self.speed),
            (ty - self.y).clamp(-self.speed, self.speed),
        )
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Sim {
    tick: u64,
    rng: Rng,
    next_id: u32,
    /// Kept in id order; all iteration is over this Vec.
    units: Vec<Unit>,
    terrain: Option<TerrainGrid>,
    projectiles: Vec<Projectile>,
    events: Vec<Event>,
    economies: std::collections::BTreeMap<u8, Economy>,
    ai: Option<ai::SkirmishAi>,
    buildings: Vec<Building>,
    /// (min, max) low-power production speed factors from the rules.
    low_power_speeds: (Fix, Fix),
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    repairs: std::collections::BTreeMap<u32, service::RepairJob>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
struct Economy {
    credits: i64,
    production: Vec<BuildOrder>,
}

impl Sim {
    pub fn new(seed: u64) -> Sim {
        Sim {
            tick: 0,
            rng: Rng::new(seed),
            next_id: 1,
            units: Vec::new(),
            terrain: None,
            projectiles: Vec::new(),
            events: Vec::new(),
            economies: std::collections::BTreeMap::new(),
            ai: None,
            buildings: Vec::new(),
            low_power_speeds: (Fix::from_raw(128), Fix::ONE),
            repairs: Default::default(),
        }
    }

    /// Attach the scenario terrain (static; fingerprinted into the hash).
    pub fn set_terrain(&mut self, terrain: TerrainGrid) {
        self.terrain = Some(terrain);
    }

    pub fn terrain(&self) -> Option<&TerrainGrid> {
        self.terrain.as_ref()
    }

    /// Id the next spawned entity will get (setup code predicts ids
    /// for queued spawn commands).
    pub fn next_id_peek(&self) -> u32 {
        self.next_id
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    pub fn projectiles(&self) -> &[Projectile] {
        &self.projectiles
    }

    /// Advance one tick: apply commands (in order), then move units (in
    /// id order). Phase order is part of the determinism contract.
    pub fn step(&mut self, commands: &[Command]) {
        for cmd in commands {
            match *cmd {
                Command::Spawn {
                    kind,
                    x,
                    y,
                    loco,
                    speed,
                    hp,
                    armor,
                    weapon,
                } => {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.units.push(Unit {
                        id,
                        kind,
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
                        guard: None,
                        shots: 0,
                        role: Role::Combat,
                        // Unassigned spawns are neutral until AssignOwner.
                        owner: OWNER_NEUTRAL,
                        cargo: 0,
                        bails: 0,
                        harvest: Harvest::Idle,
                        blocked: 0,
                    });
                }
                Command::PlaceBuilding {
                    kind,
                    x,
                    y,
                    w,
                    h,
                    hp,
                    armor,
                    power,
                    owner,
                    role,
                } => {
                    let spec = BuildingSpec {
                        w,
                        h,
                        hp,
                        armor,
                        power,
                        role,
                        adjacent: 0,
                        base_normal: true,
                        free_unit: None,
                    };
                    self.place_structure(kind, x, y, &spec, owner);
                }
                Command::PlaceMapBuilding {
                    kind,
                    x,
                    y,
                    mut spec,
                    health,
                    owner,
                } => {
                    spec.hp = spec.hp.clamp(1, i64::from(i32::MAX));
                    self.place_structure(kind, x, y, &spec, owner);
                    // Widen before multiplying even for hostile Strength values.
                    let hp = i128::from(spec.hp) * i128::from(health.min(256)) / 256;
                    self.buildings.last_mut().unwrap().hp = (hp as i64).max(1);
                }
                Command::PlaceProduced { kind, x, y } => {
                    self.place_produced(OWNER_PLAYER, kind, x, y);
                }
                Command::Repair { id, spec } => self.set_repair(id, spec),
                Command::Sell { id, cost, percent } => self.sell_structure(id, cost, percent),
                Command::AssignOwner { id, owner } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.owner = owner;
                    }
                }
                Command::Destroy { id } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.hp = 0;
                    }
                    if let Some(b) = self.buildings.iter_mut().find(|b| b.id == id) {
                        b.hp = 0;
                    }
                    self.reap_dead();
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
                        u.guard = None;
                    }
                }
                Command::AttackMove { id, x, y } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.attack = None;
                        u.guard = Some((x, y));
                        u.target = Some((x, y));
                        u.path.clear();
                    }
                }
                Command::Stop { id } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.attack = None;
                        u.guard = None;
                        u.target = None;
                        u.path.clear();
                    }
                }
                Command::Move { id, x, y } => {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                        u.attack = None;
                        u.guard = None;
                        u.target = Some((x, y));
                    }
                }
                Command::Remove { id } => {
                    self.units.retain(|u| u.id != id);
                }
                Command::Produce { .. }
                | Command::HoldProduce { .. }
                | Command::CancelProduce { .. } => {
                    let economy = self.economies.entry(OWNER_PLAYER).or_default();
                    economy.credits += production::apply(&mut economy.production, *cmd);
                }
            }
        }
        // Commands apply first, so losing a factory stops AI production this tick.
        if let Some(mut ai) = self.ai.take() {
            ai.step(self);
            self.ai = Some(ai);
        }
        self.production_phase();
        self.service_phase();
        self.harvest_phase();
        // Idle armed units defend themselves: an idle combat unit (no
        // order of any kind) engages the nearest hostile in weapon range.
        // Pursuit then follows the normal attack rules below.
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if u.role != Role::Combat
                || u.guard.is_some()
                || u.attack.is_some()
                || u.target.is_some()
            {
                continue;
            }
            if let Some(id) = self.nearest_hostile_in_range(i) {
                self.units[i].attack = Some(id);
            }
        }
        // Attack-move: engage the nearest hostile in weapon range, else
        // keep (or resume) moving to the guarded destination.
        for i in 0..self.units.len() {
            let Some(dest) = self.units[i].guard else {
                continue;
            };
            if self.units[i].attack.is_some() {
                continue;
            }
            let hostile = self.nearest_hostile_in_range(i);
            let u = &mut self.units[i];
            if let Some(id) = hostile {
                u.attack = Some(id);
                u.target = None;
                u.path.clear();
            } else if u.target.is_none() {
                // Arrived, or as close as the crowd/terrain allows.
                let near = Fix::from_int(GUARD_ARRIVE_CELLS);
                if dist2(dest.0 - u.x, dest.1 - u.y)
                    <= i64::from(near.raw()) * i64::from(near.raw())
                {
                    u.guard = None;
                } else {
                    u.target = Some(dest);
                }
            }
        }
        // Attackers close to range; in-range attackers hold position.
        for i in 0..self.units.len() {
            let Some(enemy) = self.units[i].attack else {
                continue;
            };
            let (Some(w), Some(e)) = (self.units[i].weapon, self.target_pos(enemy)) else {
                continue;
            };
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
            let Some((tx, ty)) = self.units[i].target else {
                continue;
            };
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

    /// Nearest hostile (other non-neutral owner) unit or building within
    /// unit `i`'s weapon range; ties go to the first in id order.
    /// Neutral and unarmed units never acquire.
    fn nearest_hostile_in_range(&self, i: usize) -> Option<u32> {
        let me = &self.units[i];
        if me.owner == OWNER_NEUTRAL {
            return None;
        }
        let w = me.weapon?;
        let range2 = i64::from(w.range.raw()) * i64::from(w.range.raw());
        let mut best: Option<(i64, u32)> = None;
        let mut consider = |id: u32, owner: u8, x: Fix, y: Fix| {
            if owner == me.owner || owner == OWNER_NEUTRAL {
                return;
            }
            let d = dist2(x - me.x, y - me.y);
            if d <= range2 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, id));
            }
        };
        for o in &self.units {
            if o.id != me.id {
                consider(o.id, o.owner, o.x, o.y);
            }
        }
        for b in &self.buildings {
            let (bx, by) = b.centre();
            consider(b.id, b.owner, bx, by);
        }
        best.map(|(_, id)| id)
    }

    /// Harvester state machine (runs before movement): pick ore, drive,
    /// load bails, return to the first refinery, unload into credits.
    fn harvest_phase(&mut self) {
        let Some(mut grid) = self.terrain.clone() else {
            return;
        };
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
                    let target = if (tx, ty) == (u16::MAX, u16::MAX)
                        || grid.cell(tx, ty).is_none_or(|c| c.resource == 0)
                    {
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
                                self.units[i].target = Some((
                                    Fix::from_int(i32::from(ox)) + Fix::from_raw(128),
                                    Fix::from_int(i32::from(oy)) + Fix::from_raw(128),
                                ));
                            }
                        }
                        None => {
                            // No ore left: unload whatever we carry.
                            if self.units[i].bails > 0 {
                                if let Some(rc) = self.refinery_cell(self.units[i].owner) {
                                    self.units[i].harvest = Harvest::ToRefinery(rc.0, rc.1);
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
                            if let Some(rc) = self.refinery_cell(self.units[i].owner) {
                                self.units[i].harvest = Harvest::ToRefinery(rc.0, rc.1);
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
                Harvest::ToRefinery(rx, ry) => {
                    let Some(rcell) = self.refinery_cell(self.units[i].owner) else {
                        self.units[i].target = None;
                        self.units[i].path.clear();
                        continue;
                    };
                    if rcell != (rx, ry) {
                        self.units[i].harvest = Harvest::ToRefinery(rcell.0, rcell.1);
                        self.units[i].path.clear();
                    }
                    if self.beside_refinery(self.units[i].owner, cell) {
                        self.units[i].harvest = Harvest::Unloading(UNLOAD_TICKS);
                        self.units[i].target = None;
                        self.units[i].path.clear();
                    } else {
                        self.units[i].target = Some((
                            Fix::from_int(i32::from(rcell.0)) + Fix::from_raw(128),
                            Fix::from_int(i32::from(rcell.1)) + Fix::from_raw(128),
                        ));
                    }
                }
                Harvest::Unloading(mut left) => {
                    // Losing the refinery or moving away retains the cargo.
                    // Resume the approach and restart unloading at a live host.
                    if !self.beside_refinery(self.units[i].owner, cell) {
                        self.units[i].harvest = Harvest::ToRefinery(u16::MAX, u16::MAX);
                        continue;
                    }
                    if left > 0 {
                        left -= 1;
                        self.units[i].harvest = Harvest::Unloading(left);
                    } else {
                        let value = self.units[i].cargo;
                        let economy = self.economies.entry(self.units[i].owner).or_default();
                        economy.credits += value;
                        self.units[i].cargo = 0;
                        self.units[i].bails = 0;
                        self.events.push(Event::Unloaded {
                            id: self.units[i].id,
                            value,
                            total: economy.credits,
                        });
                        self.units[i].harvest = Harvest::ToOre(u16::MAX, u16::MAX);
                    }
                }
            }
        }
        if grid_dirty {
            self.terrain = Some(grid);
        }
    }

    /// Cell a harvester unloads at: a Refinery-role building's centre
    /// cell, else a Refinery-role unit's cell.
    fn refinery_cell(&self, owner: u8) -> Option<(u16, u16)> {
        if let Some(b) = self
            .buildings
            .iter()
            .find(|b| b.owner == owner && b.role == Role::Refinery)
        {
            let (cx, cy) = b.centre();
            return Some((
                cx.to_int_floor().max(0) as u16,
                cy.to_int_floor().max(0) as u16,
            ));
        }
        self.units
            .iter()
            .find(|u| u.owner == owner && u.role == Role::Refinery)
            .map(|u| {
                (
                    u.x.to_int_floor().max(0) as u16,
                    u.y.to_int_floor().max(0) as u16,
                )
            })
    }

    /// Exterior one-cell ring around the first owned refinery. A retail
    /// 4x3 footprint has no reachable cell within one of its centre.
    fn beside_refinery(&self, owner: u8, cell: (u16, u16)) -> bool {
        let bounds = self
            .buildings
            .iter()
            .find(|b| b.owner == owner && b.role == Role::Refinery)
            .map(|b| (b.cell, b.size))
            .or_else(|| self.refinery_cell(owner).map(|c| (c, (1, 1))));
        let Some(((x, y), (w, h))) = bounds else {
            return false;
        };
        let (x, y, cx, cy) = (
            i32::from(x),
            i32::from(y),
            i32::from(cell.0),
            i32::from(cell.1),
        );
        let (right, bottom) = (x + i32::from(w), y + i32::from(h));
        let inside = cx >= x && cx < right && cy >= y && cy < bottom;
        !inside && cx >= x - 1 && cx <= right && cy >= y - 1 && cy <= bottom
    }

    /// Elapsed unloading ticks at this refinery, for presentation only.
    /// The first active harvester wins in stable id order.
    pub fn refinery_unloading(&self, id: u32) -> Option<u32> {
        let host = self
            .buildings
            .iter()
            .find(|b| b.role == Role::Refinery && b.id == id)?;
        let first = self
            .buildings
            .iter()
            .find(|b| b.owner == host.owner && b.role == Role::Refinery)?;
        if first.id != id {
            return None;
        }
        self.units.iter().find_map(|u| {
            let Harvest::Unloading(left) = u.harvest else {
                return None;
            };
            let cell = (u.x.to_int_floor() as u16, u.y.to_int_floor() as u16);
            (u.owner == host.owner && self.beside_refinery(u.owner, cell))
                .then_some(UNLOAD_TICKS.saturating_sub(left))
        })
    }

    fn fire_phase(&mut self) {
        let mut spawned: Vec<Projectile> = Vec::new();
        let mut instant: Vec<(ShotOrigin, u32, i64)> = Vec::new();
        for i in 0..self.units.len() {
            if self.units[i].cooldown > 0 {
                self.units[i].cooldown -= 1;
            }
            let u = &self.units[i];
            let (Some(w), Some(enemy_id)) = (u.weapon, u.attack) else {
                continue;
            };
            if u.cooldown > 0 {
                continue;
            }
            let Some(((ex, ey), earmor)) =
                self.target_pos(enemy_id).zip(self.target_armor(enemy_id))
            else {
                continue;
            };
            let (dx, dy) = (ex - u.x, ey - u.y);
            if dist2(dx, dy) > i64::from(w.range.raw()) * i64::from(w.range.raw()) {
                continue;
            }
            self.events.push(Event::Fired {
                shooter: u.id,
                target: enemy_id,
            });
            let damage = w.damage_against(earmor);
            let origin = ShotOrigin {
                shooter: u.id,
                weapon: w.source,
                damage: w.damage,
            };
            if w.projectile_speed == Fix::ZERO {
                instant.push((origin, enemy_id, damage));
            } else {
                spawned.push(Projectile {
                    origin: Some(origin),
                    x: u.x,
                    y: u.y,
                    owner: u.id,
                    target: enemy_id,
                    damage,
                    speed: w.projectile_speed,
                });
            }
            let u = &mut self.units[i];
            u.shots += 1;
            u.cooldown = w.rof_ticks;
        }
        self.projectiles.extend(spawned);
        for (origin, target, damage) in instant {
            self.apply_impact(target, damage, Some(origin));
        }
        self.reap_dead();
    }

    /// The homing step toward the current target position, also available after load.
    /// Missing targets have no direction and will be removed by the flight phase.
    pub fn projectile_motion(&self, p: &Projectile) -> Option<(Fix, Fix)> {
        let (tx, ty) = self.target_pos(p.target)?;
        Some(p.motion_to(tx, ty))
    }

    fn projectile_phase(&mut self) {
        let mut hits: Vec<(u32, i64, Option<ShotOrigin>)> = Vec::new();
        let mut alive: Vec<Projectile> = Vec::new();
        for mut p in std::mem::take(&mut self.projectiles) {
            let Some((tx, ty)) = self.target_pos(p.target) else {
                continue;
            };
            let (dx, dy) = (tx - p.x, ty - p.y);
            let from = (p.x, p.y);
            if dist2(dx, dy) <= i64::from(p.speed.raw()) * i64::from(p.speed.raw()) {
                self.events.push(Event::ProjectileMoved {
                    from,
                    to: (tx, ty),
                    origin: p.origin,
                });
                hits.push((p.target, p.damage, p.origin));
            } else {
                // Axis-clamped homing step (deterministic, no floats).
                let (mx, my) = p.motion_to(tx, ty);
                p.x += mx;
                p.y += my;
                self.events.push(Event::ProjectileMoved {
                    from,
                    to: (p.x, p.y),
                    origin: p.origin,
                });
                alive.push(p);
            }
        }
        self.projectiles = alive;
        for (target, damage, origin) in hits {
            self.apply_impact(target, damage, origin);
        }
        self.reap_dead();
    }

    fn apply_impact(&mut self, target: u32, damage: i64, origin: Option<ShotOrigin>) {
        if let Some((x, y)) = self.target_pos(target) {
            self.events.push(Event::Impact {
                target,
                x,
                y,
                origin,
            });
        }
        if let Some(u) = self.units.iter_mut().find(|x| x.id == target) {
            if u.hp > 0 && damage > 0 {
                u.hp -= damage;
                self.events.push(Event::Hit { target, damage });
                if u.hp <= 0 {
                    self.events.push(Event::FatalHit {
                        target,
                        kind: u.kind,
                        owner: u.owner,
                        facing: u.facing,
                        x: u.x,
                        y: u.y,
                        origin,
                    });
                }
            }
            return;
        }
        if let Some(b) = self.buildings.iter_mut().find(|b| b.id == target) {
            if b.hp > 0 && damage > 0 {
                b.hp -= damage;
                self.events.push(Event::Hit { target, damage });
                if b.hp <= 0 {
                    let (x, y) = b.centre();
                    self.events.push(Event::FatalHit {
                        target,
                        kind: b.kind,
                        owner: b.owner,
                        facing: 0,
                        x,
                        y,
                        origin,
                    });
                }
            }
        }
    }

    /// Aim point (leptons) for a unit or building id.
    pub fn target_pos(&self, id: u32) -> Option<(Fix, Fix)> {
        if let Some(u) = self.units.iter().find(|u| u.id == id) {
            return Some((u.x, u.y));
        }
        self.buildings
            .iter()
            .find(|b| b.id == id)
            .map(Building::centre)
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

    /// The player's (produced, drained) power totals.
    pub fn power(&self) -> (i64, i64) {
        self.power_of(OWNER_PLAYER)
    }

    /// (produced, drained) power totals of `owner`'s buildings.
    ///
    /// Produced power uses each building's health-scaled output
    /// (`Building::effective_power`); drained power is unscaled (#93).
    pub fn power_of(&self, owner: u8) -> (i64, i64) {
        let mut produced = 0;
        let mut drained = 0;
        for b in self.buildings.iter().filter(|b| b.owner == owner) {
            let power = b.effective_power();
            if power >= 0 {
                produced += power;
            } else {
                drained -= power;
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
        self.production_factor_of(OWNER_PLAYER)
    }

    pub fn production_factor_of(&self, owner: u8) -> Fix {
        let (produced, drained) = self.power_of(owner);
        if drained <= produced || drained == 0 {
            return Fix::ONE;
        }
        let (min, max) = self.low_power_speeds;
        // min + (max-min) * produced/drained, exact fixed-point.
        let ratio = Fix::from_raw((produced * 256 / drained) as i32);
        min + (max - min) * ratio
    }

    fn reap_dead(&mut self) {
        let dead_buildings: Vec<Building> = self
            .buildings
            .iter()
            .filter(|b| b.hp <= 0)
            .cloned()
            .collect();
        if !dead_buildings.is_empty() {
            self.buildings.retain(|b| b.hp > 0);
            for b in &dead_buildings {
                self.release_structure(b);
                self.events.push(Event::Died { id: b.id });
            }
        }
        let dead: Vec<u32> = self
            .units
            .iter()
            .filter(|u| u.hp <= 0)
            .map(|u| u.id)
            .collect();
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
        }
    }

    /// Player build orders in queue order (heads first per factory).
    pub fn production(&self) -> &[BuildOrder] {
        self.production_of(OWNER_PLAYER)
    }

    pub fn production_of(&self, owner: u8) -> &[BuildOrder] {
        self.economies
            .get(&owner)
            .map_or(&[], |e| e.production.as_slice())
    }

    fn queue_produce(
        &mut self,
        owner: u8,
        factory: Factory,
        kind: u16,
        product: Product,
        cost: i64,
        ticks: u32,
    ) {
        let economy = self.economies.entry(owner).or_default();
        production::apply(
            &mut economy.production,
            Command::Produce {
                factory,
                kind,
                product,
                cost,
                ticks,
            },
        );
    }

    /// Project pending queue controls without ticking, spending or changing RNG.
    /// Other pending command types, including placement, are ignored here.
    pub fn production_with_pending(&self, commands: &[Command]) -> Vec<BuildOrder> {
        let mut queue = self.production().to_vec();
        for &command in commands {
            production::apply(&mut queue, command);
        }
        queue
    }

    fn place_produced(&mut self, owner: u8, kind: u16, x: u16, y: u16) {
        let Some(i) = self
            .production_of(owner)
            .iter()
            .position(|o| o.factory == Factory::Building)
        else {
            return;
        };
        let order = self.production_of(owner)[i];
        let Product::Building(spec) = order.product else {
            return;
        };
        let yard = self
            .buildings
            .iter()
            .any(|b| b.owner == owner && b.role == Role::ConstructionYard);
        if order.kind != kind || !order.done() || !yard || !self.can_place(owner, &spec, x, y) {
            return;
        }
        self.economies.get_mut(&owner).unwrap().production.remove(i);
        let id = self.place_structure(kind, x, y, &spec, owner);
        self.events.push(Event::Placed { id, kind });
        if let Some((unit_kind, unit)) = spec.free_unit {
            if let Some(cell) = self.exit_cell(unit.loco, (x, y), (spec.w, spec.h)) {
                self.spawn_owned(unit_kind, &unit, cell, owner);
            }
        }
    }

    /// Advance queues in owner order, then fixed Factory::ALL order.
    /// Missing factories stall their own queue; blocked exits hold a unit.
    fn production_phase(&mut self) {
        let owners: Vec<u8> = self.economies.keys().copied().collect();
        for owner in owners {
            let factor = self.production_factor_of(owner);
            for factory in Factory::ALL {
                let Some(i) = self
                    .production_of(owner)
                    .iter()
                    .position(|o| o.factory == factory)
                else {
                    continue;
                };
                let Some(host) = self
                    .buildings
                    .iter()
                    .find(|b| b.owner == owner && b.role == factory.role())
                    .map(|b| (b.cell, b.size))
                else {
                    continue;
                };
                let economy = self.economies.get_mut(&owner).unwrap();
                let was_done = economy.production[i].done();
                if let Some(drawn) = economy.production[i].advance(factor, economy.credits) {
                    economy.credits -= drawn;
                }
                let order = economy.production[i];
                if !order.done() {
                    continue;
                }
                let spec = match order.product {
                    Product::Unit(spec) => spec,
                    Product::Building(_) => {
                        if !was_done && owner == OWNER_PLAYER {
                            self.events.push(Event::Ready { kind: order.kind });
                        }
                        continue;
                    }
                };
                let Some(cell) = self.exit_cell(spec.loco, host.0, host.1) else {
                    continue;
                };
                self.economies.get_mut(&owner).unwrap().production.remove(i);
                let id = self.spawn_owned(order.kind, &spec, cell, owner);
                self.events.push(Event::Produced {
                    id,
                    kind: order.kind,
                });
            }
        }
    }

    /// Spawn a unit from `spec` at the centre of `cell` for `owner`;
    /// harvesters start their loop straight away. Returns its id.
    fn spawn_owned(&mut self, kind: u16, s: &UnitSpec, cell: (u16, u16), owner: u8) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.units.push(Unit {
            id,
            kind,
            x: Fix::from_int(i32::from(cell.0)) + Fix::from_raw(128),
            y: Fix::from_int(i32::from(cell.1)) + Fix::from_raw(128),
            target: None,
            speed: s.speed,
            loco: s.loco,
            facing: 128,
            path: std::collections::VecDeque::new(),
            hp: s.hp,
            max_hp: s.hp,
            armor: s.armor,
            weapon: s.weapon,
            cooldown: 0,
            attack: None,
            guard: None,
            shots: 0,
            role: s.role,
            owner,
            cargo: 0,
            bails: 0,
            harvest: if s.role == Role::Harvester {
                Harvest::ToOre(u16::MAX, u16::MAX)
            } else {
                Harvest::Idle
            },
            blocked: 0,
        });
        id
    }

    /// Add a building and mark its footprint on the grid. Returns its id.
    fn place_structure(
        &mut self,
        kind: u16,
        x: u16,
        y: u16,
        spec: &BuildingSpec,
        owner: u8,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.buildings.push(Building {
            id,
            kind,
            cell: (x, y),
            size: (spec.w, spec.h),
            hp: spec.hp,
            max_hp: spec.hp,
            armor: spec.armor,
            power: spec.power,
            owner,
            role: spec.role,
            base_normal: spec.base_normal,
        });
        if let Some(g) = self.terrain.as_mut() {
            g.set_building(x, y, spec.w, spec.h, true);
        }
        id
    }

    /// Whether `owner` may put `spec`'s footprint with its top-left cell
    /// at `(x, y)`. Every cell must be on the grid, flat (no ramp, one
    /// height level), open land a tracked unit could enter (no water,
    /// cliff, wall or building), free of overlay (ore, gems, debris) and
    /// of ground units; and the footprint must come within
    /// `spec.adjacent` empty cells of a base building `owner` already has
    /// (`BaseNormal=` buildings only). The UI asks this for its ghost.
    pub fn can_place(&self, owner: u8, spec: &BuildingSpec, x: u16, y: u16) -> bool {
        let Some(grid) = &self.terrain else {
            return false;
        };
        let (w, h) = (spec.w.max(1), spec.h.max(1));
        let (Some(x1), Some(y1)) = (x.checked_add(w), y.checked_add(h)) else {
            return false;
        };
        let Some(level) = grid.cell(x, y).map(|c| c.height) else {
            return false;
        };
        for cy in y..y1 {
            for cx in x..x1 {
                let Some(c) = grid.cell(cx, cy) else {
                    return false;
                };
                if c.ramp != 0
                    || c.height != level
                    || c.overlay != terrain::Overlay::None
                    || !grid.passable(terrain::Loco::Track, cx, cy)
                {
                    return false;
                }
            }
        }
        let (x0, y0, x1, y1) = (i32::from(x), i32::from(y), i32::from(x1), i32::from(y1));
        let blocked = self.units.iter().any(|u| {
            let (ux, uy) = (u.x.to_int_floor(), u.y.to_int_floor());
            u.loco != terrain::Loco::Fly && ux >= x0 && ux < x1 && uy >= y0 && uy < y1
        });
        if blocked {
            return false;
        }
        // Gap of up to `adjacent` cells: grow the footprint by adjacent + 1
        // and look for an overlapping base building.
        let reach = i32::from(spec.adjacent) + 1;
        let (ax0, ay0, ax1, ay1) = (x0 - reach, y0 - reach, x1 + reach, y1 + reach);
        self.buildings.iter().any(|b| {
            let (bx, by) = (i32::from(b.cell.0), i32::from(b.cell.1));
            let (bw, bh) = (i32::from(b.size.0), i32::from(b.size.1));
            b.owner == owner
                && b.base_normal
                && bx < ax1
                && bx + bw > ax0
                && by < ay1
                && by + bh > ay0
        })
    }

    /// Free standable cell around a footprint, scanning rings outward
    /// from the cell below its bottom edge (where factories open).
    fn exit_cell(
        &self,
        loco: terrain::Loco,
        cell: (u16, u16),
        size: (u16, u16),
    ) -> Option<(u16, u16)> {
        let origin = (
            i32::from(cell.0) + i32::from(size.0) / 2,
            i32::from(cell.1) + i32::from(size.1),
        );
        let free = |x: i32, y: i32| -> Option<(u16, u16)> {
            let (x, y) = (u16::try_from(x).ok()?, u16::try_from(y).ok()?);
            let standable = match &self.terrain {
                Some(g) => g.passable(loco, x, y),
                None => true,
            };
            let taken = self
                .units
                .iter()
                .any(|u| u.x.to_int_floor() == i32::from(x) && u.y.to_int_floor() == i32::from(y));
            (standable && !taken).then_some((x, y))
        };
        for r in 0..=GOAL_SEARCH_CELLS * 2 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    if let Some(c) = free(origin.0 + dx, origin.1 + dy) {
                        return Some(c);
                    }
                }
            }
        }
        None
    }

    pub fn credits(&self) -> i64 {
        self.credits_of(OWNER_PLAYER)
    }

    pub fn set_credits(&mut self, credits: i64) {
        self.set_credits_of(OWNER_PLAYER, credits);
    }

    pub fn credits_of(&self, owner: u8) -> i64 {
        self.economies.get(&owner).map_or(0, |e| e.credits)
    }

    pub fn set_credits_of(&mut self, owner: u8, credits: i64) {
        self.economies.entry(owner).or_default().credits = credits;
    }

    /// Install a deterministic computer controller for this scenario.
    pub fn set_ai(&mut self, ai: Option<ai::SkirmishAi>) {
        self.ai = ai;
    }

    pub fn ai(&self) -> Option<&ai::SkirmishAi> {
        self.ai.as_ref()
    }

    /// Events since the last drain (renderer/audio consume these).
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Read-only route preview using the same impassable-goal substitution as movement.
    /// Unit occupancy may change before the order takes effect.
    pub fn can_move_to(&self, id: u32, tx: Fix, ty: Fix) -> bool {
        let Some(unit) = self.units.iter().find(|u| u.id == id && u.hp > 0) else {
            return false;
        };
        let Some(grid) = self.terrain.as_ref() else {
            return tx.raw() >= 0 && ty.raw() >= 0;
        };
        let cell = |x: Fix, y: Fix| {
            u16::try_from(x.to_int_floor())
                .ok()
                .zip(u16::try_from(y.to_int_floor()).ok())
        };
        let Some(start) = cell(unit.x, unit.y) else {
            return false;
        };
        let Some(goal) = cell(tx, ty).filter(|&(x, y)| grid.cell(x, y).is_some()) else {
            return false;
        };
        let Some(goal) = path::nearest_passable(grid, unit.loco, goal, GOAL_SEARCH_CELLS) else {
            return false;
        };
        start == goal || path::find_path(grid, unit.loco, start, goal).is_some()
    }

    fn move_on_grid(&mut self, i: usize, grid: &TerrainGrid, tx: Fix, ty: Fix) {
        let (ux, uy, loco) = {
            let u = &self.units[i];
            (u.x.to_int_floor(), u.y.to_int_floor(), u.loco)
        };
        let centre = |c: u16| Fix::from_int(i32::from(c)) + Fix::from_raw(128);
        let start = (ux as u16, uy as u16);
        let mut goal = (tx.to_int_floor() as u16, ty.to_int_floor() as u16);
        let (mut tx, mut ty) = (tx, ty);
        // A goal no unit can stand on (a building, a cliff) is replaced by
        // the nearest standable cell; the approach ends at its centre.
        if !grid.passable(loco, goal.0, goal.1) {
            let Some(sub) = path::nearest_passable(grid, loco, goal, GOAL_SEARCH_CELLS) else {
                self.units[i].target = None;
                return;
            };
            goal = sub;
            (tx, ty) = (centre(sub.0), centre(sub.1));
        }
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
            let holder = |sim: &Sim, cx: u16, cy: u16| {
                sim.units.iter().enumerate().any(|(j, o)| {
                    j != i
                        && o.loco != terrain::Loco::Fly
                        && o.x.to_int_floor() as u16 == cx
                        && o.y.to_int_floor() as u16 == cy
                })
            };
            if holder(self, wx, wy) {
                let u = &mut self.units[i];
                if u.path.len() == 1 {
                    // The goal cell itself is held: arrived as near as possible.
                    u.path.clear();
                    u.target = None;
                    u.blocked = 0;
                    return;
                }
                u.blocked += 1;
                if u.blocked >= BLOCKED_REPLAN_TICKS {
                    u.blocked = 0;
                    let avoid: Vec<(u16, u16)> = self
                        .units
                        .iter()
                        .enumerate()
                        .filter(|(j, o)| *j != i && o.loco != terrain::Loco::Fly)
                        .map(|(_, o)| (o.x.to_int_floor() as u16, o.y.to_int_floor() as u16))
                        .collect();
                    // No detour: keep the old path and wait for the cell.
                    if let Some(p) = path::find_path_avoiding(grid, loco, start, goal, &avoid) {
                        if !p.is_empty() {
                            self.units[i].path = p.into();
                        }
                    }
                }
                return;
            }
        }
        let (px, py) = (centre(wx), centre(wy));
        let factor = grid
            .speed_factor(loco, start.0, start.1)
            .unwrap_or(Fix::ONE);
        let u = &mut self.units[i];
        u.blocked = 0;
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
        hash::state(self)
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
        sim.step(&[Command::Spawn {
            kind: 0,
            x: Fix::from_int(0),
            y: Fix::from_int(0),
            loco: terrain::Loco::Track,
            speed: Fix::from_int(4),
            hp: 100,
            armor: 0,
            weapon: None,
        }]);
        sim.step(&[Command::Move {
            id: 1,
            x: Fix::from_int(2560),
            y: Fix::from_int(1280),
        }]);
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
        c.step(&[Command::Spawn {
            kind: 0,
            x: Fix::ZERO,
            y: Fix::ZERO,
            loco: terrain::Loco::Foot,
            speed: Fix::from_int(2),
            hp: 100,
            armor: 0,
            weapon: None,
        }]);
        assert_ne!(a.state_hash(), c.state_hash());
    }

    #[test]
    fn removal_and_spawn_ids_are_stable() {
        let mut sim = Sim::new(7);
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: Fix::ZERO,
                y: Fix::ZERO,
                loco: terrain::Loco::Foot,
                speed: Fix::from_int(2),
                hp: 100,
                armor: 0,
                weapon: None,
            },
            Command::Spawn {
                kind: 0,
                x: Fix::from_int(256),
                y: Fix::ZERO,
                loco: terrain::Loco::Foot,
                speed: Fix::from_int(2),
                hp: 100,
                armor: 0,
                weapon: None,
            },
            Command::Remove { id: 1 },
            Command::Spawn {
                kind: 0,
                x: Fix::from_int(512),
                y: Fix::ZERO,
                loco: terrain::Loco::Foot,
                speed: Fix::from_int(2),
                hp: 100,
                armor: 0,
                weapon: None,
            },
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
        sim.step(&[Command::Spawn {
            kind: 0,
            x: centre(0),
            y: centre(0),
            loco: terrain::Loco::Track,
            speed: Fix::from_int(64),
            hp: 100,
            armor: 0,
            weapon: None,
        }]);
        sim.step(&[Command::Move {
            id: 1,
            x: centre(5),
            y: centre(0),
        }]);
        for _ in 0..500 {
            sim.step(&[]);
            if sim.units()[0].target.is_none() {
                break;
            }
        }
        let u = &sim.units()[0];
        assert_eq!(
            (u.x, u.y),
            (centre(5), centre(0)),
            "arrived through the gap"
        );
        assert!(sim.tick() < 500, "did not time out");
    }

    #[test]
    fn ground_units_wait_for_occupied_cells() {
        let mut sim = Sim::new(3);
        sim.set_terrain(small_grid());
        let centre = |c: i32| Fix::from_int(c) + Fix::from_raw(128);
        // Leader parked in the gap cell; follower ordered through it.
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: centre(3),
                y: centre(1),
                loco: terrain::Loco::Track,
                speed: Fix::from_int(64),
                hp: 100,
                armor: 0,
                weapon: None,
            },
            Command::Spawn {
                kind: 0,
                x: centre(0),
                y: centre(1),
                loco: terrain::Loco::Track,
                speed: Fix::from_int(64),
                hp: 100,
                armor: 0,
                weapon: None,
            },
        ]);
        sim.step(&[Command::Move {
            id: 2,
            x: centre(5),
            y: centre(1),
        }]);
        for _ in 0..40 {
            sim.step(&[]);
        }
        let follower = &sim.units()[1];
        assert_ne!(
            follower.x,
            centre(5),
            "gap blocked: follower must wait, at {:?}",
            follower.x
        );
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
        WeaponSpec {
            source: None,
            damage,
            rof_ticks: rof,
            range: Fix::from_int(range_cells),
            projectile_speed: speed,
            verses,
        }
    }

    #[test]
    fn duel_kills_in_predicted_shots_with_events() {
        let mut sim = Sim::new(5);
        let w = gun(50, 10, 10, Fix::ZERO);
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: Fix::ZERO,
                y: Fix::ZERO,
                loco: terrain::Loco::Track,
                speed: Fix::from_int(4),
                hp: 200,
                armor: 0,
                weapon: Some(w),
            },
            Command::Spawn {
                kind: 0,
                x: Fix::from_int(2),
                y: Fix::ZERO,
                loco: terrain::Loco::Track,
                speed: Fix::from_int(4),
                hp: 120,
                armor: 0,
                weapon: None,
            },
            Command::Attack { id: 1, target: 2 },
        ]);
        let mut events = Vec::new();
        for _ in 0..60 {
            sim.step(&[]);
            events.extend(sim.drain_events());
        }
        assert_eq!(sim.units().len(), 1, "target died");
        assert_eq!(sim.units()[0].shots, 3, "120hp / 50dmg = 3 shots");
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::Fired { .. }))
                .count(),
            3
        );
        assert_eq!(events.last(), Some(&Event::Died { id: 2 }));
    }

    #[test]
    fn attack_move_engages_en_route_then_arrives() {
        // Arrange: attacker at 0, victim 2 cells away, destination 8 cells.
        let mut sim = Sim::new(5);
        let w = gun(50, 1, 10, Fix::ZERO);
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: Fix::ZERO,
                y: Fix::ZERO,
                loco: terrain::Loco::Track,
                speed: Fix::from_int(4),
                hp: 500,
                armor: 0,
                weapon: Some(w),
            },
            Command::Spawn {
                kind: 0,
                x: Fix::from_int(2),
                y: Fix::ZERO,
                loco: terrain::Loco::Track,
                speed: Fix::from_int(4),
                hp: 40,
                armor: 0,
                weapon: None,
            },
            Command::AssignOwner {
                id: 1,
                owner: OWNER_PLAYER,
            },
            Command::AssignOwner {
                id: 2,
                owner: OWNER_ENEMY,
            },
            Command::AttackMove {
                id: 1,
                x: Fix::from_int(8),
                y: Fix::ZERO,
            },
        ]);
        // Act: run until the attacker settles at the destination.
        for _ in 0..200 {
            sim.step(&[]);
            if sim.units().len() == 1 && sim.units()[0].target.is_none() {
                break;
            }
        }
        // Assert: victim died en route and the destination was reached.
        assert_eq!(sim.units().len(), 1, "hostile engaged and killed en route");
        assert_eq!(sim.units()[0].x, Fix::from_int(8), "resumed to destination");
        assert!(sim.units()[0].shots >= 1);
    }

    #[test]
    fn orders_onto_buildings_and_shared_goals_settle_nearby() {
        // Arrange: open 12x6 grid, a 2x2 building at (8,2); three tanks
        // all ordered onto the building's centre.
        let mut sim = Sim::new(9);
        sim.set_terrain(TerrainGrid::new(0, 0, 12, 6));
        let c = |v: i32| Fix::from_int(v) + Fix::from_raw(128);
        let tank = |y: i32| Command::Spawn {
            kind: 0,
            x: c(0),
            y: c(y),
            loco: terrain::Loco::Track,
            speed: Fix::from_int(1),
            hp: 100,
            armor: 0,
            weapon: None,
        };
        sim.step(&[
            Command::PlaceBuilding {
                kind: 0,
                x: 8,
                y: 2,
                w: 2,
                h: 2,
                hp: 500,
                armor: 0,
                power: 0,
                owner: OWNER_ENEMY,
                role: Role::Combat,
            },
            tank(1),
            tank(2),
            tank(3),
        ]);
        let goal = (Fix::from_int(9), Fix::from_int(3));
        let orders: Vec<Command> = (2..=4)
            .map(|id| Command::Move {
                id,
                x: goal.0,
                y: goal.1,
            })
            .collect();
        sim.step(&orders);
        // Act
        for _ in 0..400 {
            sim.step(&[]);
        }
        // Assert: everyone stopped, off the footprint, within 3 cells.
        for u in sim.units() {
            assert!(u.target.is_none(), "unit {} settled", u.id);
            let (x, y) = (u.x.to_int_floor(), u.y.to_int_floor());
            assert!(
                !((8..10).contains(&x) && (2..4).contains(&y)),
                "unit {} inside footprint",
                u.id
            );
            assert!(
                dist2(u.x - goal.0, u.y - goal.1) <= i64::from(Fix::from_int(3).raw()).pow(2),
                "unit {} at ({x},{y})",
                u.id
            );
        }
    }

    #[test]
    fn idle_units_defend_against_hostiles_in_range_only() {
        // Arrange: idle armed defender (range 3), an enemy 2 cells away,
        // a neutral 1 cell away and a second enemy out of range (10 cells).
        let mut sim = Sim::new(6);
        let spawn = |x: i32, hp: i64, weapon| Command::Spawn {
            kind: 0,
            x: Fix::from_int(x),
            y: Fix::ZERO,
            loco: terrain::Loco::Track,
            speed: Fix::from_int(1),
            hp,
            armor: 0,
            weapon,
        };
        sim.step(&[
            spawn(0, 500, Some(gun(50, 1, 3, Fix::ZERO))),
            spawn(2, 40, None),
            spawn(1, 40, None),
            spawn(10, 40, None),
            Command::AssignOwner {
                id: 1,
                owner: OWNER_PLAYER,
            },
            Command::AssignOwner {
                id: 2,
                owner: OWNER_ENEMY,
            },
            Command::AssignOwner {
                id: 3,
                owner: OWNER_NEUTRAL,
            },
            Command::AssignOwner {
                id: 4,
                owner: OWNER_ENEMY,
            },
        ]);
        // Act
        for _ in 0..50 {
            sim.step(&[]);
        }
        // Assert: in-range enemy killed; neutral and distant enemy untouched;
        // the defender never left its post.
        let ids: Vec<u32> = sim.units().iter().map(|u| u.id).collect();
        assert_eq!(ids, vec![1, 3, 4]);
        assert_eq!(sim.units()[0].x, Fix::ZERO);
        assert!(sim.units()[0].attack.is_none());
    }

    #[test]
    fn harvester_loop_earns_credits_and_depletes_ore() {
        let mut grid = TerrainGrid::new(0, 0, 8, 2);
        grid.set_resource(6, 0, terrain::Overlay::Ore, 40);
        let mut sim = Sim::new(4);
        sim.set_terrain(grid);
        let centre = |c: i32| Fix::from_int(c) + Fix::from_raw(128);
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: centre(0),
                y: centre(0),
                loco: terrain::Loco::Track,
                speed: Fix::from_int(256),
                hp: 100,
                armor: 0,
                weapon: None,
            },
            Command::Spawn {
                kind: 0,
                x: centre(1),
                y: centre(0),
                loco: terrain::Loco::Track,
                speed: Fix::from_int(256),
                hp: 100,
                armor: 0,
                weapon: None,
            },
            Command::AssignRole {
                id: 1,
                role: Role::Refinery,
            },
            Command::AssignRole {
                id: 2,
                role: Role::Harvester,
            },
            Command::AssignOwner {
                id: 1,
                owner: OWNER_PLAYER,
            },
            Command::AssignOwner {
                id: 2,
                owner: OWNER_PLAYER,
            },
            Command::Harvest { id: 2 },
        ]);
        // Move the refinery marker onto the spawn area cell (0,0) is fine;
        // the harvester starts at (1,0). Run until two unloads happened.
        let mut unloads = 0;
        for _ in 0..4000 {
            sim.step(&[]);
            unloads += sim
                .drain_events()
                .iter()
                .filter(|e| matches!(e, Event::Unloaded { .. }))
                .count();
            if unloads >= 2 {
                break;
            }
        }
        assert_eq!(unloads, 2, "harvester completed two loops");
        assert_eq!(
            sim.credits(),
            2 * i64::from(HARVESTER_CAPACITY) * BAIL_ORE,
            "20 bails x 25cr per loop"
        );
        let g = sim.terrain().unwrap();
        assert_eq!(
            g.cell(6, 0).map(|c| c.resource),
            Some(0),
            "ore cell depleted"
        );
        assert_eq!(
            g.cell(6, 0).map(|c| c.overlay),
            Some(terrain::Overlay::None)
        );
    }

    #[test]
    fn harvester_unloads_beside_retail_sized_refinery() {
        let mut sim = Sim::new(1);
        let mut grid = TerrainGrid::new(0, 0, 16, 12);
        grid.set_resource(12, 8, terrain::Overlay::Ore, 40);
        sim.set_terrain(grid);
        sim.step(&[Command::PlaceBuilding {
            kind: 0,
            x: 3,
            y: 3,
            w: 4,
            h: 3,
            hp: 1000,
            armor: 0,
            power: 0,
            owner: OWNER_PLAYER,
            role: Role::Refinery,
        }]);
        let id = sim.spawn_owned(
            1,
            &UnitSpec {
                loco: terrain::Loco::Track,
                speed: Fix::from_raw(128),
                hp: 100,
                armor: 0,
                weapon: None,
                role: Role::Harvester,
            },
            (8, 6),
            OWNER_PLAYER,
        );
        let mut unloads = 0;
        let mut resumed = None;
        let mut observed = false;
        for _ in 0..2000 {
            sim.step(&[]);
            if let Some(copy) = &mut resumed {
                Sim::step(copy, &[]);
                assert_eq!(sim.state_hash(), copy.state_hash());
            }
            if let Some(elapsed) = sim.refinery_unloading(1) {
                observed = true;
                let u = sim.units().iter().find(|u| u.id == id).unwrap();
                let cell = (u.x.to_int_floor() as u16, u.y.to_int_floor() as u16);
                assert!(
                    !sim.terrain()
                        .unwrap()
                        .cell(cell.0, cell.1)
                        .unwrap()
                        .building
                );
                assert!(sim.beside_refinery(OWNER_PLAYER, cell));
                assert_eq!(sim.refinery_unloading(id), None);
                if elapsed == 10 && resumed.is_none() {
                    let hash = sim.state_hash();
                    resumed = Some(Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap());
                    assert_eq!(sim.state_hash(), hash, "observing and saving are read-only");
                }
            }
            for event in sim.drain_events() {
                if let Event::Unloaded {
                    id: unit, value, ..
                } = event
                {
                    assert_eq!(unit, id);
                    assert_eq!(value, 500);
                    unloads += 1;
                }
            }
            if unloads == 2 {
                break;
            }
        }
        assert_eq!(unloads, 2, "full-size refinery must accept both loads");
        assert!(observed && resumed.is_some());
        assert_eq!(sim.refinery_unloading(1), None);
        assert_eq!(sim.credits(), 1000);
        assert_eq!(sim.terrain().unwrap().cell(12, 8).unwrap().resource, 0);
    }

    #[test]
    fn unloading_requires_a_live_owned_refinery_at_the_perimeter() {
        let mut sim = Sim::new(1);
        sim.set_terrain(TerrainGrid::new(0, 0, 16, 12));
        let refinery = |x, owner| Command::PlaceBuilding {
            kind: 0,
            x,
            y: 3,
            w: 4,
            h: 3,
            hp: 1000,
            armor: 0,
            power: 0,
            owner,
            role: Role::Refinery,
        };
        sim.step(&[refinery(3, OWNER_PLAYER), refinery(10, OWNER_ENEMY)]);
        assert!(sim.beside_refinery(OWNER_PLAYER, (2, 3)));
        assert!(sim.beside_refinery(OWNER_PLAYER, (7, 6)));
        assert!(
            !sim.beside_refinery(OWNER_PLAYER, (5, 4)),
            "inside footprint"
        );
        assert!(!sim.beside_refinery(OWNER_PLAYER, (9, 3)), "enemy refinery");
        let id = sim.spawn_owned(1, &tank_spec(), (7, 4), OWNER_PLAYER);
        let u = sim.units.last_mut().unwrap();
        u.role = Role::Harvester;
        u.harvest = Harvest::Unloading(0);
        u.cargo = 500;
        u.bails = HARVESTER_CAPACITY;
        assert_eq!(sim.refinery_unloading(1), Some(UNLOAD_TICKS));
        assert_eq!(sim.refinery_unloading(2), None);
        sim.step(&[Command::Destroy { id: 1 }]);
        assert_eq!(
            sim.credits(),
            0,
            "destroyed refineries cannot receive credits"
        );
        assert_eq!(sim.units.last().unwrap().cargo, 500);
        assert_eq!(sim.refinery_unloading(1), None);
        for _ in 0..40 {
            sim.step(&[]);
        }
        assert_eq!(sim.units.last().unwrap().cargo, 500);
        assert_eq!(sim.credits(), 0);
        sim.step(&[refinery(3, OWNER_PLAYER)]);
        for _ in 0..40 {
            sim.step(&[]);
        }
        assert_eq!(sim.credits(), 500, "delivery resumes after rebuilding");
        assert_eq!(sim.units.iter().find(|u| u.id == id).unwrap().cargo, 0);
    }

    #[test]
    fn enemy_harvester_uses_own_refinery_and_account() {
        let mut sim = Sim::new(1);
        let mut grid = TerrainGrid::new(0, 0, 12, 4);
        grid.set_resource(9, 1, terrain::Overlay::Ore, 20);
        sim.set_terrain(grid);
        let refinery = |x, owner| Command::PlaceBuilding {
            kind: 0,
            x,
            y: 0,
            w: 2,
            h: 2,
            hp: 1000,
            armor: 0,
            power: 0,
            owner,
            role: Role::Refinery,
        };
        sim.step(&[refinery(0, OWNER_PLAYER), refinery(5, OWNER_ENEMY)]);
        assert_eq!(sim.refinery_cell(OWNER_ENEMY), Some((6, 1)));
        assert_eq!(sim.refinery_cell(OWNER_NEUTRAL), None);
        sim.spawn_owned(
            1,
            &UnitSpec {
                loco: terrain::Loco::Track,
                speed: Fix::from_raw(128),
                hp: 100,
                armor: 0,
                weapon: None,
                role: Role::Harvester,
            },
            (8, 1),
            OWNER_ENEMY,
        );
        for _ in 0..1000 {
            sim.step(&[]);
        }
        assert_eq!(sim.credits(), 0, "enemy deliveries never fund the player");
        assert_eq!(sim.credits_of(OWNER_ENEMY), 500);
        assert_eq!(sim.terrain().unwrap().cell(9, 1).unwrap().resource, 0);
    }

    #[test]
    fn buildings_power_and_footprint_lifecycle() {
        let mut sim = Sim::new(6);
        sim.set_terrain(TerrainGrid::new(0, 0, 8, 8));
        // Power plant (+200) and two consumers (-150 each via two placements).
        sim.step(&[
            Command::PlaceBuilding {
                kind: 0,
                x: 0,
                y: 0,
                w: 2,
                h: 2,
                hp: 1000,
                armor: 0,
                power: 200,
                owner: 0,
                role: Role::Combat,
            },
            Command::PlaceBuilding {
                kind: 0,
                x: 4,
                y: 0,
                w: 2,
                h: 2,
                hp: 500,
                armor: 0,
                power: -150,
                owner: 0,
                role: Role::Combat,
            },
        ]);
        assert_eq!(sim.power(), (200, 150));
        assert!(!sim.low_power());
        assert_eq!(sim.production_factor(), Fix::ONE);
        let g = sim.terrain().unwrap();
        assert!(!g.passable(terrain::Loco::Foot, 0, 0), "footprint blocks");
        // Add drain past production: low power slows production.
        sim.step(&[Command::PlaceBuilding {
            kind: 0,
            x: 4,
            y: 4,
            w: 2,
            h: 2,
            hp: 500,
            armor: 0,
            power: -150,
            owner: 0,
            role: Role::Combat,
        }]);
        assert!(sim.low_power());
        let f = sim.production_factor();
        assert!(
            f < Fix::ONE && f >= Fix::from_raw(128),
            "low-power factor interpolates: {f:?}"
        );
        // A tank destroys the consumer at (4,4); footprint frees.
        let mut verses = [yr_rules::Decimal::ZERO; crate::combat::ARMOR_COUNT];
        verses[0] = yr_rules::Decimal::ONE;
        let w = WeaponSpec {
            source: None,
            damage: 250,
            rof_ticks: 1,
            range: Fix::from_int(20),
            projectile_speed: Fix::ZERO,
            verses,
        };
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: Fix::from_raw(128),
                y: Fix::from_int(6),
                loco: terrain::Loco::Track,
                speed: Fix::from_int(4),
                hp: 500,
                armor: 0,
                weapon: Some(w),
            },
            Command::Attack { id: 4, target: 3 },
        ]);
        let mut died = false;
        for _ in 0..50 {
            sim.step(&[]);
            died |= sim
                .drain_events()
                .iter()
                .any(|e| matches!(e, Event::Died { id: 3 }));
            if died {
                break;
            }
        }
        assert!(died, "building destroyed");
        assert!(
            sim.terrain().unwrap().passable(terrain::Loco::Foot, 4, 4),
            "footprint freed"
        );
        assert_eq!(sim.power(), (200, 150), "drain removed with the building");
    }

    #[test]
    fn flight_segments_cover_travel_and_arrival_after_restore_and_shooter_loss() {
        let mut sim = Sim::new(42);
        let mut weapon = gun(50, 100, 10, Fix::ONE);
        weapon.source = Some(123);
        let spawn = |x, weapon| Command::Spawn {
            kind: 0,
            x: Fix::from_int(x),
            y: Fix::ZERO,
            loco: terrain::Loco::Track,
            speed: Fix::ZERO,
            hp: 200,
            armor: 0,
            weapon,
        };
        sim.step(&[
            spawn(0, Some(weapon)),
            spawn(4, None),
            Command::AssignOwner {
                id: 2,
                owner: OWNER_ENEMY,
            },
            Command::Attack { id: 1, target: 2 },
        ]);
        let launch = sim.drain_events();
        assert!(launch.iter().any(|e| matches!(e, Event::ProjectileMoved { from, to, .. } if *from == (Fix::ZERO,Fix::ZERO) && *to == (Fix::ONE,Fix::ZERO))));
        let mut lost_target = sim.clone();
        lost_target.step(&[Command::Destroy { id: 2 }]);
        assert!(lost_target.projectiles().is_empty());
        assert!(!lost_target
            .drain_events()
            .iter()
            .any(|e| matches!(e, Event::ProjectileMoved { .. })));
        sim.step(&[Command::Destroy { id: 1 }]);
        sim.drain_events();
        let save = sim.save_bytes().unwrap();
        let mut restored = Sim::from_save_bytes(&save).unwrap();
        let mut last = (sim.projectiles()[0].x, sim.projectiles()[0].y);
        let mut hit = false;
        for _ in 0..4 {
            sim.step(&[]);
            restored.step(&[]);
            let events = sim.drain_events();
            assert_eq!(events, restored.drain_events());
            assert_eq!(sim.state_hash(), restored.state_hash());
            assert_eq!(sim.save_bytes().unwrap(), restored.save_bytes().unwrap());
            let movement = events
                .iter()
                .find_map(|e| match e {
                    Event::ProjectileMoved { from, to, origin } => Some((*from, *to, *origin)),
                    _ => None,
                })
                .unwrap();
            assert_eq!(movement.0, last);
            assert_eq!(movement.2.unwrap().weapon, Some(123));
            last = movement.1;
            if let Some(index) = events
                .iter()
                .position(|e| matches!(e, Event::Impact { .. }))
            {
                assert!(events[..index]
                    .iter()
                    .any(|e| matches!(e, Event::ProjectileMoved { .. })));
                assert_eq!(last, (Fix::from_int(4), Fix::ZERO));
                assert!(sim.projectiles().is_empty());
                hit = true;
                break;
            }
        }
        assert!(hit);
        sim.step(&[]);
        assert!(!sim
            .drain_events()
            .iter()
            .any(|e| matches!(e, Event::ProjectileMoved { .. })));
    }

    #[test]
    fn projectile_travel_delays_the_hit() {
        let mut sim = Sim::new(5);
        // 256 leptons/tick over 4 cells (1024 leptons): hits land ~4 ticks after firing.
        let w = gun(50, 1, 10, Fix::from_int(1));
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: Fix::ZERO,
                y: Fix::ZERO,
                loco: terrain::Loco::Track,
                speed: Fix::from_int(4),
                hp: 200,
                armor: 0,
                weapon: Some(w),
            },
            Command::Spawn {
                kind: 0,
                x: Fix::from_int(4),
                y: Fix::ZERO,
                loco: terrain::Loco::Track,
                speed: Fix::from_int(4),
                hp: 40,
                armor: 0,
                weapon: None,
            },
            Command::Attack { id: 1, target: 2 },
        ]);
        sim.step(&[]);
        let first = sim.drain_events();
        assert!(
            first.iter().any(|e| matches!(e, Event::Fired { .. })),
            "fired at once: {first:?}"
        );
        assert!(
            !first.iter().any(|e| matches!(e, Event::Hit { .. })),
            "but not yet hit: {first:?}"
        );
        let mut died_at = None;
        for t in 0..30 {
            sim.step(&[]);
            if sim
                .drain_events()
                .iter()
                .any(|e| matches!(e, Event::Died { .. }))
            {
                died_at = Some(t);
                break;
            }
        }
        assert!(died_at.is_some(), "projectile eventually kills");
    }

    fn tank_spec() -> UnitSpec {
        UnitSpec {
            loco: terrain::Loco::Track,
            speed: Fix::from_int(1),
            hp: 300,
            armor: 0,
            weapon: None,
            role: Role::Combat,
        }
    }

    #[test]
    fn factory_draws_credits_stalls_cancels_and_rolls_out() {
        let mut sim = Sim::new(9);
        sim.set_terrain(TerrainGrid::new(0, 0, 12, 12));
        sim.set_credits(150);
        let produce = |kind| Command::Produce {
            factory: Factory::Vehicle,
            kind,
            product: Product::Unit(tank_spec()),
            cost: 100,
            ticks: 10,
        };
        // No war factory yet: the order waits and draws nothing.
        sim.step(&[produce(0)]);
        for _ in 0..20 {
            sim.step(&[]);
        }
        assert_eq!((sim.credits(), sim.production()[0].progress), (150, 0));
        sim.step(&[Command::PlaceBuilding {
            kind: 0,
            x: 2,
            y: 2,
            w: 3,
            h: 3,
            hp: 1000,
            armor: 0,
            power: 0,
            owner: OWNER_PLAYER,
            role: Role::WarFactory,
        }]);
        let factory_id = sim.buildings()[0].id;
        // 10 ticks at full speed; the first runs in the placement tick.
        let mut produced = Vec::new();
        for _ in 0..12 {
            sim.step(&[]);
            for e in sim.drain_events() {
                if let Event::Produced { id, kind } = e {
                    produced.push((id, kind));
                }
            }
        }
        assert_eq!(produced, vec![(factory_id + 1, 0)]);
        assert_eq!(sim.credits(), 50);
        let u = &sim.units()[0];
        assert_eq!(u.owner, OWNER_PLAYER);
        // Rolled out just below the footprint (rows 2..5), not on it.
        assert_eq!((u.x.to_int_floor(), u.y.to_int_floor()), (3, 5));
        // 50 credits left: a second 100-credit build stalls halfway.
        sim.step(&[produce(1)]);
        for _ in 0..30 {
            sim.step(&[]);
        }
        let o = sim.production()[0];
        assert_eq!((o.paid, sim.credits()), (50, 0));
        assert!(o.progress < o.total());
        // Cancel refunds what was drawn.
        sim.step(&[Command::CancelProduce {
            factory: Factory::Vehicle,
            kind: 1,
        }]);
        assert!(sim.production().is_empty());
        assert_eq!(sim.credits(), 50);
    }

    #[test]
    fn production_is_deterministic_and_hashed() {
        let run = |cancel: bool| {
            let mut sim = Sim::new(3);
            sim.set_terrain(TerrainGrid::new(0, 0, 8, 8));
            sim.set_credits(1000);
            sim.step(&[
                Command::PlaceBuilding {
                    kind: 0,
                    x: 0,
                    y: 0,
                    w: 2,
                    h: 2,
                    hp: 500,
                    armor: 0,
                    power: 0,
                    owner: OWNER_PLAYER,
                    role: Role::Barracks,
                },
                Command::Produce {
                    factory: Factory::Infantry,
                    kind: 2,
                    product: Product::Unit(tank_spec()),
                    cost: 200,
                    ticks: 50,
                },
            ]);
            for t in 0..20 {
                let cmds = if cancel && t == 5 {
                    vec![Command::CancelProduce {
                        factory: Factory::Infantry,
                        kind: 2,
                    }]
                } else {
                    Vec::new()
                };
                sim.step(&cmds);
            }
            sim.state_hash()
        };
        assert_eq!(run(false), run(false));
        assert_ne!(run(false), run(true));
    }

    fn yard_at(x: u16, y: u16) -> Command {
        Command::PlaceBuilding {
            kind: 0,
            x,
            y,
            w: 2,
            h: 2,
            hp: 1000,
            armor: 0,
            power: 0,
            owner: OWNER_PLAYER,
            role: Role::ConstructionYard,
        }
    }

    fn plant_spec() -> BuildingSpec {
        BuildingSpec {
            w: 2,
            h: 2,
            hp: 750,
            armor: 0,
            power: 200,
            role: Role::Combat,
            adjacent: 2,
            base_normal: true,
            free_unit: None,
        }
    }

    fn build(kind: u16, spec: BuildingSpec) -> Command {
        Command::Produce {
            factory: Factory::Building,
            kind,
            product: Product::Building(spec),
            cost: 100,
            ticks: 10,
        }
    }

    /// Yard at (2,2)-(3,3), a finished 2x2 structure of kind 7 waiting.
    fn sim_with_ready(spec: BuildingSpec) -> Sim {
        let mut sim = Sim::new(5);
        sim.set_terrain(TerrainGrid::new(0, 0, 16, 16));
        sim.set_credits(1000);
        sim.step(&[yard_at(2, 2), build(7, spec)]);
        for _ in 0..12 {
            sim.step(&[]);
        }
        sim
    }

    #[test]
    fn structure_builds_then_waits_for_placement() {
        let mut sim = Sim::new(5);
        sim.set_terrain(TerrainGrid::new(0, 0, 16, 16));
        sim.set_credits(1000);
        // No construction yard: the order waits and draws nothing.
        sim.step(&[build(7, plant_spec())]);
        sim.step(&[]);
        assert_eq!((sim.credits(), sim.production()[0].progress), (1000, 0));
        sim.step(&[yard_at(2, 2)]);
        let mut ready = 0;
        for _ in 0..30 {
            sim.step(&[]);
            for e in sim.drain_events() {
                assert!(
                    !matches!(e, Event::Produced { .. }),
                    "structures never roll out"
                );
                if e == (Event::Ready { kind: 7 }) {
                    ready += 1;
                }
            }
        }
        assert_eq!(ready, 1, "Ready fires once");
        assert_eq!(sim.credits(), 900, "full cost drawn while building");
        assert!(sim.production()[0].done(), "held at the head of the queue");
        assert_eq!(sim.buildings().len(), 1);
    }

    #[test]
    fn placement_rejects_bad_spots_and_accepts_adjacent_ground() {
        let mut sim = sim_with_ready(plant_spec());
        let place = |x, y| Command::PlaceProduced { kind: 7, x, y };
        // A neutral tank parked at (5,2).
        sim.step(&[Command::Spawn {
            kind: 1,
            x: Fix::from_int(5) + Fix::from_raw(128),
            y: Fix::from_int(2) + Fix::from_raw(128),
            loco: terrain::Loco::Track,
            speed: Fix::ZERO,
            hp: 100,
            armor: 0,
            weapon: None,
        }]);
        let spec = plant_spec();
        assert!(!sim.can_place(OWNER_PLAYER, &spec, 2, 2), "on the yard");
        assert!(!sim.can_place(OWNER_PLAYER, &spec, 5, 2), "on a unit");
        assert!(
            !sim.can_place(OWNER_PLAYER, &spec, 7, 2),
            "3 cells from the base"
        );
        assert!(
            !sim.can_place(OWNER_PLAYER, &spec, 15, 2),
            "off the grid edge"
        );
        assert!(
            !sim.can_place(OWNER_ENEMY, &spec, 6, 2),
            "not the enemy's base"
        );
        assert!(
            sim.can_place(OWNER_PLAYER, &spec, 6, 2),
            "2 empty cells away"
        );
        for bad in [place(2, 2), place(5, 2), place(7, 2), place(15, 2)] {
            sim.step(&[bad]);
        }
        sim.step(&[Command::PlaceProduced {
            kind: 8,
            x: 6,
            y: 2,
        }]);
        assert_eq!(sim.buildings().len(), 1, "every bad placement ignored");
        assert_eq!(sim.production().len(), 1);
        sim.step(&[place(6, 2)]);
        let placed: Vec<_> = sim
            .drain_events()
            .into_iter()
            .filter(|e| matches!(e, Event::Placed { kind: 7, .. }))
            .collect();
        assert_eq!(placed.len(), 1);
        assert!(sim.production().is_empty());
        let b = sim.buildings().last().unwrap();
        assert_eq!((b.cell, b.size, b.owner), ((6, 2), (2, 2), OWNER_PLAYER));
        assert_eq!(sim.power(), (200, 0));
        let g = sim.terrain().unwrap();
        assert!(g.cell(7, 3).unwrap().building, "footprint blocks movement");
    }

    #[test]
    fn ready_structures_cannot_cover_an_absent_tile() {
        let spec = plant_spec();
        let mut sim = sim_with_ready(spec);
        assert!(sim.can_place(OWNER_PLAYER, &spec, 5, 2));
        let mut grid = TerrainGrid::from_tiles(
            0,
            0,
            16,
            16,
            (0..16).flat_map(|y| {
                (0..16).filter_map(move |x| ((x, y) != (6, 3)).then_some((x, y, 0, 13, 0)))
            }),
        );
        for b in sim.buildings() {
            grid.set_building(b.cell.0, b.cell.1, b.size.0, b.size.1, true);
        }
        sim.set_terrain(grid);
        assert!(!sim.can_place(OWNER_PLAYER, &spec, 5, 2));
        sim.step(&[Command::PlaceProduced {
            kind: 7,
            x: 5,
            y: 2,
        }]);
        assert_eq!(sim.buildings().len(), 1);
        assert!(
            sim.production()[0].done(),
            "rejected placement keeps the paid structure"
        );
    }

    #[test]
    fn placement_needs_flat_clear_land_and_base_buildings() {
        let mut sim = sim_with_ready(BuildingSpec {
            base_normal: false,
            ..plant_spec()
        });
        let spec = plant_spec();
        let mut grid = sim.terrain().unwrap().clone();
        grid.set_resource(5, 2, terrain::Overlay::Ore, 3);
        grid.cell_mut(5, 5).unwrap().ramp = 1;
        grid.cell_mut(0, 1).unwrap().ground = terrain::Ground::Water;
        sim.set_terrain(grid);
        assert!(
            !sim.can_place(OWNER_PLAYER, &spec, 4, 2),
            "ore under the footprint"
        );
        assert!(!sim.can_place(OWNER_PLAYER, &spec, 4, 5), "sloped cell");
        assert!(!sim.can_place(OWNER_PLAYER, &spec, 0, 0), "water");
        // A defence (BaseNormal=no) below the yard (rows 4-5) does not
        // extend the base: row 7 is 1 cell from it but 3 from the yard.
        sim.step(&[Command::PlaceProduced {
            kind: 7,
            x: 2,
            y: 4,
        }]);
        assert_eq!(sim.buildings().len(), 2, "defence placed");
        assert!(!sim.can_place(OWNER_PLAYER, &spec, 2, 7));
        assert!(
            sim.can_place(OWNER_PLAYER, &spec, 2, 6),
            "2 cells from the yard"
        );
    }

    #[test]
    fn power_counts_only_the_owners_buildings() {
        let mut sim = Sim::new(4);
        sim.set_terrain(TerrainGrid::new(0, 0, 16, 16));
        let plant = |x, power, owner| Command::PlaceBuilding {
            kind: 0,
            x,
            y: 0,
            w: 2,
            h: 2,
            hp: 500,
            armor: 0,
            power,
            owner,
            role: Role::Combat,
        };
        sim.step(&[
            plant(0, 200, OWNER_PLAYER),
            plant(2, -150, OWNER_PLAYER),
            plant(4, -500, OWNER_ENEMY),
            plant(6, -300, OWNER_NEUTRAL),
        ]);
        assert_eq!(sim.power(), (200, 150));
        assert_eq!(sim.power_of(OWNER_ENEMY), (0, 500));
        assert!(!sim.low_power(), "other owners' drain is not the player's");
        assert_eq!(sim.production_factor(), Fix::ONE);
    }

    #[test]
    fn damaged_generator_scales_power_and_save_load_preserves_it() {
        let mut sim = Sim::new(4);
        sim.set_terrain(TerrainGrid::new(0, 0, 16, 16));
        let place = |x, power, owner| Command::PlaceBuilding {
            kind: 0,
            x,
            y: 0,
            w: 2,
            h: 2,
            hp: 1000,
            armor: 0,
            power,
            owner,
            role: Role::Combat,
        };
        sim.step(&[
            place(0, 200, OWNER_PLAYER),
            place(2, -150, OWNER_PLAYER),
            place(4, 200, OWNER_ENEMY),
        ]);
        assert_eq!(sim.power(), (200, 150));
        assert!(!sim.low_power());
        // Half health on the player plant halves its output. The base
        // goes low-power and production slows; drain stays at 150 even
        // when the consumer itself is damaged.
        sim.buildings[0].hp = 500;
        sim.buildings[1].hp = 10;
        assert_eq!(sim.power(), (100, 150));
        assert!(sim.low_power());
        assert!(sim.production_factor() < Fix::ONE);
        // Enemy output is isolated and still at full health.
        assert_eq!(sim.power_of(OWNER_ENEMY), (200, 0));
        sim.buildings[2].hp = 250;
        assert_eq!(sim.power_of(OWNER_ENEMY), (50, 0));
        assert_eq!(sim.power(), (100, 150), "enemy damage does not leak");
        // Save/load round-trips the damaged totals and the state hash.
        let bytes = sim.save_bytes().unwrap();
        let restored = Sim::from_save_bytes(&bytes).unwrap();
        assert_eq!(restored.power(), (100, 150));
        assert_eq!(restored.power_of(OWNER_ENEMY), (50, 0));
        assert_eq!(restored.state_hash(), sim.state_hash());
        // Destroying the damaged plant removes its remaining output.
        let plant_id = sim.buildings[0].id;
        sim.step(&[Command::Destroy { id: plant_id }]);
        assert_eq!(sim.power(), (0, 150));
    }

    #[test]
    fn finished_structure_cancels_for_full_refund() {
        let mut sim = sim_with_ready(plant_spec());
        assert_eq!(sim.credits(), 900);
        sim.step(&[Command::CancelProduce {
            factory: Factory::Building,
            kind: 7,
        }]);
        assert_eq!(sim.credits(), 1000);
        assert!(sim.production().is_empty());
    }

    #[test]
    fn free_unit_rolls_out_and_starts_harvesting() {
        let harvester = UnitSpec {
            role: Role::Harvester,
            ..tank_spec()
        };
        let mut sim = sim_with_ready(BuildingSpec {
            role: Role::Refinery,
            free_unit: Some((3, harvester)),
            ..plant_spec()
        });
        let mut grid = sim.terrain().unwrap().clone();
        grid.set_resource(12, 12, terrain::Overlay::Ore, 5);
        sim.set_terrain(grid);
        sim.step(&[Command::PlaceProduced {
            kind: 7,
            x: 2,
            y: 5,
        }]);
        let u = sim.units().last().expect("free unit");
        assert_eq!(
            (u.kind, u.owner, u.role),
            (3, OWNER_PLAYER, Role::Harvester)
        );
        // Rolled out below the refinery footprint (rows 5-6), not on it.
        assert!(u.y.to_int_floor() >= 7);
        let id = u.id;
        let start = sim.credits();
        for _ in 0..600 {
            sim.step(&[]);
        }
        assert!(
            sim.credits() > start,
            "harvester delivered ore to the refinery"
        );
        assert!(sim.units().iter().any(|u| u.id == id));
    }

    #[test]
    fn placement_is_deterministic_and_hashed() {
        let run = |place: bool| {
            let mut sim = sim_with_ready(plant_spec());
            let cmds = if place {
                vec![Command::PlaceProduced {
                    kind: 7,
                    x: 4,
                    y: 2,
                }]
            } else {
                Vec::new()
            };
            sim.step(&cmds);
            for _ in 0..5 {
                sim.step(&[]);
            }
            sim.state_hash()
        };
        assert_eq!(run(true), run(true));
        assert_ne!(run(true), run(false));
    }
}
