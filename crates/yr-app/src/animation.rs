//! Presentation clocks sampled from simulation state. Drawing a frame
//! never advances a clock or changes the simulation.

use std::collections::BTreeMap;
use yr_render::sprites::InfantryAnimation;
use yr_sim::{Fix, Sim};

/// The sim's first owned factory of a role hosts its production queue.
/// Reading progress directly also freezes the clip when credits stall
/// and stops it when a finished structure waits for placement.
pub fn building_animation(
    sim: &Sim,
    building: &yr_sim::Building,
) -> yr_render::sprites::BuildingAnimation {
    let production = (|| {
        let host = sim
            .buildings()
            .iter()
            .find(|b| b.owner == building.owner && b.role == building.role)?;
        if host.id != building.id {
            return None;
        }
        let order = sim
            .production_of(building.owner)
            .iter()
            .find(|o| o.factory.role() == building.role)?;
        (!order.done()).then_some(order.progress.max(0) as u64 / 256)
    })();
    let (produced, drained) = sim.power_of(building.owner);
    yr_render::sprites::BuildingAnimation {
        elapsed: sim.tick(),
        production,
        unloading: sim.refinery_unloading(building.id).map(u64::from),
        damaged: i128::from(building.hp) * 2 <= i128::from(building.max_hp),
        powered: produced >= drained,
    }
}

/// Shared presentation inputs sampled once for one redraw. Rebuild even when
/// the tick is unchanged so editor/test mutations cannot leave stale state.
pub struct BuildingAnimations {
    tick: u64,
    owners: BTreeMap<u8, BuildingOwner>,
}

#[derive(Default)]
struct BuildingOwner {
    power: (i64, i64),
    hosts: Vec<(yr_sim::Role, u32, Option<u64>)>,
    unloading: Option<u64>,
}

impl BuildingAnimations {
    pub fn new(sim: &Sim) -> Self {
        let mut owners: BTreeMap<u8, BuildingOwner> = BTreeMap::new();
        for b in sim.buildings() {
            let owner = owners.entry(b.owner).or_default();
            let power = b.effective_power();
            if power >= 0 {
                owner.power.0 += power;
            } else {
                owner.power.1 -= power;
            }
            // Role has six variants, so this search stays bounded as bases grow.
            if !owner.hosts.iter().any(|h| h.0 == b.role) {
                let production = sim
                    .production_of(b.owner)
                    .iter()
                    .find(|o| o.factory.role() == b.role)
                    .filter(|o| !o.done())
                    .map(|o| o.progress.max(0) as u64 / 256);
                owner.hosts.push((b.role, b.id, production));
            }
        }
        for owner in owners.values_mut() {
            if let Some(host) = owner.hosts.iter().find(|h| h.0 == yr_sim::Role::Refinery) {
                // Keep the simulation's docking policy in one place. Query only
                // the first refinery per owner, never every building marker.
                owner.unloading = sim.refinery_unloading(host.1).map(u64::from);
            }
        }
        Self {
            tick: sim.tick(),
            owners,
        }
    }

    pub fn get(&self, b: &yr_sim::Building) -> yr_render::sprites::BuildingAnimation {
        let owner = self.owners.get(&b.owner);
        let host = owner.and_then(|o| o.hosts.iter().find(|h| h.0 == b.role && h.1 == b.id));
        yr_render::sprites::BuildingAnimation {
            elapsed: self.tick,
            production: host.and_then(|h| h.2),
            unloading: host
                .filter(|h| h.0 == yr_sim::Role::Refinery)
                .and_then(|_| owner?.unloading),
            damaged: i128::from(b.hp) * 2 <= i128::from(b.max_hp),
            powered: owner.is_none_or(|o| o.power.0 >= o.power.1),
        }
    }
}

struct Observation {
    position: (Fix, Fix),
    shots: u32,
    moving: bool,
    changed_at: u64,
    fired_at: Option<u64>,
    seen_at: u64,
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
        for u in sim.units() {
            let position = (u.x, u.y);
            let previous = self.units.entry(u.id).or_insert(Observation {
                position,
                shots: u.shots,
                moving: false,
                changed_at: tick,
                fired_at: None,
                seen_at: tick,
            });
            previous.seen_at = tick;
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
        // Prune once after observing live IDs, without rescanning the army
        // for every retained clock. Survivors keep their action phases.
        self.units.retain(|_, state| state.seen_at == tick);
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

    #[test]
    fn building_production_uses_host_progress_stalls_and_stops_when_ready() {
        use yr_sim::{BuildingSpec, Factory, Product, Role, OWNER_ENEMY, OWNER_PLAYER};
        let mut sim = Sim::new(4);
        let yard = |x, owner| Command::PlaceBuilding {
            kind: 0,
            x,
            y: 0,
            w: 2,
            h: 2,
            hp: 1000,
            armor: 0,
            power: 0,
            owner,
            role: Role::ConstructionYard,
        };
        let spec = BuildingSpec {
            w: 2,
            h: 2,
            hp: 500,
            armor: 0,
            power: 200,
            role: Role::Combat,
            adjacent: 2,
            base_normal: true,
            free_unit: None,
        };
        sim.step(&[
            yard(0, OWNER_PLAYER),
            yard(4, OWNER_PLAYER),
            yard(8, OWNER_ENEMY),
            Command::Produce {
                factory: Factory::Building,
                kind: 1,
                product: Product::Building(spec),
                cost: 100,
                ticks: 4,
            },
        ]);
        assert_eq!(
            building_animation(&sim, &sim.buildings()[0]).production,
            Some(0)
        );
        assert_eq!(
            building_animation(&sim, &sim.buildings()[1]).production,
            None
        );
        assert_eq!(
            building_animation(&sim, &sim.buildings()[2]).production,
            None
        );
        sim.set_credits(50);
        sim.step(&[]);
        sim.step(&[]);
        assert_eq!(
            building_animation(&sim, &sim.buildings()[0]).production,
            Some(2)
        );
        sim.step(&[]);
        assert_eq!(
            building_animation(&sim, &sim.buildings()[0]).production,
            Some(2)
        );
        let hash = sim.state_hash();
        for _ in 0..10 {
            building_animation(&sim, &sim.buildings()[0]);
        }
        assert_eq!(sim.state_hash(), hash);
        sim.set_credits(50);
        sim.step(&[]);
        sim.step(&[]);
        assert!(sim.production()[0].done());
        assert_eq!(
            building_animation(&sim, &sim.buildings()[0]).production,
            None
        );
        let mut damaged = sim.buildings()[0].clone();
        damaged.hp = 500;
        assert!(building_animation(&sim, &damaged).damaged);
        damaged.hp = 501;
        assert!(!building_animation(&sim, &damaged).damaged);
    }

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
                    source: None,
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

    #[test]
    fn army_churn_preserves_survivor_phases_and_releases_removed_clocks() {
        let mut sim = Sim::new(7);
        sim.step(&(0..256).map(|x| spawn(x, None)).collect::<Vec<_>>());
        let mut animation = UnitAnimations::default();
        animation.update(&sim);
        sim.step(&[Command::Move {
            id: 1,
            x: Fix::from_int(-100),
            y: Fix::ZERO,
        }]);
        animation.update(&sim);
        assert!(animation.get(1).moving);

        for cycle in 0..8 {
            let removed: Vec<_> = sim.units()[192..].iter().map(|u| u.id).collect();
            let first_new = sim.next_id_peek();
            let commands: Vec<_> = removed
                .iter()
                .map(|&id| Command::Remove { id })
                .chain((0..64).map(|x| spawn(256 + cycle * 64 + x, None)))
                .collect();
            sim.step(&commands);
            let saved = sim.save_bytes().unwrap();
            animation.update(&sim);
            assert_eq!(animation.units.len(), 256);
            assert_eq!(animation.get(1).elapsed, sim.tick() - 2);
            assert!(animation.get(1).moving);
            assert_eq!(animation.get(2).elapsed, sim.tick() - 1);
            for id in removed {
                assert!(!animation.units.contains_key(&id));
                assert_eq!(animation.get(id), InfantryAnimation::default());
            }
            for id in first_new..sim.next_id_peek() {
                assert_eq!(animation.get(id), InfantryAnimation::default());
                assert!(animation.units.contains_key(&id));
            }
            let observed: Vec<_> = sim.units().iter().map(|u| animation.get(u.id)).collect();
            for _ in 0..4 {
                animation.update(&sim);
                assert_eq!(
                    sim.units()
                        .iter()
                        .map(|u| animation.get(u.id))
                        .collect::<Vec<_>>(),
                    observed
                );
            }
            assert_eq!(sim.save_bytes().unwrap(), saved);
        }
        let removed: Vec<_> = sim
            .units()
            .iter()
            .map(|u| Command::Remove { id: u.id })
            .collect();
        sim.step(&removed);
        animation.update(&sim);
        assert!(animation.units.is_empty());
    }

    #[test]
    #[ignore = "manual animation-clock scaling probe; timings are not a CI assertion"]
    fn animation_clock_scaling_probe() {
        use std::{hint::black_box, time::Instant};
        for count in [512, 2048, 8192] {
            let mut sim = Sim::new(7);
            sim.step(&(0..count).map(|x| spawn(x, None)).collect::<Vec<_>>());
            let mut animation = UnitAnimations::default();
            animation.update(&sim);
            let mut elapsed = std::time::Duration::ZERO;
            for _ in 0..48 {
                sim.step(&[]);
                let start = Instant::now();
                animation.update(black_box(&sim));
                elapsed += start.elapsed();
                black_box(animation.get(1));
            }
            println!(
                "{count} units: {:.1} us/update (48 ticks)",
                elapsed.as_secs_f64() * 1e6 / 48.0
            );
        }
    }
}

#[cfg(test)]
mod building_tests;
