use super::*;

fn duel(speed: Fix, multiplier: yr_rules::Decimal) -> Sim {
    let weapon = WeaponSpec {
        source: Some(7),
        damage: 65,
        rof_ticks: 1000,
        range: Fix::from_int(12),
        projectile_speed: speed,
        verses: [multiplier; combat::ARMOR_COUNT],
    };
    let spawn = |x, weapon| Command::Spawn {
        kind: 3,
        x: Fix::from_int(x),
        y: Fix::ONE,
        loco: terrain::Loco::Track,
        speed: Fix::ONE,
        hp: 200,
        armor: 0,
        weapon,
    };
    let mut sim = Sim::new(42);
    sim.step(&[
        spawn(1, Some(weapon)),
        spawn(8, None),
        Command::AssignOwner {
            id: 2,
            owner: OWNER_ENEMY,
        },
        Command::Attack { id: 1, target: 2 },
    ]);
    sim
}

#[test]
fn instant_impact_records_delivered_damage_even_when_armour_blocks_the_hit() {
    let mut sim = duel(Fix::ZERO, yr_rules::Decimal::ZERO);
    let events = sim.drain_events();
    assert!(events.contains(&Event::Impact {
        target: 2,
        x: Fix::from_int(8),
        y: Fix::ONE,
        origin: Some(ShotOrigin {
            shooter: 1,
            weapon: Some(7),
            damage: 65
        }),
    }));
    assert!(!events.iter().any(|e| matches!(e, Event::Hit { .. })));
    assert_eq!(sim.units()[1].hp, 200);
    assert!(sim.projectiles.is_empty());
}

#[test]
fn in_flight_provenance_survives_shooter_death_target_movement_and_save_load() {
    let mut sim = duel(Fix::from_raw(128), yr_rules::Decimal::parse("10%").unwrap());
    assert_eq!(sim.projectiles.len(), 1);
    assert!(!sim
        .drain_events()
        .iter()
        .any(|e| matches!(e, Event::Impact { .. })));
    sim.step(&[
        Command::Destroy { id: 1 },
        Command::Move {
            id: 2,
            x: Fix::from_int(9),
            y: Fix::from_int(2),
        },
    ]);
    sim.drain_events();
    assert!(!sim.units().iter().any(|u| u.id == 1));
    let bytes = sim.save_bytes().unwrap();
    let mut restored = Sim::from_save_bytes(&bytes).unwrap();
    let mut impacts = Vec::new();
    for _ in 0..30 {
        sim.step(&[]);
        restored.step(&[]);
        assert_eq!(sim.state_hash(), restored.state_hash());
        let events = sim.drain_events();
        assert_eq!(events, restored.drain_events());
        impacts.extend(
            events
                .into_iter()
                .filter(|e| matches!(e, Event::Impact { .. })),
        );
    }
    assert_eq!(
        impacts,
        [Event::Impact {
            target: 2,
            x: Fix::from_int(9),
            y: Fix::from_int(2),
            origin: Some(ShotOrigin {
                shooter: 1,
                weapon: Some(7),
                damage: 65
            }),
        }]
    );
    assert_eq!(
        sim.units()[0].hp,
        194,
        "art uses 65, health loses floor(65 * 10%)"
    );
}

#[test]
fn optional_provenance_preserves_old_save_hashes_and_unknown_flights_do_not_guess() {
    let mut sim = duel(Fix::from_raw(128), yr_rules::Decimal::ONE);
    sim.drain_events();
    let bytes = sim.save_bytes().unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
    json["units"][0]["weapon"]
        .as_object_mut()
        .unwrap()
        .remove("source");
    json["projectiles"][0]
        .as_object_mut()
        .unwrap()
        .remove("origin");
    let mut legacy = bytes[..12].to_vec();
    legacy[8..12].copy_from_slice(&1u32.to_le_bytes());
    legacy.extend(serde_json::to_vec(&json).unwrap());
    let mut restored = Sim::from_save_bytes(&legacy).unwrap();
    let projection = sim.setup_fingerprint_bytes().unwrap();
    assert_eq!(&projection[8..12], &1u32.to_le_bytes());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&projection[12..]).unwrap(),
        json
    );
    assert_eq!(sim.state_hash(), restored.state_hash());
    assert_eq!(restored.units[0].weapon.unwrap().source, None);
    assert_eq!(restored.projectiles[0].origin, None);
    let mut unknown = false;
    for _ in 0..20 {
        sim.step(&[]);
        restored.step(&[]);
        assert_eq!(sim.state_hash(), restored.state_hash());
        unknown |= restored
            .drain_events()
            .iter()
            .any(|e| matches!(e, Event::Impact { origin: None, .. }));
        sim.drain_events();
    }
    assert!(unknown);
}

#[test]
fn hostile_saved_provenance_is_rejected() {
    let sim = duel(Fix::from_raw(128), yr_rules::Decimal::ONE);
    for (field, value) in [("damage", i64::MAX), ("shooter", 99)] {
        let mut json = serde_json::to_value(&sim).unwrap();
        json["projectiles"][0]["origin"][field] = value.into();
        let mut bytes = sim.save_bytes().unwrap()[..12].to_vec();
        bytes.extend(serde_json::to_vec(&json).unwrap());
        assert!(Sim::from_save_bytes(&bytes).is_err(), "{field}");
    }
}

#[test]
fn ai_build_profiles_preserve_the_legacy_serialized_setup_shape() {
    let make = |source| {
        let unit = UnitSpec {
            loco: terrain::Loco::Track,
            speed: Fix::ONE,
            hp: 100,
            armor: 0,
            role: Role::Combat,
            weapon: Some(WeaponSpec {
                source,
                damage: 10,
                rof_ticks: 10,
                range: Fix::ONE,
                projectile_speed: Fix::ZERO,
                verses: [yr_rules::Decimal::ONE; combat::ARMOR_COUNT],
            }),
        };
        let building = BuildingSpec {
            w: 1,
            h: 1,
            hp: 100,
            armor: 0,
            power: 0,
            role: Role::Refinery,
            adjacent: 1,
            base_normal: true,
            free_unit: Some((3, unit)),
        };
        let mut sim = Sim::new(42);
        sim.set_ai(Some(ai::SkirmishAi::new(
            OWNER_ENEMY,
            OWNER_PLAYER,
            [Product::Unit(unit), Product::Building(building)]
                .into_iter()
                .map(|product| ai::AiBuild {
                    factory: match product {
                        Product::Unit(_) => Factory::Vehicle,
                        Product::Building(_) => Factory::Building,
                    },
                    kind: 3,
                    product,
                    cost: 100,
                    ticks: 10,
                    prerequisites: vec![],
                })
                .collect(),
        )));
        sim
    };
    let legacy = make(None);
    let current = make(Some(7));
    let mut legacy_bytes = legacy.save_bytes().unwrap();
    legacy_bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(current.setup_fingerprint_bytes().unwrap(), legacy_bytes);
    assert_eq!(current.state_hash(), legacy.state_hash());
    assert_ne!(
        current.save_bytes().unwrap(),
        legacy.save_bytes().unwrap(),
        "normal saves retain full AI provenance"
    );
}
