//! Versioned snapshots of authoritative state. Byte conversion only, no I/O.

use crate::{BuildOrder, Product, Sim, UnitSpec, WeaponSpec};
use std::fmt;

const MAGIC: &[u8; 8] = b"OYRSIM\0\0";
pub const VERSION: u32 = 1;
pub const MAX_SAVE_BYTES: usize = 64 * 1024 * 1024;
const MAX_STAT: i64 = i32::MAX as i64;
const MAX_COORD: i32 = 1 << 24;

#[derive(Debug)]
pub struct SaveError(pub(crate) String);

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SaveError {}

pub(crate) fn require(ok: bool, message: &str) -> Result<(), SaveError> {
    if ok {
        Ok(())
    } else {
        Err(SaveError(format!("invalid save: {message}")))
    }
}

impl Sim {
    /// Snapshot without advancing a tick, drawing RNG or draining events.
    /// Fixed-point and decimal values are encoded as their raw integers.
    pub fn save_bytes(&self) -> Result<Vec<u8>, SaveError> {
        self.validate_save()?;
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        serde_json::to_writer(&mut bytes, self).map_err(|e| SaveError(e.to_string()))?;
        require(bytes.len() <= MAX_SAVE_BYTES, "snapshot exceeds 64 MiB")?;
        Ok(bytes)
    }

    /// Restore a complete snapshot. Reject the version before decoding state.
    /// JSON arrays grow from actual elements, never a claimed header count.
    pub fn from_save_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        require(bytes.len() <= MAX_SAVE_BYTES, "snapshot exceeds 64 MiB")?;
        require(bytes.len() >= 12, "truncated simulation header")?;
        require(&bytes[..8] == MAGIC, "not an OpenYR simulation snapshot")?;
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(SaveError(format!(
                "unsupported simulation save version {version}; expected {VERSION}"
            )));
        }
        let sim: Self = serde_json::from_slice(&bytes[12..])
            .map_err(|e| SaveError(format!("invalid save JSON: {e}")))?;
        sim.validate_save()?;
        Ok(sim)
    }

    fn validate_save(&self) -> Result<(), SaveError> {
        // Leave ample headroom for ordinary tick/id increments after loading.
        require(self.tick < u64::MAX / 2, "tick out of range")?;
        require(
            self.next_id > 0 && self.next_id < u32::MAX / 2,
            "next id out of range",
        )?;
        require(
            self.low_power_speeds.0.raw() > 0
                && self.low_power_speeds.0 <= self.low_power_speeds.1
                && self.low_power_speeds.1 <= crate::Fix::ONE,
            "invalid low-power speeds",
        )?;
        if let Some(g) = &self.terrain {
            g.validate_save()?;
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut previous = 0;
        for u in &self.units {
            require(
                u.id > previous && u.id < self.next_id,
                "unit ids not ordered or next id stale",
            )?;
            ids.insert(u.id);
            previous = u.id;
            point((u.x, u.y))?;
            for p in [u.target, u.guard].into_iter().flatten() {
                point(p)?;
            }
            require(
                (0..=MAX_COORD).contains(&u.speed.raw()),
                "unit speed out of range",
            )?;
            health(u.hp, u.max_hp, u.armor)?;
            require(u.shots < u32::MAX / 2, "shot counter out of range")?;
            if let Some(w) = u.weapon {
                weapon(w)?;
            }
            require((0..=MAX_STAT).contains(&u.cargo), "cargo out of range")?;
            require(
                u.bails <= crate::HARVESTER_CAPACITY,
                "too many carried bails",
            )?;
            for &(x, y) in &u.path {
                require(
                    self.terrain
                        .as_ref()
                        .is_some_and(|g| g.cell(x, y).is_some()),
                    "path outside terrain",
                )?;
            }
        }
        previous = 0;
        for b in &self.buildings {
            require(
                b.id > previous && b.id < self.next_id && ids.insert(b.id),
                "building ids not ordered, duplicate or next id stale",
            )?;
            previous = b.id;
            health(b.hp, b.max_hp, b.armor)?;
            require(
                (-MAX_STAT..=MAX_STAT).contains(&b.power),
                "power out of range",
            )?;
            let (x, y) = b.cell;
            let (w, h) = b.size;
            require(w > 0 && h > 0, "empty building footprint")?;
            let end_x = x.checked_add(w - 1);
            let end_y = y.checked_add(h - 1);
            require(
                end_x.is_some() && end_y.is_some(),
                "building footprint overflows",
            )?;
            if let Some(g) = &self.terrain {
                require(
                    g.cell(x, y).is_some() && g.cell(end_x.unwrap(), end_y.unwrap()).is_some(),
                    "building outside terrain",
                )?;
            }
        }
        for (&id, job) in &self.repairs {
            let b = self.buildings.iter().find(|b| {
                b.id == id && b.owner == crate::OWNER_PLAYER && b.hp > 0 && b.hp < b.max_hp
            });
            require(b.is_some(), "repair target is absent or ineligible")?;
            require(job.spec.valid(), "invalid repair recipe")?;
            require(
                job.remaining > 0 && job.remaining <= job.spec.interval,
                "invalid repair countdown",
            )?;
            let denominator = i128::from(b.unwrap().max_hp) * 1_000_000;
            require(
                job.remainder >= 0 && i128::from(job.remainder) < denominator,
                "invalid repair fraction",
            )?;
        }
        for p in &self.projectiles {
            point((p.x, p.y))?;
            require(
                (0..=MAX_COORD).contains(&p.speed.raw()),
                "projectile speed out of range",
            )?;
            require(
                (-MAX_STAT..=MAX_STAT).contains(&p.damage),
                "projectile damage out of range",
            )?;
        }
        for e in self.economies.values() {
            require((0..=MAX_STAT).contains(&e.credits), "credits out of range")?;
            for o in &e.production {
                order(o)?;
            }
        }
        if let Some(ai) = &self.ai {
            ai.validate_save()?;
        }
        Ok(())
    }
}

fn point(p: (crate::Fix, crate::Fix)) -> Result<(), SaveError> {
    require(
        (-MAX_COORD..=MAX_COORD).contains(&p.0.raw())
            && (-MAX_COORD..=MAX_COORD).contains(&p.1.raw()),
        "coordinate out of range",
    )
}

fn health(hp: i64, max_hp: i64, armor: u8) -> Result<(), SaveError> {
    require(
        hp > 0 && hp <= max_hp && max_hp <= MAX_STAT,
        "health out of range",
    )?;
    require(
        usize::from(armor) < crate::combat::ARMOR_COUNT,
        "armour out of range",
    )
}

fn weapon(w: WeaponSpec) -> Result<(), SaveError> {
    require(
        (-MAX_STAT..=MAX_STAT).contains(&w.damage),
        "weapon damage out of range",
    )?;
    require(
        (0..=MAX_COORD).contains(&w.range.raw())
            && (0..=MAX_COORD).contains(&w.projectile_speed.raw()),
        "weapon range/speed out of range",
    )?;
    require(
        w.verses
            .iter()
            .all(|v| (-1_000_000_000..=1_000_000_000).contains(&v.units())),
        "damage multiplier out of range",
    )
}

fn unit(u: UnitSpec) -> Result<(), SaveError> {
    health(u.hp, u.hp, u.armor)?;
    require(
        (0..=MAX_COORD).contains(&u.speed.raw()),
        "product speed out of range",
    )?;
    if let Some(w) = u.weapon {
        weapon(w)?;
    }
    Ok(())
}

pub(crate) fn product(p: Product) -> Result<(), SaveError> {
    match p {
        Product::Unit(u) => unit(u),
        Product::Building(b) => {
            health(b.hp, b.hp, b.armor)?;
            require(
                b.w > 0 && b.h > 0 && b.w <= 256 && b.h <= 256,
                "product footprint out of range",
            )?;
            require(
                (-MAX_STAT..=MAX_STAT).contains(&b.power),
                "product power out of range",
            )?;
            if let Some((_, u)) = b.free_unit {
                unit(u)?;
            }
            Ok(())
        }
    }
}

pub(crate) fn order(o: &BuildOrder) -> Result<(), SaveError> {
    product(o.product)?;
    require(
        (0..=MAX_STAT).contains(&o.cost) && o.ticks > 0,
        "build cost/time out of range",
    )?;
    require(
        o.progress >= 0 && o.progress <= o.total() && o.paid >= 0 && o.paid <= o.cost,
        "build payment/progress out of range",
    )?;
    require(
        matches!(
            (o.factory, o.product),
            (crate::Factory::Building, Product::Building(_))
                | (
                    crate::Factory::Vehicle | crate::Factory::Infantry,
                    Product::Unit(_)
                )
        ),
        "product in wrong queue",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_does_not_advance_tick_rng_or_drain_events() {
        let mut sim = Sim::new(u64::MAX);
        sim.next_random();
        sim.events.push(crate::Event::Died { id: 1 });
        let before = sim.state_hash();
        let bytes = sim.save_bytes().unwrap();
        assert_eq!(before, sim.state_hash());
        let mut restored = Sim::from_save_bytes(&bytes).unwrap();
        assert_eq!(before, restored.state_hash());
        assert_eq!(sim.drain_events(), restored.drain_events());
        assert_eq!(sim.next_random(), restored.next_random());
        assert_eq!(
            bytes,
            Sim::from_save_bytes(&bytes).unwrap().save_bytes().unwrap()
        );
    }

    #[test]
    fn hostile_headers_and_grid_shapes_fail_before_use() {
        let bytes = Sim::new(1).save_bytes().unwrap();
        for end in [0, 7, 8, 11, 12, bytes.len() - 1] {
            assert!(Sim::from_save_bytes(&bytes[..end]).is_err());
        }
        let mut future = bytes.clone();
        future[8..12].copy_from_slice(&99u32.to_le_bytes());
        assert!(Sim::from_save_bytes(&future)
            .unwrap_err()
            .to_string()
            .contains("version 99"));
        let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
        json["terrain"] = serde_json::json!({"origin_x": 0, "origin_y": 0, "width": u64::MAX, "height": u64::MAX, "cells": []});
        let mut bad = bytes[..12].to_vec();
        bad.extend(serde_json::to_vec(&json).unwrap());
        assert!(Sim::from_save_bytes(&bad).is_err());
        json["terrain"]["width"] = 2.into();
        json["terrain"]["height"] = 2.into();
        bad.truncate(12);
        bad.extend(serde_json::to_vec(&json).unwrap());
        assert!(Sim::from_save_bytes(&bad)
            .unwrap_err()
            .to_string()
            .contains("terrain"));
    }

    #[test]
    fn private_motion_combat_and_harvest_state_survives_a_snapshot() {
        use crate::{terrain::Loco, Command, Fix, Harvest, Role};
        let mut sim = Sim::new(123);
        sim.set_terrain(crate::TerrainGrid::new(0, 0, 16, 16));
        sim.step(&[Command::Spawn {
            kind: 9,
            x: Fix::ONE,
            y: Fix::ONE,
            loco: Loco::Track,
            speed: Fix::from_raw(32),
            hp: 500,
            armor: 5,
            weapon: None,
        }]);
        let u = &mut sim.units[0];
        u.target = Some((Fix::from_int(10), Fix::ONE));
        u.guard = Some((Fix::from_int(12), Fix::ONE));
        u.path.extend([(2, 1), (3, 1)]);
        u.blocked = 7;
        u.attack = Some(77); // A dead target is legal until the next combat phase.
        u.cooldown = 13;
        u.shots = 19;
        u.cargo = 250;
        u.bails = 10;
        u.role = Role::Harvester;
        sim.projectiles.push(crate::Projectile {
            x: Fix::ONE,
            y: Fix::ONE,
            owner: 1,
            target: 77,
            damage: 65,
            speed: Fix::from_raw(16),
        });
        sim.terrain
            .as_mut()
            .unwrap()
            .set_resource(5, 5, crate::terrain::Overlay::Gem, 17);
        sim.terrain.as_ref().unwrap().hash(); // Cached fingerprints are rebuilt.
        for harvest in [
            Harvest::Idle,
            Harvest::ToOre(5, 5),
            Harvest::Loading(11),
            Harvest::ToRefinery(2, 2),
            Harvest::Unloading(23),
        ] {
            sim.units[0].harvest = harvest;
            let bytes = sim.save_bytes().unwrap();
            let mut restored = Sim::from_save_bytes(&bytes).unwrap();
            assert_eq!(sim.state_hash(), restored.state_hash());
            assert_eq!(sim.units, restored.units);
            assert_eq!(bytes, restored.save_bytes().unwrap());
            let mut uninterrupted = sim.clone();
            for _ in 0..30 {
                uninterrupted.step(&[]);
                restored.step(&[]);
                assert_eq!(uninterrupted.state_hash(), restored.state_hash());
            }
        }
        let bytes = sim.save_bytes().unwrap();
        for (field, value) in [
            ("shots", serde_json::json!(u32::MAX)),
            ("id", serde_json::json!(sim.next_id_peek())),
            ("x", serde_json::json!(1.5)),
        ] {
            let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
            json["units"][0][field] = value;
            let mut bad = bytes[..12].to_vec();
            bad.extend(serde_json::to_vec(&json).unwrap());
            assert!(
                Sim::from_save_bytes(&bad).is_err(),
                "accepted invalid {field}"
            );
        }
    }
}
