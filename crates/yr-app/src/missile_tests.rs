use super::*;

#[test]
fn retail_missiles_draw_from_saved_motion_after_shooter_loss() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let dir = dir.to_str().unwrap();
    let fs = yr_assets::AssetFs::open(std::path::Path::new(dir)).unwrap();
    let rules = yr_rules::Rules::load(
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap())),
        &yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap())),
    );
    let weapon = yr_sim::WeaponSpec::from_rules(&rules, "HoverMissile").unwrap();
    assert_eq!(weapon.projectile_speed.raw(), 160);
    let mut app = new_app(dir.into(), "EB1.MMX".into(), 1);
    app.map = Some(yr_render::render_map(dir, "EB1.MMX", 1).unwrap());
    app.sprites = load_sprites(dir, "EB1.MMX");
    let session = app.load_session().unwrap();
    let grid = session.sim.terrain().unwrap().clone();
    let (cx, cy) = (6..grid.height().saturating_sub(6))
        .find_map(|ly| {
            (6..grid.width().saturating_sub(6))
                .map(|lx| grid.cell_at_local(lx, ly))
                .find(|&(x, y)| {
                    (-5..=5).all(|dy| {
                        (-5..=5).all(|dx| {
                            let (x2, y2) = ((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16);
                            grid.passable(yr_sim::terrain::Loco::Track, x2, y2)
                                && grid.cell(x2, y2).unwrap().height
                                    == grid.cell(x, y).unwrap().height
                        })
                    })
                })
        })
        .unwrap();
    let kind = session.kinds.iter().position(|k| k == "FV").unwrap() as u16;
    app.session = Some(session);
    app.view.zoom = 4.0;
    app.center_world_at((f32::from(cx), f32::from(cy)), (WIN_W - SIDEBAR_W, WIN_H));
    let output = std::env::var_os("OPENYR_MISSILE_PROOF").map(PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    for (direction, (dx, dy)) in [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ]
    .into_iter()
    .enumerate()
    {
        app.session.as_mut().unwrap().sim = yr_sim::Sim::new(42);
        app.session.as_mut().unwrap().sim.set_terrain(grid.clone());
        app.session.as_mut().unwrap().outcome = Outcome::Ongoing;
        app.effects = Default::default();
        app.animations = Default::default();
        let spawn = |x, y, weapon| Command::Spawn {
            kind,
            x: Fix::from_int(x),
            y: Fix::from_int(y),
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::from_raw(32),
            hp: 2000,
            armor: 0,
            weapon,
        };
        app.pending.extend([
            spawn(i32::from(cx) - dx * 2, i32::from(cy) - dy * 2, Some(weapon)),
            Command::AssignOwner {
                id: 1,
                owner: yr_sim::OWNER_PLAYER,
            },
            spawn(i32::from(cx) + dx * 2, i32::from(cy) + dy * 2, None),
            Command::AssignOwner {
                id: 2,
                owner: yr_sim::OWNER_ENEMY,
            },
            Command::Attack { id: 1, target: 2 },
        ]);
        app.tick_session();
        assert_eq!(app.session.as_ref().unwrap().sim.projectiles().len(), 1);
        app.pending.push(Command::Destroy { id: 1 });
        app.tick_session();
        app.tick_session();
        let session = app.session.as_ref().unwrap();
        assert!(session.sim.units().iter().all(|u| u.id != 1));
        let p = session.sim.projectiles()[0];
        let motion = session.sim.projectile_motion(&p).unwrap();
        assert_eq!((motion.0.raw(), motion.1.raw()), (dx * 160, dy * 160));
        let tick = session.sim.tick();
        let hash = session.sim.state_hash();
        let save = yr_app::snapshot::encode(session).unwrap();
        let frame = compose(&app);
        assert_eq!(compose(&app), frame);
        assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            save
        );
        let mut missing_target = app.session.as_ref().unwrap().sim.clone();
        missing_target.step(&[Command::Destroy { id: 2 }]);
        assert!(missing_target.projectile_motion(&p).is_none());
        let original = app.session.as_ref().unwrap().sim.clone();
        let mut hidden = serde_json::to_value(&original).unwrap();
        for p in hidden["projectiles"].as_array_mut().unwrap() {
            p["origin"] = serde_json::Value::Null;
        }
        app.session.as_mut().unwrap().sim = serde_json::from_value(hidden).unwrap();
        assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
        let baseline = compose(&app);
        app.session.as_mut().unwrap().sim = original;
        let changed = frame
            .as_chunks::<4>()
            .0
            .iter()
            .zip(baseline.as_chunks::<4>().0.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed > 0,
            "missile must be visible, direction {direction}"
        );
        yr_app::snapshot::restore(app.session.as_mut().unwrap(), &save).unwrap();
        app.sprites = load_sprites(dir, "EB1.MMX");
        assert_eq!(
            compose(&app),
            frame,
            "saved direction reconstructs without shooter or draw history"
        );
        if let Some(out) = &output {
            for (label, pixels) in [("visible", &frame), ("hidden", &baseline)] {
                std::fs::write(
                    out.join(format!("direction-{direction}-{label}-t{tick:03}.png")),
                    yr_render::png::encode_rgba(WIN_W, WIN_H, pixels),
                )
                .unwrap();
            }
            std::fs::write(out.join(format!("direction-{direction}-t{tick:03}.json")),serde_json::to_vec_pretty(&serde_json::json!({"tick":tick,"direction":direction,"motion":[motion.0.raw(),motion.1.raw()],"state_hash":format!("{hash:016x}"),"changed_pixels":changed,"projectile":p})).unwrap()).unwrap();
            if direction == 0 {
                std::fs::write(out.join("missile-in-flight.oyrsave"), &save).unwrap();
            }
        }
        // A changed target changes the next flight step, independently of
        // the dead shooter's pose and any previous rendering calls.
        app.pending.push(Command::Move {
            id: 2,
            x: Fix::from_int(i32::from(cx) + 3),
            y: Fix::from_int(i32::from(cy) + 3),
        });
        let mut arrived = false;
        for _ in 0..40 {
            app.tick_session();
            let sim = &app.session.as_ref().unwrap().sim;
            if sim.projectiles().is_empty() {
                arrived = true;
                break;
            }
            let p = sim.projectiles()[0];
            let expected = sim.projectile_motion(&p).unwrap();
            let mut next = sim.clone();
            next.step(&[Command::Stop { id: 2 }]);
            if let Some(q) = next.projectiles().first() {
                assert_eq!((q.x - p.x, q.y - p.y), expected);
            }
        }
        assert!(arrived, "missile arrives after target movement");
        assert!(app.session.as_ref().unwrap().sim.units()[0].hp < 2000);
        println!("direction {direction}: tick {tick}, hash {hash:016x}, {changed} visible body pixels, restored after shooter loss");
    }
}

fn compose(app: &App) -> Vec<u8> {
    let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose(&mut frame, WIN_W, WIN_H);
    frame
}
