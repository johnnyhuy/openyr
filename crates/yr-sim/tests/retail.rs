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
    let primary = htnk.primary.as_deref().expect("Rhino primary");
    assert_eq!(primary, "120mm");
    let spec = WeaponSpec::from_rules(&rules, primary).expect("Rhino primary resolves");
    assert_eq!(spec.damage, 90);
    let per_shot = spec.damage_against(armor_idx);
    assert_eq!(per_shot, 90, "120mm hurts heavy armour");
    let expected_shots = (strength + per_shot - 1) / per_shot;

    let spawn = |x: i32, weapon| Command::Spawn {
        kind: 0,
        x: Fix::from_int(x),
        y: Fix::ZERO,
        loco: yr_sim::terrain::Loco::Track,
        speed: Fix::ZERO,
        hp: strength,
        armor: armor_idx,
        weapon,
    };
    let mut sim = Sim::new(9);
    sim.step(&[
        spawn(0, Some(spec)),
        spawn(4, None),
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
    println!("HTNK duel with {primary}: {per_shot} dmg/shot, {expected_shots} shots as predicted");
}

#[test]
fn retail_rules_distinguish_cannon_flight_from_invisible_infantry_hits() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let ini = |n| yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read(n).unwrap()));
    let rules = Rules::load(&ini("RULESMD.INI"), &ini("ARTMD.INI"));
    for (id, expected_primary, damage, arrival) in [
        ("MTNK", "105mm", 65, 7),
        ("HTNK", "120mm", 90, 7),
        ("E1", "M60", 15, 1),
    ] {
        let t = rules.techno(id).unwrap();
        let primary = t.primary.as_deref().unwrap();
        assert_eq!(primary, expected_primary);
        let weapon = WeaponSpec::from_rules(&rules, primary).unwrap();
        assert_eq!(weapon.damage, damage);
        assert_eq!(
            weapon.projectile_speed.raw(),
            if id == "E1" { 0 } else { 160 }
        );
        let spawn = |x, weapon| Command::Spawn {
            kind: 0,
            x: Fix::from_int(x),
            y: Fix::ONE,
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::ZERO,
            hp: 1000,
            armor: 0,
            weapon,
        };
        let mut sim = Sim::new(42);
        sim.step(&[
            spawn(0, Some(weapon)),
            spawn(4, None),
            Command::Attack { id: 1, target: 2 },
        ]);
        for tick in 1..=arrival {
            let events = sim.drain_events();
            assert_eq!(
                events.iter().any(|e| matches!(e, Event::Fired { .. })),
                tick == 1
            );
            assert_eq!(
                events.iter().any(|e| matches!(e, Event::Impact { .. })),
                tick == arrival,
                "{id} at tick {tick}"
            );
            if tick != arrival {
                assert_eq!(sim.units()[1].hp, 1000);
                assert_eq!(sim.projectiles().len(), 1);
                assert_eq!(sim.projectiles()[0].x.raw(), 160 * tick);
                let mut loaded = Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap();
                loaded.step(&[]);
                sim.step(&[]);
                assert_eq!(loaded.state_hash(), sim.state_hash());
                assert_eq!(loaded.save_bytes().unwrap(), sim.save_bytes().unwrap());
            }
        }
        assert_eq!(sim.units()[1].hp, 1000 - weapon.damage_against(0));
        println!(
            "Retail {id}/{primary}: launch 1, arrival {arrival}, raw speed {}, delivered damage {}",
            weapon.projectile_speed.raw(),
            weapon.damage_against(0)
        );
    }
}
