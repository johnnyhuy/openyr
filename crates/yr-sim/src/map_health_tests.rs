use super::*;

fn spec(hp: i64) -> BuildingSpec {
    BuildingSpec {
        w: 1,
        h: 1,
        hp,
        armor: 0,
        power: 100,
        role: Role::Combat,
        adjacent: 0,
        base_normal: true,
        free_unit: None,
    }
}

#[test]
fn map_health_preserves_maximum_and_bounds_fraction_arithmetic() {
    for (strength, fraction, current, maximum) in [
        (100, 256, 100, 100),
        (100, 128, 50, 100),
        (100, 1, 1, 100),
        (100, 0, 1, 100),
        (100, u16::MAX, 100, 100),
        (i64::MAX, u16::MAX, i64::from(i32::MAX), i64::from(i32::MAX)),
        (i64::MAX, 128, i64::from(i32::MAX) / 2, i64::from(i32::MAX)),
        (0, 256, 1, 1),
        (i64::MIN, 0, 1, 1),
    ] {
        let mut sim = Sim::new(42);
        sim.step(&[Command::PlaceMapBuilding {
            kind: 7,
            x: 2,
            y: 3,
            spec: spec(strength),
            health: fraction,
            owner: OWNER_PLAYER,
        }]);
        let b = &sim.buildings()[0];
        assert_eq!((b.hp, b.max_hp), (current, maximum));
        assert_eq!((b.kind, b.cell, b.owner), (7, (2, 3), OWNER_PLAYER));
        let saved = sim.save_bytes().unwrap();
        let restored = Sim::from_save_bytes(&saved).unwrap();
        assert_eq!(restored.save_bytes().unwrap(), saved);
        assert_eq!(restored.state_hash(), sim.state_hash());
    }
}

#[test]
fn map_health_full_placement_preserves_prior_snapshot_and_repair_uses_maximum() {
    let mut prior = Sim::new(42);
    prior.step(&[Command::PlaceBuilding {
        kind: 7,
        x: 2,
        y: 3,
        w: 1,
        h: 1,
        hp: 100,
        armor: 0,
        power: 100,
        owner: OWNER_PLAYER,
        role: Role::Combat,
    }]);
    let mut current = Sim::new(42);
    current.step(&[Command::PlaceMapBuilding {
        kind: 7,
        x: 2,
        y: 3,
        spec: spec(100),
        health: 256,
        owner: OWNER_PLAYER,
    }]);
    assert_eq!(current.save_bytes().unwrap(), prior.save_bytes().unwrap());
    assert_eq!(current.state_hash(), prior.state_hash());

    let mut damaged = Sim::new(42);
    damaged.set_credits(100);
    damaged.step(&[Command::PlaceMapBuilding {
        kind: 7,
        x: 2,
        y: 3,
        spec: spec(100),
        health: 128,
        owner: OWNER_PLAYER,
    }]);
    assert_eq!(damaged.power_of(OWNER_PLAYER).0, 50);
    damaged.step(&[Command::Repair {
        id: 1,
        spec: Some(RepairSpec {
            cost: 100,
            percent: yr_rules::Decimal::ONE,
            step: 50,
            interval: 1,
        }),
    }]);
    assert_eq!(
        (damaged.buildings()[0].hp, damaged.buildings()[0].max_hp),
        (100, 100)
    );
    assert_eq!(damaged.credits(), 50);
    assert_eq!(damaged.power_of(OWNER_PLAYER).0, 100);
    println!("Map-health bounds/repair: full placement retains prior bytes; half-health power is 50/100, repairs to 100/100 for 50 credits.");
}
