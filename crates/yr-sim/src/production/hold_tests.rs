use crate::*;

fn product() -> Product {
    Product::Unit(UnitSpec {
        loco: terrain::Loco::Track,
        speed: Fix::ONE,
        hp: 100,
        armor: 0,
        weapon: None,
        role: Role::Combat,
    })
}

fn factory(owner: u8, role: Role, x: u16) -> Command {
    Command::PlaceBuilding {
        kind: 0,
        x,
        y: 2,
        w: 2,
        h: 2,
        hp: 1000,
        armor: 0,
        power: 0,
        owner,
        role,
    }
}

fn hold(kind: u16, held: bool) -> Command {
    Command::HoldProduce {
        factory: Factory::Vehicle,
        kind,
        held,
    }
}

fn fixture() -> Sim {
    let mut sim = Sim::new(21);
    sim.set_terrain(TerrainGrid::new(0, 0, 20, 20));
    sim.set_credits(1000);
    sim.set_credits_of(OWNER_ENEMY, 1000);
    sim.step(&[
        factory(OWNER_PLAYER, Role::WarFactory, 2),
        factory(OWNER_PLAYER, Role::Barracks, 6),
        factory(OWNER_ENEMY, Role::WarFactory, 10),
    ]);
    for (owner, kind, factory) in [
        (OWNER_PLAYER, 1, Factory::Vehicle),
        (OWNER_PLAYER, 1, Factory::Vehicle),
        (OWNER_PLAYER, 2, Factory::Vehicle),
        (OWNER_PLAYER, 3, Factory::Infantry),
        (OWNER_ENEMY, 1, Factory::Vehicle),
    ] {
        sim.queue_produce(owner, factory, kind, product(), 100, 20);
    }
    sim
}

#[test]
fn hold_freezes_paid_head_blocks_followers_and_isolates_factories_and_owners() {
    let mut sim = fixture();
    sim.step(&[]);
    let head = sim.production()[0];
    assert_eq!((head.progress, head.paid), (256, 5));
    sim.step(&[hold(2, true)]);
    assert!(
        !sim.production()[0].held,
        "a waiting kind cannot suspend the head"
    );
    sim.step(&[hold(1, true), hold(1, true)]);
    let head = sim.production()[0];
    assert!(head.held);
    let credits = sim.credits();
    for _ in 0..10 {
        sim.step(&[]);
    }
    assert_eq!(sim.production()[0], head);
    assert_eq!(
        sim.production()[1].progress,
        0,
        "same-type successor cannot overtake"
    );
    assert_eq!(
        sim.production()[2].progress,
        0,
        "other waiting type cannot overtake"
    );
    assert!(sim.production()[3].progress > head.progress);
    assert!(sim.production_of(OWNER_ENEMY)[0].progress > head.progress);
    assert_eq!(
        credits - sim.credits(),
        50,
        "only the infantry queue spends credits"
    );
    sim.step(&[hold(1, false)]);
    assert_eq!(sim.production()[0].progress, head.progress + 256);
    assert_eq!(sim.production()[0].paid, head.paid + 5);
}

#[test]
fn held_head_cancels_with_exact_refund_and_successor_is_not_held() {
    let mut sim = fixture();
    sim.step(&[]);
    sim.step(&[hold(1, true)]);
    let paid = sim.production()[0].paid;
    // Remove unrelated spending so the refund can be measured exactly.
    sim.economies
        .get_mut(&OWNER_PLAYER)
        .unwrap()
        .production
        .retain(|o| o.factory == Factory::Vehicle);
    sim.step(&[Command::Destroy {
        id: sim.buildings()[0].id,
    }]);
    let credits = sim.credits();
    sim.step(&[Command::CancelProduce {
        factory: Factory::Vehicle,
        kind: 1,
    }]);
    assert_eq!(sim.credits(), credits + paid);
    assert_eq!(sim.production().len(), 2);
    assert_eq!(
        (sim.production()[0].paid, sim.production()[0].progress),
        (0, 0)
    );
    assert!(!sim.production()[0].held);
    sim.step(&[factory(OWNER_PLAYER, Role::WarFactory, 2)]);
    assert_eq!(sim.production()[0].progress, 256);
    sim.set_credits(0);
    sim.step(&[hold(1, true)]);
    sim.set_credits(1000);
    let head = sim.production()[0];
    sim.step(&[]);
    assert_eq!(
        sim.production()[0],
        head,
        "credit recovery cannot resume an explicit hold"
    );
    sim.step(&[hold(1, false)]);
    assert!(sim.production()[0].progress > head.progress);
}

#[test]
fn held_save_roundtrip_legacy_defaults_hash_and_hostile_flags() {
    let mut sim = fixture();
    let hash = sim.state_hash();
    sim.economies.get_mut(&OWNER_PLAYER).unwrap().production[0].held = true;
    assert_ne!(sim.state_hash(), hash, "holding changes future play");
    let bytes = sim.save_bytes().unwrap();
    let mut restored = Sim::from_save_bytes(&bytes).unwrap();
    assert_eq!(bytes, restored.save_bytes().unwrap());
    for t in 0..40 {
        let commands = if t == 8 { vec![hold(1, false)] } else { vec![] };
        sim.step(&commands);
        restored.step(&commands);
        assert_eq!(sim.state_hash(), restored.state_hash());
        assert_eq!(sim.drain_events(), restored.drain_events());
    }
    let plain = fixture();
    for version in 1u32..=3 {
        let mut old = plain.save_bytes().unwrap();
        assert!(!String::from_utf8_lossy(&old).contains("held"));
        old[8..12].copy_from_slice(&version.to_le_bytes());
        assert_eq!(Sim::from_save_bytes(&old).unwrap().state_hash(), hash);
    }
    let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
    for flag in [
        serde_json::json!(1),
        serde_json::json!("true"),
        serde_json::Value::Null,
    ] {
        json["economies"][OWNER_PLAYER.to_string()]["production"][0]["held"] = flag;
        let mut bad = bytes[..12].to_vec();
        bad.extend(serde_json::to_vec(&json).unwrap());
        assert!(Sim::from_save_bytes(&bad).is_err());
    }
    let mut done = fixture();
    let head = &mut done.economies.get_mut(&OWNER_PLAYER).unwrap().production[0];
    head.progress = head.total();
    done.step(&[hold(1, true)]);
    assert!(
        !done.production()[0].held,
        "completed orders cannot be suspended"
    );
}

#[test]
fn pending_queue_preview_matches_command_application_without_advancing_the_sim() {
    let mut sim = fixture();
    sim.step(&[]);
    let saved = sim.save_bytes().unwrap();
    let commands = [
        hold(1, true),
        Command::CancelProduce {
            factory: Factory::Vehicle,
            kind: 1,
        },
        hold(1, true),
    ];
    let preview = sim.production_with_pending(&commands);
    assert_eq!(sim.save_bytes().unwrap(), saved);
    // Freeze other production for a direct comparison of the command phase.
    sim.buildings.clear();
    sim.step(&commands);
    assert_eq!(sim.production(), preview);
}

#[test]
fn resume_retains_credit_stalls_and_low_power_speed() {
    let mut sim = fixture();
    sim.set_low_power_speeds(Fix::from_raw(128), Fix::from_raw(128));
    sim.buildings[0].power = -100;
    assert_eq!(sim.production_factor(), Fix::from_raw(128));
    sim.step(&[hold(1, true)]);
    assert_eq!(sim.production()[0].progress, 0);
    sim.set_credits(0);
    sim.step(&[hold(1, false)]);
    assert!(!sim.production()[0].held);
    assert_eq!(
        sim.production()[0].progress,
        0,
        "resume cannot bypass the payment stall"
    );
    sim.set_credits(1000);
    sim.step(&[]);
    assert_eq!(
        sim.production()[0].progress,
        128,
        "resume retains low-power speed"
    );
}
