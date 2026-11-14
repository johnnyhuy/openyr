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

/// Match the bounded velocity domain accepted by simulation snapshots.
pub(crate) const MAX_PROJECTILE_SPEED: i32 = 1 << 24;

/// Match the range domain accepted by simulation snapshots.
pub(crate) const MAX_WEAPON_RANGE: i32 = 1 << 24;

/// A resolved weapon the sim can fire.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WeaponSpec {
    /// Stable rules weapon identity for presentation provenance.
    /// Absent for old saves or caller-created weapons. Never affects damage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<u64>,
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
    /// `None` when data is incomplete or its range/speed is unsupported.
    pub fn from_rules(rules: &Rules, weapon_id: &str) -> Option<WeaponSpec> {
        let w = rules.weapons.get(weapon_id)?;
        if !w.defined {
            return None;
        }
        let range = w.range?;
        if range.units() < 0 || !(0..=i64::from(MAX_WEAPON_RANGE)).contains(&range.to_leptons()) {
            return None;
        }
        let warhead = w.warhead.as_ref().and_then(|id| rules.warheads.get(id))?;
        let mut verses = [Decimal::ZERO; ARMOR_COUNT];
        for (i, armor) in Armor::ALL.iter().enumerate() {
            verses[i] = warhead.verse(*armor);
        }
        // Four raw leptons/tick per weapon Speed unit is the native policy,
        // pending original timing observation. One Fix backing unit is one
        // lepton; from_int would multiply the velocity by another 256.
        let projectile_speed = match w.projectile.as_deref() {
            None => Fix::ZERO,
            Some(id) => {
                let p = rules.projectiles.get(id).filter(|p| p.defined)?;
                if p.inviso == Some(true) {
                    Fix::ZERO
                } else {
                    let raw = i32::try_from(w.speed.unwrap_or(0).checked_mul(4)?).ok()?;
                    if !(0..=MAX_PROJECTILE_SPEED).contains(&raw) {
                        return None;
                    }
                    Fix::from_raw(raw)
                }
            }
        };
        Some(WeaponSpec {
            source: (rules
                .weapons
                .iter()
                .filter(|entry| entry.source_id() == w.source_id())
                .count()
                == 1)
                .then(|| w.source_id()),
            damage: w.damage?,
            rof_ticks: w.rof.unwrap_or(0).max(0) as u32,
            range: cells_to_leptons(range),
            projectile_speed,
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

/// Cells to a signed Fix, truncating toward zero and saturating at i32 limits.
/// Weapon resolution separately enforces the smaller snapshot range domain.
pub fn cells_to_leptons(cells: Decimal) -> Fix {
    Fix::from_raw(
        cells
            .to_leptons()
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cannon_rules(speed: &str, projectile: &str) -> Rules {
        Rules::load(&yr_formats::IniFile::parse(&format!(
            "[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage=65\nROF=60\nRange=5\nSpeed={speed}\nProjectile=Bullet\nWarhead=AP\n[Bullet]\n{projectile}\n[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n"
        )), &yr_formats::IniFile::parse(""))
    }

    #[test]
    fn rules_cannon_speed_keeps_launch_and_arrival_on_separate_ticks() {
        use crate::{Command, Event, Sim};
        let rules = cannon_rules("40", "Inviso=no");
        let weapon = WeaponSpec::from_rules(&rules, "Gun").unwrap();
        let mut sim = Sim::new(42);
        let spawn = |x, weapon| Command::Spawn {
            kind: 0,
            x: Fix::from_int(x),
            y: Fix::ZERO,
            loco: crate::terrain::Loco::Track,
            speed: Fix::ZERO,
            hp: 1000,
            armor: 5,
            weapon,
        };
        sim.step(&[
            spawn(0, Some(weapon)),
            spawn(4, None),
            Command::Attack { id: 1, target: 2 },
        ]);
        let launch = sim.drain_events();
        assert!(launch.iter().any(|e| matches!(
            e,
            Event::Fired {
                shooter: 1,
                target: 2
            }
        )));
        assert!(
            !launch.iter().any(|e| matches!(e, Event::Impact { .. })),
            "rules Speed=40 must not cross four cells on the launch tick: {launch:?}"
        );
        assert_eq!(
            weapon.projectile_speed.raw(),
            160,
            "four raw leptons per rules speed unit"
        );
        assert_eq!(sim.projectiles[0].x.raw(), 160);
        let mut restored = Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap();
        for tick in 2..=7 {
            let commands = if tick == 2 {
                vec![Command::Destroy { id: 1 }]
            } else {
                vec![]
            };
            sim.step(&commands);
            restored.step(&commands);
            assert_eq!(sim.state_hash(), restored.state_hash());
            let events = sim.drain_events();
            assert_eq!(events, restored.drain_events());
            assert_eq!(
                events
                    .iter()
                    .any(|e| matches!(e, Event::Impact { target: 2, .. })),
                tick == 7,
                "arrival on tick {tick}"
            );
        }
        assert_eq!(sim.units()[0].hp, 935);
    }

    #[test]
    fn invisible_rules_projectiles_remain_instant() {
        for speed in ["40", "100", "0"] {
            let rules = cannon_rules(speed, "Inviso=yes");
            assert_eq!(
                WeaponSpec::from_rules(&rules, "Gun")
                    .unwrap()
                    .projectile_speed,
                Fix::ZERO
            );
        }
    }

    #[test]
    fn physical_speed_conversion_rejects_negative_overflow_and_unsaveable_values() {
        for speed in ["-1", "4194305", "536870912", "9223372036854775807"] {
            assert!(
                WeaponSpec::from_rules(&cannon_rules(speed, "Inviso=no"), "Gun").is_none(),
                "unsafe speed {speed}"
            );
        }
        for (speed, raw) in [("0", 0), ("1", 4), ("4194304", MAX_PROJECTILE_SPEED)] {
            assert_eq!(
                WeaponSpec::from_rules(&cannon_rules(speed, "Inviso=no"), "Gun")
                    .unwrap()
                    .projectile_speed
                    .raw(),
                raw
            );
        }
        // Effective invisible velocity is zero even when the unused speed is huge.
        assert_eq!(
            WeaponSpec::from_rules(&cannon_rules("9223372036854775807", "Inviso=yes"), "Gun")
                .unwrap()
                .projectile_speed,
            Fix::ZERO
        );
        let absent = Rules::load(&yr_formats::IniFile::parse("[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage=65\nRange=5\nProjectile=Missing\nWarhead=AP\n[AP]"), &yr_formats::IniFile::parse(""));
        assert!(
            WeaponSpec::from_rules(&absent, "Gun").is_none(),
            "undefined projectile must not silently become hitscan"
        );
    }

    #[test]
    fn weapon_source_identity_survives_registry_reordering_and_case() {
        let load = |types| {
            Rules::load(&yr_formats::IniFile::parse(&format!(
            "[VehicleTypes]\n{types}\n[A]\nPrimary=One\n[B]\nPrimary=Two\n[One]\nDamage=65\nRange=5\nWarhead=AP\n[Two]\nDamage=15\nRange=5\nWarhead=SA\n[AP]\n[SA]\n"
        )), &yr_formats::IniFile::parse(""))
        };
        let first = load("0=A\n1=B");
        let second = load("0=B\n1=A");
        let source =
            |rules: &Rules, name| WeaponSpec::from_rules(rules, name).unwrap().source.unwrap();
        assert_eq!(source(&first, "One"), source(&second, "oNe"));
        assert_ne!(source(&first, "One"), source(&first, "Two"));
    }

    #[test]
    fn damage_math_is_exact() {
        let mut spec = WeaponSpec {
            source: None,
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
