//! Production queues: each owner's factories turn credits into units.
//!
//! Observed RA2 behaviour this follows: the cost is drawn from the
//! credit pool gradually as the build progresses (not up front), the
//! build halts while the pool cannot cover the next slice, a queue
//! builds one item at a time per factory kind, cancelling refunds what
//! was spent, and low power slows progress by the production factor.
//! A queue with no live factory building of its kind does not progress.
//! Structures build at the construction yard and, once done, wait at the
//! head of their queue until placed on the map (`Command::PlaceProduced`).
//!
//! Each owner has a separate credit pool and independent factory queues.

use crate::combat::WeaponSpec;
use crate::{terrain::Loco, Fix, Role};

/// Which factory a build order comes out of.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum Factory {
    /// War factory: vehicles.
    Vehicle,
    /// Barracks: infantry.
    Infantry,
    /// Construction yard: structures, placed by the player when done.
    Building,
}

impl Factory {
    pub const ALL: [Factory; 3] = [Factory::Vehicle, Factory::Infantry, Factory::Building];

    /// The building role that hosts this queue.
    pub fn role(self) -> Role {
        match self {
            Factory::Vehicle => Role::WarFactory,
            Factory::Infantry => Role::Barracks,
            Factory::Building => Role::ConstructionYard,
        }
    }
}

/// Everything the sim needs to spawn a produced unit.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UnitSpec {
    pub loco: Loco,
    pub speed: Fix,
    pub hp: i64,
    pub armor: u8,
    pub weapon: Option<WeaponSpec>,
    /// `Role::Harvester` units start their harvest loop on exit.
    pub role: Role,
}

/// Everything the sim needs to put a structure on the map.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuildingSpec {
    /// Footprint in cells.
    pub w: u16,
    pub h: u16,
    pub hp: i64,
    pub armor: u8,
    /// Positive = produced, negative = drained.
    pub power: i64,
    pub role: Role,
    /// Rules `Adjacent=`: the most empty cells allowed between the
    /// footprint and a base building of the same owner.
    pub adjacent: u8,
    /// Rules `BaseNormal=`: whether it lets later buildings place
    /// against it (defences and walls do not).
    pub base_normal: bool,
    /// Rules `FreeUnit=`: a unit (kind tag, spec) that rolls out beside
    /// the building once it is placed.
    pub free_unit: Option<(u16, UnitSpec)>,
}

/// What a build order turns into when done.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Product {
    /// Rolls out of the factory by itself.
    Unit(UnitSpec),
    /// Waits for the player to place it.
    Building(BuildingSpec),
}

impl Product {
    /// Omit presentation metadata from the legacy setup compatibility key.
    pub fn without_weapon_source(mut self) -> Self {
        let unit = match &mut self {
            Self::Unit(unit) => Some(unit),
            Self::Building(building) => building.free_unit.as_mut().map(|(_, unit)| unit),
        };
        if let Some(weapon) = unit.and_then(|unit| unit.weapon.as_mut()) {
            weapon.source = None;
        }
        self
    }
}

/// One queued build. `kind` is a caller-chosen type index (the UI's
/// buildable slot) echoed back in events.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuildOrder {
    pub factory: Factory,
    pub kind: u16,
    pub product: Product,
    pub cost: i64,
    /// Build time in ticks at full speed (>= 1).
    pub ticks: u32,
    /// Progress in 1/256 ticks; done at `ticks * 256`.
    pub progress: i64,
    /// Credits drawn so far.
    pub paid: i64,
    /// Explicit player suspension, separate from credit/power/factory stalls.
    #[serde(default, skip_serializing_if = "is_false")]
    pub held: bool,
}

fn is_false(value: &bool) -> bool {
    !value
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
        if self.held {
            return None;
        }
        let total = self.total();
        if self.progress >= total {
            return Some(0);
        }
        let step = i64::from(factor.raw()).max(1);
        let next = (self.progress + step).min(total);
        let due = (i128::from(self.cost.max(0)) * i128::from(next) / i128::from(total)) as i64;
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

/// Shared queue edits for tick application and pending UI intent previews.
/// Returns the exact refund; previews never apply it to live credits.
pub(crate) fn apply(queue: &mut Vec<BuildOrder>, command: crate::Command) -> i64 {
    use crate::Command;
    match command {
        Command::Produce {
            factory,
            kind,
            product,
            cost,
            ticks,
        } => {
            queue.push(BuildOrder {
                factory,
                kind,
                product,
                cost: cost.max(0),
                ticks: ticks.max(1),
                progress: 0,
                paid: 0,
                held: false,
            });
        }
        Command::HoldProduce {
            factory,
            kind,
            held,
        } => {
            if let Some(order) = queue
                .iter_mut()
                .find(|o| o.factory == factory)
                .filter(|o| o.kind == kind && !o.done())
            {
                order.held = held;
            }
        }
        Command::CancelProduce { factory, kind } => {
            let held_head = queue
                .iter()
                .position(|o| o.factory == factory)
                .filter(|&i| queue[i].kind == kind && queue[i].held);
            if let Some(i) = held_head.or_else(|| {
                queue
                    .iter()
                    .rposition(|o| o.factory == factory && o.kind == kind)
            }) {
                return queue.remove(i).paid;
            }
        }
        _ => {}
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(cost: i64, ticks: u32) -> BuildOrder {
        BuildOrder {
            factory: Factory::Vehicle,
            kind: 0,
            product: Product::Unit(UnitSpec {
                loco: Loco::Track,
                speed: Fix::from_int(1),
                hp: 100,
                armor: 0,
                weapon: None,
                role: Role::Combat,
            }),
            cost,
            ticks,
            progress: 0,
            paid: 0,
            held: false,
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

#[cfg(test)]
mod hold_tests;
