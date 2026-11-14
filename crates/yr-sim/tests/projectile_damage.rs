use yr_rules::Rules;
use yr_sim::{terrain::Loco, Command, Fix, Sim, WeaponSpec};

fn flight(damage: &str, verse: &str) -> Sim {
    let verses = [verse; 11].join(",");
    let rules = Rules::load(
        &yr_formats::IniFile::parse(&format!(
            "[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage={damage}\nROF=60\nRange=5.75\nWarhead=WH\nProjectile=Bullet\nSpeed=40\n[Bullet]\nInviso=no\n[WH]\nVerses={verses}\n"
        )),
        &yr_formats::IniFile::default(),
    );
    let weapon = WeaponSpec::from_rules(&rules, "Gun").unwrap();
    let spawn = |x, weapon| Command::Spawn {
        kind: 0,
        x: Fix::from_int(x),
        y: Fix::ZERO,
        loco: Loco::Track,
        speed: Fix::ZERO,
        hp: 1000,
        armor: 5,
        weapon,
    };
    let mut sim = Sim::new(42);
    sim.step(&[
        spawn(0, Some(weapon)),
        spawn(4, None),
        Command::Attack { id: 1, target: 2 },
    ]);
    sim
}

#[test]
fn native_scaled_projectile_remains_saveable_after_launch() {
    let sim = flight("2147483647", "200%");
    assert_eq!(sim.projectiles().len(), 1);
    let projectile = serde_json::to_value(sim.projectiles()[0]).unwrap();
    let damage = projectile["damage"].as_i64().unwrap();
    assert_eq!(damage, 4_294_967_294);
    let result = sim.save_bytes();
    println!(
        "native applied damage {damage}: save result {:?}",
        result.as_ref().map(Vec::len)
    );
    assert!(
        result.is_ok(),
        "native-generated flight is not saveable: {result:?}"
    );
}

#[test]
fn signed_extreme_flights_restore_without_clipping_or_changing_impact() {
    use yr_sim::Event;
    const MAX: i64 = 2_147_483_647_000;
    for (damage, verse, expected) in [
        ("2147483647", "100000%", MAX),
        ("2147483647", "-100000%", -MAX),
        ("-2147483647", "100000%", -MAX),
        ("-2147483647", "-100000%", MAX),
    ] {
        let mut sim = flight(damage, verse);
        let p = serde_json::to_value(sim.projectiles()[0]).unwrap();
        assert_eq!(p["damage"].as_i64(), Some(expected));
        let mut restored = Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap();
        assert_eq!(sim.drain_events(), restored.drain_events());
        let mut hits = Vec::new();
        for _ in 0..8 {
            sim.step(&[]);
            restored.step(&[]);
            let events = sim.drain_events();
            assert_eq!(events, restored.drain_events());
            hits.extend(events.iter().filter_map(|e| match e {
                Event::Hit { target: 2, damage } => Some(*damage),
                _ => None,
            }));
            assert_eq!(sim.state_hash(), restored.state_hash());
            assert_eq!(sim.save_bytes().unwrap(), restored.save_bytes().unwrap());
        }
        assert!(sim.projectiles().is_empty());
        assert_eq!(hits, if expected > 0 { vec![expected] } else { vec![] });
        assert_eq!(
            sim.units().iter().find(|u| u.id == 2).map_or(0, |u| u.hp),
            if expected > 0 { 0 } else { 1000 }
        );
    }
}

#[test]
fn snapshot_damage_bounds_keep_applied_and_raw_fields_distinct() {
    let bytes = flight("65", "90%").save_bytes().unwrap();
    let original: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
    let encode = |json: &serde_json::Value| {
        let mut out = bytes[..12].to_vec();
        out.extend(serde_json::to_vec(json).unwrap());
        out
    };
    for damage in [-2_147_483_647_000i64, 2_147_483_647_000] {
        let mut json = original.clone();
        json["projectiles"][0]["damage"] = damage.into();
        assert!(Sim::from_save_bytes(&encode(&json)).is_ok());
    }
    for damage in [-2_147_483_647_001i64, 2_147_483_647_001, i64::MIN, i64::MAX] {
        let mut json = original.clone();
        json["projectiles"][0]["damage"] = damage.into();
        assert!(Sim::from_save_bytes(&encode(&json))
            .unwrap_err()
            .to_string()
            .contains("projectile damage out of range"));
    }
    for damage in [-2_147_483_648i64, 2_147_483_648] {
        let mut json = original.clone();
        json["projectiles"][0]["origin"]["damage"] = damage.into();
        assert!(Sim::from_save_bytes(&encode(&json))
            .unwrap_err()
            .to_string()
            .contains("shot source damage out of range"));
        let mut json = original.clone();
        json["units"][0]["weapon"]["damage"] = damage.into();
        assert!(Sim::from_save_bytes(&encode(&json))
            .unwrap_err()
            .to_string()
            .contains("weapon damage out of range"));
    }
    for version in 1u32..=4 {
        let mut legacy = original.clone();
        legacy["projectiles"][0]
            .as_object_mut()
            .unwrap()
            .remove("origin");
        legacy["units"][0]["weapon"]
            .as_object_mut()
            .unwrap()
            .remove("source");
        let mut legacy = encode(&legacy);
        legacy[8..12].copy_from_slice(&version.to_le_bytes());
        assert!(Sim::from_save_bytes(&legacy).is_ok());
    }
}
