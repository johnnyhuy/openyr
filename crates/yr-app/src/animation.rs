//! Presentation clocks for infantry. Observe movement and shots after
//! each sim step; drawing a frame never advances these clocks.

use std::collections::BTreeMap;
use yr_render::sprites::InfantryAnimation;
use yr_sim::{Fix, Sim};

struct Observation {
    position: (Fix, Fix),
    shots: u32,
    moving: bool,
    changed_at: u64,
    fired_at: Option<u64>,
}

#[derive(Default)]
pub struct UnitAnimations {
    units: BTreeMap<u32, Observation>,
    tick: Option<u64>,
}

impl UnitAnimations {
    /// Sample once per tick, including once before the first step to
    /// establish starting positions. Dead units release their clocks.
    pub fn update(&mut self, sim: &Sim) {
        let tick = sim.tick();
        if self.tick == Some(tick) {
            return;
        }
        self.tick = Some(tick);
        self.units
            .retain(|id, _| sim.units().iter().any(|u| u.id == *id));
        for u in sim.units() {
            let position = (u.x, u.y);
            let previous = self.units.entry(u.id).or_insert(Observation {
                position,
                shots: u.shots,
                moving: false,
                changed_at: tick,
                fired_at: None,
            });
            let moving = position != previous.position;
            if moving != previous.moving {
                previous.moving = moving;
                previous.changed_at = tick;
            }
            if u.shots != previous.shots {
                previous.fired_at = Some(tick);
            }
            previous.position = position;
            previous.shots = u.shots;
        }
    }

    pub fn get(&self, id: u32) -> InfantryAnimation {
        let Some(state) = self.units.get(&id) else {
            return InfantryAnimation::default();
        };
        let tick = self.tick.unwrap_or(0);
        InfantryAnimation {
            moving: state.moving,
            elapsed: tick.saturating_sub(state.changed_at),
            since_shot: state.fired_at.map(|t| tick.saturating_sub(t)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_sim::{Command, WeaponSpec};

    fn spawn(x: i32, weapon: Option<WeaponSpec>) -> Command {
        Command::Spawn {
            kind: 0,
            x: Fix::from_int(x),
            y: Fix::ZERO,
            loco: yr_sim::terrain::Loco::Foot,
            speed: Fix::from_raw(128),
            hp: 1000,
            armor: 0,
            weapon,
        }
    }

    #[test]
    fn movement_uses_positions_and_freezes_between_sim_ticks() {
        let mut sim = Sim::new(1);
        sim.step(&[spawn(0, None)]);
        let mut animation = UnitAnimations::default();
        animation.update(&sim);
        sim.step(&[Command::Move {
            id: 1,
            x: Fix::from_int(2),
            y: Fix::ZERO,
        }]);
        animation.update(&sim);
        assert!(animation.get(1).moving);
        sim.step(&[]);
        animation.update(&sim);
        assert_eq!(animation.get(1).elapsed, 1);
        let frozen = animation.get(1);
        for _ in 0..10 {
            animation.update(&sim);
            assert_eq!(animation.get(1), frozen);
        }
        sim.step(&[Command::Stop { id: 1 }]);
        animation.update(&sim);
        assert!(!animation.get(1).moving);
        assert_eq!(animation.get(1).elapsed, 0);
        // A move order with zero speed must remain standing.
        let mut immobile = Sim::new(2);
        let mut command = spawn(0, None);
        if let Command::Spawn { speed, .. } = &mut command {
            *speed = Fix::ZERO;
        }
        immobile.step(&[command]);
        let mut animation = UnitAnimations::default();
        animation.update(&immobile);
        immobile.step(&[Command::Move {
            id: 1,
            x: Fix::from_int(4),
            y: Fix::ZERO,
        }]);
        animation.update(&immobile);
        assert!(!animation.get(1).moving);
    }

    #[test]
    fn shots_restart_the_clip_and_dead_units_release_their_clocks() {
        let mut sim = Sim::new(3);
        sim.step(&[
            spawn(
                0,
                Some(WeaponSpec {
                    damage: 1,
                    rof_ticks: 4,
                    range: Fix::from_int(10),
                    projectile_speed: Fix::ZERO,
                    verses: [yr_rules::Decimal::ONE; 11],
                }),
            ),
            spawn(2, None),
        ]);
        let mut animation = UnitAnimations::default();
        animation.update(&sim);
        sim.step(&[Command::Attack { id: 1, target: 2 }]);
        animation.update(&sim);
        assert_eq!(animation.get(1).since_shot, Some(0));
        sim.step(&[]);
        animation.update(&sim);
        assert_eq!(animation.get(1).since_shot, Some(1));
        for _ in 0..3 {
            sim.step(&[]);
            animation.update(&sim);
        }
        assert_eq!(animation.get(1).since_shot, Some(0));
        sim.step(&[Command::Remove { id: 1 }]);
        animation.update(&sim);
        assert!(!animation.units.contains_key(&1));
    }
}
