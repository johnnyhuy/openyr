//! Buildings in the sim: footprints, health and power.
//!
//! Power follows the rules convention verified in `rulesmd.ini`:
//! `Power=` positive produces (GAPOWR = 200), negative drains
//! (GAWEAP = -25). While drained > produced the base is on low power
//! and production slows between the rules' min/max low-power speeds.

use crate::Fix;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Building {
    pub id: u32,
    /// Top-left footprint cell and size in cells.
    pub cell: (u16, u16),
    pub size: (u16, u16),
    pub hp: i64,
    pub max_hp: i64,
    /// Armour class index (`Armor::ALL` order).
    pub armor: u8,
    /// Positive = produced, negative = drained.
    pub power: i64,
}

impl Building {
    /// Centre of the footprint in leptons (aim point).
    pub fn centre(&self) -> (Fix, Fix) {
        (
            Fix::from_int(i32::from(self.cell.0)) + Fix::from_raw(i32::from(self.size.0) * 128),
            Fix::from_int(i32::from(self.cell.1)) + Fix::from_raw(i32::from(self.size.1) * 128),
        )
    }
}
