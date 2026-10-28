//! Small skirmish controller, deliberately separate from original AI fidelity.
//! Decisions run after external commands, before production, at fixed sim ticks.

use crate::{hash::Hasher, Factory, Product, Role, Sim};

/// One rules-resolved choice. Every prerequisite group needs one owned kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiBuild {
    pub factory: Factory,
    pub kind: u16,
    pub product: Product,
    pub cost: i64,
    pub ticks: u32,
    pub prerequisites: Vec<Vec<u16>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishAi {
    owner: u8,
    opponent: u8,
    builds: Vec<AiBuild>,
    next_attack: u64,
    groups_sent: u64,
    scout_sent: bool,
    attackers: Vec<u32>,
}

impl SkirmishAi {
    pub fn new(owner: u8, opponent: u8, builds: Vec<AiBuild>) -> Self {
        Self {
            owner,
            opponent,
            builds,
            next_attack: 1350,
            groups_sent: 0,
            scout_sent: false,
            attackers: Vec::new(),
        }
    }

    pub fn groups_sent(&self) -> u64 {
        self.groups_sent
    }
    pub fn scout_sent(&self) -> bool {
        self.scout_sent
    }

    fn allowed(&self, sim: &Sim, build: &AiBuild) -> bool {
        build.prerequisites.iter().all(|group| {
            sim.buildings
                .iter()
                .any(|b| b.owner == self.owner && group.contains(&b.kind))
        })
    }

    pub(crate) fn step(&mut self, sim: &mut Sim) {
        if !sim.tick.is_multiple_of(30) {
            return;
        }
        let targets: Vec<_> = sim
            .buildings
            .iter()
            .filter(|b| b.owner == self.opponent)
            .map(|b| (b.id, b.centre()))
            .chain(
                sim.units
                    .iter()
                    .filter(|u| u.owner == self.opponent)
                    .map(|u| (u.id, (u.x, u.y))),
            )
            .collect();
        if targets.is_empty() {
            return;
        }
        let base_alive = sim.buildings.iter().any(|b| b.owner == self.owner);
        // Economy-lite: a refinery earns $150 every ten seconds. Losing it
        // stops income immediately. Production always pays the retail cost.
        if sim.tick.is_multiple_of(150)
            && sim
                .buildings
                .iter()
                .any(|b| b.owner == self.owner && b.role == Role::Refinery)
        {
            sim.set_credits_of(self.owner, sim.credits_of(self.owner).saturating_add(150));
        }
        if base_alive {
            self.build(sim);
        }
        self.attackers
            .retain(|id| sim.units.iter().any(|u| u.id == *id));
        // Send one trained infantry scout after thirty seconds. The scout
        // uses normal attack-move and can be intercepted like any other unit.
        if !self.scout_sent && sim.tick >= 450 {
            if let Some(u) = sim.units.iter().find(|u| {
                u.owner == self.owner
                    && u.role == Role::Combat
                    && u.weapon.is_some()
                    && u.loco == crate::terrain::Loco::Foot
            }) {
                self.attackers.push(u.id);
                self.scout_sent = true;
            }
        }
        let waiting: Vec<u32> = sim
            .units
            .iter()
            .filter(|u| {
                u.owner == self.owner
                    && u.role == Role::Combat
                    && u.weapon.is_some()
                    && !self.attackers.contains(&u.id)
            })
            .map(|u| u.id)
            .collect();
        // After the base falls, surviving guards join the last stand.
        if (!base_alive || (sim.tick >= self.next_attack && waiting.len() >= 4))
            && !waiting.is_empty()
        {
            self.attackers
                .extend(
                    waiting
                        .into_iter()
                        .take(if base_alive { 6 } else { usize::MAX }),
                );
            self.groups_sent += 1;
            self.next_attack = sim.tick.saturating_add(900);
        }
        for &id in &self.attackers {
            let Some(u) = sim.units.iter().find(|u| u.id == id) else {
                continue;
            };
            if u.target.is_some() || u.attack.is_some() || u.guard.is_some() {
                continue;
            }
            let nearest = targets.iter().min_by_key(|(target, (x, y))| {
                let dx = i64::from(x.raw()) - i64::from(u.x.raw());
                let dy = i64::from(y.raw()) - i64::from(u.y.raw());
                (dx * dx + dy * dy, *target)
            });
            if let Some(&(_, (x, y))) = nearest {
                // Attack-move preserves self-defence along the route. An
                // idle unit near a building directly targets it to finish it.
                let close = (i64::from(x.raw()) - i64::from(u.x.raw()))
                    .abs()
                    .max((i64::from(y.raw()) - i64::from(u.y.raw())).abs())
                    <= 6 * 256;
                let u = sim.units.iter_mut().find(|u| u.id == id).unwrap();
                if close {
                    u.attack = nearest.map(|(id, _)| *id);
                } else {
                    u.guard = Some((x, y));
                    u.target = Some((x, y));
                    u.path.clear();
                }
            }
        }
    }

    fn build(&self, sim: &mut Sim) {
        if let Some(order) = sim
            .production_of(self.owner)
            .iter()
            .find(|o| o.factory == Factory::Building && o.done())
            .copied()
        {
            if let Product::Building(spec) = order.product {
                // Bounded search in building id order, then rings, rows and
                // columns. Use the same placement rules as the human player.
                let bases: Vec<_> = sim
                    .buildings
                    .iter()
                    .filter(|b| b.owner == self.owner && b.base_normal)
                    .map(|b| b.cell)
                    .collect();
                'sites: for (bx, by) in bases {
                    for r in 1..=16i32 {
                        for dy in -r..=r {
                            for dx in -r..=r {
                                if dx.abs().max(dy.abs()) != r {
                                    continue;
                                }
                                let (Ok(x), Ok(y)) = (
                                    u16::try_from(i32::from(bx) + dx),
                                    u16::try_from(i32::from(by) + dy),
                                ) else {
                                    continue;
                                };
                                if sim.can_place(self.owner, &spec, x, y) {
                                    sim.place_produced(self.owner, order.kind, x, y);
                                    break 'sites;
                                }
                            }
                        }
                    }
                }
            }
        }
        let army = sim
            .units
            .iter()
            .filter(|u| u.owner == self.owner && u.role == Role::Combat)
            .count();
        for factory in Factory::ALL {
            if sim
                .production_of(self.owner)
                .iter()
                .any(|o| o.factory == factory)
                || !sim
                    .buildings
                    .iter()
                    .any(|b| b.owner == self.owner && b.role == factory.role())
            {
                continue;
            }
            if factory != Factory::Building
                && army
                    + sim
                        .production_of(self.owner)
                        .iter()
                        .filter(|o| matches!(o.product, Product::Unit(_)))
                        .count()
                    >= 12
            {
                continue;
            }
            let choice = self.builds.iter().find(|b| {
                b.factory == factory
                    && self.allowed(sim, b)
                    && (factory != Factory::Building
                        || !sim
                            .buildings
                            .iter()
                            .any(|owned| owned.owner == self.owner && owned.kind == b.kind))
            });
            if let Some(b) = choice {
                sim.queue_produce(self.owner, b.factory, b.kind, b.product, b.cost, b.ticks);
            }
        }
    }

    pub(crate) fn hash(&self, h: &mut Hasher) {
        h.write_u64(u64::from(self.owner));
        h.write_u64(u64::from(self.opponent));
        h.write_u64(self.next_attack);
        h.write_u64(self.groups_sent);
        h.write_u64(u64::from(self.scout_sent));
        h.write_u64(self.attackers.len() as u64);
        for &id in &self.attackers {
            h.write_u64(u64::from(id));
        }
        h.write_u64(self.builds.len() as u64);
        for b in &self.builds {
            h.write_u64(b.factory as u64);
            h.write_u64(u64::from(b.kind));
            h.write_i64(b.cost);
            h.write_u64(u64::from(b.ticks));
            b.product.hash(h);
            h.write_u64(b.prerequisites.len() as u64);
            for group in &b.prerequisites {
                h.write_u64(group.len() as u64);
                for &kind in group {
                    h.write_u64(u64::from(kind));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        terrain::Loco, BuildingSpec, Command, Fix, TerrainGrid, UnitSpec, WeaponSpec,
        OWNER_ENEMY as E, OWNER_PLAYER as P,
    };

    fn soldier(loco: Loco) -> UnitSpec {
        UnitSpec {
            loco,
            speed: Fix::from_raw(128),
            hp: 100,
            armor: 0,
            weapon: Some(WeaponSpec {
                damage: 10,
                rof_ticks: 10,
                range: Fix::from_int(3),
                projectile_speed: Fix::ZERO,
                verses: [yr_rules::Decimal::ONE; crate::combat::ARMOR_COUNT],
            }),
            role: Role::Combat,
        }
    }

    fn structure(role: Role, power: i64) -> Product {
        Product::Building(BuildingSpec {
            w: 2,
            h: 2,
            hp: 1000,
            armor: 0,
            power,
            role,
            adjacent: 2,
            base_normal: true,
            free_unit: None,
        })
    }

    fn scenario() -> Sim {
        let mut sim = Sim::new(7);
        sim.set_terrain(TerrainGrid::new(0, 0, 64, 16));
        sim.set_credits(1000);
        sim.set_credits_of(E, 1000);
        for (kind, role, x, y, power) in [
            (10, Role::WarFactory, 50, 2, -25),
            (11, Role::Barracks, 50, 5, -10),
            (12, Role::Combat, 47, 5, 200),
            (13, Role::Refinery, 47, 8, -50),
            (14, Role::ConstructionYard, 47, 2, 0),
        ] {
            sim.step(&[Command::PlaceBuilding {
                kind,
                x,
                y,
                w: 2,
                h: 2,
                hp: 1000,
                armor: 0,
                power,
                owner: E,
                role,
            }]);
        }
        sim.step(&[Command::PlaceBuilding {
            kind: 14,
            x: 2,
            y: 2,
            w: 2,
            h: 2,
            hp: 1000,
            armor: 0,
            power: 0,
            owner: P,
            role: Role::ConstructionYard,
        }]);
        let mut builds = vec![
            AiBuild {
                factory: Factory::Vehicle,
                kind: 0,
                product: Product::Unit(soldier(Loco::Track)),
                cost: 100,
                ticks: 60,
                prerequisites: vec![vec![10]],
            },
            AiBuild {
                factory: Factory::Infantry,
                kind: 1,
                product: Product::Unit(soldier(Loco::Foot)),
                cost: 50,
                ticks: 30,
                prerequisites: vec![vec![11]],
            },
        ];
        for (kind, role, power) in [
            (12, Role::Combat, 200),
            (13, Role::Refinery, -50),
            (10, Role::WarFactory, -25),
            (11, Role::Barracks, -10),
        ] {
            builds.push(AiBuild {
                factory: Factory::Building,
                kind,
                product: structure(role, power),
                cost: 200,
                ticks: 60,
                prerequisites: vec![vec![14]],
            });
        }
        sim.set_ai(Some(SkirmishAi::new(E, P, builds)));
        sim
    }

    #[test]
    fn production_pays_own_account_and_missing_factory_stalls() {
        let mut sim = scenario();
        while sim.tick() < 100 {
            sim.step(&[]);
        }
        assert!(sim.units.iter().any(|u| u.owner == E && u.kind == 0));
        assert!(sim.units.iter().any(|u| u.owner == E && u.kind == 1));
        assert_eq!(sim.credits(), 1000);
        assert!(sim.credits_of(E) < 1000);
        let factory = sim
            .buildings
            .iter()
            .find(|b| b.owner == E && b.kind == 10)
            .unwrap()
            .id;
        sim.step(&[Command::Destroy { id: factory }]);
        let progress = sim
            .production_of(E)
            .iter()
            .find(|o| o.factory == Factory::Vehicle)
            .unwrap()
            .progress;
        for _ in 0..20 {
            sim.step(&[]);
        }
        assert_eq!(
            sim.production_of(E)
                .iter()
                .find(|o| o.factory == Factory::Vehicle)
                .unwrap()
                .progress,
            progress
        );
    }

    #[test]
    fn rebuild_uses_paid_structure_and_normal_placement() {
        let mut sim = scenario();
        let id = sim.buildings.iter().find(|b| b.kind == 10).unwrap().id;
        sim.step(&[Command::Destroy { id }]);
        assert!(!sim.buildings.iter().any(|b| b.kind == 10));
        while sim.tick() < 300 {
            sim.step(&[]);
        }
        let rebuilt = sim
            .buildings
            .iter()
            .find(|b| b.owner == E && b.kind == 10)
            .expect("rebuilt factory");
        assert_ne!(rebuilt.id, id);
        assert!(
            sim.units.iter().any(|u| u.owner == E && u.kind == 0),
            "rebuilt factory rolls out tanks"
        );
        assert_eq!(sim.credits(), 1000);
        assert!(sim.credits_of(E) < 1000 + 150 - 200, "replacement paid for");
    }

    #[test]
    fn no_credits_stalls_and_refinery_loss_stops_income() {
        let mut sim = scenario();
        // No yard prevents rebuilding the destroyed refinery.
        let kills: Vec<_> = sim
            .buildings
            .iter()
            .filter(|b| b.owner == E && matches!(b.role, Role::Refinery | Role::ConstructionYard))
            .map(|b| Command::Destroy { id: b.id })
            .collect();
        sim.step(&kills);
        sim.set_credits_of(E, 0);
        while sim.tick() < 600 {
            sim.step(&[]);
        }
        assert!(sim.units.is_empty(), "no free reinforcements");
        assert_eq!(sim.credits_of(E), 0);
        assert_eq!(sim.credits(), 1000);
    }

    #[test]
    fn independent_clones_match_through_scout_and_group_decisions() {
        let mut sim = scenario();
        let mut twin = sim.clone();
        for _ in 0..1600 {
            sim.step(&[]);
            twin.step(&[]);
            assert_eq!(sim.state_hash(), twin.state_hash());
        }
        assert!(sim.ai().unwrap().scout_sent());
        assert!(sim.ai().unwrap().groups_sent() > 0);
        let original = sim.state_hash();
        sim.ai.as_mut().unwrap().next_attack += 1;
        assert_ne!(original, sim.state_hash(), "AI deadlines are authoritative");
    }
}
