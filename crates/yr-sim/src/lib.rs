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

mod fix;
mod hash;
pub mod path;
mod rng;
pub mod terrain;

pub use fix::Fix;
pub use rng::Rng;
pub use terrain::TerrainGrid;

/// One cell in leptons: positions are in leptons (256 per cell), the
/// original game's sub-cell unit.
pub const LEPTONS_PER_CELL: i32 = 256;

/// Commands are the only way to mutate the sim from outside. Applied in
/// slice order at the start of a tick, so replays are command streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Spawn { x: Fix, y: Fix, loco: terrain::Loco, speed: Fix },
    Move { id: u32, x: Fix, y: Fix },
    Remove { id: u32 },
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
}

#[derive(Debug, Clone)]
pub struct Sim {
    tick: u64,
    rng: Rng,
    next_id: u32,
    /// Kept in id order; all iteration is over this Vec.
    units: Vec<Unit>,
    terrain: Option<TerrainGrid>,
}

impl Sim {
    pub fn new(seed: u64) -> Sim {
        Sim { tick: 0, rng: Rng::new(seed), next_id: 1, units: Vec::new(), terrain: None }
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
                Command::Spawn { x, y, loco, speed } => {
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
                    });
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
        self.tick += 1;
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
        h.finish()
    }
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
        sim.step(&[Command::Spawn { x: Fix::from_int(0), y: Fix::from_int(0), loco: terrain::Loco::Track, speed: Fix::from_int(4) }]);
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
        c.step(&[Command::Spawn { x: Fix::ZERO, y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2) }]);
        assert_ne!(a.state_hash(), c.state_hash());
    }

    #[test]
    fn removal_and_spawn_ids_are_stable() {
        let mut sim = Sim::new(7);
        sim.step(&[
            Command::Spawn { x: Fix::ZERO, y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2) },
            Command::Spawn { x: Fix::from_int(256), y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2) },
            Command::Remove { id: 1 },
            Command::Spawn { x: Fix::from_int(512), y: Fix::ZERO, loco: terrain::Loco::Foot, speed: Fix::from_int(2) },
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
        sim.step(&[Command::Spawn { x: centre(0), y: centre(0), loco: terrain::Loco::Track, speed: Fix::from_int(64) }]);
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
            Command::Spawn { x: centre(3), y: centre(1), loco: terrain::Loco::Track, speed: Fix::from_int(64) },
            Command::Spawn { x: centre(0), y: centre(1), loco: terrain::Loco::Track, speed: Fix::from_int(64) },
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
}
