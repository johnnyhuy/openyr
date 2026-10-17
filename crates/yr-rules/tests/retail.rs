//! Load the retail rules through the VFS and check known answers. Skips when
//! `~/Games/RA2YR/prefix/drive_c/Westwood/RA2` is absent.

use std::path::PathBuf;
use yr_assets::AssetFs;
use yr_formats::IniFile;
use yr_rules::{Armor, Decimal, Rules, Severity, TechnoKind};

fn retail() -> Option<Rules> {
    let dir = PathBuf::from(std::env::var("HOME").ok()?)
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
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
    assert!(r.issues.iter().any(|i| i.message.contains("NAPSYA listed twice")));
    // Every referenced weapon, warhead and projectile has a section, and
    // every armor value parses (retail mixes `plate` and `Plate`).
    let warnings: Vec<_> = r.issues.iter().filter(|i| i.severity == Severity::Warning).collect();
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
    assert_eq!((t.strength, t.armor, t.cost), (Some(400), Some(Armor::Heavy), Some(900)));
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
