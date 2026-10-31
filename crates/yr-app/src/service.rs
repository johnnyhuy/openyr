//! Resolve structure services from the merged retail and map rules.

use std::collections::BTreeMap;
use yr_rules::{Decimal, Rules};
use yr_sim::{Command, RepairSpec, OWNER_PLAYER};

#[derive(Default)]
pub(crate) struct Services {
    types: BTreeMap<String, (i64, bool, bool)>,
    repair: Option<RepairSpec>,
    refund: Option<Decimal>,
}

impl Services {
    pub fn load(rules: &Rules) -> Self {
        let globals = &rules.globals;
        let repair = (|| {
            let rate = globals.repair_rate?.units();
            let interval = u32::try_from(
                (i128::from(rate) * 60 * i128::from(crate::session::TICKS_PER_SEC) + 999_999)
                    / 1_000_000,
            )
            .ok()?;
            let percent = globals.repair_percent?;
            let step = globals.repair_step?;
            (rate > 0
                && interval > 0
                && (0..=1_000_000).contains(&percent.units())
                && (1..=i64::from(i32::MAX)).contains(&step))
            .then_some(RepairSpec {
                cost: 0,
                percent,
                step,
                interval,
            })
        })();
        Self {
            types: rules
                .buildings
                .iter()
                .filter_map(|t| {
                    let cost = t.cost?;
                    (0..=i64::from(i32::MAX)).contains(&cost).then(|| {
                        (
                            t.id.to_ascii_uppercase(),
                            (
                                cost,
                                t.repairable != Some(false) && t.click_repairable != Some(false),
                                t.unsellable != Some(true),
                            ),
                        )
                    })
                })
                .collect(),
            repair,
            refund: globals
                .refund_percent
                .filter(|d| (0..=1_000_000).contains(&d.units())),
        }
    }
}

impl crate::session::Session {
    fn service_type(&self, id: u32) -> Option<(i64, bool, bool)> {
        let building = self
            .sim
            .buildings()
            .iter()
            .find(|b| b.id == id && b.owner == OWNER_PLAYER && b.hp > 0)?;
        self.services
            .types
            .get(
                &self
                    .kinds
                    .get(usize::from(building.kind))?
                    .to_ascii_uppercase(),
            )
            .copied()
    }

    /// Toggle the wrench job on a live owned structure. Full health is a no-op.
    pub fn repair(&self, id: u32) -> Option<Command> {
        if self.sim.repairing(id) {
            return Some(Command::Repair { id, spec: None });
        }
        let (cost, allowed, _) = self.service_type(id)?;
        let building = self.sim.buildings().iter().find(|b| b.id == id)?;
        if !allowed || building.hp >= building.max_hp {
            return None;
        }
        let mut spec = self.services.repair?;
        spec.cost = cost;
        Some(Command::Repair {
            id,
            spec: Some(spec),
        })
    }

    pub fn sell(&self, id: u32) -> Option<Command> {
        let (cost, _, allowed) = self.service_type(id)?;
        allowed.then_some(Command::Sell {
            id,
            cost,
            percent: self.services.refund?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_formats::IniFile;

    #[test]
    fn map_flags_and_hostile_repair_globals_are_respected() {
        let mut ini = IniFile::parse("[General]\nRefundPercent=50%\nRepairPercent=15%\nRepairRate=.016\nRepairStep=8\n[BuildingTypes]\n0=A\n1=B\n2=C\n[A]\nCost=101\n[B]\nCost=900\nRepairable=no\n[C]\nCost=100\nClickRepairable=no\nUnsellable=yes");
        let rules = Rules::load(&ini, &IniFile::default());
        let services = Services::load(&rules);
        assert_eq!(services.types["A"], (101, true, true));
        assert_eq!(services.types["B"], (900, false, true));
        assert_eq!(services.types["C"], (100, false, false));
        assert_eq!(services.repair.unwrap().interval, 15);
        ini.merge_from(&IniFile::parse(
            "[General]\nRepairRate=-1\nRefundPercent=200%\n[A]\nClickRepairable=no\nUnsellable=yes",
        ));
        let services = Services::load(&Rules::load(&ini, &IniFile::default()));
        assert_eq!(services.types["A"], (101, false, false));
        assert!(services.repair.is_none());
        assert!(services.refund.is_none());
    }
}
