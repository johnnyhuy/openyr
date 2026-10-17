//! Typed Yuri's Revenge game data from `rulesmd.ini` + `artmd.ini`.
//!
//! Pure: callers supply parsed INIs (map overrides merged with
//! [`yr_formats::IniFile::merge_from`]) and get typed technos, weapons,
//! warheads and projectiles plus a validation report. Semantics and evidence:
//! `docs/formats/rules.md`.
//!
//! Unspecified keys stay `None` ("engine default"). Engine defaults are only
//! filled in where documented, so nothing here guesses a number the original
//! game would compute differently.

pub mod value;

use std::collections::HashMap;
use value::{parse_list, parse_ref};
use yr_formats::{IniFile, IniSection};

pub use value::{Armor, Decimal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TechnoKind {
    Infantry,
    Vehicle,
    Aircraft,
    Building,
}

impl TechnoKind {
    pub const ALL: [TechnoKind; 4] = [
        TechnoKind::Infantry,
        TechnoKind::Vehicle,
        TechnoKind::Aircraft,
        TechnoKind::Building,
    ];

    /// The `rulesmd.ini` type list that registers this kind.
    pub fn list_section(self) -> &'static str {
        match self {
            TechnoKind::Infantry => "InfantryTypes",
            TechnoKind::Vehicle => "VehicleTypes",
            TechnoKind::Aircraft => "AircraftTypes",
            TechnoKind::Building => "BuildingTypes",
        }
    }
}

/// Art data from the `artmd.ini` section named by `Image=` (or the ID).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtInfo {
    pub section: String,
    /// `Voxel=yes`: drawn from `<image>.VXL` (+ turret/barrel) instead of SHP.
    pub voxel: bool,
    /// `Theater=yes`: the art file carries the theater extension
    /// (e.g. `.TEM`) instead of `.SHP` (terrain objects, overlays).
    pub theater: bool,
    /// `NewTheater=yes`: the art filename's second character is replaced
    /// by the theater letter (e.g. `GACNST` -> `GTCNST.SHP`), with the
    /// generic `G` variant as fallback (buildings).
    pub new_theater: bool,
    /// `Foundation=` footprint in cells, e.g. `4x4` (buildings).
    pub foundation: Option<(u8, u8)>,
    pub cameo: Option<String>,
    pub alt_cameo: Option<String>,
    pub remapable: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnoType {
    pub id: String,
    pub kind: TechnoKind,
    /// Position in its type list (the engine's array index).
    pub index: usize,
    /// False when listed but without a rules section (engine defaults only).
    pub defined: bool,
    pub name: Option<String>,
    /// CSF label for the display name, e.g. `Name:HTNK`.
    pub ui_name: Option<String>,
    pub strength: Option<i64>,
    /// Building `Power=` (positive produces, negative drains).
    pub power: Option<i64>,
    pub armor: Option<Armor>,
    pub cost: Option<i64>,
    pub points: Option<i64>,
    pub tech_level: Option<i64>,
    pub sight: Option<i64>,
    pub speed: Option<i64>,
    pub owner: Vec<String>,
    pub prerequisite: Vec<String>,
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub elite_primary: Option<String>,
    pub elite_secondary: Option<String>,
    /// `Weapon1`..`WeaponN` (IFV-style and multi-weapon units).
    pub weapons: Vec<String>,
    /// `Image=`, defaulting to the ID.
    pub image: String,
    pub art: Option<ArtInfo>,
}

impl TechnoType {
    /// Every weapon this type can fire, deduplicated, in declaration order.
    pub fn all_weapons(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        let named = [
            &self.primary,
            &self.secondary,
            &self.elite_primary,
            &self.elite_secondary,
        ];
        for w in named.into_iter().flatten().chain(self.weapons.iter()) {
            if !out.iter().any(|o| o.eq_ignore_ascii_case(w)) {
                out.push(w);
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponType {
    pub id: String,
    pub defined: bool,
    pub damage: Option<i64>,
    /// Rate of fire: frames between shots.
    pub rof: Option<i64>,
    /// In cells; `-2` is the RA2 "ignore range" special value.
    pub range: Option<Decimal>,
    pub minimum_range: Option<Decimal>,
    pub speed: Option<i64>,
    pub burst: Option<i64>,
    pub projectile: Option<String>,
    pub warhead: Option<String>,
    pub report: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarheadType {
    pub id: String,
    pub defined: bool,
    /// Multiplier per armor type, `Verses=` order. `None` = not specified.
    pub verses: Option<[Decimal; 11]>,
    pub cell_spread: Option<Decimal>,
    pub percent_at_max: Option<Decimal>,
    pub wall: Option<bool>,
}

impl WarheadType {
    /// Multiplier against `armor`; unspecified verses count as 100%
    /// (ModEnc "Verses"; to be confirmed against the oracle).
    pub fn verse(&self, armor: Armor) -> Decimal {
        self.verses.map_or(Decimal::ONE, |v| v[armor.index()])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectileType {
    pub id: String,
    pub defined: bool,
    pub arcing: Option<bool>,
    pub inviso: Option<bool>,
    pub anti_air: Option<bool>,
    pub anti_ground: Option<bool>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Legal and present in retail data, but worth knowing (e.g. a listed
    /// type without a section).
    Note,
    /// Likely a data bug: an unparseable value or a dangling reference.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub section: String,
    pub message: String,
}

/// Ordered registry: insertion order plus case-insensitive lookup.
#[derive(Debug, Clone)]
pub struct Registry<T> {
    items: Vec<T>,
    index: HashMap<String, usize>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Registry {
            items: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<T> Registry<T> {
    pub fn get(&self, id: &str) -> Option<&T> {
        self.index
            .get(&id.to_ascii_uppercase())
            .map(|&i| &self.items[i])
    }

    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(&id.to_ascii_uppercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn insert(&mut self, id: &str, item: T) -> bool {
        let key = id.to_ascii_uppercase();
        if self.index.contains_key(&key) {
            return false;
        }
        self.items.push(item);
        self.index.insert(key, self.items.len() - 1);
        true
    }
}

/// Game-wide tuning from `[General]` and `[CombatDamage]`.
/// `None` means the retail file does not specify it (engine default,
/// deliberately not guessed here).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Globals {
    pub build_speed: Option<Decimal>,
    pub buildup_time: Option<Decimal>,
    pub low_power_penalty_modifier: Option<Decimal>,
    pub min_low_power_production_speed: Option<Decimal>,
    pub max_low_power_production_speed: Option<Decimal>,
    pub wall_build_speed_coefficient: Option<Decimal>,
    pub refund_percent: Option<Decimal>,
    pub repair_percent: Option<Decimal>,
    pub repair_rate: Option<Decimal>,
    pub repair_step: Option<i64>,
    pub ore_growth_rate: Option<Decimal>,
    pub tiberium_grows: Option<bool>,
    pub tiberium_spreads: Option<bool>,
    pub harvester_immune: Option<bool>,
    pub destroyable_bridges: Option<bool>,
    pub ballistic_scatter: Option<Decimal>,
    pub bridge_strength: Option<i64>,
    pub atom_damage: Option<i64>,
    pub ion_cannon_damage: Option<i64>,
}

/// Skirmish defaults from `[MultiplayerDialogSettings]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkirmishDefaults {
    pub money: Option<i64>,
    pub min_money: Option<i64>,
    pub max_money: Option<i64>,
    pub unit_count: Option<i64>,
    pub tech_level: Option<i64>,
}

/// A playable house/country from `[Sides]` (GDI/Nod/ThirdSide/...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct House {
    pub id: String,
    /// The `[Sides]` list it belongs to (e.g. `GDI`, `Nod`, `ThirdSide`).
    pub side: String,
    pub color: Option<String>,
    /// `(hue, saturation, value)` 0-255 from `[Colors]`.
    pub color_hsv: Option<(i64, i64, i64)>,
    /// Art prefix letter (art `Suffix=`), e.g. `Allied`/`Soviet`.
    pub suffix: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Rules {
    pub infantry: Registry<TechnoType>,
    pub vehicles: Registry<TechnoType>,
    pub aircraft: Registry<TechnoType>,
    pub buildings: Registry<TechnoType>,
    pub weapons: Registry<WeaponType>,
    pub warheads: Registry<WarheadType>,
    pub projectiles: Registry<ProjectileType>,
    pub globals: Globals,
    pub skirmish: SkirmishDefaults,
    pub houses: Registry<House>,
    pub issues: Vec<Issue>,
}

impl Rules {
    pub fn load(rules: &IniFile, art: &IniFile) -> Rules {
        let mut out = Rules::default();
        for kind in TechnoKind::ALL {
            for (index, id) in type_list(rules, kind.list_section(), &mut out.issues)
                .into_iter()
                .enumerate()
            {
                let t = load_techno(rules, art, &id, kind, index, &mut out.issues);
                let reg = out.registry_mut(kind);
                if !reg.insert(&id, t) {
                    out.issues.push(issue(
                        Severity::Note,
                        kind.list_section(),
                        format!("{id} listed twice; first kept"),
                    ));
                }
            }
        }

        // Weapons exist when referenced (there is no weapon type list).
        let refs: Vec<(String, String)> = TechnoKind::ALL
            .iter()
            .flat_map(|&k| out.registry(k).iter())
            .flat_map(|t| {
                t.all_weapons()
                    .into_iter()
                    .map(|w| (w.to_string(), t.id.clone()))
            })
            .collect();
        for (w, user) in refs {
            if out.weapons.contains(&w) {
                continue;
            }
            let weapon = load_weapon(rules, &w);
            if !weapon.defined {
                out.issues.push(issue(
                    Severity::Warning,
                    &user,
                    format!("weapon {w} has no section"),
                ));
            }
            out.weapons.insert(&w, weapon);
        }

        // Warheads: the [Warheads] list, then any referenced by weapons.
        let mut warhead_ids = type_list(rules, "Warheads", &mut out.issues);
        warhead_ids.extend(out.weapons.iter().filter_map(|w| w.warhead.clone()));
        for id in warhead_ids {
            if !out.warheads.contains(&id) {
                let wh = load_warhead(rules, &id, &mut out.issues);
                out.warheads.insert(&id, wh);
            }
        }

        let projectile_ids: Vec<String> = out
            .weapons
            .iter()
            .filter_map(|w| w.projectile.clone())
            .collect();
        for id in projectile_ids {
            if !out.projectiles.contains(&id) {
                let p = load_projectile(rules, &id);
                if !p.defined {
                    out.issues.push(issue(
                        Severity::Warning,
                        &id,
                        "projectile has no section".into(),
                    ));
                }
                out.projectiles.insert(&id, p);
            }
        }
        out.globals = load_globals(rules);
        out.skirmish = load_skirmish(rules);
        out.houses = load_houses(rules);
        out
    }

    pub fn registry(&self, kind: TechnoKind) -> &Registry<TechnoType> {
        match kind {
            TechnoKind::Infantry => &self.infantry,
            TechnoKind::Vehicle => &self.vehicles,
            TechnoKind::Aircraft => &self.aircraft,
            TechnoKind::Building => &self.buildings,
        }
    }

    fn registry_mut(&mut self, kind: TechnoKind) -> &mut Registry<TechnoType> {
        match kind {
            TechnoKind::Infantry => &mut self.infantry,
            TechnoKind::Vehicle => &mut self.vehicles,
            TechnoKind::Aircraft => &mut self.aircraft,
            TechnoKind::Building => &mut self.buildings,
        }
    }

    /// Look a techno up in any of the four registries.
    pub fn techno(&self, id: &str) -> Option<&TechnoType> {
        TechnoKind::ALL
            .iter()
            .find_map(|&k| self.registry(k).get(id))
    }

    pub fn technos(&self) -> impl Iterator<Item = &TechnoType> {
        TechnoKind::ALL
            .into_iter()
            .flat_map(move |k| self.registry(k).iter())
    }
}

fn issue(severity: Severity, section: &str, message: String) -> Issue {
    Issue {
        severity,
        section: section.to_string(),
        message,
    }
}

/// Values of a type-list section in engine order. A reused index keeps its
/// first position but takes the last value (later lines overwrite).
fn type_list(rules: &IniFile, section: &str, issues: &mut Vec<Issue>) -> Vec<String> {
    let Some(sec) = rules.section(section) else {
        issues.push(issue(
            Severity::Warning,
            section,
            "type list missing".into(),
        ));
        return Vec::new();
    };
    let mut keys: Vec<String> = Vec::new();
    let mut values: HashMap<String, String> = HashMap::new();
    for (k, v) in &sec.entries {
        let key = k.to_ascii_uppercase();
        if values.insert(key.clone(), v.trim().to_string()).is_some() {
            issues.push(issue(
                Severity::Note,
                section,
                format!("index {k} reused; last value wins"),
            ));
        } else {
            keys.push(key);
        }
    }
    keys.iter().filter_map(|k| parse_ref(&values[k])).collect()
}

fn int(sec: Option<&IniSection>, key: &str) -> Option<i64> {
    sec?.get_i64(key)
}

fn dec(sec: Option<&IniSection>, key: &str) -> Option<Decimal> {
    Decimal::parse(sec?.get(key)?)
}

fn reference(sec: Option<&IniSection>, key: &str) -> Option<String> {
    parse_ref(sec?.get(key)?)
}

fn string(sec: Option<&IniSection>, key: &str) -> Option<String> {
    sec?.get(key).map(str::to_string)
}

fn load_techno(
    rules: &IniFile,
    art: &IniFile,
    id: &str,
    kind: TechnoKind,
    index: usize,
    issues: &mut Vec<Issue>,
) -> TechnoType {
    let sec = rules.section(id);
    if sec.is_none() {
        issues.push(issue(
            Severity::Note,
            id,
            format!("listed in [{}] but has no section", kind.list_section()),
        ));
    }
    let armor = sec.and_then(|s| s.get("Armor")).map(|text| {
        let a = Armor::parse(text);
        if a.is_none() {
            issues.push(issue(
                Severity::Warning,
                id,
                format!("unknown Armor={text}"),
            ));
        }
        a
    });
    let mut weapons = Vec::new();
    if let Some(s) = sec {
        let count = s.get_i64("WeaponCount").unwrap_or(0).clamp(0, 64);
        for n in 1..=count {
            if let Some(w) = reference(Some(s), &format!("Weapon{n}")) {
                weapons.push(w);
            }
        }
    }
    let image = reference(sec, "Image").unwrap_or_else(|| id.to_string());
    let art_sec = art.section(&image);
    let art_info = art_sec.map(|a| ArtInfo {
        section: a.name.clone(),
        voxel: a.get_bool("Voxel").unwrap_or(false),
        theater: a.get_bool("Theater").unwrap_or(false),
        new_theater: a.get_bool("NewTheater").unwrap_or(false),
        foundation: a.get("Foundation").and_then(|v| {
            let (w, h) = v.trim().split_once(['x', 'X'])?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        }),
        cameo: reference(Some(a), "Cameo"),
        alt_cameo: reference(Some(a), "AltCameo"),
        remapable: a.get_bool("Remapable"),
    });
    if sec.is_some() && art_info.is_none() {
        issues.push(issue(
            Severity::Note,
            id,
            format!("no artmd.ini section [{image}]"),
        ));
    }
    TechnoType {
        id: id.to_string(),
        kind,
        index,
        defined: sec.is_some(),
        name: string(sec, "Name"),
        ui_name: string(sec, "UIName"),
        strength: int(sec, "Strength"),
        power: int(sec, "Power"),
        armor: armor.flatten(),
        cost: int(sec, "Cost"),
        points: int(sec, "Points"),
        tech_level: int(sec, "TechLevel"),
        sight: int(sec, "Sight"),
        speed: int(sec, "Speed"),
        owner: sec
            .and_then(|s| s.get("Owner"))
            .map(parse_list)
            .unwrap_or_default(),
        prerequisite: sec
            .and_then(|s| s.get("Prerequisite"))
            .map(parse_list)
            .unwrap_or_default(),
        primary: reference(sec, "Primary"),
        secondary: reference(sec, "Secondary"),
        elite_primary: reference(sec, "ElitePrimary"),
        elite_secondary: reference(sec, "EliteSecondary"),
        weapons,
        image,
        art: art_info,
    }
}

fn load_weapon(rules: &IniFile, id: &str) -> WeaponType {
    let sec = rules.section(id);
    WeaponType {
        id: id.to_string(),
        defined: sec.is_some(),
        damage: int(sec, "Damage"),
        rof: int(sec, "ROF"),
        range: dec(sec, "Range"),
        minimum_range: dec(sec, "MinimumRange"),
        speed: int(sec, "Speed"),
        burst: int(sec, "Burst"),
        projectile: reference(sec, "Projectile"),
        warhead: reference(sec, "Warhead"),
        report: sec
            .and_then(|s| s.get("Report"))
            .map(parse_list)
            .unwrap_or_default(),
    }
}

fn load_warhead(rules: &IniFile, id: &str, issues: &mut Vec<Issue>) -> WarheadType {
    let sec = rules.section(id);
    if sec.is_none() {
        issues.push(issue(
            Severity::Warning,
            id,
            "warhead has no section".into(),
        ));
    }
    let verses = sec.and_then(|s| s.get("Verses")).and_then(|text| {
        let parts: Vec<Option<Decimal>> = text.split(',').map(Decimal::parse).collect();
        if parts.len() != 11 || parts.iter().any(Option::is_none) {
            issues.push(issue(
                Severity::Warning,
                id,
                format!("Verses needs 11 values, got {text}"),
            ));
            return None;
        }
        let mut v = [Decimal::ONE; 11];
        for (slot, p) in v.iter_mut().zip(parts) {
            *slot = p.unwrap_or(Decimal::ONE);
        }
        Some(v)
    });
    WarheadType {
        id: id.to_string(),
        defined: sec.is_some(),
        verses,
        cell_spread: dec(sec, "CellSpread"),
        percent_at_max: dec(sec, "PercentAtMax"),
        wall: sec.and_then(|s| s.get_bool("Wall")),
    }
}

fn load_projectile(rules: &IniFile, id: &str) -> ProjectileType {
    let sec = rules.section(id);
    let flag = |k: &str| sec.and_then(|s| s.get_bool(k));
    ProjectileType {
        id: id.to_string(),
        defined: sec.is_some(),
        arcing: flag("Arcing"),
        inviso: flag("Inviso"),
        anti_air: flag("AA"),
        anti_ground: flag("AG"),
        image: reference(sec, "Image"),
    }
}

fn load_globals(rules: &IniFile) -> Globals {
    let g = rules.section("General");
    let c = rules.section("CombatDamage");
    let dec = |s: Option<&IniSection>, k: &str| s.and_then(|x| x.get(k)).and_then(Decimal::parse);
    let int = |s: Option<&IniSection>, k: &str| s.and_then(|x| x.get_i64(k));
    let boolean = |s: Option<&IniSection>, k: &str| s.and_then(|x| x.get_bool(k));
    Globals {
        build_speed: dec(g, "BuildSpeed"),
        buildup_time: dec(g, "BuildupTime"),
        low_power_penalty_modifier: dec(g, "LowPowerPenaltyModifier"),
        min_low_power_production_speed: dec(g, "MinLowPowerProductionSpeed"),
        max_low_power_production_speed: dec(g, "MaxLowPowerProductionSpeed"),
        wall_build_speed_coefficient: dec(g, "WallBuildSpeedCoefficient"),
        refund_percent: dec(g, "RefundPercent"),
        repair_percent: dec(g, "RepairPercent"),
        repair_rate: dec(g, "RepairRate"),
        repair_step: int(g, "RepairStep"),
        ore_growth_rate: dec(g, "GrowthRate"),
        tiberium_grows: boolean(g, "TiberiumGrows"),
        tiberium_spreads: boolean(g, "TiberiumSpreads"),
        harvester_immune: boolean(c, "HarvesterImmune"),
        destroyable_bridges: boolean(c, "DestroyableBridges"),
        ballistic_scatter: dec(c, "BallisticScatter"),
        bridge_strength: int(c, "BridgeStrength"),
        atom_damage: int(c, "AtomDamage"),
        ion_cannon_damage: int(c, "IonCannonDamage"),
    }
}

fn load_skirmish(rules: &IniFile) -> SkirmishDefaults {
    let s = rules.section("MultiplayerDialogSettings");
    let int = |k: &str| s.and_then(|x| x.get_i64(k));
    SkirmishDefaults {
        money: int("Money"),
        min_money: int("MinMoney"),
        max_money: int("MaxMoney"),
        unit_count: int("UnitCount"),
        tech_level: int("TechLevel"),
    }
}

fn load_houses(rules: &IniFile) -> Registry<House> {
    let mut out = Registry::default();
    let Some(sides) = rules.section("Sides") else {
        return out;
    };
    let colors = rules.section("Colors");
    for side in ["GDI", "Nod", "ThirdSide", "Civilian", "Mutant"] {
        let Some(list) = sides.get(side) else {
            continue;
        };
        for id in parse_list(list) {
            let sec = rules.section(&id);
            let color = reference(sec, "Color");
            let color_hsv = color.as_ref().and_then(|name| {
                let v = colors?.get(name)?;
                let mut it = v
                    .split(';')
                    .next()?
                    .split(',')
                    .map(|p| p.trim().parse::<i64>().ok());
                Some((it.next()??, it.next()??, it.next()??))
            });
            let house = House {
                side: side.to_string(),
                color,
                color_hsv,
                suffix: reference(sec, "Suffix"),
                id: id.clone(),
            };
            out.insert(&id, house);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &str = "\
[InfantryTypes]
1=E1
[VehicleTypes]
1=HTNK
2=GHOST
1=MTNK
3=HTNK
[AircraftTypes]
[BuildingTypes]
[Warheads]
1=AP
[E1]
Strength=125
Armor=Flak
Primary=M60
[HTNK]
Name=Rhino Tank
Strength=400
Armor=Heavy
Primary=105mm
Cost=900
Owner=Russians,Confederation
[MTNK]
Strength=300
Armor=heavy
Primary=105mm
Image=GTNK
[M60]
Damage=15
ROF=20
Range=5.75
Projectile=Invisible
Warhead=SA
[105mm]
Damage=65
ROF=65
Range=5.75
Projectile=Cannon
Warhead=AP
[AP]
Verses=25%,25%,25%,90%,75%,75%,50%,25%,25%,100%,100%
CellSpread=.3
[SA]
Verses=100%,80%
[Cannon]
Image=120mm
AG=yes
";

    const ART: &str = "[HTNK]\nVoxel=yes\nCameo=HTNKICON\n[GTNK]\nVoxel=yes\n";

    fn load() -> Rules {
        Rules::load(&IniFile::parse(RULES), &IniFile::parse(ART))
    }

    #[test]
    fn type_lists_follow_engine_order() {
        let r = load();
        let ids: Vec<&str> = r.vehicles.iter().map(|t| t.id.as_str()).collect();
        // index 1 reused -> MTNK takes HTNK's slot; HTNK again at 3 is new.
        assert_eq!(ids, ["MTNK", "GHOST", "HTNK"]);
        assert_eq!(r.vehicles.get("htnk").unwrap().index, 2);
        assert!(!r.vehicles.get("GHOST").unwrap().defined);
    }

    #[test]
    fn techno_fields_and_art() {
        let r = load();
        let h = r.techno("HTNK").unwrap();
        assert_eq!(
            (h.strength, h.armor, h.cost),
            (Some(400), Some(Armor::Heavy), Some(900))
        );
        assert_eq!(h.owner, ["Russians", "Confederation"]);
        let art = h.art.as_ref().unwrap();
        assert!(art.voxel);
        assert_eq!(art.cameo.as_deref(), Some("HTNKICON"));
        assert_eq!(
            r.techno("MTNK").unwrap().art.as_ref().unwrap().section,
            "GTNK"
        );
        assert_eq!(r.techno("E1").unwrap().armor, Some(Armor::Flak));
    }

    #[test]
    fn weapon_chain_and_damage() {
        let r = load();
        let w = r.weapons.get("105mm").unwrap();
        assert_eq!(w.range.unwrap().to_leptons(), 1472);
        let wh = r.warheads.get(w.warhead.as_deref().unwrap()).unwrap();
        assert_eq!(wh.verse(Armor::Light).mul_int(w.damage.unwrap()), 58); // 65 * 90%
        assert_eq!(wh.cell_spread.unwrap().units(), 300_000);
        assert!(r.projectiles.get("Cannon").unwrap().anti_ground.unwrap());
    }

    #[test]
    fn issues_report_data_problems() {
        let r = load();
        let has = |sev, sec: &str, needle: &str| {
            r.issues
                .iter()
                .any(|i| i.severity == sev && i.section == sec && i.message.contains(needle))
        };
        assert!(has(Severity::Note, "GHOST", "no section"));
        assert!(has(Severity::Note, "VehicleTypes", "index 1 reused"));
        assert!(has(Severity::Warning, "SA", "Verses needs 11"));
        assert!(has(
            Severity::Warning,
            "Invisible",
            "projectile has no section"
        ));
        // Unspecified verses fall back to 100%.
        assert_eq!(
            r.warheads.get("SA").unwrap().verse(Armor::Flak),
            Decimal::ONE
        );
    }
}
