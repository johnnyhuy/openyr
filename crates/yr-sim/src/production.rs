//! Production queues: the player's factories turn credits into units.
//!
//! Observed RA2 behaviour this follows: the cost is drawn from the
//! credit pool gradually as the build progresses (not up front), the
//! build halts while the pool cannot cover the next slice, a queue
//! builds one item at a time per factory kind, cancelling refunds what
//! was spent, and low power slows progress by the production factor.
//! A queue with no live factory building of its kind does not progress.
//!
//! Credits are one pool (the player's, see `Sim::credits`), so only the
//! player produces; the scripted enemy gets its forces from waves.

use crate::combat::WeaponSpec;
use crate::{terrain::Loco, Fix, Role};

/// Which factory a build order comes out of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Factory {
    /// War factory: vehicles.
    Vehicle,
    /// Barracks: infantry.
    Infantry,
}

impl Factory {
    pub const ALL: [Factory; 2] = [Factory::Vehicle, Factory::Infantry];

    /// The building role that hosts this queue.
    pub fn role(self) -> Role {
        match self {
            Factory::Vehicle => Role::WarFactory,
            Factory::Infantry => Role::Barracks,
        }
    }
}

/// Everything the sim needs to spawn a produced unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitSpec {
    pub loco: Loco,
    pub speed: Fix,
    pub hp: i64,
    pub armor: u8,
    pub weapon: Option<WeaponSpec>,
    /// `Role::Harvester` units start their harvest loop on exit.
    pub role: Role,
}

/// One queued build. `kind` is a caller-chosen type index (the UI's
/// buildable slot) echoed back in events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildOrder {
    pub factory: Factory,
    pub kind: u16,
    pub spec: UnitSpec,
    pub cost: i64,
    /// Build time in ticks at full speed (>= 1).
    pub ticks: u32,
    /// Progress in 1/256 ticks; done at `ticks * 256`.
    pub progress: i64,
    /// Credits drawn so far.
    pub paid: i64,
}

impl BuildOrder {
    pub fn total(&self) -> i64 {
        i64::from(self.ticks.max(1)) * 256
    }

    /// Progress as (done, total) for UI clocks.
    pub fn fraction(&self) -> (i64, i64) {
        (self.progress, self.total())
    }

    /// Advance by one tick at `factor` if `credits` covers the slice.
    /// Returns the credits drawn (0 when stalled or already done).
    pub(crate) fn advance(&mut self, factor: Fix, credits: i64) -> Option<i64> {
        let total = self.total();
        if self.progress >= total {
            return Some(0);
        }
        let step = i64::from(factor.raw()).max(1);
        let next = (self.progress + step).min(total);
        let due = self.cost.max(0) * next / total;
        let delta = due - self.paid;
        if delta > credits {
            return None;
        }
        self.progress = next;
        self.paid = due;
        Some(delta)
    }

    pub fn done(&self) -> bool {
        self.progress >= self.total()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(cost: i64, ticks: u32) -> BuildOrder {
        BuildOrder {
            factory: Factory::Vehicle,
            kind: 0,
            spec: UnitSpec {
                loco: Loco::Track,
                speed: Fix::from_int(1),
                hp: 100,
                armor: 0,
                weapon: None,
                role: Role::Combat,
            },
            cost,
            ticks,
            progress: 0,
            paid: 0,
        }
    }

    #[test]
    fn cost_is_drawn_gradually_and_sums_exactly() {
        let mut o = order(700, 441);
        let mut spent = 0;
        let mut n = 0;
        while !o.done() {
            spent += o.advance(Fix::ONE, i64::MAX).unwrap();
            n += 1;
        }
        assert_eq!((spent, n), (700, 441));
    }

    #[test]
    fn stalls_without_credits_and_slows_at_half_power() {
        let mut o = order(100, 10);
        assert_eq!(o.advance(Fix::ONE, 5), None);
        assert_eq!(o.progress, 0);
        let mut n = 0;
        while !o.done() {
            o.advance(Fix::from_raw(128), i64::MAX).unwrap();
            n += 1;
        }
        assert_eq!(n, 20);
    }
}
