use yr_rules::Rules;
use yr_sim::WeaponSpec;

fn rules(range: &str) -> Rules {
    Rules::load(
        &yr_formats::IniFile::parse(&format!(
            "[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage=65\nROF=60\nRange={range}\nWarhead=WH\n[WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n"
        )),
        &yr_formats::IniFile::default(),
    )
}

#[test]
fn rules_weapon_range_rejects_wraparound_and_snapshot_domain_overflow() {
    for text in [
        "16777216",
        "-16777216",
        "8388608",
        "65536.003907",
        "-0.003907",
        "-0.000001",
        "9223372036854.775807",
        "-9223372036854.775808",
    ] {
        let weapon = WeaponSpec::from_rules(&rules(text), "Gun");
        println!(
            "Range={text}: resolved range {:?}",
            weapon.map(|w| w.range.raw())
        );
        assert!(
            weapon.is_none(),
            "unsupported Range={text} became {weapon:?}"
        );
    }
}

#[test]
fn valid_range_quantization_and_saved_weapon_domain_are_preserved() {
    use yr_sim::{terrain::Loco, Command, Fix, Sim};
    for (text, expected) in [
        ("0", 0),
        ("0.003906", 0),
        ("0.003907", 1),
        ("1.4", 358),
        ("5.75", 1472),
        ("65536", 1 << 24),
        ("65536.003906", 1 << 24),
    ] {
        let weapon = WeaponSpec::from_rules(&rules(text), "Gun").unwrap();
        assert_eq!(weapon.range.raw(), expected);
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
        for invalid in [-1, (1 << 24) + 1] {
            let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
            json["units"][0]["weapon"]["range"] = invalid.into();
            let mut bad = bytes[..12].to_vec();
            bad.extend(serde_json::to_vec(&json).unwrap());
            assert!(Sim::from_save_bytes(&bad).is_err());
        }
    }
}

#[test]
fn public_signed_cell_conversion_saturates_instead_of_wrapping() {
    use yr_rules::Decimal;
    use yr_sim::combat::cells_to_leptons;
    for (text, expected) in [
        ("16777216", i32::MAX),
        ("-16777216", i32::MIN),
        ("9223372036854.775807", i32::MAX),
        ("-9223372036854.775808", i32::MIN),
        ("-1.4", -358),
        ("1.4", 358),
        ("-0.003906", 0),
        ("8388607.996093", i32::MAX - 1),
        ("8388607.996094", i32::MAX),
        ("-8388608", i32::MIN),
    ] {
        assert_eq!(
            cells_to_leptons(Decimal::parse(text).unwrap()).raw(),
            expected,
            "{text}"
        );
    }
}
