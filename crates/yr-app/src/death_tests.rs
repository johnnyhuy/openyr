use super::*;

/// Actual fatal shots feed the same clocks and compositor as the window.
#[test]
fn retail_infantry_death_composition_proof() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let dir = dir.to_str().unwrap();
    let mut app = new_app(dir.into(), "EB1.MMX".into(), 1);
    app.map = Some(yr_render::render_map(dir, "EB1.MMX", 1).unwrap());
    app.sprites = load_sprites(dir, "EB1.MMX");
    let mut session = app.load_session().unwrap();
    session.sim.set_ai(None);
    let fs = yr_assets::AssetFs::open(std::path::Path::new(dir)).unwrap();
    let rules = yr_rules::Rules::load(
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap())),
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap())),
    );
    let grid = session.sim.terrain().unwrap();
    let (cx, cy) = (6..grid.height().saturating_sub(6))
        .find_map(|ly| {
            (6..grid.width().saturating_sub(6))
                .map(|lx| grid.cell_at_local(lx, ly))
                .find(|&(x, y)| {
                    (-4..=4).all(|dy| {
                        (-4..=4).all(|dx| {
                            let (x2, y2) = ((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16);
                            grid.passable(yr_sim::terrain::Loco::Foot, x2, y2)
                                && grid.cell(x2, y2).unwrap().height
                                    == grid.cell(x, y).unwrap().height
                        })
                    })
                })
        })
        .expect("retail clear patch");
    let infantry = session.kinds.iter().position(|k| k == "E1").unwrap() as u16;
    // Remove starting armies so the controlled proof has no incidental shots.
    let remove: Vec<_> = session
        .sim
        .units()
        .iter()
        .map(|u| Command::Remove { id: u.id })
        .collect();
    session.step(&remove);
    session.sim.drain_events();
    // The Allied sandbox does not offer these infantry. Register the retail
    // types only for the controlled special-death fatal-shot fixtures.
    for kind in ["SHK", "DESO", "YURI"] {
        if !session.kinds.iter().any(|registered| registered == kind) {
            session.kinds.push(kind.into());
        }
    }
    app.session = Some(session);
    app.view.zoom = 4.0;
    app.center_world_at(
        (f32::from(cx) + 0.5, f32::from(cy) + 0.5),
        (WIN_W - SIDEBAR_W, WIN_H),
    );
    let output = std::env::var_os("OPENYR_INFANTRY_DEATH_PROOF").map(PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    for (weapon, shooter_type, sequence, last_age) in [
        ("M60", "E1", 1, 36),
        ("HoverMissile", "FV", 2, 36),
        ("ElectricBolt", "SHK", 5, 36),
        ("RadBeamWeapon", "DESO", 7, 72),
        ("PsiWave", "YURI", 6, 36),
    ] {
        let shooter_kind = app
            .session
            .as_ref()
            .unwrap()
            .kinds
            .iter()
            .position(|k| k == shooter_type)
            .unwrap() as u16;
        let spec = yr_sim::WeaponSpec::from_rules(&rules, weapon).unwrap();
        let spawn = |kind, x, hp, weapon| Command::Spawn {
            kind,
            x: Fix::from_int(x) + Fix::from_raw(128),
            y: Fix::from_int(i32::from(cy)) + Fix::from_raw(128),
            loco: yr_sim::terrain::Loco::Foot,
            speed: Fix::ZERO,
            hp,
            armor: 0,
            weapon,
        };
        // PsiWave has a one-cell range. This checks its lethal-hit art;
        // deployment and area damage are not simulated by this fixture.
        let distance = if sequence == 6 { 1 } else { 2 };
        app.pending.push(spawn(
            shooter_kind,
            i32::from(cx) - distance,
            1000,
            Some(spec),
        ));
        app.tick_session();
        let shooter = app.session.as_ref().unwrap().sim.units().last().unwrap().id;
        app.pending.extend([
            Command::AssignOwner {
                id: shooter,
                owner: yr_sim::OWNER_PLAYER,
            },
            spawn(infantry, i32::from(cx), 1, None),
        ]);
        app.tick_session();
        let target = app.session.as_ref().unwrap().sim.units().last().unwrap().id;
        app.pending.extend([
            Command::AssignOwner {
                id: target,
                owner: yr_sim::OWNER_ENEMY,
            },
            Command::Attack {
                id: shooter,
                target,
            },
        ]);
        app.tick_session();
        for _ in 0..60 {
            if !app.effects.deaths.is_empty() {
                break;
            }
            app.tick_session();
        }
        assert_eq!(app.effects.deaths.len(), 1);
        assert_eq!(app.effects.deaths[0].sequence, sequence);
        assert_eq!(app.effects.deaths[0].owner, yr_sim::OWNER_ENEMY);
        assert!(!app
            .session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .any(|u| u.id == target));
        for age in 0..=last_age {
            if age > 0 {
                app.tick_session();
            }
            if sequence <= 2 && age % 2 != 0 {
                continue;
            }
            let tick = app.session.as_ref().unwrap().sim.tick();
            let hash = app.session.as_ref().unwrap().sim.state_hash();
            let save = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
            app.compose(&mut frame, WIN_W, WIN_H);
            let comparison = if age == 8 || age == last_age {
                let deaths = std::mem::take(&mut app.effects.deaths);
                let mut before = vec![0; frame.len()];
                app.compose(&mut before, WIN_W, WIN_H);
                app.effects.deaths = deaths;
                let changed = frame
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(before.as_chunks::<4>().0)
                    .filter(|(a, b)| a != b)
                    .count();
                if age == 8 {
                    assert!(changed > 0, "death {sequence} art is visible");
                } else {
                    assert_eq!(changed, 0, "one-shot expires");
                }
                Some((before, changed))
            } else {
                None
            };
            if age == 0 {
                let mut repeated = vec![0; frame.len()];
                app.compose(&mut repeated, WIN_W, WIN_H);
                assert_eq!(frame, repeated, "redraw does not advance the clip");
            }
            assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                save
            );
            if let Some(out) = &output {
                let name = format!("die{sequence}-age{age:02}-t{tick:05}");
                std::fs::write(
                    out.join(format!("{name}.png")),
                    yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
                )
                .unwrap();
                if age == 8 {
                    std::fs::write(
                        out.join(format!("die{sequence}-without.png")),
                        yr_render::png::encode_rgba(WIN_W, WIN_H, &comparison.as_ref().unwrap().0),
                    )
                    .unwrap();
                }
                std::fs::write(
                    out.join(format!("{name}.json")),
                    serde_json::to_vec_pretty(&serde_json::json!({
                        "tick": tick, "age": age, "sequence": sequence, "weapon": weapon,
                        "shooter": shooter, "target": target, "x": cx, "y": cy,
                    "changed_pixels": comparison.as_ref().map(|(_, changed)| changed), "state_hash": format!("{hash:016x}"),
                    }))
                    .unwrap(),
                )
                .unwrap();
            }
        }
        app.pending.push(Command::Remove { id: shooter });
        app.tick_session();
    }
}
