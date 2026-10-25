//! Load the retail rules through the VFS and check known answers. Skips when
//! `~/Games/RA2YR/prefix/drive_c/Westwood/RA2` is absent.

use std::path::PathBuf;
use yr_assets::AssetFs;
use yr_formats::IniFile;
use yr_rules::{Armor, Decimal, Rules, Severity, TechnoKind};

fn retail() -> Option<Rules> {
    let dir =
        PathBuf::from(std::env::var("HOME").ok()?).join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return None;
    }
    let fs = AssetFs::open(&dir).expect("install opens");
    let ini = |n: &str| IniFile::parse(&String::from_utf8_lossy(fs.read(n).expect(n)));
    Some(Rules::load(&ini("RULESMD.INI"), &ini("ARTMD.INI")))
}

#[test]
fn registries_match_type_lists() {
    let Some(r) = retail() else {
        eprintln!("skip: retail install not present");
        return;
    };
    assert_eq!(r.infantry.len(), 65);
    assert_eq!(r.vehicles.len(), 80);
    assert_eq!(r.aircraft.len(), 12);
    // 403 list entries; NAPSYA is listed twice.
    assert_eq!(r.buildings.len(), 402);
    assert!(r
        .issues
        .iter()
        .any(|i| i.message.contains("NAPSYA listed twice")));
    // Every referenced weapon, warhead and projectile has a section, and
    // every armor value parses (retail mixes `plate` and `Plate`).
    let warnings: Vec<_> = r
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .collect();
    assert!(warnings.is_empty(), "{warnings:#?}");
}

#[test]
fn rhino_tank_known_answers() {
    let Some(r) = retail() else {
        eprintln!("skip: retail install not present");
        return;
    };
    let t = r.techno("HTNK").unwrap();
    assert_eq!(t.kind, TechnoKind::Vehicle);
    assert_eq!(
        (t.strength, t.armor, t.cost),
        (Some(400), Some(Armor::Heavy), Some(900))
    );
    assert_eq!(t.primary.as_deref(), Some("120mm"));
    assert!(t.art.as_ref().unwrap().voxel);
    let w = r.weapons.get("120mm").unwrap();
    assert_eq!((w.damage, w.rof), (Some(90), Some(65)));
    assert_eq!(w.range, Decimal::parse("5.75"));
    let ap = r.warheads.get("AP").unwrap();
    assert_eq!(ap.verse(Armor::Heavy).mul_int(90), 90);
}

#[test]
fn mirage_header_comment_does_not_corrupt_lunarwh() {
    let Some(r) = retail() else {
        eprintln!("skip: retail install not present");
        return;
    };
    // `[MirageWH]\t// Supposed to be a heat ray.` must open its own section.
    let mirage = r.warheads.get("MirageWH").unwrap();
    assert!(mirage.defined);
    assert_eq!(mirage.verse(Armor::Wood), Decimal::parse("30%").unwrap());
    let lunar = r.warheads.get("LUNARWH").unwrap();
    assert_eq!(lunar.verse(Armor::Plate), Decimal::ONE);
    assert_eq!(lunar.verse(Armor::Wood), Decimal::ONE);
}

#[test]
fn globals_houses_and_skirmish_defaults() {
    let Some(rules) = retail() else {
        eprintln!("skip: retail corpus not present");
        return;
    };
    // [General] BuildSpeed=.7 (minutes per 1000 credits), exact decimal.
    assert_eq!(rules.globals.build_speed.map(|d| d.units()), Some(700_000));
    assert_eq!(
        rules.globals.refund_percent.map(|d| d.units()),
        Some(500_000),
        "RefundPercent=50%"
    );
    assert_eq!(rules.globals.atom_damage, Some(1000));
    assert_eq!(rules.globals.bridge_strength, Some(1500));
    assert_eq!(rules.globals.tiberium_grows, Some(true));
    assert_eq!(rules.skirmish.money, Some(10_000));
    assert_eq!(rules.skirmish.unit_count, Some(10));
    // [Sides]: 5 Allied + 4 Soviet + Yuri + Neutral + Special.
    assert_eq!(rules.houses.len(), 12);
    let usa = rules.houses.get("Americans").expect("Americans house");
    assert_eq!(usa.side, "GDI");
    assert_eq!(usa.color.as_deref(), Some("Gold"));
    assert_eq!(usa.color_hsv, Some((43, 239, 255)));
    assert_eq!(
        rules.houses.get("YuriCountry").map(|h| h.side.as_str()),
        Some("ThirdSide")
    );
    assert_eq!(
        rules.houses.get("Neutral").map(|h| h.side.as_str()),
        Some("Civilian")
    );
}

#[test]
fn building_placement_keys_and_prerequisite_groups() {
    let Some(rules) = retail() else {
        eprintln!("skip: retail corpus not present");
        return;
    };
    let refinery = rules.techno("GAREFN").unwrap();
    assert_eq!(refinery.adjacent, Some(2));
    assert_eq!(refinery.free_unit.as_deref(), Some("CMIN"));
    assert_eq!(refinery.refinery, Some(true));
    assert_eq!(refinery.prerequisite, ["POWER", "GACNST"]);
    let yard = rules.techno("GACNST").unwrap();
    assert_eq!(
        (
            yard.construction_yard,
            yard.factory.as_deref(),
            yard.tech_level
        ),
        (Some(true), Some("BuildingType"), Some(-1))
    );
    // Defences and walls do not extend the base.
    assert_eq!(rules.techno("GAPILL").unwrap().base_normal, Some(false));
    assert_eq!(rules.techno("GAWALL").unwrap().wall, Some(true));
    assert_eq!(rules.techno("GAYARD").unwrap().water_bound, Some(true));
    assert_eq!(
        rules.techno("AMRADR").unwrap().required_houses,
        ["Americans"]
    );
    assert_eq!(
        rules.techno("GAAIRC").unwrap().forbidden_houses,
        ["Americans"]
    );
    let group = |t: &str| rules.prerequisite_group(t).map(<[String]>::to_vec);
    assert_eq!(
        group("POWER").unwrap(),
        ["GAPOWR", "NAPOWR", "NANRCT", "YAPOWR"]
    );
    assert_eq!(
        group("proc").unwrap(),
        ["GAREFN", "NAREFN", "YAREFN", "SMIN"]
    );
    assert!(group("RADAR").unwrap().iter().any(|i| i == "AMRADR"));
    assert_eq!(group("GACNST"), None);
}
