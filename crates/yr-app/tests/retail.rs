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

/// Headless playthrough: scripted waves launch on schedule and march on
/// the player; the player attack-moves everything at the enemy base and
/// the match resolves (Won or Lost) without a crash.
#[test]
fn eb1_waves_launch_and_headless_playthrough_resolves() {
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
    let (_, total) = session.waves_progress();
    assert_eq!(total, 5, "wave plan");
    let first =
        session.next_wave_in_secs().expect("first wave pending") * yr_app::session::TICKS_PER_SEC;
    let before = enemy_count(&session);
    for _ in 0..first {
        session.step(&[]);
    }
    session.step(&[]);
    assert_eq!(session.waves_progress().0, 1, "first wave launched");
    let after = enemy_count(&session);
    assert!(after > before, "wave units spawned: {before} -> {after}");
    let marching = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_ENEMY && u.target.is_some())
        .count();
    assert!(marching >= 2, "wave marches on the player: {marching}");

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
        "EB1 playthrough: {:?} after {ticks} ticks, waves {:?}",
        session.outcome,
        session.waves_progress()
    );
    assert_ne!(
        session.outcome,
        Outcome::Ongoing,
        "match resolved within 15 minutes"
    );
}
