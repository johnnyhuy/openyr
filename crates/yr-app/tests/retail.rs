//! Retail skirmish setup: EB1.MMX becomes a running session with a
//! player base, starting force, enemy base and rules credits; scripted
//! destruction drives it to a win. Skips without the retail install.

use std::path::PathBuf;
use yr_app::session::{setup, Outcome};
use yr_assets::AssetFs;
use yr_formats::{IniFile, MapFile};
use yr_rules::Rules;
use yr_sim::Command;

#[test]
fn eb1_skirmish_setup_and_scripted_win() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");

    let mut session = setup(&mut fs, &map, &rules, &overlay_types, 42).expect("session sets up");
    assert_eq!(session.sim.credits(), 10_000, "rules starting money");
    let player_units = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
        .count();
    let player_bases = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_PLAYER)
        .count();
    let enemy_bases = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_ENEMY)
        .count();
    assert!(player_units >= 8, "starting force spawned: {player_units}");
    let neutral_units = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_NEUTRAL)
        .count();
    assert!(
        neutral_units >= 1,
        "map pre-placed units stay neutral: {neutral_units}"
    );
    let enemy_units = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
        .count();
    assert_eq!(enemy_units, 3, "enemy guards spawned");
    assert!(player_bases >= 3, "player base placed: {player_bases}");
    assert!(enemy_bases >= 3, "enemy base placed: {enemy_bases}");
    assert_eq!(session.outcome, Outcome::Ongoing);

    // Scripted win: destroy every enemy entity, step, expect Won.
    let kills: Vec<Command> = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
        .map(|u| Command::Destroy { id: u.id })
        .chain(
            session
                .sim
                .buildings()
                .iter()
                .filter(|b| b.owner == yr_sim::OWNER_ENEMY)
                .map(|b| Command::Destroy { id: b.id }),
        )
        .collect();
    // Determinism: a second setup with the same seed hashes identically.
    let session2 = setup(&mut fs, &map, &rules, &overlay_types, 42).expect("second setup");
    assert_eq!(session.sim.state_hash(), session2.sim.state_hash());

    assert!(!kills.is_empty());
    session.step(&kills);
    session.step(&[]);
    assert_eq!(session.outcome, Outcome::Won);
    println!("EB1 session: {player_units} player units, {player_bases} player buildings, {enemy_bases} enemy buildings -> Won");
}

/// Headless playthrough: deterministic AI produces, scouts and attacks
/// the player; the player attack-moves everything at the enemy base and
/// the match resolves (Won or Lost) without a crash.
#[test]
fn eb1_ai_builds_scouts_attacks_and_resolves() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    let mut session = setup(&mut fs, &map, &rules, &overlay_types, 7).expect("session sets up");

    let enemy_count = |s: &yr_app::session::Session| {
        s.sim
            .units()
            .iter()
            .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
            .count()
    };
    let before = enemy_count(&session);
    let mut twin = session.sim.clone();
    for _ in 0..1500 {
        session.step(&[]);
        twin.step(&[]);
        assert_eq!(session.sim.state_hash(), twin.state_hash());
    }
    assert!(session.sim.ai().unwrap().scout_sent(), "scout deployed");
    assert!(
        session.sim.ai().unwrap().groups_sent() >= 1,
        "attack group deployed"
    );
    let after = enemy_count(&session);
    assert!(
        after > before,
        "retail production grew the army: {before} -> {after}"
    );
    assert!(
        session.sim.credits_of(yr_sim::OWNER_ENEMY) < 10_000,
        "AI paid for its army"
    );
    assert!(
        session
            .sim
            .units()
            .iter()
            .any(|u| u.owner == yr_sim::OWNER_ENEMY && (u.target.is_some() || u.attack.is_some())),
        "AI marches on the player"
    );

    // Player counter-attack: every armed player unit attack-moves to the
    // enemy base (centre of its first building), re-issued every 10 s.
    let mut ticks = 0u64;
    while session.outcome == Outcome::Ongoing && ticks < 15 * 60 * 15 {
        let orders: Vec<Command> = match session
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        {
            Some(b) if ticks.is_multiple_of(150) => {
                let (x, y) = b.centre();
                session
                    .sim
                    .units()
                    .iter()
                    .filter(|u| {
                        u.owner == yr_sim::OWNER_PLAYER
                            && u.weapon.is_some()
                            && u.role == yr_sim::Role::Combat
                    })
                    .map(|u| Command::AttackMove { id: u.id, x, y })
                    .collect()
            }
            _ => Vec::new(),
        };
        session.step(&orders);
        ticks += 1;
    }
    println!(
        "EB1 AI playthrough: {:?} after {ticks} ticks, groups {}",
        session.outcome,
        session.sim.ai().unwrap().groups_sent()
    );
    assert_ne!(
        session.outcome,
        Outcome::Ongoing,
        "match resolved within 15 minutes"
    );
}

/// Production sidebar data on EB1: every allied slot resolves with a
/// retail cost and decoded cameo, the player base hosts a war factory
/// and barracks, and queued builds roll out of them on schedule.
#[test]
fn eb1_sidebar_slots_produce_units() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    let mut session = setup(&mut fs, &map, &rules, &overlay_types, 5).expect("session sets up");

    let slots: Vec<(&str, i64)> = session
        .buildables
        .iter()
        .map(|b| (b.id.as_str(), b.cost))
        .collect();
    // Sandbox unit picks (the British-only sniper is not American), then
    // every Allied structure from `[BuildingTypes]` in rules order.
    assert_eq!(
        slots,
        vec![
            ("MTNK", 700),
            ("FV", 600),
            ("SREF", 1200),
            ("CMIN", 1400),
            ("E1", 200),
            ("GGI", 400),
            ("GAPOWR", 800),
            ("GAREFN", 2000),
            ("GAPILE", 500),
            ("GADEPT", 800),
            ("GATECH", 2000),
            ("GAWEAP", 2000),
            ("NASAM", 1000),
            ("GACSPH", 2500),
            ("GAWEAT", 5000),
            ("ATESLA", 1500),
            ("GASPYSAT", 1500),
            ("GAGAP", 1000),
            ("GAPILL", 500),
            ("GAOREP", 2500),
            ("AMRADR", 1000),
            ("GAROBO", 600),
        ]
    );
    for b in &session.buildables {
        let (w, h, px) = b.cameo.as_ref().unwrap_or_else(|| panic!("{} cameo", b.id));
        assert_eq!((*w, *h), (60, 48), "{} cameo size", b.id);
        assert!(px.chunks(4).any(|p| p[3] != 0), "{} cameo has pixels", b.id);
    }
    // Retail BuildSpeed: Grizzly 700 credits -> 441 ticks.
    assert_eq!(session.buildables[0].ticks, 441);
    let roles: Vec<yr_sim::Role> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_PLAYER)
        .map(|b| b.role)
        .collect();
    assert!(roles.contains(&yr_sim::Role::WarFactory), "{roles:?}");
    assert!(roles.contains(&yr_sim::Role::Barracks), "{roles:?}");

    let cmds = vec![session.produce(0).unwrap(), session.produce(4).unwrap()];
    session.step(&cmds);
    let mut produced = Vec::new();
    for _ in 0..600 {
        session.step(&[]);
        for e in session.sim.drain_events() {
            if let yr_sim::Event::Produced { id, kind } = e {
                if !session
                    .sim
                    .units()
                    .iter()
                    .any(|u| u.id == id && u.owner == yr_sim::OWNER_PLAYER)
                {
                    continue;
                }
                produced.push((session.sim.tick(), id, kind));
            }
        }
    }
    let kinds: Vec<u16> = produced.iter().map(|p| p.2).collect();
    assert_eq!(kinds, vec![4, 0], "GI first (shorter), then the Grizzly");
    for (_, id, _) in &produced {
        let u = session.sim.units().iter().find(|u| u.id == *id);
        assert_eq!(u.map(|u| u.owner), Some(yr_sim::OWNER_PLAYER));
    }

    // The full sandbox loop: build more, then send the army at the enemy
    // base every 30 s; produced reinforcements turn it into a win.
    let more: Vec<Command> = [0, 0, 4, 4, 1, 5]
        .iter()
        .filter_map(|&i| session.produce(i))
        .collect();
    session.step(&more);
    let mut ticks = 0u64;
    while session.outcome == Outcome::Ongoing && ticks < 10 * 60 * 15 {
        ticks += 1;
        let target = session
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_ENEMY)
            .map(|b| b.centre());
        let orders: Vec<Command> = match target {
            Some((x, y)) if ticks.is_multiple_of(450) => session
                .sim
                .units()
                .iter()
                .filter(|u| {
                    u.owner == yr_sim::OWNER_PLAYER
                        && u.weapon.is_some()
                        && u.role == yr_sim::Role::Combat
                })
                .map(|u| Command::AttackMove { id: u.id, x, y })
                .collect(),
            _ => Vec::new(),
        };
        session.step(&orders);
    }
    println!(
        "EB1 produced-army playthrough: {:?} after {ticks} ticks",
        session.outcome
    );
    assert_eq!(session.outcome, Outcome::Won);
}

fn eb1_session(seed: u64) -> Option<yr_app::session::Session> {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return None;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    Some(setup(&mut fs, &map, &rules, &overlay_types, seed).expect("session sets up"))
}

fn slot_of(session: &yr_app::session::Session, id: &str) -> usize {
    session
        .buildables
        .iter()
        .position(|b| b.id == id)
        .unwrap_or_else(|| panic!("{id} slot"))
}

/// Queue structure `slot`, step until it is ready, return the ticks taken.
fn build_ready(session: &mut yr_app::session::Session, slot: usize) -> u64 {
    let cmd = session.produce(slot).expect("structure queues");
    session.step(&[cmd]);
    let mut ticks = 1;
    while session.ready_structure() != Some(slot) {
        session.step(&[]);
        ticks += 1;
        assert!(ticks < 5000, "structure never finished");
    }
    ticks
}

/// First valid origin for `slot` in rings around the player's yard.
fn spot_near_yard(session: &yr_app::session::Session, slot: usize) -> ((u16, u16), (u16, u16)) {
    let yard = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::ConstructionYard)
        .expect("player construction yard")
        .cell;
    for r in 0..16i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                let o = (
                    (i32::from(yard.0) + dx) as u16,
                    (i32::from(yard.1) + dy) as u16,
                );
                if session.can_place(slot, o) {
                    return (yard, o);
                }
            }
        }
    }
    panic!("no valid spot near the yard");
}

/// Issue #46: structures build at the construction yard, wait, and go
/// down only on valid ground; a placed power plant powers, a placed
/// refinery brings its free harvester, a placed war factory produces.
#[test]
fn eb1_structures_build_place_and_work() {
    let Some(mut session) = eb1_session(11) else {
        return;
    };
    // Exercise the long placement sequence without an opposing army.
    session.sim.set_ai(None);
    let plant = slot_of(&session, "GAPOWR");
    let lab = slot_of(&session, "GATECH");
    assert!(session.prerequisites_met(plant), "GACNST owned");
    assert!(!session.prerequisites_met(lab), "no radar yet");
    assert!(
        session.produce(lab).is_none(),
        "missing prerequisite refuses"
    );

    let credits = session.sim.credits();
    let ticks = build_ready(&mut session, plant);
    assert_eq!(session.sim.credits(), credits - 800, "full Cost= drawn");
    assert!(
        ticks >= u64::from(session.buildables[plant].ticks),
        "not instant"
    );
    assert!(
        session.produce(slot_of(&session, "GAREFN")).is_none(),
        "one structure at a time"
    );

    let (yard, good) = spot_near_yard(&session, plant);
    assert!(!session.can_place(plant, yard), "on the construction yard");
    assert!(session.place(plant, yard).is_none());
    assert!(
        !session.can_place(plant, (good.0 + 40, good.1 + 40)),
        "far from the base"
    );
    assert!(!session.can_place(plant, (0, 0)), "off the playable grid");

    // Determinism: the hash reflects the placement.
    let mut unplaced = session.sim.clone();
    let (power_before, _) = session.sim.power();
    let buildings_before = session.sim.buildings().len();
    let cmd = session.place(plant, good).expect("valid spot places");
    session.step(&[cmd]);
    unplaced.step(&[]);
    assert_ne!(session.sim.state_hash(), unplaced.state_hash());
    assert_eq!(session.sim.buildings().len(), buildings_before + 1);
    assert_eq!(
        session.sim.power().0,
        power_before + 200,
        "power plant powers"
    );
    assert_eq!(session.ready_structure(), None);
    let placed = session.sim.buildings().last().unwrap();
    assert_eq!((placed.cell, placed.owner), (good, yr_sim::OWNER_PLAYER));

    // Refinery: placement brings its `FreeUnit=` harvester.
    let refinery = slot_of(&session, "GAREFN");
    build_ready(&mut session, refinery);
    let (_, spot) = spot_near_yard(&session, refinery);
    let units_before: Vec<u32> = session.sim.units().iter().map(|u| u.id).collect();
    session.step(&[session.place(refinery, spot).unwrap()]);
    let free = session
        .sim
        .units()
        .iter()
        .find(|u| !units_before.contains(&u.id))
        .expect("free harvester");
    assert_eq!(session.kinds[usize::from(free.kind)], "CMIN");
    assert_eq!(
        (free.owner, free.role),
        (yr_sim::OWNER_PLAYER, yr_sim::Role::Harvester)
    );
    assert_eq!(
        session.sim.buildings().last().unwrap().role,
        yr_sim::Role::Refinery
    );

    // Cancel after completion refunds the full cost.
    let pile = slot_of(&session, "GAPILE");
    build_ready(&mut session, pile);
    let before = session.sim.credits();
    session.step(&[session.cancel(pile).unwrap()]);
    assert_eq!(session.sim.credits(), before + 500);
    assert_eq!(session.ready_structure(), None);

    // A second war factory placed on the map takes over production once
    // the starting one is gone.
    let weap = slot_of(&session, "GAWEAP");
    build_ready(&mut session, weap);
    let (_, spot) = spot_near_yard(&session, weap);
    session.step(&[session.place(weap, spot).unwrap()]);
    let new_weap = session.sim.buildings().last().unwrap().id;
    let old: Vec<Command> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| {
            b.owner == yr_sim::OWNER_PLAYER
                && b.role == yr_sim::Role::WarFactory
                && b.id != new_weap
        })
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    assert!(!old.is_empty());
    session.step(&old);
    let tank = session.produce(0).expect("tank queues at the new factory");
    session.step(&[tank]);
    let mut rolled = false;
    for _ in 0..2000 {
        session.step(&[]);
        if session
            .sim
            .drain_events()
            .iter()
            .any(|e| matches!(e, yr_sim::Event::Produced { kind: 0, .. }))
        {
            rolled = true;
            break;
        }
    }
    assert!(rolled, "placed war factory produces");
}

/// The retail war factory replacement uses the same queue, cost, footprint
/// checks and production host lookup as a placed player structure.
#[test]
fn eb1_ai_rebuilds_its_destroyed_factory() {
    let Some(mut session) = eb1_session(23) else {
        return;
    };
    let factory = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY && b.role == yr_sim::Role::WarFactory)
        .unwrap()
        .id;
    session.step(&[Command::Destroy { id: factory }]);
    let mut rebuilt = None;
    for _ in 0..2000 {
        session.step(&[]);
        rebuilt = session
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_ENEMY && b.role == yr_sim::Role::WarFactory)
            .map(|b| b.id);
        if rebuilt.is_some() {
            break;
        }
    }
    assert!(rebuilt.is_some_and(|id| id != factory), "AI rebuilt GAWEAP");
    assert_eq!(
        session.sim.credits(),
        10_000,
        "replacement never spends player money"
    );
    assert!(
        session.sim.credits_of(yr_sim::OWNER_ENEMY) < 9000,
        "replacement paid for"
    );
    println!("EB1 AI rebuilt GAWEAP at tick {}", session.sim.tick());
}
