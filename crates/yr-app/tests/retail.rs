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
