use super::*;

#[test]
fn retail_damage_smoke_follows_health_movement_and_shared_composition() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let dir = dir.to_str().unwrap();
    let mut app = new_app(dir.into(), "EB1.MMX".into(), 1);
    app.map = Some(yr_render::render_map_background(dir, "EB1.MMX", 1).unwrap());
    app.sprites = load_sprites(dir, "EB1.MMX");
    let mut session = app.load_session().unwrap();
    session.sim.set_ai(None);
    let grid = session.sim.terrain().unwrap();
    let (cx, cy) = (6..grid.height().saturating_sub(6))
        .find_map(|y| {
            (6..grid.width().saturating_sub(6))
                .map(|x| grid.cell_at_local(x, y))
                .find(|&(x, y)| {
                    (-4..=4).all(|dy| {
                        (-4..=4).all(|dx| {
                            let (nx, ny) = ((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16);
                            grid.passable(yr_sim::terrain::Loco::Track, nx, ny)
                                && grid.cell(nx, ny).unwrap().height
                                    == grid.cell(x, y).unwrap().height
                        })
                    })
                })
        })
        .unwrap();
    let kind = session.kinds.iter().position(|k| k == "MTNK").unwrap() as u16;
    let remove: Vec<_> = session
        .sim
        .units()
        .iter()
        .map(|u| Command::Remove { id: u.id })
        .collect();
    session.step(&remove);
    let spawn = |x, weapon| Command::Spawn {
        kind,
        x: Fix::from_int(x) + Fix::from_raw(128),
        y: Fix::from_int(i32::from(cy)) + Fix::from_raw(128),
        loco: yr_sim::terrain::Loco::Track,
        speed: Fix::from_raw(64),
        hp: 300,
        armor: 5,
        weapon,
    };
    session.step(&[spawn(i32::from(cx), None)]);
    let target = session.sim.units()[0].id;
    let fs = yr_assets::AssetFs::open(std::path::Path::new(dir)).unwrap();
    let rules = yr_rules::Rules::load(
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap())),
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap())),
    );
    let mut weapon = yr_sim::WeaponSpec::from_rules(&rules, "105mm").unwrap();
    weapon.projectile_speed = Fix::ZERO;
    weapon.damage = 150;
    session.step(&[
        Command::AssignOwner {
            id: target,
            owner: yr_sim::OWNER_PLAYER,
        },
        spawn(i32::from(cx) - 2, Some(weapon)),
    ]);
    let shooter = session.sim.units()[1].id;
    session.step(&[Command::AssignOwner {
        id: shooter,
        owner: yr_sim::OWNER_PLAYER,
    }]);
    app.session = Some(session);
    app.view.zoom = 4.0;
    app.center_world_at(
        (f32::from(cx) + 0.5, f32::from(cy) + 0.5),
        (WIN_W - SIDEBAR_W, WIN_H),
    );
    let compose = |app: &App| {
        let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
        app.compose(&mut frame, WIN_W, WIN_H);
        frame
    };
    app.tick_session();
    assert!(app.effects.smoke.is_empty(), "healthy vehicles do not emit");
    app.pending.push(Command::AssignOwner {
        id: shooter,
        owner: yr_sim::OWNER_ENEMY,
    });
    app.pending.push(Command::Attack {
        id: shooter,
        target,
    });
    app.tick_session();
    let target_unit = &app.session.as_ref().unwrap().sim.units()[0];
    assert_eq!((target_unit.hp, target_unit.max_hp), (150, 300));
    app.pending.push(Command::Remove { id: shooter });
    let out = std::env::var_os("OPENYR_DAMAGE_SMOKE_PROOF").map(PathBuf::from);
    if let Some(out) = &out {
        std::fs::create_dir_all(out).unwrap();
    }
    for age in 0..24 {
        app.tick_session();
        let session = app.session.as_ref().unwrap();
        let tick = session.sim.tick();
        let hash = session.sim.state_hash();
        let save = yr_app::snapshot::encode(session).unwrap();
        assert!(app.effects.smoke.len() <= 7);
        assert!(app
            .effects
            .smoke
            .iter()
            .all(|p| p.emitter == target && p.particle == "SMALLGREYSMOKE"));
        let visible = compose(&app);
        assert_eq!(
            compose(&app),
            visible,
            "redrawing never emits or advances smoke"
        );
        let puffs = std::mem::take(&mut app.effects.smoke);
        let hidden = compose(&app);
        app.effects.smoke = puffs;
        let changed = visible
            .as_chunks::<4>()
            .0
            .iter()
            .zip(hidden.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count();
        if age == 12 {
            assert!(changed > 0, "damaged vehicle smoke is visible");
        }
        assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            save
        );
        if let Some(out) = &out {
            if age == 0 {
                std::fs::write(out.join("damaged.oyrsave"), &save).unwrap();
            }
            for (label, pixels) in [("smoke", &visible), ("hidden", &hidden)] {
                std::fs::write(
                    out.join(format!("{label}-t{tick:03}.png")),
                    yr_render::png::encode_rgba(WIN_W, WIN_H, pixels),
                )
                .unwrap();
            }
            println!(
                "tick {tick}: {} puffs, {changed} visible smoke pixels, hash {hash:016x}",
                app.effects.smoke.len()
            );
        }
    }
    let old = app.effects.smoke.last().unwrap();
    let old_position = (old.x, old.y, old.born);
    app.pending.push(Command::Move {
        id: target,
        x: Fix::from_int(i32::from(cx) + 3),
        y: Fix::from_int(i32::from(cy)) + Fix::from_raw(128),
    });
    app.tick_session();
    assert!(
        app.effects
            .smoke
            .iter()
            .any(|p| (p.x, p.y, p.born) == old_position),
        "emitted puffs stay at their world position"
    );
    let checkpoint = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let mut sequences = Vec::new();
    for _ in 0..2 {
        yr_app::snapshot::restore(app.session.as_mut().unwrap(), &checkpoint).unwrap();
        app.effects = Default::default();
        app.animations = Default::default();
        app.sprites = load_sprites(dir, "EB1.MMX");
        let mut sequence = Vec::new();
        for _ in 0..24 {
            app.tick_session();
            sequence.push((
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                compose(&app),
            ));
        }
        sequences.push(sequence);
    }
    assert!(
        sequences[0] == sequences[1],
        "saved motion resumes the same future smoke without old history"
    );
    let tick = app.session.as_ref().unwrap().sim.tick();
    app.pending.push(Command::SetMapUnitHealth {
        id: target,
        health: 256,
    });
    for _ in 0..24 {
        app.tick_session();
    }
    assert!(
        app.effects.smoke.iter().all(|p| p.born <= tick),
        "recovered health stops emission"
    );
    assert!(
        app.effects.smoke.is_empty(),
        "old puffs expire without redraws"
    );
    app.pending.extend([
        Command::SetMapUnitHealth {
            id: target,
            health: 64,
        },
        Command::Stop { id: target },
    ]);
    for _ in 0..6 {
        app.tick_session();
    }
    assert!(!app.effects.smoke.is_empty());
    app.pending.push(Command::Destroy { id: target });
    app.tick_session();
    assert!(
        !app.effects.smoke.is_empty(),
        "death does not erase already emitted puffs"
    );
    for _ in 0..24 {
        app.tick_session();
    }
    assert!(app.effects.smoke.is_empty());
}
