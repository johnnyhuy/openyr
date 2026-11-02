use super::*;
use std::path::PathBuf;
use yr_sim::{Command, OWNER_PLAYER};

#[test]
fn retail_infantry_deaths_use_lethal_weapon_after_saved_flight_and_shooter_loss() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let rules = yr_rules::Rules::load(
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap())),
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap())),
    );
    let dir = dir.to_str().unwrap();
    let mut session = crate::session::load(dir, "EB1.MMX", 42).unwrap();
    let kind = session.kinds.iter().position(|k| k == "E1").unwrap() as u16;
    let mut bank = SpriteBank::open(dir, "EB1.MMX").unwrap();
    for (weapon, expected) in [
        ("M60", Some(1)),
        ("HoverMissile", Some(2)),
        ("ElectricBolt", Some(5)),
        ("105mm", None),
    ] {
        session.sim = yr_sim::Sim::new(42);
        let mut spec = yr_sim::WeaponSpec::from_rules(&rules, weapon).unwrap();
        assert_eq!(
            bank.infantry_death_sequence("E1", spec.source.unwrap()),
            expected
        );
        spec.projectile_speed = Fix::from_raw(128);
        let spawn = |x, weapon| Command::Spawn {
            kind,
            x: Fix::from_int(x),
            y: Fix::ONE,
            loco: yr_sim::terrain::Loco::Foot,
            speed: Fix::from_raw(32),
            hp: 1,
            armor: 0,
            weapon,
        };
        session.sim.step(&[
            spawn(1, Some(spec)),
            spawn(3, None),
            Command::AssignOwner {
                id: 2,
                owner: yr_sim::OWNER_ENEMY,
            },
            Command::Attack { id: 1, target: 2 },
        ]);
        session.sim.drain_events();
        session.sim.step(&[Command::Destroy { id: 1 }]);
        session.sim.drain_events();
        session.sim.step(&[Command::Move {
            id: 2,
            x: Fix::from_int(4),
            y: Fix::ONE,
        }]);
        session.sim.drain_events();
        let save = crate::snapshot::encode(&session).unwrap();
        crate::snapshot::restore(&mut session, &save).unwrap();
        let mut scene = EffectScene::default();
        scene.remember(&session);
        let mut died = false;
        for _ in 0..12 {
            scene.remember(&session);
            session.sim.step(&[]);
            let events = session.sim.drain_events();
            let save = crate::snapshot::encode(&session).unwrap();
            let hash = session.sim.state_hash();
            scene.update(&session, &events, &mut bank);
            assert_eq!(session.sim.state_hash(), hash);
            assert_eq!(crate::snapshot::encode(&session).unwrap(), save);
            if events.contains(&Event::Died { id: 2 }) {
                died = true;
                assert_eq!(scene.deaths.first().map(|d| d.sequence), expected);
                if let Some(death) = scene.deaths.first() {
                    let position = events
                        .iter()
                        .find_map(|e| match *e {
                            Event::FatalHit {
                                target: 2, x, y, ..
                            } => Some((x, y)),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(
                        (&*death.kind, death.owner, death.x, death.y, death.born),
                        (
                            "E1",
                            yr_sim::OWNER_ENEMY,
                            position.0,
                            position.1,
                            session.sim.tick()
                        )
                    );
                    assert!(
                        death.x > Fix::from_int(3),
                        "death uses the moved fatal-hit position"
                    );
                }
                break;
            }
        }
        assert!(died);
        // Sim updates expire the one-shot even if no frame was drawn.
        for _ in 0..40 {
            session.sim.step(&[]);
        }
        scene.update(&session, &[], &mut bank);
        assert!(scene.deaths.is_empty());
        assert!(EffectScene::default().deaths.is_empty());
    }
    // A victim created and reassigned in the fatal tick has no previous pose.
    session.sim = yr_sim::Sim::new(42);
    let spawn = |weapon| Command::Spawn {
        kind,
        x: Fix::ONE,
        y: Fix::ONE,
        loco: yr_sim::terrain::Loco::Foot,
        speed: Fix::ZERO,
        hp: 1,
        armor: 0,
        weapon,
    };
    session.sim.step(&[
        spawn(yr_sim::WeaponSpec::from_rules(&rules, "M60")),
        Command::AssignOwner {
            id: 1,
            owner: OWNER_PLAYER,
        },
    ]);
    let mut scene = EffectScene::default();
    scene.remember(&session);
    assert!(!scene.entities.contains_key(&2));
    session.sim.step(&[
        spawn(None),
        Command::AssignOwner {
            id: 2,
            owner: yr_sim::OWNER_ENEMY,
        },
        Command::Attack { id: 1, target: 2 },
    ]);
    let events = session.sim.drain_events();
    scene.update(&session, &events, &mut bank);
    assert_eq!(scene.deaths.len(), 1);
    assert_eq!(
        (
            &*scene.deaths[0].kind,
            scene.deaths[0].owner,
            scene.deaths[0].facing
        ),
        ("E1", yr_sim::OWNER_ENEMY, 128)
    );
}

#[test]
fn retail_impacts_use_the_fired_weapon_after_shooter_death_and_restore() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let mut ini =
        yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    let map =
        yr_formats::MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).unwrap();
    ini.merge_from(&map.ini);
    let art = yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = yr_rules::Rules::load(&ini, &art);
    let mut session = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    let kind = u16::try_from(session.kinds.iter().position(|k| k == "MTNK").unwrap()).unwrap();
    let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    assert!(bank.impact_effect(u64::MAX, 100).is_none());
    for (weapon, expected) in [("105mm", "S_CLSN22"), ("M60", "PIFFPIFF")] {
        session.sim = yr_sim::Sim::new(42);
        let mut spec = yr_sim::WeaponSpec::from_rules(&rules, weapon).unwrap();
        let delivered = spec.damage;
        assert_eq!(
            bank.impact_effect(spec.source.unwrap(), delivered)
                .as_deref(),
            Some(expected)
        );
        spec.projectile_speed = Fix::from_raw(128);
        let spawn = |x, weapon| Command::Spawn {
            kind,
            x: Fix::from_int(x),
            y: Fix::ONE,
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::ZERO,
            hp: 2000,
            armor: 5,
            weapon,
        };
        session.sim.step(&[
            spawn(1, Some(spec)),
            spawn(5, None),
            Command::AssignOwner {
                id: 2,
                owner: yr_sim::OWNER_ENEMY,
            },
            Command::Attack { id: 1, target: 2 },
        ]);
        assert!(!session
            .sim
            .drain_events()
            .iter()
            .any(|e| matches!(e, Event::Impact { .. })));
        session.sim.step(&[Command::Destroy { id: 1 }]);
        session.sim.drain_events();
        let save = crate::snapshot::encode(&session).unwrap();
        crate::snapshot::restore(&mut session, &save).unwrap();
        // Rebuild presentation after loading: there is no remembered shooter.
        let mut scene = EffectScene::default();
        scene.remember(&session);
        assert!(!scene.entities.contains_key(&1));
        let mut impacts = 0;
        for _ in 0..20 {
            session.sim.step(&[]);
            let events = session.sim.drain_events();
            let hash = session.sim.state_hash();
            let bytes = crate::snapshot::encode(&session).unwrap();
            scene.update(&session, &events, &mut bank);
            assert_eq!(session.sim.state_hash(), hash);
            assert!(crate::snapshot::encode(&session).unwrap() == bytes);
            if events.iter().any(|e| matches!(e, Event::Impact { .. })) {
                impacts += 1;
                assert_eq!(scene.active.len(), 1);
                assert_eq!(
                    scene.active[0].name, expected,
                    "same MTNK source/victim types, different fired weapons"
                );
                assert_eq!(
                    (scene.active[0].x, scene.active[0].y),
                    (Fix::from_int(5), Fix::ONE)
                );
            }
        }
        assert_eq!(impacts, 1);
        assert_eq!(session.sim.units()[0].hp, 2000 - spec.damage_against(5));
        scene.active.clear();
        scene.update(
            &session,
            &[Event::Impact {
                target: 2,
                x: Fix::from_int(5),
                y: Fix::ONE,
                origin: None,
            }],
            &mut bank,
        );
        assert!(
            scene.active.is_empty(),
            "old in-flight shots have no guessed art"
        );
    }
}

#[test]
fn retail_effects_follow_shots_deaths_and_leave_sim_and_saves_unchanged() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let dir = dir.to_str().unwrap();
    let mut session = crate::session::load(dir, "EB1.MMX", 42).unwrap();
    session.sim.set_ai(None);
    let mut bank = SpriteBank::open(dir, "EB1.MMX").unwrap();
    let mut scene = EffectScene::default();
    let tank = session
        .sim
        .units()
        .iter()
        .find(|u| {
            u.owner == OWNER_PLAYER
                && u.weapon.is_some()
                && bank
                    .muzzle_effect(&session.kinds[usize::from(u.kind)], u.facing)
                    .as_deref()
                    == Some("GUNFIRE")
                && bank
                    .destruction_effect(&session.kinds[usize::from(u.kind)])
                    .is_some()
        })
        .unwrap()
        .clone();
    let kind = &session.kinds[usize::from(tank.kind)];
    assert_eq!(
        bank.muzzle_effect(kind, tank.facing).as_deref(),
        Some("GUNFIRE")
    );
    for facing in [0, 32, 64, 96, 128, 160, 192, 224] {
        let name = bank.muzzle_effect("E1", facing).unwrap();
        assert!(name.starts_with("MGUN-"));
        assert!(bank.get_effect(&name, 0).is_some());
    }
    scene.remember(&session);
    session.step(&[Command::Destroy { id: tank.id }]);
    let hash = session.sim.state_hash();
    let save = crate::snapshot::encode(&session).unwrap();
    scene.update(
        &session,
        &[
            Event::Fired {
                shooter: tank.id,
                target: 0,
            },
            Event::Died { id: tank.id },
        ],
        &mut bank,
    );
    assert_eq!(
        scene.active.len(),
        2,
        "a shooter which dies in the same tick retains both effects"
    );
    assert_eq!((scene.active[0].x, scene.active[0].y), (tank.x, tank.y));
    assert_eq!(scene.active[0].name, "GUNFIRE");
    assert_eq!(scene.active[1].name, "TWLT070");
    for _ in 0..10 {
        assert!(bank.get_effect(&scene.active[0].name, 0).is_some());
    }
    assert_eq!(session.sim.state_hash(), hash);
    assert_eq!(crate::snapshot::encode(&session).unwrap(), save);
    scene.remember(&session);
    let survivor = session
        .sim
        .units()
        .iter()
        .find(|u| {
            u.owner == OWNER_PLAYER
                && bank
                    .muzzle_effect(&session.kinds[usize::from(u.kind)], u.facing)
                    .is_some()
        })
        .unwrap()
        .id;
    let events = vec![
        Event::Fired {
            shooter: survivor,
            target: 0
        };
        300
    ];
    scene.update(&session, &events, &mut bank);
    assert_eq!(scene.active.len(), MAX_EFFECTS);
    for _ in 0..300 {
        session.step(&[]);
    }
    scene.update(&session, &[], &mut bank);
    assert!(
        scene.active.is_empty(),
        "one-shot art expires without any redraws"
    );
    let restored = EffectScene::default();
    assert!(
        restored.active.is_empty(),
        "transient effects are not replayed on load"
    );
}
