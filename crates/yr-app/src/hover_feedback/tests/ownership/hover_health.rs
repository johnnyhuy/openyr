use super::*;

fn health_pixels(app: &App, id: u32, size: (u32, u32)) -> (Vec<u8>, Vec<usize>) {
    let session = app.session.as_ref().unwrap();
    let map = app.map.as_ref().unwrap();
    let (cell, half, fraction) = session
        .sim
        .units()
        .iter()
        .find(|u| u.id == id)
        .map(|u| {
            (
                (u.x, u.y),
                (14.0 * app.view.zoom).max(5.0),
                hp_frac(u.hp, u.max_hp),
            )
        })
        .or_else(|| {
            session
                .sim
                .buildings()
                .iter()
                .find(|b| b.id == id)
                .map(|b| {
                    (
                        b.centre(),
                        ((f32::from(b.size.0) + f32::from(b.size.1)) * 15.0
                            / map.scale.max(1) as f32
                            * app.view.zoom)
                            .max(8.0),
                        hp_frac(b.hp, b.max_hp),
                    )
                })
        })
        .unwrap();
    let ground = yr_app::interaction::ground_point(
        map,
        session.sim.terrain(),
        (cell.0.raw() as f32 / 256.0, cell.1.raw() as f32 / 256.0),
    );
    let center = (
        (ground.0 - app.view.cam_x) * app.view.zoom,
        (ground.1 - app.view.cam_y) * app.view.zoom,
    );
    let mut expected = vec![0; (size.0 * size.1 * 4) as usize];
    crate::unit_status::draw_health(
        &mut expected,
        size,
        (center.0, center.1 - (half.max(8.0) + 6.0) * 0.5 - 6.0),
        half * 2.0,
        fraction,
    );
    let indices = expected
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter_map(|(i, p)| (p[3] != 0).then_some(i * 4))
        .collect();
    (expected, indices)
}

fn assert_bar(app: &App, id: u32, size: (u32, u32), visible: bool) {
    let (expected, indices) = health_pixels(app, id, size);
    assert!(!indices.is_empty());
    let mut frame = vec![0; expected.len()];
    app.compose(&mut frame, size.0, size.1);
    let matches = indices
        .iter()
        .all(|&i| frame[i..i + 4] == expected[i..i + 4]);
    assert_eq!(matches, visible, "health visibility for {id} at {size:?}");
}

fn canvas(size: (u32, u32)) -> (App, u32, u32) {
    let (mut app, own, enemy) = scene(size);
    app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
        .into_iter()
        .cycle()
        .take(1600 * 1200 * 4)
        .collect();
    (app, own, enemy)
}

#[test]
fn hover_health_shows_healthy_selection_attack_and_structure_previews() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.5, 1.0, 2.0] {
            let (mut app, own, enemy) = canvas(size);
            app.view.zoom = zoom;
            let building = app.session.as_ref().unwrap().sim.buildings()[0].id;
            let enemy_building = app.session.as_ref().unwrap().sim.next_id_peek();
            app.session
                .as_mut()
                .unwrap()
                .sim
                .step(&[Command::PlaceBuilding {
                    kind: 0,
                    x: 12,
                    y: 12,
                    w: 2,
                    h: 2,
                    hp: 100,
                    armor: 0,
                    power: 0,
                    owner: yr_sim::OWNER_ENEMY,
                    role: yr_sim::Role::Combat,
                }]);
            for mode in 0..5 {
                app.selected = if mode == 1 || mode == 4 {
                    [own].into()
                } else {
                    BTreeSet::new()
                };
                app.service_mode = match mode {
                    2 => ServiceMode::Sell,
                    3 => ServiceMode::Repair,
                    _ => ServiceMode::None,
                };
                let id = match mode {
                    0 => own,
                    1 => enemy,
                    4 => enemy_building,
                    _ => building,
                };
                point_at(&mut app, id, size);
                let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                let selected = app.selected.clone();
                let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
                app.pending = vec![Command::Stop { id: own }];
                assert_eq!(app.hover_target(size).map(|t| t.id), Some(id));
                assert_bar(&app, id, size, true);
                if zoom == 1.0 {
                    capture(&app, size, &format!("{mode}-hover"), id);
                }
                app.pointer_moved((app.cursor.0 + 90.0, app.cursor.1 + 90.0));
                assert_bar(&app, id, size, false);
                if zoom == 1.0 {
                    capture(&app, size, &format!("{mode}-away"), id);
                }
                assert_eq!(app.selected, selected);
                assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
                assert_eq!(app.pending, [Command::Stop { id: own }]);
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    before
                );
            }
        }
    }
    println!("45 layout/zoom/target cases show exact existing health pips on resolved healthy hover targets and remove them on leave, preserving selection/camera/orders/Session bytes.");
}

#[test]
fn hover_health_respects_input_ownership_and_keeps_prior_visibility() {
    let size = (640, 480);
    for guard in 0..13 {
        let (mut app, own, enemy) = canvas(size);
        point_at(&mut app, enemy, size);
        match guard {
            0 => app.pointer_button(ElementState::Pressed, MouseButton::Left),
            1 => app.pointer_button(ElementState::Pressed, MouseButton::Right),
            2 => app.radar_drag = true,
            3 => key(&mut app, KeyCode::ArrowRight, ElementState::Pressed),
            4 => app.placing = Some(0),
            5 => app.menu.set(Page::Pause),
            6 => app.focus_changed(false),
            7 => app.invalidate_pointer(),
            8 => app.shift = true,
            9 => app.selected.clear(),
            10 => app.session.as_mut().unwrap().outcome = Outcome::Won,
            11 => app.pointer_moved((1.0, 240.0)),
            _ => app.pointer_moved((639.0, 240.0)),
        }
        let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        assert!(app.hover_target(size).is_none());
        assert_bar(&app, enemy, size, false);
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            before
        );
        assert!(app.pending.is_empty());
        assert!(app.selected.is_empty() || app.selected == [own].into());
    }
    let (mut app, own, enemy) = canvas(size);
    app.session.as_mut().unwrap().sim.step(&[Command::Attack {
        id: own,
        target: enemy,
    }]);
    point_at(&mut app, enemy, size);
    app.pointer_moved((app.cursor.0 + 90.0, app.cursor.1 + 90.0));
    assert!(app.current_attack_targets().contains(&enemy));
    assert_bar(&app, enemy, size, false);
    point_at(&mut app, own, size);
    app.pointer_moved((app.cursor.0 + 90.0, app.cursor.1 + 90.0));
    assert_bar(&app, own, size, true);
    app.selected.clear();
    let sim = &mut app.session.as_mut().unwrap().sim;
    let bytes = sim.save_bytes().unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
    for unit in json["units"].as_array_mut().unwrap() {
        if unit["id"] == enemy {
            unit["hp"] = 70.into();
        }
    }
    let mut bytes = bytes[..12].to_vec();
    bytes.extend(serde_json::to_vec(&json).unwrap());
    *sim = yr_sim::Sim::from_save_bytes(&bytes).unwrap();
    point_at(&mut app, enemy, size);
    app.pointer_moved((app.cursor.0 + 90.0, app.cursor.1 + 90.0));
    assert!(app.hover_target(size).is_none());
    assert_bar(&app, enemy, size, true);
    println!("Thirteen ownership/modal/focus/selection guards suppress extra healthy bars; selected and damaged bars remain visible without hover.");
}

fn capture(app: &App, size: (u32, u32), phase: &str, target: u32) {
    let Some(dir) = std::env::var_os("OPENYR_HOVER_HEALTH_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut frame, size.0, size.1);
    let name = format!("{}-{phase}", size.0);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(dir.join(format!("{name}.json")), serde_json::to_vec_pretty(&serde_json::json!({
        "target": target, "hover": app.hover_target(size).map(|t| t.id),
        "cursor_kind": format!("{:?}", app.cursor_kind(size)), "selected": app.selected,
        "pending": format!("{:?}", app.pending), "camera": [app.view.cam_x, app.view.cam_y, app.view.zoom],
        "tick": session.sim.tick(), "sim_hash": format!("{:016x}", session.sim.state_hash()),
        "snapshot_bytes": yr_app::snapshot::encode(session).unwrap().len(), "retail_art": false,
    })).unwrap()).unwrap();
}
