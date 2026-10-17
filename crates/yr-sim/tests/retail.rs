//! Retail duel: two Rhino tanks (HTNK) fight with rules-derived data;
//! the kill must take exactly the rules-predicted number of shots.
//! Skips when the retail install is absent.
use std::path::PathBuf;
use yr_rules::Rules;
use yr_sim::combat::WeaponSpec;
use yr_sim::{Command, Event, Fix, Sim};

#[test]
fn rhino_duel_matches_rules_prediction() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).expect("install opens");
    let ini = |n: &str| yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read(n).expect(n)));
    let rules = Rules::load(&ini("RULESMD.INI"), &ini("ARTMD.INI"));
    let htnk = rules.techno("HTNK").expect("HTNK exists");
    let strength = htnk.strength.expect("strength");
    let armor_idx = yr_rules::Armor::ALL
        .iter()
        .position(|a| *a == htnk.armor.unwrap())
        .unwrap() as u8;
    let spec = WeaponSpec::from_rules(&rules, "105mm").expect("105mm resolves");
    let per_shot = spec.damage_against(armor_idx);
    assert!(per_shot > 0, "105mm hurts plate");
    let expected_shots = (strength + per_shot - 1) / per_shot;

    let spawn = |x: i32, weapon| Command::Spawn {
        x: Fix::from_int(x),
        y: Fix::ZERO,
        loco: yr_sim::terrain::Loco::Track,
        speed: Fix::from_int(4),
        hp: strength,
        armor: armor_idx,
        weapon,
    };
    let mut sim = Sim::new(9);
    sim.step(&[
        spawn(0, Some(spec)),
        spawn(512, None),
        Command::Attack { id: 1, target: 2 },
    ]);
    let mut died = false;
    for _ in 0..20_000 {
        sim.step(&[]);
        if sim
            .drain_events()
            .iter()
            .any(|e| matches!(e, Event::Died { id: 2 }))
        {
            died = true;
            break;
        }
    }
    assert!(died, "rhino eventually dies");
    assert_eq!(sim.units().len(), 1);
    assert_eq!(
        i64::from(sim.units()[0].shots),
        expected_shots,
        "kill took the predicted shots"
    );
    println!("HTNK duel: {per_shot} dmg/shot, {expected_shots} shots as predicted");
}
