//! Structure repair and sale commands. All prices and timers use integers.

use crate::{Event, Sim, OWNER_PLAYER};
use yr_rules::Decimal;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepairSpec {
    pub cost: i64,
    pub percent: Decimal,
    pub step: i64,
    pub interval: u32,
}

impl RepairSpec {
    pub(crate) fn valid(self) -> bool {
        (0..=i64::from(i32::MAX)).contains(&self.cost)
            && (0..=1_000_000).contains(&self.percent.units())
            && (1..=i64::from(i32::MAX)).contains(&self.step)
            && self.interval > 0
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepairJob {
    pub spec: RepairSpec,
    pub remaining: u32,
    pub remainder: i64,
}

impl Sim {
    pub fn repairing(&self, id: u32) -> bool {
        self.repairs.contains_key(&id)
    }

    pub(crate) fn set_repair(&mut self, id: u32, spec: Option<RepairSpec>) {
        let Some(building) = self
            .buildings
            .iter()
            .find(|b| b.id == id && b.owner == OWNER_PLAYER && b.hp > 0)
        else {
            return;
        };
        let Some(spec) = spec else {
            self.repairs.remove(&id);
            return;
        };
        if !spec.valid() || building.hp >= building.max_hp || building.max_hp > i64::from(i32::MAX)
        {
            return;
        }
        // Repeated enable commands must not restart an existing timer.
        if let std::collections::btree_map::Entry::Vacant(entry) = self.repairs.entry(id) {
            entry.insert(RepairJob {
                spec,
                remaining: spec.interval,
                remainder: 0,
            });
            self.events.push(Event::RepairStarted { id });
        }
    }

    pub(crate) fn service_phase(&mut self) {
        let ids: Vec<u32> = self.repairs.keys().copied().collect();
        for id in ids {
            let Some(index) = self
                .buildings
                .iter()
                .position(|b| b.id == id && b.owner == OWNER_PLAYER && b.hp > 0 && b.hp < b.max_hp)
            else {
                self.repairs.remove(&id);
                continue;
            };
            let job = self.repairs.get_mut(&id).unwrap();
            if job.remaining > 1 {
                job.remaining -= 1;
                continue;
            }
            let building = &self.buildings[index];
            let healed = job.spec.step.min(building.max_hp - building.hp);
            let denominator = i128::from(building.max_hp) * 1_000_000;
            let numerator = i128::from(job.spec.cost)
                * i128::from(job.spec.percent.units())
                * i128::from(healed)
                + i128::from(job.remainder);
            let due = (numerator / denominator) as i64;
            let credits = self.economies.entry(OWNER_PLAYER).or_default();
            if due > credits.credits {
                continue;
            }
            credits.credits -= due;
            job.remainder = (numerator % denominator) as i64;
            job.remaining = job.spec.interval;
            self.buildings[index].hp += healed;
            if self.buildings[index].hp == self.buildings[index].max_hp {
                self.repairs.remove(&id);
            }
        }
    }

    pub(crate) fn sell_structure(&mut self, id: u32, cost: i64, percent: Decimal) {
        if !(0..=i64::from(i32::MAX)).contains(&cost) || !(0..=1_000_000).contains(&percent.units())
        {
            return;
        }
        let Some(index) = self
            .buildings
            .iter()
            .position(|b| b.id == id && b.owner == OWNER_PLAYER && b.hp > 0)
        else {
            return;
        };
        let building = self.buildings.remove(index);
        let value = (i128::from(cost) * i128::from(percent.units()) / 1_000_000) as i64;
        let economy = self.economies.entry(OWNER_PLAYER).or_default();
        // Keep the same bounded credit domain accepted by snapshots.
        economy.credits = economy
            .credits
            .saturating_add(value)
            .min(i64::from(i32::MAX));
        let total = economy.credits;
        self.release_structure(&building);
        self.events.push(Event::Sold { id, value, total });
    }

    pub(crate) fn release_structure(&mut self, building: &crate::Building) {
        self.repairs.remove(&building.id);
        if let Some(grid) = self.terrain.as_mut() {
            grid.set_building(
                building.cell.0,
                building.cell.1,
                building.size.0,
                building.size.1,
                false,
            );
        }
        for unit in &mut self.units {
            if unit.attack == Some(building.id) {
                unit.attack = None;
                unit.target = None;
                unit.path.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{terrain::TerrainGrid, Command, Role};

    fn damaged() -> Sim {
        let mut sim = Sim::new(7);
        sim.set_terrain(TerrainGrid::from_tiles(
            0,
            0,
            5,
            5,
            (0..5).flat_map(|y| (0..5).map(move |x| (x, y, 0, 13, 0))),
        ));
        sim.step(&[Command::PlaceBuilding {
            kind: 0,
            x: 1,
            y: 1,
            w: 2,
            h: 2,
            hp: 100,
            armor: 0,
            power: 200,
            owner: OWNER_PLAYER,
            role: Role::Combat,
        }]);
        sim.buildings[0].hp = 20;
        sim.set_credits(100);
        sim.drain_events();
        sim
    }

    fn spec() -> RepairSpec {
        RepairSpec {
            cost: 100,
            percent: Decimal::parse("15%").unwrap(),
            step: 8,
            interval: 3,
        }
    }

    #[test]
    fn repair_prices_accumulate_and_repeated_enable_preserves_timer() {
        let mut sim = damaged();
        let enable = Command::Repair {
            id: 1,
            spec: Some(spec()),
        };
        sim.step(&[enable]);
        assert_eq!(sim.repairs[&1].remaining, 2);
        sim.step(&[enable]);
        assert_eq!(sim.repairs[&1].remaining, 1);
        sim.step(&[]);
        assert_eq!(sim.buildings[0].hp, 28);
        assert_eq!(sim.credits(), 99);
        for _ in 0..27 {
            sim.step(&[]);
        }
        assert_eq!(sim.buildings[0].hp, 100);
        assert_eq!(sim.credits(), 88, "80 HP costs exactly 12 credits");
        assert!(!sim.repairing(1));
    }

    #[test]
    fn credit_stall_freezes_health_and_fraction_then_resumes() {
        let mut sim = damaged();
        sim.set_credits(0);
        sim.step(&[Command::Repair {
            id: 1,
            spec: Some(spec()),
        }]);
        for _ in 0..12 {
            sim.step(&[]);
        }
        assert_eq!(sim.buildings[0].hp, 20);
        assert_eq!(
            (sim.repairs[&1].remaining, sim.repairs[&1].remainder),
            (1, 0)
        );
        sim.set_credits(1);
        sim.step(&[]);
        assert_eq!((sim.buildings[0].hp, sim.credits()), (28, 0));
        sim.step(&[Command::Repair { id: 1, spec: None }]);
        assert!(!sim.repairing(1));
        sim.set_credits(100);
        for _ in 0..10 {
            sim.step(&[]);
        }
        assert_eq!(sim.buildings[0].hp, 28);
    }

    #[test]
    fn active_repairs_hash_and_resume_exactly_and_hostile_jobs_fail() {
        let mut sim = damaged();
        let mut idle = Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap();
        sim.step(&[Command::Repair {
            id: 1,
            spec: Some(spec()),
        }]);
        idle.step(&[]);
        assert_ne!(sim.state_hash(), idle.state_hash());
        let bytes = sim.save_bytes().unwrap();
        let mut loaded = Sim::from_save_bytes(&bytes).unwrap();
        for _ in 0..40 {
            sim.step(&[]);
            loaded.step(&[]);
            assert_eq!(sim.state_hash(), loaded.state_hash());
        }
        let base: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
        for (field, value) in [
            ("remaining", 0),
            ("remaining", 4),
            ("remainder", -1),
            ("remainder", 100_000_000),
        ] {
            let mut bad = base.clone();
            bad["repairs"]["1"][field] = value.into();
            let mut encoded = bytes[..12].to_vec();
            encoded.extend(serde_json::to_vec(&bad).unwrap());
            assert!(Sim::from_save_bytes(&encoded).is_err(), "{field}={value}");
        }
        let mut bad = base.clone();
        bad["repairs"]["1"]["spec"]["step"] = i64::MAX.into();
        let mut encoded = bytes[..12].to_vec();
        encoded.extend(serde_json::to_vec(&bad).unwrap());
        assert!(Sim::from_save_bytes(&encoded).is_err());
    }

    #[test]
    fn selling_refunds_once_cancels_repairs_and_releases_the_footprint() {
        let mut sim = damaged();
        sim.step(&[Command::Repair {
            id: 1,
            spec: Some(spec()),
        }]);
        sim.drain_events();
        let sell = Command::Sell {
            id: 1,
            cost: 101,
            percent: Decimal::parse("50%").unwrap(),
        };
        sim.step(&[sell, sell]);
        assert!(sim.buildings().is_empty());
        assert!(!sim.repairing(1));
        assert_eq!(sim.credits(), 150);
        assert!(sim
            .terrain()
            .unwrap()
            .passable(crate::terrain::Loco::Track, 1, 1));
        assert_eq!(
            sim.drain_events(),
            [Event::Sold {
                id: 1,
                value: 50,
                total: 150
            }]
        );
        Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap();
    }

    #[test]
    fn enemy_and_invalid_service_commands_cannot_mutate_a_structure() {
        let mut sim = damaged();
        sim.buildings[0].owner = crate::OWNER_ENEMY;
        sim.step(&[
            Command::Repair {
                id: 1,
                spec: Some(spec()),
            },
            Command::Sell {
                id: 1,
                cost: 100,
                percent: Decimal::ONE,
            },
        ]);
        assert_eq!((sim.buildings[0].hp, sim.credits()), (20, 100));
        assert!(!sim.repairing(1));
        sim.buildings[0].owner = OWNER_PLAYER;
        let mut bad = spec();
        bad.interval = 0;
        sim.step(&[
            Command::Repair {
                id: 1,
                spec: Some(bad),
            },
            Command::Sell {
                id: 1,
                cost: -1,
                percent: Decimal::ONE,
            },
        ]);
        assert_eq!(sim.buildings.len(), 1);
        assert!(!sim.repairing(1));
    }

    #[test]
    fn competing_jobs_spend_in_stable_id_order() {
        let mut sim = damaged();
        let mut second = sim.buildings[0].clone();
        second.id = 2;
        second.cell = (3, 3);
        sim.buildings.push(second);
        sim.next_id = 3;
        let mut recipe = spec();
        recipe.interval = 1;
        sim.set_credits(1);
        sim.step(&[
            Command::Repair {
                id: 2,
                spec: Some(recipe),
            },
            Command::Repair {
                id: 1,
                spec: Some(recipe),
            },
        ]);
        assert_eq!((sim.buildings[0].hp, sim.buildings[1].hp), (28, 20));
    }
    #[test]
    fn selling_last_factory_stalls_paid_queue_until_replacement_exists() {
        let mut sim = damaged();
        sim.buildings[0].role = Role::Barracks;
        let product = crate::Product::Unit(crate::UnitSpec {
            loco: crate::terrain::Loco::Foot,
            speed: crate::Fix::ONE,
            hp: 100,
            armor: 0,
            weapon: None,
            role: Role::Combat,
        });
        sim.step(&[Command::Produce {
            factory: crate::Factory::Infantry,
            kind: 9,
            product,
            cost: 20,
            ticks: 10,
        }]);
        let progress = sim.production()[0].progress;
        let paid = sim.production()[0].paid;
        sim.step(&[Command::Sell {
            id: 1,
            cost: 100,
            percent: Decimal::parse("50%").unwrap(),
        }]);
        for _ in 0..20 {
            sim.step(&[]);
        }
        assert_eq!(
            (sim.production()[0].progress, sim.production()[0].paid),
            (progress, paid)
        );
        sim.step(&[Command::PlaceBuilding {
            kind: 0,
            x: 1,
            y: 1,
            w: 2,
            h: 2,
            hp: 100,
            armor: 0,
            power: 0,
            owner: OWNER_PLAYER,
            role: Role::Barracks,
        }]);
        for _ in 0..10 {
            sim.step(&[]);
        }
        assert!(sim.production().is_empty());
        assert!(sim.units().iter().any(|u| u.kind == 9));
    }
}
