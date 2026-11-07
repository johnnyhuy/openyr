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
    /// Building sidebar group, e.g. `BuildCat=Combat` for defences.
    pub build_category: Option<String>,
    pub strength: Option<i64>,
    /// Building `Power=` (positive produces, negative drains).
    pub power: Option<i64>,
    pub armor: Option<Armor>,
    pub cost: Option<i64>,
    pub repairable: Option<bool>,
    pub click_repairable: Option<bool>,
    pub unsellable: Option<bool>,
    /// `Factory=`: what a building produces (`UnitType`, `InfantryType`,
    /// `BuildingType`, `AircraftType`).
    pub factory: Option<String>,
    /// `Harvester=`: gathers ore/gems for a refinery.
    pub harvester: Option<bool>,
    pub points: Option<i64>,
    pub tech_level: Option<i64>,
    pub sight: Option<i64>,
    pub speed: Option<i64>,
    pub owner: Vec<String>,
    pub prerequisite: Vec<String>,
    /// Countries that alone may build it (`RequiredHouses=`, e.g. the
    /// British sniper) and countries that may not (`ForbiddenHouses=`).
    pub required_houses: Vec<String>,
    pub forbidden_houses: Vec<String>,
    /// Building `Adjacent=`: how many cells from an existing base
    /// building the footprint may be placed.
    pub adjacent: Option<i64>,
    /// Building `BaseNormal=`: whether it extends the buildable area
    /// (defences and walls say `no`).
    pub base_normal: Option<bool>,
    /// Building `FreeUnit=`: spawned beside it once placed (refinery
    /// harvesters).
    pub free_unit: Option<String>,
    pub refinery: Option<bool>,
    pub construction_yard: Option<bool>,
    /// Building `Radar=`: provides a radar display while operational.
    pub radar: Option<bool>,
    /// `RadarInvisible=`: explicit exclusion from the radar display.
    pub radar_invisible: Option<bool>,
    pub wall: Option<bool>,
    /// Building `WaterBound=`: must stand on water (shipyards).
    pub water_bound: Option<bool>,
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub elite_primary: Option<String>,
    pub elite_secondary: Option<String>,
    /// `Weapon1`..`WeaponN` (IFV-style and multi-weapon units).
    pub weapons: Vec<String>,
    /// Building `SuperWeapon=`: the superweapon this structure grants.
    pub super_weapon: Option<String>,
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

impl WeaponType {
    /// Case-insensitive FNV-1a identity for presentation provenance. Consumers
    /// reject collisions rather than assigning art from an ambiguous source.
    pub fn source_id(&self) -> u64 {
        self.id.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte.to_ascii_uppercase())).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
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

/// A superweapon from `[SuperWeaponTypes]` (plus any granted by a building
/// through `SuperWeapon=`). Behaviour fields stay verbatim where the engine
/// meaning is unverified; nothing here fires a superweapon yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperWeaponType {
    pub id: String,
    pub defined: bool,
    /// CSF label for the display name, e.g. `Name:Nuke`.
    pub ui_name: Option<String>,
    pub name: Option<String>,
    /// `Type=` engine behaviour key, verbatim (e.g. `MultiMissile`).
    pub kind: Option<String>,
    /// `Action=` cursor/action key, verbatim.
    pub action: Option<String>,
    pub is_powered: Option<bool>,
    /// `RechargeTime=` in minutes, exact decimal.
    pub recharge_time: Option<Decimal>,
    pub show_timer: Option<bool>,
    pub disableable_from_shell: Option<bool>,
    pub sidebar_image: Option<String>,
    /// `WeaponType=`: weapon delivered by missile-style superweapons.
    pub weapon_type: Option<String>,
    /// `Range=` in cells.
    pub range: Option<Decimal>,
    pub line_multiplier: Option<i64>,
    pub pre_click: Option<bool>,
    pub post_click: Option<bool>,
    /// `PreDependent=`: superweapon that must be charged first.
    pub pre_dependent: Option<String>,
    pub recharge_voice: Option<String>,
    pub charging_voice: Option<String>,
    pub impatient_voice: Option<String>,
    pub suspend_voice: Option<String>,
    /// `SpecialSound=` / `StartSound=` sound-event names, verbatim.
    pub special_sound: Option<String>,
    pub start_sound: Option<String>,
    pub flash_sidebar_tab_frames: Option<i64>,
    pub ai_defend_against: Option<bool>,
}

/// Fallback values from `soundmd.ini` `[Defaults]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoundDefaults {
    pub min_volume: Option<i64>,
    /// `Range=` in cells.
    pub range: Option<Decimal>,
    pub volume: Option<i64>,
    pub limit: Option<i64>,
    /// `Type=` delivery tokens, whitespace separated in the file.
    pub event_type: Vec<String>,
    pub priority: Option<String>,
}

/// One `soundmd.ini` sound event. `sounds` keeps the file stems verbatim,
/// including the leading `$` some entries carry; the playback meaning of
/// that marker is unverified. Numeric ranges keep the first and last value
/// written in the file (a single value reads as both bounds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundEvent {
    pub id: String,
    /// Position in registration order (`[SoundList]` first).
    pub index: usize,
    /// False when listed but without a section (engine defaults only).
    pub defined: bool,
    pub sounds: Vec<String>,
    /// `Control=` tokens, e.g. `random interrupt`.
    pub control: Vec<String>,
    pub volume: Option<i64>,
    pub priority: Option<String>,
    /// `Type=` delivery tokens, whitespace separated in the file.
    pub event_type: Vec<String>,
    /// `Range=` in cells.
    pub range: Option<Decimal>,
    pub limit: Option<i64>,
    /// `Delay=` bounds in milliseconds.
    pub delay_ms: Option<(i64, i64)>,
    /// `FShift=` percent bounds.
    pub fshift: Option<(i64, i64)>,
    pub vshift: Option<i64>,
    pub min_volume: Option<i64>,
    pub loop_count: Option<i64>,
    pub attack: Option<i64>,
    pub decay: Option<i64>,
}

/// Typed `soundmd.ini`: the `[SoundList]` registry plus `[Defaults]`.
#[derive(Debug, Clone, Default)]
pub struct SoundData {
    pub events: Registry<SoundEvent>,
    pub defaults: SoundDefaults,
    pub issues: Vec<Issue>,
}

impl SoundData {
    pub fn load(sound: &IniFile) -> SoundData {
        let mut out = SoundData::default();
        let mut ids = type_list(sound, "SoundList", &mut out.issues);
        // Sections the list never names still resolve by ID, as with
        // weapons that exist when referenced.
        for sec in &sound.sections {
            if !sec.name.eq_ignore_ascii_case("SoundList")
                && !sec.name.eq_ignore_ascii_case("Defaults")
                && !ids.iter().any(|id| id.eq_ignore_ascii_case(&sec.name))
            {
                ids.push(sec.name.clone());
            }
        }
        for (index, id) in ids.into_iter().enumerate() {
            let event = load_sound_event(sound, &id, index, &mut out.issues);
            if !out.events.insert(&id, event) {
                out.issues.push(issue(
                    Severity::Note,
                    "SoundList",
                    format!("{id} listed twice; first kept"),
                ));
            }
        }
        out.defaults = load_sound_defaults(sound);
        out
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
    /// Generic `Prerequisite=` tokens and the buildings that satisfy them,
    /// from `PrerequisitePower=` etc. (`POWER`, `FACTORY`, `BARRACKS`,
    /// `RADAR`, `TECH`, `PROC`; `PROC` also takes `PrerequisiteProcAlternate`).
    pub prerequisite_groups: Vec<(String, Vec<String>)>,
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
    pub superweapons: Registry<SuperWeaponType>,
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

        // Superweapons: the [SuperWeaponTypes] list, then any granted by
        // buildings through `SuperWeapon=` (ChronoWarpSpecial only arrives
        // this way in retail).
        let mut super_ids = type_list(rules, "SuperWeaponTypes", &mut out.issues);
        super_ids.extend(
            TechnoKind::ALL
                .iter()
                .flat_map(|&k| out.registry(k).iter())
                .filter_map(|t| t.super_weapon.clone()),
        );
        for id in super_ids {
            if !out.superweapons.contains(&id) {
                let sw = load_superweapon(rules, &id, &mut out.issues);
                out.superweapons.insert(&id, sw);
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

    /// Buildings that satisfy a generic prerequisite token such as
    /// `POWER`; `None` when `token` names a concrete type instead.
    pub fn prerequisite_group(&self, token: &str) -> Option<&[String]> {
        self.globals
            .prerequisite_groups
            .iter()
            .find(|(t, _)| t.eq_ignore_ascii_case(token))
            .map(|(_, ids)| ids.as_slice())
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

fn list(sec: Option<&IniSection>, key: &str) -> Vec<String> {
    sec.and_then(|s| s.get(key))
        .map(parse_list)
        .unwrap_or_default()
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
        build_category: string(sec, "BuildCat"),
        strength: int(sec, "Strength"),
        power: int(sec, "Power"),
        armor: armor.flatten(),
        cost: int(sec, "Cost"),
        repairable: sec.and_then(|x| x.get_bool("Repairable")),
        click_repairable: sec.and_then(|x| x.get_bool("ClickRepairable")),
        unsellable: sec.and_then(|x| x.get_bool("Unsellable")),
        factory: string(sec, "Factory"),
        harvester: sec.and_then(|x| x.get_bool("Harvester")),
        points: int(sec, "Points"),
        tech_level: int(sec, "TechLevel"),
        sight: int(sec, "Sight"),
        speed: int(sec, "Speed"),
        owner: list(sec, "Owner"),
        prerequisite: list(sec, "Prerequisite"),
        required_houses: list(sec, "RequiredHouses"),
        forbidden_houses: list(sec, "ForbiddenHouses"),
        adjacent: int(sec, "Adjacent"),
        base_normal: sec.and_then(|x| x.get_bool("BaseNormal")),
        free_unit: reference(sec, "FreeUnit"),
        refinery: sec.and_then(|x| x.get_bool("Refinery")),
        construction_yard: sec.and_then(|x| x.get_bool("ConstructionYard")),
        radar: sec.and_then(|x| x.get_bool("Radar")),
        radar_invisible: sec.and_then(|x| x.get_bool("RadarInvisible")),
        wall: sec.and_then(|x| x.get_bool("Wall")),
        water_bound: sec.and_then(|x| x.get_bool("WaterBound")),
        primary: reference(sec, "Primary"),
        secondary: reference(sec, "Secondary"),
        elite_primary: reference(sec, "ElitePrimary"),
        elite_secondary: reference(sec, "EliteSecondary"),
        weapons,
        super_weapon: reference(sec, "SuperWeapon"),
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

fn load_superweapon(rules: &IniFile, id: &str, issues: &mut Vec<Issue>) -> SuperWeaponType {
    let sec = rules.section(id);
    if sec.is_none() {
        issues.push(issue(
            Severity::Warning,
            id,
            "superweapon has no section".into(),
        ));
    }
    let flag = |k: &str| sec.and_then(|s| s.get_bool(k));
    SuperWeaponType {
        id: id.to_string(),
        defined: sec.is_some(),
        ui_name: string(sec, "UIName"),
        name: string(sec, "Name"),
        kind: string(sec, "Type"),
        action: string(sec, "Action"),
        is_powered: flag("IsPowered"),
        recharge_time: dec(sec, "RechargeTime"),
        show_timer: flag("ShowTimer"),
        disableable_from_shell: flag("DisableableFromShell"),
        sidebar_image: reference(sec, "SidebarImage"),
        weapon_type: reference(sec, "WeaponType"),
        range: dec(sec, "Range"),
        line_multiplier: int(sec, "LineMultiplier"),
        pre_click: flag("PreClick"),
        post_click: flag("PostClick"),
        pre_dependent: reference(sec, "PreDependent"),
        recharge_voice: reference(sec, "RechargeVoice"),
        charging_voice: reference(sec, "ChargingVoice"),
        impatient_voice: reference(sec, "ImpatientVoice"),
        suspend_voice: reference(sec, "SuspendVoice"),
        special_sound: reference(sec, "SpecialSound"),
        start_sound: reference(sec, "StartSound"),
        flash_sidebar_tab_frames: int(sec, "FlashSidebarTabFrames"),
        ai_defend_against: flag("AIDefendAgainst"),
    }
}

/// Whitespace-separated tokens of `key` (sound `Control=`/`Type=`/`Sounds=`).
fn tokens(sec: Option<&IniSection>, key: &str) -> Vec<String> {
    sec.and_then(|s| s.get(key))
        .map(|v| v.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

/// First and last integer written for `key`; one value reads as both bounds.
fn int_bounds(sec: Option<&IniSection>, key: &str) -> Option<(i64, i64)> {
    let values: Vec<i64> = sec
        .and_then(|s| s.get(key))?
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    Some((*values.first()?, *values.last()?))
}

fn load_sound_event(
    sound: &IniFile,
    id: &str,
    index: usize,
    issues: &mut Vec<Issue>,
) -> SoundEvent {
    let sec = sound.section(id);
    if sec.is_none() {
        issues.push(issue(
            Severity::Note,
            id,
            "listed in [SoundList] but has no section".into(),
        ));
    }
    SoundEvent {
        id: id.to_string(),
        index,
        defined: sec.is_some(),
        sounds: tokens(sec, "Sounds"),
        control: tokens(sec, "Control"),
        volume: int(sec, "Volume"),
        priority: string(sec, "Priority"),
        event_type: tokens(sec, "Type"),
        range: dec(sec, "Range"),
        limit: int(sec, "Limit"),
        delay_ms: int_bounds(sec, "Delay"),
        fshift: int_bounds(sec, "FShift"),
        vshift: int(sec, "VShift").or_else(|| int(sec, "Vshift")),
        // One retail section spells the key `MinVol=`; accept both.
        min_volume: int(sec, "MinVolume").or_else(|| int(sec, "MinVol")),
        loop_count: int(sec, "Loop"),
        attack: int(sec, "Attack"),
        decay: int(sec, "Decay"),
    }
}

fn load_sound_defaults(sound: &IniFile) -> SoundDefaults {
    let sec = sound.section("Defaults");
    SoundDefaults {
        min_volume: int(sec, "MinVolume"),
        range: dec(sec, "Range"),
        volume: int(sec, "Volume"),
        limit: int(sec, "Limit"),
        event_type: tokens(sec, "Type"),
        priority: string(sec, "Priority"),
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
        prerequisite_groups: PREREQUISITE_GROUPS
            .iter()
            .map(|(token, keys)| {
                let ids = keys.iter().flat_map(|k| list(g, k)).collect();
                (token.to_string(), ids)
            })
            .collect(),
    }
}

/// `[General]` keys behind each generic prerequisite token.
const PREREQUISITE_GROUPS: [(&str, &[&str]); 6] = [
    ("POWER", &["PrerequisitePower"]),
    ("FACTORY", &["PrerequisiteFactory"]),
    ("BARRACKS", &["PrerequisiteBarracks"]),
    ("RADAR", &["PrerequisiteRadar"]),
    ("TECH", &["PrerequisiteTech"]),
    ("PROC", &["PrerequisiteProc", "PrerequisiteProcAlternate"]),
];

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
1=GACSPH
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
[SuperWeaponTypes]
1=NukeSpecial
2=GhostSpecial
[GACSPH]
SuperWeapon=ChronoWarpSpecial
[NukeSpecial]
UIName=Name:Nuke
Name=N U K E !!!
Type=MultiMissile
Action=Nuke
IsPowered=true
RechargeTime=10
ShowTimer=yes
DisableableFromShell=yes
SidebarImage=NukeIcon
WeaponType=NukeCarrier
Range=7
LineMultiplier=2
RechargeVoice=00-I154
[ChronoWarpSpecial]
Type=ChronoWarp
PostClick=yes
PreDependent=ChronoSphereSpecial
IsPowered=false
RechargeTime=1
";

    const SOUND: &str = "\
[Defaults]
MinVolume=50
Range=10
Volume=80
Limit=5
Type= NORMAL SCREEN UNSHROUD
Priority=NORMAL
[SoundList]
1=GISelect
2=GhostSound
3=GISelect
[GISelect]
Sounds= $igisea $igiseb
Control= random
Volume=85
Delay= 400
FShift= -5 5
VShift=10
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

    #[test]
    fn superweapons_from_list_and_building_grants() {
        let r = load();
        let ids: Vec<&str> = r.superweapons.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["NukeSpecial", "GhostSpecial", "ChronoWarpSpecial"]);
        let nuke = r.superweapons.get("nukespecial").unwrap();
        assert!(nuke.defined);
        assert_eq!(nuke.kind.as_deref(), Some("MultiMissile"));
        assert_eq!(nuke.weapon_type.as_deref(), Some("NukeCarrier"));
        assert_eq!(nuke.recharge_time, Decimal::parse("10"));
        assert_eq!(nuke.is_powered, Some(true));
        assert_eq!(nuke.range.map(|d| d.to_leptons()), Some(1792));
        assert!(!r.superweapons.get("GhostSpecial").unwrap().defined);
        assert!(r.issues.iter().any(|i| i.severity == Severity::Warning
            && i.section == "GhostSpecial"
            && i.message.contains("superweapon has no section")));
        // The building grant registers ChronoWarpSpecial and resolves back.
        assert_eq!(
            r.techno("GACSPH").unwrap().super_weapon.as_deref(),
            Some("ChronoWarpSpecial")
        );
        let warp = r.superweapons.get("ChronoWarpSpecial").unwrap();
        assert_eq!(warp.pre_dependent.as_deref(), Some("ChronoSphereSpecial"));
        assert_eq!(warp.post_click, Some(true));
    }

    #[test]
    fn sound_events_defaults_and_missing_sections() {
        let s = SoundData::load(&IniFile::parse(SOUND));
        assert_eq!(s.events.len(), 2); // GISelect listed twice, kept once.
        let gi = s.events.get("giselect").unwrap();
        assert!(gi.defined);
        assert_eq!(gi.sounds, ["$igisea", "$igiseb"]);
        assert_eq!(gi.control, ["random"]);
        assert_eq!(gi.volume, Some(85));
        assert_eq!(gi.delay_ms, Some((400, 400)));
        assert_eq!(gi.fshift, Some((-5, 5)));
        assert_eq!(gi.vshift, Some(10));
        assert!(!s.events.get("GhostSound").unwrap().defined);
        assert!(s.issues.iter().any(|i| i.section == "GhostSound"
            && i.message
                .contains("listed in [SoundList] but has no section")));
        assert_eq!(s.defaults.volume, Some(80));
        assert_eq!(s.defaults.limit, Some(5));
        assert_eq!(s.defaults.event_type, ["NORMAL", "SCREEN", "UNSHROUD"]);
        assert_eq!(s.defaults.priority.as_deref(), Some("NORMAL"));
    }
}
