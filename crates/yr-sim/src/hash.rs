//! FNV-1a 64-bit state hasher (deterministic across platforms/runs).

pub struct Hasher {
    h: u64,
}

impl Hasher {
    pub fn new() -> Hasher {
        Hasher {
            h: 0xcbf2_9ce4_8422_2325,
        }
    }

    pub fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.h ^= u64::from(b);
            self.h = self.h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    pub fn finish(&self) -> u64 {
        self.h
    }
}

impl crate::WeaponSpec {
    pub(crate) fn hash(&self, h: &mut Hasher) {
        // Optional source IDs only select presentation art, never future play.
        h.write_i64(self.damage);
        h.write_u64(u64::from(self.rof_ticks));
        h.write_i64(i64::from(self.range.raw()));
        h.write_i64(i64::from(self.projectile_speed.raw()));
        for verse in self.verses {
            h.write_i64(verse.units());
        }
    }
}

impl crate::UnitSpec {
    pub(crate) fn hash(&self, h: &mut Hasher) {
        h.write_u64(self.loco as u64);
        h.write_i64(i64::from(self.speed.raw()));
        h.write_i64(self.hp);
        h.write_u64(u64::from(self.armor));
        weapon(h, self.weapon);
        h.write_u64(self.role as u64);
    }
}

impl crate::Product {
    pub(crate) fn hash(&self, h: &mut Hasher) {
        match self {
            Self::Unit(u) => {
                h.write_u64(0);
                u.hash(h);
            }
            Self::Building(b) => {
                h.write_u64(1);
                h.write_u64(u64::from(b.w));
                h.write_u64(u64::from(b.h));
                h.write_i64(b.hp);
                h.write_u64(u64::from(b.armor));
                h.write_i64(b.power);
                h.write_u64(b.role as u64);
                h.write_u64(u64::from(b.adjacent));
                h.write_u64(u64::from(b.base_normal));
                h.write_u64(u64::from(b.free_unit.is_some()));
                if let Some((kind, unit)) = b.free_unit {
                    h.write_u64(u64::from(kind));
                    unit.hash(h);
                }
            }
        }
    }
}

fn weapon(h: &mut Hasher, w: Option<crate::WeaponSpec>) {
    h.write_u64(u64::from(w.is_some()));
    if let Some(w) = w {
        w.hash(h);
    }
}

fn point(h: &mut Hasher, p: Option<(crate::Fix, crate::Fix)>) {
    h.write_u64(u64::from(p.is_some()));
    if let Some((x, y)) = p {
        h.write_i64(i64::from(x.raw()));
        h.write_i64(i64::from(y.raw()));
    }
}

pub(crate) fn state(sim: &crate::Sim) -> u64 {
    let mut h = Hasher::new();
    h.write_u64(sim.tick);
    h.write_u64(sim.rng.state());
    h.write_u64(u64::from(sim.next_id));
    h.write_u64(u64::from(sim.terrain.is_some()));
    h.write_u64(sim.terrain.as_ref().map_or(0, crate::TerrainGrid::hash));
    h.write_i64(i64::from(sim.low_power_speeds.0.raw()));
    h.write_i64(i64::from(sim.low_power_speeds.1.raw()));
    h.write_u64(sim.units.len() as u64);
    for u in &sim.units {
        h.write_u64(u64::from(u.id));
        h.write_u64(u64::from(u.kind));
        point(&mut h, Some((u.x, u.y)));
        point(&mut h, u.target);
        point(&mut h, u.guard);
        h.write_i64(i64::from(u.speed.raw()));
        h.write_u64(u.loco as u64);
        h.write_u64(u64::from(u.facing));
        h.write_u64(u.path.len() as u64);
        for &(x, y) in &u.path {
            h.write_u64(u64::from(x));
            h.write_u64(u64::from(y));
        }
        h.write_i64(u.hp);
        h.write_i64(u.max_hp);
        h.write_u64(u64::from(u.armor));
        weapon(&mut h, u.weapon);
        h.write_u64(u64::from(u.cooldown));
        h.write_u64(u.attack.map_or(0, u64::from));
        h.write_u64(u64::from(u.shots));
        h.write_u64(u.role as u64);
        h.write_u64(u64::from(u.owner));
        h.write_i64(u.cargo);
        h.write_u64(u64::from(u.bails));
        match u.harvest {
            crate::Harvest::Idle => h.write_u64(0),
            crate::Harvest::ToOre(x, y) => {
                h.write_u64(1);
                h.write_u64(u64::from(x));
                h.write_u64(u64::from(y));
            }
            crate::Harvest::Loading(t) => {
                h.write_u64(2);
                h.write_u64(u64::from(t));
            }
            crate::Harvest::ToRefinery(x, y) => {
                h.write_u64(3);
                h.write_u64(u64::from(x));
                h.write_u64(u64::from(y));
            }
            crate::Harvest::Unloading(t) => {
                h.write_u64(4);
                h.write_u64(u64::from(t));
            }
        }
        h.write_u64(u64::from(u.blocked));
    }
    h.write_u64(sim.economies.len() as u64);
    for (&owner, e) in &sim.economies {
        h.write_u64(u64::from(owner));
        h.write_i64(e.credits);
        h.write_u64(e.production.len() as u64);
        for o in &e.production {
            h.write_u64(o.factory as u64);
            h.write_u64(u64::from(o.kind));
            o.product.hash(&mut h);
            h.write_i64(o.cost);
            h.write_u64(u64::from(o.ticks));
            h.write_i64(o.progress);
            h.write_i64(o.paid);
        }
    }
    h.write_u64(sim.buildings.len() as u64);
    for b in &sim.buildings {
        h.write_u64(u64::from(b.id));
        h.write_u64(u64::from(b.kind));
        h.write_u64(u64::from(b.cell.0));
        h.write_u64(u64::from(b.cell.1));
        h.write_u64(u64::from(b.size.0));
        h.write_u64(u64::from(b.size.1));
        h.write_i64(b.hp);
        h.write_i64(b.max_hp);
        h.write_u64(u64::from(b.armor));
        h.write_i64(b.power);
        h.write_u64(u64::from(b.owner));
        h.write_u64(b.role as u64);
        h.write_u64(u64::from(b.base_normal));
    }
    h.write_u64(sim.projectiles.len() as u64);
    for p in &sim.projectiles {
        // Shot provenance is saved for impact art but does not affect motion.
        point(&mut h, Some((p.x, p.y)));
        h.write_u64(u64::from(p.owner));
        h.write_u64(u64::from(p.target));
        h.write_i64(p.damage);
        h.write_i64(i64::from(p.speed.raw()));
    }
    h.write_u64(u64::from(sim.ai.is_some()));
    if let Some(ai) = &sim.ai {
        ai.hash(&mut h);
    }
    // Events are presentation output. Draining them cannot change future play.
    // Empty service state preserves pre-service hashes and old saves.
    if !sim.repairs.is_empty() {
        h.write_u64(0x524550414952);
        h.write_u64(sim.repairs.len() as u64);
        for (&id, job) in &sim.repairs {
            h.write_u64(u64::from(id));
            h.write_i64(job.spec.cost);
            h.write_i64(job.spec.percent.units());
            h.write_i64(job.spec.step);
            h.write_u64(u64::from(job.spec.interval));
            h.write_u64(u64::from(job.remaining));
            h.write_i64(job.remainder);
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn owners_weapons_path_tails_and_harvest_clocks_change_hashes() {
        let mut sim = Sim::new(1);
        sim.step(&[Command::Spawn {
            kind: 0,
            x: Fix::ZERO,
            y: Fix::ZERO,
            loco: terrain::Loco::Track,
            speed: Fix::ONE,
            hp: 100,
            armor: 0,
            weapon: None,
        }]);
        sim.units[0].path.extend([(1, 0), (2, 0)]);
        let reference = sim.state_hash();
        let different = |edit: fn(&mut Sim)| {
            let mut s = sim.clone();
            edit(&mut s);
            assert_ne!(reference, s.state_hash());
        };
        different(|s| s.units[0].owner = OWNER_ENEMY);
        different(|s| s.units[0].role = Role::Harvester);
        different(|s| s.units[0].harvest = Harvest::Loading(3));
        different(|s| *s.units[0].path.back_mut().unwrap() = (3, 0));
        different(|s| {
            s.units[0].weapon = Some(WeaponSpec {
                source: None,
                damage: 20,
                rof_ticks: 2,
                range: Fix::ONE,
                projectile_speed: Fix::ONE,
                verses: [yr_rules::Decimal::ONE; combat::ARMOR_COUNT],
            })
        });
        let before = sim.state_hash();
        sim.drain_events();
        assert_eq!(
            before,
            sim.state_hash(),
            "presentation output is not authoritative"
        );
    }

    #[test]
    fn projectile_payload_and_queued_specs_change_hashes() {
        let mut sim = Sim::new(1);
        sim.projectiles.push(Projectile {
            origin: None,
            x: Fix::ZERO,
            y: Fix::ZERO,
            owner: 1,
            target: 2,
            damage: 50,
            speed: Fix::ONE,
        });
        let before = sim.state_hash();
        let mut changed = sim.clone();
        changed.projectiles[0].target = 3;
        assert_ne!(before, changed.state_hash());
        changed = sim.clone();
        changed.projectiles[0].damage = 51;
        assert_ne!(before, changed.state_hash());
        let unit = UnitSpec {
            loco: terrain::Loco::Foot,
            speed: Fix::ONE,
            hp: 100,
            armor: 0,
            weapon: None,
            role: Role::Combat,
        };
        sim.queue_produce(
            OWNER_PLAYER,
            Factory::Infantry,
            0,
            Product::Unit(unit),
            100,
            10,
        );
        let before = sim.state_hash();
        changed = sim.clone();
        changed.economies.get_mut(&OWNER_PLAYER).unwrap().production[0].product =
            Product::Unit(UnitSpec { hp: 200, ..unit });
        assert_ne!(before, changed.state_hash());
        changed = sim.clone();
        changed.economies.get_mut(&OWNER_PLAYER).unwrap().production[0].ticks += 1;
        assert_ne!(before, changed.state_hash());
    }

    #[test]
    fn terrain_origin_shape_and_mutable_cell_edits_change_hashes() {
        let a = TerrainGrid::new(0, 0, 4, 4);
        assert_ne!(a.hash(), TerrainGrid::new(1, 0, 4, 4).hash());
        assert_ne!(a.hash(), TerrainGrid::new(0, 0, 2, 8).hash());
        let mut b = a.clone();
        b.cell_mut(0, 0).unwrap().ramp = 1;
        assert_ne!(a.hash(), b.hash());
    }
}
