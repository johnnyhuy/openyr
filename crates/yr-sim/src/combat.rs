//! Combat data for the sim: weapon specs and damage application.
//!
//! Damage rule (this slice): `applied = floor(damage * verse)` where
//! verse is the warhead's multiplier against the target's armour class,
//! both as exact decimals from the rules tables. Splash, falloff and
//! radiation are later slices (parity issue tracks verification).

use crate::Fix;
use yr_rules::{Armor, Decimal, Rules};

/// Armour classes are indexed by `yr_rules::Armor::ALL` order.
pub const ARMOR_COUNT: usize = 11;

/// A resolved weapon the sim can fire.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WeaponSpec {
    pub damage: i64,
    /// Frames (ticks) between shots.
    pub rof_ticks: u32,
    /// Maximum range in leptons.
    pub range: Fix,
    /// Projectile speed in leptons per tick; zero = instant hit.
    pub projectile_speed: Fix,
    /// Damage multiplier per armour class (exact decimal).
    pub verses: [Decimal; ARMOR_COUNT],
}

impl WeaponSpec {
    /// Resolve a rules weapon (by ID) against all armour classes.
    /// `None` when the weapon or its data is incomplete.
    pub fn from_rules(rules: &Rules, weapon_id: &str) -> Option<WeaponSpec> {
        let w = rules.weapons.get(weapon_id)?;
        if !w.defined {
            return None;
        }
        let warhead = w.warhead.as_ref().and_then(|id| rules.warheads.get(id))?;
        let mut verses = [Decimal::ZERO; ARMOR_COUNT];
        for (i, armor) in Armor::ALL.iter().enumerate() {
            verses[i] = warhead.verse(*armor);
        }
        // Projectile speed: rules `Speed=` for the projectile object is a
        // velocity class; map it onto leptons/tick at 4 leptons per unit.
        let projectile = w
            .projectile
            .as_ref()
            .and_then(|id| rules.projectiles.get(id));
        let speed_units = if projectile.is_some() {
            w.speed.unwrap_or(0)
        } else {
            0
        };
        Some(WeaponSpec {
            damage: w.damage?,
            rof_ticks: w.rof.unwrap_or(0).max(0) as u32,
            range: cells_to_leptons(w.range?),
            projectile_speed: Fix::from_int((speed_units * 4) as i32),
            verses,
        })
    }

    /// Damage this weapon deals to armour class `armor` (index into
    /// `Armor::ALL`).
    pub fn damage_against(&self, armor: u8) -> i64 {
        let verse = self
            .verses
            .get(armor as usize)
            .copied()
            .unwrap_or(Decimal::ZERO);
        verse.mul_int(self.damage)
    }
}

/// Rules range is in cells (exact decimal); 1 cell = 256 leptons.
pub fn cells_to_leptons(cells: Decimal) -> Fix {
    Fix::from_raw(cells.to_leptons() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_math_is_exact() {
        let mut spec = WeaponSpec {
            damage: 65,
            rof_ticks: 10,
            range: Fix::from_int(1280),
            projectile_speed: Fix::ZERO,
            verses: [Decimal::ZERO; ARMOR_COUNT],
        };
        spec.verses[2] = Decimal::parse("90%").unwrap();
        assert_eq!(spec.damage_against(2), 58, "floor(65 * 0.9)");
        assert_eq!(spec.damage_against(0), 0);
        assert_eq!(
            cells_to_leptons(Decimal::parse("5.75").unwrap()),
            Fix::from_raw(1472)
        );
    }
}
