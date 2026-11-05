//! Retail participant choices must reach setup, AI, presentation and save reload.
use yr_app::{
    match_config::{Country, GameSpeed, MatchConfig},
    session::{self, Outcome, StartOptions},
    snapshot,
};
use yr_sim::{Command, OWNER_ENEMY, OWNER_PLAYER};

fn install() -> Option<String> {
    let path = std::env::var_os("OPENYR_GAMEDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
        });
    path.is_dir().then(|| path.to_str().unwrap().to_owned())
}

#[test]
fn every_supported_country_gets_its_own_base_force_and_production() {
    let Some(dir) = install() else { return };
    for country in Country::ALL {
        let mut config = MatchConfig::default();
        config.players[0].country = country;
        config.players[1].country = if country.side() == yr_app::match_config::Side::Allied {
            Country::Russia
        } else {
            Country::America
        };
        let mut session =
            session::load_with_config(&dir, "EB1.MMX", 42, StartOptions::default(), config.clone())
                .unwrap();
        for (owner, player) in config.players.iter().enumerate() {
            let side = player.country.side();
            for id in side.buildings() {
                assert!(session.sim.buildings().iter().any(|b| b.owner == owner as u8
                    && session.kinds[usize::from(b.kind)] == id), "{country:?} owner {owner} missing {id}");
            }
            for id in side.force() {
                assert!(session.sim.units().iter().any(|u| u.owner == owner as u8
                    && session.kinds[usize::from(u.kind)] == id), "{country:?} owner {owner} missing {id}");
            }
        }
        let tank = country.side().force()[2];
        assert!(session.buildables.iter().any(|b| b.id == tank));
        let other_tank = config.players[1].country.side().force()[2];
        assert!(!session.buildables.iter().any(|b| b.id == other_tank));
        assert_eq!(
            session.buildables.iter().any(|b| b.id == "SNIPE"),
            country == Country::Britain
        );
        let initial_enemy = session
            .sim
            .units()
            .iter()
            .filter(|u| u.owner == OWNER_ENEMY)
            .count();
        for _ in 0..1000 {
            session.step(&[]);
        }
        assert!(
            session
                .sim
                .units()
                .iter()
                .filter(|u| u.owner == OWNER_ENEMY)
                .count()
                > initial_enemy,
            "{country:?}: computer must produce units"
        );
        assert!(session
            .sim
            .units()
            .iter()
            .filter(|u| u.owner == OWNER_ENEMY)
            .all(|u| {
                let id = &session.kinds[usize::from(u.kind)];
                config.players[1]
                    .country
                    .side()
                    .force()
                    .contains(&id.as_str())
            }));
        println!("{country:?}: selected base, force, production and opposing AI verified");
    }
}

#[test]
fn configured_match_reload_retains_identity_rules_colors_and_determinism() {
    let Some(dir) = install() else { return };
    let mut config = MatchConfig::default();
    config.players[0].name = "Johnny".into();
    config.players[0].country = Country::Iraq;
    config.players[0].color = 6;
    config.players[0].team = 1;
    config.players[0].start = Some(1);
    config.players[1].name = "Opponent".into();
    config.players[1].country = Country::Britain;
    config.players[1].color = 4;
    config.players[1].team = 2;
    config.players[1].start = Some(0);
    config.speed = GameSpeed::Fast;
    config.short_game = true;
    let options = StartOptions {
        credits: Some(7500),
        units: Some(6),
        ..Default::default()
    };
    let mut session =
        session::load_with_config(&dir, "EB1.MMX", 99, options, config.clone()).unwrap();
    for _ in 0..120 {
        session.step(&[]);
    }
    let path = std::env::temp_dir().join(format!("openyr-participants-{}.oyr", std::process::id()));
    snapshot::save(&session, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 4);
    let (mut loaded, map) =
        snapshot::load_from_menu_with_metadata(&dir, "XMP29U2.MAP", &path, |map, metadata| {
            assert_eq!(map, "EB1.MMX");
            assert_eq!(metadata, &config);
            Ok(())
        })
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(map, "EB1.MMX");
    assert_eq!(loaded.match_config, config);
    assert_eq!(loaded.start_options, options);
    assert_eq!(loaded.owner_colors, session.owner_colors);
    assert_ne!(loaded.owner_colors[0], loaded.owner_colors[1]);
    assert_eq!(snapshot::encode(&loaded).unwrap(), bytes);
    for _ in 0..200 {
        session.step(&[]);
        loaded.step(&[]);
        assert_eq!(loaded.sim.state_hash(), session.sim.state_hash());
    }
    println!(
        "Version 4: country, names, colors, teams, starts, rules and 200 resumed ticks verified"
    );
}

#[test]
fn auto_start_avoids_explicit_opponent_and_short_game_counts_structures() {
    let Some(dir) = install() else { return };
    let mut config = MatchConfig::default();
    config.players[1].start = Some(0);
    let mut session =
        session::load_with_config(&dir, "EB1.MMX", 42, StartOptions::default(), config).unwrap();
    let yards: Vec<_> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| session.kinds[usize::from(b.kind)] == "GACNST")
        .collect();
    assert_eq!(yards.len(), 2);
    assert_ne!(yards[0].cell, yards[1].cell);
    session.sim.set_ai(None);
    let destroy: Vec<_> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == OWNER_ENEMY)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    session.step(&destroy);
    assert!(session.sim.units().iter().any(|u| u.owner == OWNER_ENEMY));
    assert_eq!(session.outcome, Outcome::Ongoing);
    session.match_config.short_game = true;
    session.step(&[]);
    assert_eq!(session.outcome, Outcome::Won);
    assert!(session
        .sim
        .buildings()
        .iter()
        .any(|b| b.owner == OWNER_PLAYER));
}
