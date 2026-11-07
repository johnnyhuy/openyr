use super::*;

#[test]
fn inactive_scenes_have_no_current_attack_targets() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.selected = [1, u32::MAX].into();
    assert!(app.current_attack_targets().is_empty());
    let mut frame = vec![0; 640 * 480 * 4];
    app.draw_attack_targets(&mut frame, (640, 480));
    assert!(frame.iter().all(|v| *v == 0));
}

#[test]
fn retail_current_targets_follow_sim_selection_and_lifecycle_without_changing_saves() {
    let directory = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.session.as_mut().unwrap().sim.set_ai(None);
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let sim = &app.session.as_ref().unwrap().sim;
    let attackers: Vec<_> = sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some_and(|w| w.damage > 0))
        .take(2)
        .map(|u| u.id)
        .collect();
    assert_eq!(attackers.len(), 2);
    let enemy = sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_ENEMY)
        .unwrap()
        .id;
    let building = sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        .unwrap()
        .id;
    let initial = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    for target in [enemy, building] {
        yr_app::snapshot::restore(app.session.as_mut().unwrap(), &initial).unwrap();
        app.selected = attackers.iter().copied().chain([enemy, u32::MAX]).collect();
        point_at(&mut app, target, (1280, 800));
        let expected = app.world_action(app.cursor).unwrap().commands;
        assert_eq!(expected.len(), 2);
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        assert_eq!(app.pending, expected);
        assert!(
            app.current_attack_targets().is_empty(),
            "pending orders are not current sim targets"
        );
        app.tick_session();
        assert_eq!(
            app.current_attack_targets(),
            [target].into(),
            "multiple selected attackers share one highlight"
        );
        let active = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        // Clearing the short notice and pointer ownership cannot remove actual target state.
        app.order_feedback.clear();
        app.invalidate_pointer();
        for size in [(640, 480), (1000, 700), (1280, 800)] {
            app.resize_window(size, 1.0).unwrap();
            for zoom in [0.5, 1.0, 2.0] {
                app.view.zoom = zoom;
                center(&mut app, target, size);
                assert_eq!(app.current_attack_targets(), [target].into());
                let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
                app.draw_attack_targets(&mut frame, size);
                let pixels = frame
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| **p == ENEMY_COL)
                    .count();
                if target == enemy || zoom <= 1.0 {
                    assert_eq!(pixels, 44, "{target} {size:?} {zoom}");
                } else {
                    assert!(pixels <= 44, "large zoomed building corners may clip");
                }
                for y in 0..size.1 {
                    let start = ((y * size.0 + size.0 - SIDEBAR_W) * 4) as usize;
                    assert!(frame[start..((y + 1) * size.0 * 4) as usize]
                        .iter()
                        .all(|v| *v == 0));
                }
                assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == active);
                if zoom == 1.0 {
                    proof(
                        &app,
                        size,
                        if target == enemy { "unit" } else { "building" },
                    );
                }
            }
        }
        app.pointer.focused = false;
        assert!(app.current_attack_targets().is_empty());
        app.pointer.focused = true;
        app.menu.set(yr_app::menu::Page::Pause);
        assert!(app.current_attack_targets().is_empty());
        app.menu.set(yr_app::menu::Page::Closed);
        app.placing = Some(0);
        assert!(app.current_attack_targets().is_empty());
        app.placing = None;
        app.service_mode = ServiceMode::Repair;
        assert!(app.current_attack_targets().is_empty());
        app.service_mode = ServiceMode::None;
        app.session.as_mut().unwrap().outcome = Outcome::Won;
        assert!(app.current_attack_targets().is_empty());
        app.session.as_mut().unwrap().outcome = Outcome::Ongoing;
        app.selected = [enemy, u32::MAX].into();
        assert!(app.current_attack_targets().is_empty());
        app.selected = attackers.iter().copied().collect();
        assert_eq!(app.current_attack_targets(), [target].into());
        assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == active);
        yr_app::snapshot::restore(app.session.as_mut().unwrap(), &initial).unwrap();
        assert!(app.current_attack_targets().is_empty());
        yr_app::snapshot::restore(app.session.as_mut().unwrap(), &active).unwrap();
        assert_eq!(app.current_attack_targets(), [target].into());
        app.unit_order_key(KeyCode::KeyS, ElementState::Pressed, false, Instant::now());
        let commands = app.pending.clone();
        assert_eq!(app.current_attack_targets(), [target].into());
        let mut reference = app.load_session().unwrap();
        yr_app::snapshot::restore(&mut reference, &active).unwrap();
        app.tick_session();
        reference.step(&commands);
        reference.sim.drain_events();
        assert!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
                == yr_app::snapshot::encode(&reference).unwrap()
        );
        assert!(
            app.current_attack_targets().is_empty(),
            "far targets stay cleared after Stop"
        );
        yr_app::snapshot::restore(app.session.as_mut().unwrap(), &active).unwrap();
        if target == enemy {
            // Two valid in-cell poses isolate drawing from parked guards.
            let size = app.camera_size();
            let mut before = vec![0; (size.0 * size.1 * 4) as usize];
            app.draw_attack_targets(&mut before, size);
            let sim = &mut app.session.as_mut().unwrap().sim;
            let raw = sim.save_bytes().unwrap();
            let mut json: serde_json::Value = serde_json::from_slice(&raw[12..]).unwrap();
            for unit in json["units"].as_array_mut().unwrap() {
                if unit["id"] == target {
                    unit["x"] = (unit["x"].as_i64().unwrap() + 64).into();
                }
            }
            let mut raw = raw[..12].to_vec();
            raw.extend(serde_json::to_vec(&json).unwrap());
            *sim = yr_sim::Sim::from_save_bytes(&raw).unwrap();
            let mut after = vec![0; before.len()];
            app.draw_attack_targets(&mut after, size);
            assert_ne!(before, after);
            assert_eq!(app.current_attack_targets(), [target].into());
        }
        app.session
            .as_mut()
            .unwrap()
            .step(&[Command::Destroy { id: target }]);
        assert!(app.current_attack_targets().is_empty());
    }
    // A quiet controlled live fixture uses a stationary, durable target in range.
    yr_app::snapshot::restore(app.session.as_mut().unwrap(), &initial).unwrap();
    let own = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .units()
        .iter()
        .find(|u| u.id == attackers[0])
        .unwrap()
        .clone();
    let destination = (-4..=4)
        .find_map(|dy| {
            (-4..=4).find_map(|dx| {
                let (x, y) = (
                    Fix::from_int(own.x.to_int_floor() + dx) + Fix::from_raw(128),
                    Fix::from_int(own.y.to_int_floor() + dy) + Fix::from_raw(128),
                );
                let distance = dx * dx + dy * dy;
                ((2..=8).contains(&distance)
                    && app.session.as_ref().unwrap().sim.can_move_to(own.id, x, y)
                    && app
                        .session
                        .as_ref()
                        .unwrap()
                        .sim
                        .terrain()
                        .unwrap()
                        .passable(own.loco, x.to_int_floor() as u16, y.to_int_floor() as u16)
                    && app.session.as_ref().unwrap().sim.units().iter().all(|u| {
                        u.x.to_int_floor() != x.to_int_floor()
                            || u.y.to_int_floor() != y.to_int_floor()
                    }))
                .then_some((x, y))
            })
        })
        .unwrap();
    let sim = &mut app.session.as_mut().unwrap().sim;
    let target = sim.next_id_peek();
    sim.step(&[
        Command::Spawn {
            kind: own.kind,
            x: destination.0,
            y: destination.1,
            loco: own.loco,
            speed: Fix::ZERO,
            hp: 1_000_000,
            armor: own.armor,
            weapon: None,
        },
        Command::AssignOwner {
            id: target,
            owner: yr_sim::OWNER_ENEMY,
        },
    ]);
    app.session
        .as_mut()
        .unwrap()
        .step(&[Command::Attack { id: own.id, target }]);
    assert_eq!(
        app.session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .find(|u| u.id == own.id)
            .unwrap()
            .attack,
        Some(target)
    );
    if let Some(out) = std::env::var_os("OPENYR_CURRENT_TARGET_PROOF") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        yr_app::snapshot::save(app.session.as_ref().unwrap(), &out.join("active.oyrsave")).unwrap();
        std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"own_id":own.id,"target_id":target,"own_ground":anchor(&app,own.id),"target_ground":anchor(&app,target),"map_size":[app.map.as_ref().unwrap().width,app.map.as_ref().unwrap().height],"tick":app.session.as_ref().unwrap().sim.tick(),"hash":format!("{:016x}",app.session.as_ref().unwrap().sim.state_hash())})).unwrap()).unwrap();
    }
}

fn anchor(app: &App, id: u32) -> (f32, f32) {
    let sim = &app.session.as_ref().unwrap().sim;
    let (x, y) = sim
        .units()
        .iter()
        .find(|u| u.id == id)
        .map(|u| (u.x, u.y))
        .or_else(|| {
            sim.buildings()
                .iter()
                .find(|b| b.id == id)
                .map(|b| b.centre())
        })
        .unwrap();
    yr_app::interaction::ground_point(
        app.map.as_ref().unwrap(),
        sim.terrain(),
        (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
    )
}
fn center(app: &mut App, id: u32, size: (u32, u32)) {
    let (x, y) = anchor(app, id);
    app.view.cam_x = x - (size.0 - SIDEBAR_W) as f32 / (2.0 * app.view.zoom);
    app.view.cam_y = y - size.1 as f32 / (2.0 * app.view.zoom);
}
fn point_at(app: &mut App, id: u32, size: (u32, u32)) {
    center(app, id, size);
    app.pointer_moved((f64::from(size.0 - SIDEBAR_W) / 2.0, f64::from(size.1) / 2.0));
}
fn proof(app: &App, size: (u32, u32), name: &str) {
    if let Some(out) = std::env::var_os("OPENYR_CURRENT_TARGET_PROOF") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
        app.compose(&mut frame, size.0, size.1);
        std::fs::write(
            out.join(format!("{}x{}-{name}.png", size.0, size.1)),
            yr_render::png::encode_rgba(size.0, size.1, &frame),
        )
        .unwrap();
    }
}
