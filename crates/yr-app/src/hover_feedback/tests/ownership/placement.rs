use super::*;

fn ready(size: (u32, u32), zoom: f32) -> (App, u32, usize) {
    let (mut app, own, _) = scene(size);
    app.view.zoom = zoom;
    let session = app.session.as_mut().unwrap();
    let slot = session
        .buildables
        .iter()
        .position(|b| b.id == "GAPOWR")
        .unwrap();
    let produce = session.produce(slot).unwrap();
    session.sim.step(&[produce]);
    for _ in 0..10 {
        if session.ready_structure() == Some(slot) {
            break;
        }
        session.sim.step(&[]);
    }
    assert_eq!(session.ready_structure(), Some(slot));
    app.sidebar_view.select(session.buildables[slot].category);
    (app, own, slot)
}

fn pickup(app: &mut App, slot: usize, size: (u32, u32)) {
    let index = sidebar::slots(app.session.as_ref().unwrap(), app.sidebar_view.category)
        .position(|s| s == slot)
        .unwrap();
    let origin = sidebar::slot_origin(size.0, index);
    app.pointer_moved((f64::from(origin.0 + 10), f64::from(origin.1 + 10)));
    assert_eq!(app.sidebar_slot_at(app.cursor, size), Some(slot));
    click(app);
    assert_eq!(app.placing, Some(slot));
}

fn valid(app: &App, slot: usize) -> (u16, u16) {
    let session = app.session.as_ref().unwrap();
    let grid = session.sim.terrain().unwrap();
    (0..grid.height())
        .find_map(|y| {
            (0..grid.width())
                .map(|x| grid.cell_at_local(x, y))
                .find(|&cell| session.can_place(slot, cell))
        })
        .unwrap()
}

#[test]
fn placement_mode_pickup_and_completion_do_not_resurrect_attack_move() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.5, 1.0, 2.0] {
            for finish in 0..3 {
                let (mut app, own, slot) = ready(size, zoom);
                let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                let hash = app.session.as_ref().unwrap().sim.state_hash();
                let queued = Command::Stop { id: own };
                app.pending.push(queued);
                key(&mut app, KeyCode::KeyA, ElementState::Pressed);
                assert!(app.attack_move);
                pickup(&mut app, slot, size);
                assert!(!app.attack_move);
                assert_eq!(app.pending, [queued]);
                let mut expected = vec![queued];
                match finish {
                    0 => {
                        let origin = valid(&app, slot);
                        let command = app.session.as_ref().unwrap().place(slot, origin).unwrap();
                        point_cell(
                            &mut app,
                            (f32::from(origin.0) + 0.5, f32::from(origin.1) + 0.5),
                            size,
                        );
                        assert_eq!(app.placement_target(), Some((slot, origin, true)));
                        click(&mut app);
                        expected.push(command);
                    }
                    1 => {
                        point_cell(&mut app, (12.0, 12.0), size);
                        app.pointer_button(ElementState::Pressed, MouseButton::Right);
                        app.pointer_button(ElementState::Released, MouseButton::Right);
                    }
                    _ => {
                        let occupied = app.session.as_ref().unwrap().sim.buildings()[0].cell;
                        point_cell(
                            &mut app,
                            (f32::from(occupied.0) + 0.5, f32::from(occupied.1) + 0.5),
                            size,
                        );
                        assert_eq!(app.placement_target(), Some((slot, occupied, false)));
                        click(&mut app);
                        assert_eq!(app.placing, Some(slot));
                        assert_eq!(app.pending, expected);
                        key(&mut app, KeyCode::Escape, ElementState::Pressed);
                    }
                }
                assert!(app.placing.is_none() && !app.attack_move);
                point_cell(&mut app, (12.0, 12.0), size);
                assert_eq!(app.cursor_kind(size), Kind::Move);
                click(&mut app);
                let (x, y) = App::cell_leptons((12.0, 12.0));
                expected.push(Command::Move { id: own, x, y });
                assert_eq!(app.pending, expected);
                assert_eq!(app.selected, [own].into());
                assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    saved
                );
            }
        }
    }
    println!("Ready-cameo pickup, success/cancel/invalid recovery and ordinary Move pass at 27 layout/zoom/outcome combinations; saves, selection and prior intents unchanged.");
}

#[test]
fn placement_mode_new_attack_move_replaces_placement_but_production_does_not() {
    let size = (640, 480);
    let (mut app, own, slot) = ready(size, 1.0);
    pickup(&mut app, slot, size);
    assert!(!app.attack_move);
    key(&mut app, KeyCode::KeyA, ElementState::Pressed);
    key(&mut app, KeyCode::KeyA, ElementState::Released);
    assert!(app.attack_move && app.placing.is_none());
    point_cell(&mut app, (12.0, 12.0), size);
    click(&mut app);
    let (x, y) = App::cell_leptons((12.0, 12.0));
    assert_eq!(app.pending, [Command::AttackMove { id: own, x, y }]);
    assert!(!app.attack_move);
    key(&mut app, KeyCode::KeyA, ElementState::Pressed);
    let unit_slot = app
        .session
        .as_ref()
        .unwrap()
        .buildables
        .iter()
        .position(|b| b.id == "MTNK")
        .unwrap();
    app.sidebar_view
        .select(app.session.as_ref().unwrap().buildables[unit_slot].category);
    let index = sidebar::slots(app.session.as_ref().unwrap(), app.sidebar_view.category)
        .position(|s| s == unit_slot)
        .unwrap();
    let (x, y) = sidebar::slot_origin(size.0, index);
    app.pointer_moved((f64::from(x + 10), f64::from(y + 10)));
    assert_eq!(app.sidebar_slot_at(app.cursor, size), Some(unit_slot));
    click(&mut app);
    assert!(app.attack_move && app.placing.is_none());
    assert!(matches!(app.pending.last(), Some(Command::Produce { .. })));
    println!("A fresh deliberate A replaces placement and queues one AttackMove; an ordinary unit-production click keeps the armed mode.");
}

#[test]
fn placement_mode_native_compositor_proof() {
    let size = (640, 480);
    let (mut app, own, slot) = ready(size, 1.0);
    app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
        .into_iter()
        .cycle()
        .take(1600 * 1200 * 4)
        .collect();
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    point_cell(&mut app, (12.0, 12.0), size);
    key(&mut app, KeyCode::KeyA, ElementState::Pressed);
    assert_eq!(app.cursor_kind(size), Kind::AttackMove);
    capture(&app, "00-attack-move-armed");
    pickup(&mut app, slot, size);
    let origin = valid(&app, slot);
    let command = app.session.as_ref().unwrap().place(slot, origin).unwrap();
    point_cell(
        &mut app,
        (f32::from(origin.0) + 0.5, f32::from(origin.1) + 0.5),
        size,
    );
    assert!(!app.attack_move);
    assert_eq!(app.placement_target(), Some((slot, origin, true)));
    capture(&app, "01-ready-placement");
    click(&mut app);
    assert_eq!(app.pending, [command]);
    assert!(app.placing.is_none() && !app.attack_move);
    capture(&app, "02-placement-intent");
    point_cell(&mut app, (12.0, 12.0), size);
    assert_eq!(app.cursor_kind(size), Kind::Move);
    click(&mut app);
    let (x, y) = App::cell_leptons((12.0, 12.0));
    assert_eq!(app.pending, [command, Command::Move { id: own, x, y }]);
    capture(&app, "03-ordinary-move-intent");
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
}

fn capture(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_PLACEMENT_MODE_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(640, 480, &frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "cursor_kind":format!("{:?}", app.cursor_kind((640,480))),"attack_move":app.attack_move,
            "placing":app.placing,"pending":format!("{:?}",app.pending),"selected":app.selected,
            "tick":session.sim.tick(),"sim_hash":format!("{:016x}",session.sim.state_hash()),
            "snapshot_bytes":yr_app::snapshot::encode(session).unwrap().len(),"retail_art":false,
        }))
        .unwrap(),
    )
    .unwrap();
}
