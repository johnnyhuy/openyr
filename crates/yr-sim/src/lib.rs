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
mod rng;

pub use fix::Fix;
pub use rng::Rng;

/// One cell in leptons: positions are in leptons (256 per cell), the
/// original game's sub-cell unit.
pub const LEPTONS_PER_CELL: i32 = 256;

/// Commands are the only way to mutate the sim from outside. Applied in
/// slice order at the start of a tick, so replays are command streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Spawn { x: Fix, y: Fix },
    Move { id: u32, x: Fix, y: Fix },
    Remove { id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    pub id: u32,
    pub x: Fix,
    pub y: Fix,
    pub target: Option<(Fix, Fix)>,
    /// Leptons per tick.
    pub speed: Fix,
}

#[derive(Debug, Clone)]
pub struct Sim {
    tick: u64,
    rng: Rng,
    next_id: u32,
    /// Kept in id order; all iteration is over this Vec.
    units: Vec<Unit>,
}

impl Sim {
    pub fn new(seed: u64) -> Sim {
        Sim { tick: 0, rng: Rng::new(seed), next_id: 1, units: Vec::new() }
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
                Command::Spawn { x, y } => {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.units.push(Unit {
                        id,
                        x,
                        y,
                        target: None,
                        speed: Fix::from_int(4),
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
        for u in &mut self.units {
            let Some((tx, ty)) = u.target else { continue };
            let (dx, dy) = (tx - u.x, ty - u.y);
            let step = u.speed;
            // Axis-wise clamped step (skeleton movement; pathfinding
            // replaces this in the movement issue).
            u.x += dx.clamp(-step, step);
            u.y += dy.clamp(-step, step);
            if u.x == tx && u.y == ty {
                u.target = None;
            }
        }
        self.tick += 1;
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
        h.write_u64(self.units.len() as u64);
        for u in &self.units {
            h.write_u64(u64::from(u.id));
            h.write_i64(i64::from(u.x.raw()));
            h.write_i64(i64::from(u.y.raw()));
            h.write_i64(i64::from(u.speed.raw()));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn script(sim: &mut Sim, ticks: u64) -> u64 {
        sim.step(&[Command::Spawn { x: Fix::from_int(0), y: Fix::from_int(0) }]);
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
        c.step(&[Command::Spawn { x: Fix::ZERO, y: Fix::ZERO }]);
        assert_ne!(a.state_hash(), c.state_hash());
    }

    #[test]
    fn removal_and_spawn_ids_are_stable() {
        let mut sim = Sim::new(7);
        sim.step(&[
            Command::Spawn { x: Fix::ZERO, y: Fix::ZERO },
            Command::Spawn { x: Fix::from_int(256), y: Fix::ZERO },
            Command::Remove { id: 1 },
            Command::Spawn { x: Fix::from_int(512), y: Fix::ZERO },
        ]);
        let ids: Vec<u32> = sim.units().iter().map(|u| u.id).collect();
        assert_eq!(ids, [2, 3], "ids never reused, order stable");
    }
}
