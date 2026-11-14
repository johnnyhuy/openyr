use super::*;
use std::path::PathBuf;
use yr_sim::{Command, OWNER_PLAYER};

#[test]
fn retail_damage_smoke_has_global_and_per_emitter_caps_without_duplicate_tick_emission() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let dir = dir.to_str().unwrap();
    let mut session = crate::session::load(dir, "EB1.MMX", 42).unwrap();
    let kind = session.kinds.iter().position(|k| k == "MTNK").unwrap() as u16;
    session.sim = yr_sim::Sim::new(42);
    let spawn = Command::Spawn {
        kind,
        x: Fix::ONE,
        y: Fix::ONE,
        loco: yr_sim::terrain::Loco::Track,
        speed: Fix::ZERO,
        hp: 300,
        armor: 5,
        weapon: None,
    };
    session.sim.step(&vec![spawn; 140]);
    let health: Vec<_> = (1..=140)
        .map(|id| Command::SetMapUnitHealth { id, health: 128 })
        .collect();
    session.sim.step(&health);
    session.sim.step(&[]);
    let mut bank = SpriteBank::open(dir, "EB1.MMX").unwrap();
    let mut scene = EffectScene::default();
    let save = crate::snapshot::encode(&session).unwrap();
    scene.update(&session, &[], &mut bank);
    assert_eq!(scene.smoke.len(), MAX_SMOKE);
    assert_eq!(scene.smoke.first().unwrap().emitter, 13);
    assert_eq!(scene.smoke.last().unwrap().emitter, 140);
    let first_born = scene.smoke[0].born;
    scene.update(&session, &[], &mut bank);
    assert_eq!(scene.smoke.len(), MAX_SMOKE);
    assert!(scene.smoke.iter().all(|p| p.born == first_born));
    assert_eq!(crate::snapshot::encode(&session).unwrap(), save);
    let remove: Vec<_> = (2..=140).map(|id| Command::Remove { id }).collect();
    session.sim.step(&remove);
    scene.update(&session, &[], &mut bank);
    assert_eq!(scene.smoke.len(), MAX_SMOKE, "old puffs survive removal");
    for _ in 0..90 {
        session.sim.step(&[]);
        scene.update(&session, &[], &mut bank);
        assert!(scene.smoke.len() <= MAX_SMOKE);
        assert!(scene.smoke.iter().filter(|p| p.emitter == 1).count() <= 7);
    }
    assert_eq!(scene.smoke.len(), 7);
    assert!(scene.smoke.iter().all(|p| p.emitter == 1));
    session.sim.step(&[Command::Destroy { id: 1 }]);
    for _ in 0..24 {
        session.sim.step(&[]);
    }
    scene.update(&session, &[], &mut bank);
    assert!(scene.smoke.is_empty());
}

#[test]
fn retail_building_impacts_and_destruction_retain_their_foundation() {
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
    let yard = session.kinds.iter().position(|k| k == "GACNST").unwrap() as u16;
    let tank = session.kinds.iter().position(|k| k == "MTNK").unwrap() as u16;
    session.sim = yr_sim::Sim::new(42);
    session
        .sim
        .set_terrain(yr_sim::TerrainGrid::new(0, 0, 12, 12));
    let mut weapon = yr_sim::WeaponSpec::from_rules(&rules, "105mm").unwrap();
    weapon.projectile_speed = Fix::ZERO;
    weapon.rof_ticks = 1;
    session.sim.step(&[
        Command::PlaceBuilding {
            kind: yard,
            x: 2,
            y: 2,
            w: 4,
            h: 4,
            hp: 1000,
            armor: 0,
            power: 0,
            owner: yr_sim::OWNER_ENEMY,
            role: yr_sim::Role::ConstructionYard,
        },
        Command::Spawn {
            kind: tank,
            x: Fix::ONE,
            y: Fix::from_int(3),
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::ZERO,
            hp: 1000,
            armor: 0,
            weapon: Some(weapon),
        },
        Command::AssignOwner {
            id: 2,
            owner: OWNER_PLAYER,
        },
    ]);
    session.sim.drain_events();
    let mut bank = SpriteBank::open(dir, "EB1.MMX").unwrap();
    let mut scene = EffectScene::default();
    scene.remember(&session);
    session.sim.step(&[Command::Attack { id: 2, target: 1 }]);
    let events = session.sim.drain_events();
    scene.update(&session, &events, &mut bank);
    let foundation = Some(Foundation {
        cell: (2, 2),
        size: (4, 4),
    });
    let impact = scene.active.iter().find(|e| e.name == "S_CLSN22").unwrap();
    assert_eq!(impact.foundation, foundation);
    assert_eq!((impact.x, impact.y), (Fix::from_int(4), Fix::from_int(4)));
    let muzzle = scene.active.iter().find(|e| e.name == "GUNFIRE").unwrap();
    assert_eq!(muzzle.foundation, None);
    session.sim.step(&[Command::Destroy { id: 1 }]);
    let events = session.sim.drain_events();
    scene.update(&session, &events, &mut bank);
    assert!(session.sim.buildings().is_empty());
    let destruction = scene
        .active
        .iter()
        .find(|e| e.born == session.sim.tick())
        .unwrap();
    assert_eq!(destruction.foundation, foundation);
    assert_eq!(
        (destruction.x, destruction.y),
        (Fix::from_int(4), Fix::from_int(4))
    );
    // Unknown targets keep the authoritative impact point without inventing
    // the foundation of another building at the same coordinates.
    scene.update(
        &session,
        &[Event::Impact {
            target: 999,
            x: Fix::from_int(4),
            y: Fix::from_int(4),
            origin: Some(yr_sim::ShotOrigin {
                shooter: 2,
                weapon: weapon.source,
                damage: weapon.damage,
            }),
        }],
        &mut bank,
    );
    assert_eq!(scene.active.last().unwrap().foundation, None);
}

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
        ("PsiWave", Some(6)),
        ("SuperPsiWave", Some(6)),
        ("RadBeamWeapon", Some(7)),
        ("RadEruptionWeapon", Some(7)),
        ("Virusgun", Some(8)),
        ("VirusgunE", Some(8)),
        ("Punch", Some(10)),
        ("PunchE", Some(10)),
        ("105mm", Some(3)),
        ("120mm", Some(3)),
        ("PsychicJab", Some(4)),
        ("MirageGun", Some(4)),
    ] {
        session.sim = yr_sim::Sim::new(42);
        let mut spec = yr_sim::WeaponSpec::from_rules(&rules, weapon).unwrap();
        assert_eq!(
            bank.infantry_death_sequence("E1", spec.source.unwrap()),
            expected
        );
        // Keep the synthetic flight alive through restore, including the
        // short PsiWave/Punch ranges. Their retail projectiles are instant.
        spec.projectile_speed = Fix::from_raw(if matches!(expected, Some(6 | 10)) {
            64
        } else {
            128
        });
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
            spawn(
                if matches!(expected, Some(6 | 10)) {
                    2
                } else {
                    1
                },
                Some(spec),
            ),
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
        assert_eq!(session.sim.projectiles().len(), 1, "{weapon} in flight");
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
        assert!(died, "{weapon} must arrive after restore");
        // Sim updates expire the one-shot even if no frame was drawn.
        for _ in 0..80 {
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

#[test]
fn retail_line_trail_history_is_bounded_and_expires_without_draws() {
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
    let session = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    let mut scene = EffectScene::default();
    let event = Event::ProjectileMoved {
        from: (Fix::ZERO, Fix::ZERO),
        to: (Fix::ONE, Fix::ONE),
        origin: Some(yr_sim::ShotOrigin {
            shooter: 999,
            weapon: Some(rules.weapons.get("HoverMissile").unwrap().source_id()),
            damage: 25,
        }),
    };
    let hash = session.sim.state_hash();
    let save = crate::snapshot::encode(&session).unwrap();
    scene.update(&session, &vec![event; 600], &mut bank);
    assert_eq!(scene.trails.len(), MAX_TRAILS);
    let old_count = scene.trails.len();
    scene.update(
        &session,
        &[
            Event::ProjectileMoved {
                from: (Fix::ONE, Fix::ONE),
                to: (Fix::ONE, Fix::ONE),
                origin: None,
            },
            Event::ProjectileMoved {
                from: (Fix::ZERO, Fix::ZERO),
                to: (Fix::ONE, Fix::ONE),
                origin: None,
            },
        ],
        &mut bank,
    );
    assert_eq!(scene.trails.len(), old_count);
    assert_eq!(session.sim.state_hash(), hash);
    assert_eq!(crate::snapshot::encode(&session).unwrap(), save);
    for trail in &mut scene.trails {
        trail.born = session.sim.tick() + 1;
    }
    scene.update(&session, &[], &mut bank);
    assert!(
        scene.trails.is_empty(),
        "future trails are discarded after clock replacement"
    );
    scene.update(&session, &[event], &mut bank);
    let mut later = session;
    for _ in 0..16 {
        later.sim.step(&[]);
    }
    scene.update(&later, &[], &mut bank);
    assert!(scene.trails.is_empty());
}

#[test]
fn retail_projectile_trailer_history_is_bounded_and_ignores_missing_provenance() {
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
    let mut session = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    session.sim = yr_sim::Sim::new(42);
    let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    let mut scene = EffectScene::default();
    let event = |weapon| Event::ProjectileMoved {
        from: (Fix::ZERO, Fix::ZERO),
        to: (Fix::ONE, Fix::ONE),
        origin: Some(yr_sim::ShotOrigin {
            shooter: 999,
            weapon,
            damage: 100,
        }),
    };
    let source = Some(rules.weapons.get("SubTorpedo").unwrap().source_id());
    let hash = session.sim.state_hash();
    let save = crate::snapshot::encode(&session).unwrap();
    scene.update(&session, &vec![event(source); 600], &mut bank);
    assert_eq!(scene.active.len(), MAX_EFFECTS);
    assert!(scene
        .active
        .iter()
        .all(|e| e.name == "BBBLELRG" && e.foundation.is_none()));
    let count = scene.active.len();
    scene.update(
        &session,
        &[
            event(None),
            event(Some(u64::MAX)),
            Event::ProjectileMoved {
                from: (Fix::ONE, Fix::ONE),
                to: (Fix::ONE, Fix::ONE),
                origin: Some(yr_sim::ShotOrigin {
                    shooter: 999,
                    weapon: source,
                    damage: 100,
                }),
            },
        ],
        &mut bank,
    );
    assert_eq!(scene.active.len(), count);
    assert_eq!(session.sim.state_hash(), hash);
    assert_eq!(crate::snapshot::encode(&session).unwrap(), save);
    scene.active.clear();
    session.sim.step(&[]); // Tick one is outside the three-tick bubble emission phase.
    scene.update(&session, &[event(source)], &mut bank);
    assert!(scene.active.is_empty());
    session.sim.step(&[]);
    session.sim.step(&[]);
    scene.update(&session, &[event(source)], &mut bank);
    assert_eq!(scene.active.len(), 1);
    for _ in 0..300 {
        session.sim.step(&[]);
    }
    scene.update(&session, &[], &mut bank);
    assert!(scene.active.is_empty(), "clips expire without redraws");
}
