use super::*;

#[test]
fn retail_missiles_draw_from_saved_motion_after_shooter_loss() {
    let Some((mut app, cx, cy, kind, weapon)) = missile_app() else {
        return;
    };
    let dir = app.gamedir.clone();
    let dir = dir.as_str();
    let grid = app.session.as_ref().unwrap().sim.terrain().unwrap().clone();
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

fn missile_app() -> Option<(App, u16, u16, u16, yr_sim::WeaponSpec)> {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return None;
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
    Some((app, cx, cy, kind, weapon))
}

#[test]
fn retail_line_trails_follow_actual_flight_and_restart_after_load() {
    let Some((mut app, cx, cy, kind, weapon)) = missile_app() else {
        return;
    };
    let output = std::env::var_os("OPENYR_TRAIL_PROOF").map(PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    let grid = app.session.as_ref().unwrap().sim.terrain().unwrap().clone();
    app.session.as_mut().unwrap().sim = yr_sim::Sim::new(42);
    app.session.as_mut().unwrap().sim.set_terrain(grid);
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
        spawn(i32::from(cx) - 2, i32::from(cy) - 2, Some(weapon)),
        Command::AssignOwner {
            id: 1,
            owner: yr_sim::OWNER_PLAYER,
        },
        spawn(i32::from(cx) + 2, i32::from(cy) + 2, None),
        Command::AssignOwner {
            id: 2,
            owner: yr_sim::OWNER_ENEMY,
        },
        Command::Attack { id: 1, target: 2 },
    ]);
    app.tick_session();
    app.pending.push(Command::Destroy { id: 1 });
    app.tick_session();
    let checkpoint = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let mut expected = Vec::new();
    for pass in 0..2 {
        yr_app::snapshot::restore(app.session.as_mut().unwrap(), &checkpoint).unwrap();
        app.effects = Default::default();
        app.animations = Default::default();
        app.sprites = load_sprites(&app.gamedir, &app.map_name);
        assert!(
            app.effects.trails.is_empty(),
            "past trail history resets on load"
        );
        let mut found_turn = false;
        let mut last_direction = None;
        let mut arrived = false;
        for age in 1..=32 {
            if age == 1 {
                app.pending.push(Command::Move {
                    id: 2,
                    x: Fix::from_int(i32::from(cx) - 3),
                    y: Fix::from_int(i32::from(cy) + 3),
                });
            }
            app.tick_session();
            let session = app.session.as_ref().unwrap();
            let tick = session.sim.tick();
            let hash = session.sim.state_hash();
            let save = yr_app::snapshot::encode(session).unwrap();
            if let Some(trail) = app.effects.trails.last().filter(|t| t.born == tick) {
                assert_eq!(trail.style.color, [216, 216, 255]);
                assert_eq!(trail.style.decrement, 16);
                let delta = (
                    (trail.to.0 - trail.from.0).raw(),
                    (trail.to.1 - trail.from.1).raw(),
                );
                let direction = (delta.0.signum(), delta.1.signum());
                found_turn |= last_direction.is_some_and(|d| d != direction);
                last_direction = Some(direction);
                assert!(delta.0.abs() <= 160 && delta.1.abs() <= 160);
            }
            if session.sim.projectiles().is_empty() && !arrived {
                arrived = true;
                assert!(
                    app.effects.trails.iter().any(|t| t.born == tick),
                    "arrival keeps the last segment"
                );
            }
            let visible = compose(&app);
            assert_eq!(compose(&app), visible, "redraws never advance trails");
            assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                save
            );
            let trails = std::mem::take(&mut app.effects.trails);
            let hidden = compose(&app);
            app.effects.trails = trails;
            let changed = visible
                .as_chunks::<4>()
                .0
                .iter()
                .zip(hidden.as_chunks::<4>().0)
                .filter(|(a, b)| a != b)
                .count();
            if age == 4 {
                assert!(changed > 0, "actual flight leaves visible trail pixels");
            }
            if pass == 0 {
                expected.push((hash, save, visible.clone()));
                if let Some(out) = &output {
                    if age <= 32 {
                        for (label, pixels) in [("trail", &visible), ("hidden", &hidden)] {
                            std::fs::write(
                                out.join(format!("{label}-t{tick:03}.png")),
                                yr_render::png::encode_rgba(WIN_W, WIN_H, pixels),
                            )
                            .unwrap();
                        }
                        std::fs::write(out.join(format!("state-t{tick:03}.json")),serde_json::to_vec_pretty(&serde_json::json!({"tick":tick,"hash":format!("{hash:016x}"),"changed_pixels":changed,"segments":app.effects.trails.iter().map(|t| serde_json::json!({"from":[t.from.0.raw(),t.from.1.raw()],"to":[t.to.0.raw(),t.to.1.raw()],"born":t.born,"color":t.color(tick)})).collect::<Vec<_>>()})).unwrap()).unwrap();
                    }
                }
            } else {
                let (old_hash, old_save, old_frame) = &expected[age - 1];
                assert_eq!(*old_hash, hash);
                assert_eq!(*old_save, save);
                assert_eq!(
                    *old_frame, visible,
                    "fresh-cache saved flight reproduces tick {tick}"
                );
            }
            println!("pass {pass}, tick {tick}, {} segments, {changed} changed trail pixels, hash {hash:016x}",app.effects.trails.len());
        }
        assert!(found_turn, "moving target bends the observed trail");
        assert!(arrived);
        assert!(
            app.effects.trails.is_empty(),
            "past segments expire after arrival without redraw dependence"
        );
    }
    if let Some(out) = output {
        std::fs::write(out.join("saved-flight.oyrsave"), checkpoint).unwrap();
    }
}
