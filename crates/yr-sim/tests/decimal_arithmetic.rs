use yr_rules::Decimal;
use yr_sim::{combat::WeaponSpec, terrain::Loco, Command, Fix, Sim};

fn weapon(verse: &str, damage: i64) -> WeaponSpec {
    WeaponSpec {
        source: None,
        damage,
        rof_ticks: 2,
        range: Fix::from_int(6),
        projectile_speed: Fix::from_raw(160),
        verses: [Decimal::parse(verse).unwrap(); 11],
    }
}

#[test]
fn damage_caller_retains_large_representable_products_and_saturates_overflow() {
    for (verse, damage, expected) in [
        ("100%", i64::MAX, i64::MAX),
        ("100%", i64::MIN, i64::MIN),
        ("-100%", i64::MIN, i64::MAX),
        ("200%", i64::MAX, i64::MAX),
        ("200%", i64::MIN, i64::MIN),
        ("90%", 65, 58),
    ] {
        let spec = weapon(verse, damage);
        for armor in 0..11 {
            assert_eq!(spec.damage_against(armor), expected);
        }
        assert_eq!(spec.damage_against(255), 0);
    }
}

#[test]
fn ordinary_decimal_combat_saves_and_continuations_remain_identical() {
    let mut proof = Vec::new();
    for verse in ["90%", "125%", "33.333333%", "0%"] {
        let mut sim = Sim::new(42);
        let spec = weapon(verse, 65);
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
        sim.step(&[
            spawn(0, Some(spec)),
            spawn(4, None),
            Command::Attack { id: 1, target: 2 },
        ]);
        sim.drain_events();
        let mut restored = Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap();
        for _ in 0..16 {
            sim.step(&[]);
            restored.step(&[]);
            assert_eq!(sim.drain_events(), restored.drain_events());
            assert_eq!(sim.state_hash(), restored.state_hash());
            let bytes = sim.save_bytes().unwrap();
            assert_eq!(bytes, restored.save_bytes().unwrap());
            proof.extend((bytes.len() as u64).to_le_bytes());
            proof.extend(bytes);
        }
        println!(
            "ordinary {verse}: tick {} hash {:016x}",
            sim.tick(),
            sim.state_hash()
        );
    }
    if let Some(path) = std::env::var_os("OPENYR_DECIMAL_SAVE_PROOF") {
        std::fs::write(path, proof).unwrap();
    }
}
