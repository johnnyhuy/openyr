//! Buildings in the sim: footprints, health and power.
//!
//! Power follows the rules convention verified in `rulesmd.ini`:
//! `Power=` positive produces (GAPOWR = 200), negative drains
//! (GAWEAP = -25). Produced power scales with the producer's health
//! fraction (#93); drain does not. While drained > produced the base
//! is on low power and production slows between the rules' min/max
//! low-power speeds.

use crate::Fix;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Building {
    pub id: u32,
    /// Caller type tag from `PlaceBuilding` (renderer art lookup).
    pub kind: u16,
    /// Top-left footprint cell and size in cells.
    pub cell: (u16, u16),
    pub size: (u16, u16),
    pub hp: i64,
    pub max_hp: i64,
    /// Armour class index (`Armor::ALL` order).
    pub armor: u8,
    /// Positive = produced, negative = drained.
    pub power: i64,
    pub owner: u8,
    pub role: crate::Role,
    /// Later buildings may place against this one (`BaseNormal=`).
    pub base_normal: bool,
}

impl Building {
    /// Power this building currently contributes to its owner totals.
    ///
    /// Producers scale linearly with the health fraction, via the RA2
    /// manual, Game Basics, Power: output depends on the health of the
    /// power sources. The exact original rounding is unobserved, so this
    /// uses truncating integer division, `power * hp / max_hp`, with
    /// checked widening arithmetic. Consumers keep their full drain at
    /// any health, as in the issue scope for #93. A missing or invalid
    /// `max_hp` keeps the authored output rather than dividing by zero.
    pub fn effective_power(&self) -> i64 {
        if self.power <= 0 || self.max_hp <= 0 {
            return self.power;
        }
        let hp = self.hp.clamp(0, self.max_hp);
        (i128::from(self.power) * i128::from(hp) / i128::from(self.max_hp)) as i64
    }

    /// Centre of the footprint in leptons (aim point).
    pub fn centre(&self) -> (Fix, Fix) {
        (
            Fix::from_int(i32::from(self.cell.0)) + Fix::from_raw(i32::from(self.size.0) * 128),
            Fix::from_int(i32::from(self.cell.1)) + Fix::from_raw(i32::from(self.size.1) * 128),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn building(power: i64, hp: i64, max_hp: i64) -> Building {
        Building {
            id: 1,
            kind: 0,
            cell: (0, 0),
            size: (2, 2),
            hp,
            max_hp,
            armor: 0,
            power,
            owner: 0,
            role: crate::Role::Combat,
            base_normal: true,
        }
    }

    #[test]
    fn producer_output_scales_with_health_and_floors() {
        assert_eq!(building(200, 1000, 1000).effective_power(), 200);
        assert_eq!(building(200, 500, 1000).effective_power(), 100);
        assert_eq!(building(200, 250, 1000).effective_power(), 50);
        assert_eq!(building(200, 999, 1000).effective_power(), 199);
        assert_eq!(building(200, 1, 1000).effective_power(), 0);
        assert_eq!(building(200, 0, 1000).effective_power(), 0);
        assert_eq!(building(75, 1, 3).effective_power(), 25);
    }

    #[test]
    fn drain_and_invalid_health_keep_authored_power() {
        assert_eq!(building(-150, 1, 1000).effective_power(), -150);
        assert_eq!(building(-150, 500, 1000).effective_power(), -150);
        assert_eq!(building(0, 1, 1000).effective_power(), 0);
        assert_eq!(building(200, 500, 0).effective_power(), 200);
        assert_eq!(building(200, 1500, 1000).effective_power(), 200);
        assert_eq!(building(200, -5, 1000).effective_power(), 0);
    }
}
