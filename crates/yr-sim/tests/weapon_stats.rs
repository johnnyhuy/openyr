use yr_rules::Rules;
use yr_sim::WeaponSpec;

fn rules(rof: Option<&str>, damage: &str, verse: &str) -> Rules {
    let verses = [verse; 11].join(",");
    rules_with_verses(rof, damage, &verses)
}

fn rules_with_verses(rof: Option<&str>, damage: &str, verses: &str) -> Rules {
    let reload = rof.map(|v| format!("ROF={v}\n")).unwrap_or_default();
    Rules::load(
        &yr_formats::IniFile::parse(&format!(
            "[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage={damage}\n{reload}Range=5.75\nWarhead=WH\n[WH]\nVerses={verses}\n"
        )),
        &yr_formats::IniFile::default(),
    )
}

#[test]
fn rules_reload_rejects_overflow_instead_of_wrapping_to_instant_fire() {
    for rof in ["4294967296", "4294967297", "9223372036854775807"] {
        let weapon = WeaponSpec::from_rules(&rules(Some(rof), "65", "100%"), "Gun");
        println!(
            "ROF={rof}: resolved reload {:?}",
            weapon.map(|w| w.rof_ticks)
        );
        assert!(weapon.is_none(), "unsupported reload became {weapon:?}");
    }
}

#[test]
fn rules_damage_and_verses_reject_values_outside_weapon_snapshot_domain() {
    for (damage, verse) in [
        ("2147483648", "100%"),
        ("-2147483648", "100%"),
        ("65", "100000.0001%"),
        ("65", "-100000.0001%"),
    ] {
        let weapon = WeaponSpec::from_rules(&rules(Some("2"), damage, verse), "Gun");
        println!("Damage={damage}, verse={verse}: resolved {weapon:?}");
        assert!(weapon.is_none());
    }
}

#[test]
fn every_armor_entry_uses_the_existing_snapshot_multiplier_bound() {
    for index in 0..11 {
        for verse in ["100000.0001%", "-100000.0001%"] {
            let mut entries = ["100%"; 11];
            entries[index] = verse;
            let rules = rules_with_verses(Some("2"), "65", &entries.join(","));
            assert!(WeaponSpec::from_rules(&rules, "Gun").is_none());
        }
    }
}

fn save_weapon(weapon: WeaponSpec) -> Vec<u8> {
    use yr_sim::{terrain::Loco, Command, Fix, Sim};
    let mut sim = Sim::new(42);
    sim.step(&[Command::Spawn {
        kind: 0,
        x: Fix::ZERO,
        y: Fix::ZERO,
        loco: Loco::Track,
        speed: Fix::ZERO,
        hp: 1000,
        armor: 5,
        weapon: Some(weapon),
    }]);
    let bytes = sim.save_bytes().unwrap();
    let restored = Sim::from_save_bytes(&bytes).unwrap();
    assert_eq!(restored.state_hash(), sim.state_hash());
    assert_eq!(restored.save_bytes().unwrap(), bytes);
    bytes
}

#[test]
fn supported_reload_defaults_and_stat_boundaries_remain_saveable() {
    for (rof, expected) in [
        (None, 0),
        (Some("-9223372036854775808"), 0),
        (Some("-1"), 0),
        (Some("0"), 0),
        (Some("2"), 2),
        (Some("4294967295"), u32::MAX),
    ] {
        let weapon = WeaponSpec::from_rules(&rules(rof, "65", "100%"), "Gun").unwrap();
        assert_eq!(weapon.rof_ticks, expected);
        save_weapon(weapon);
    }
    for damage in ["-2147483647", "0", "2147483647"] {
        for verse in ["-100000%", "0%", "100000%"] {
            let weapon = WeaponSpec::from_rules(&rules(Some("2"), damage, verse), "Gun").unwrap();
            let bytes = save_weapon(weapon);
            for value in [-2147483648i64, 2147483648] {
                let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
                json["units"][0]["weapon"]["damage"] = value.into();
                let mut bad = bytes[..12].to_vec();
                bad.extend(serde_json::to_vec(&json).unwrap());
                assert!(yr_sim::Sim::from_save_bytes(&bad).is_err());
            }
        }
    }
}
