use super::*;
use type_selection::spawn;

fn press_n(app: &mut App) {
    key(app, KeyCode::KeyN, ElementState::Pressed);
    key(app, KeyCode::KeyN, ElementState::Released);
}
fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}
fn actors(size: (u32, u32)) -> (App, u32, u32, u32, u32, u32) {
    let (mut app, own, enemy) = scene(size);
    let buddy = spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
    let far = spawn(&mut app, 2, (31, 31), yr_sim::OWNER_PLAYER);
    let neutral = spawn(&mut app, 0, (6, 4), yr_sim::OWNER_NEUTRAL);
    let dead = spawn(&mut app, 0, (7, 4), yr_sim::OWNER_PLAYER);
    app.session.as_mut().unwrap().sim.step(&[
        Command::AssignRole {
            id: far,
            role: yr_sim::Role::Harvester,
        },
        Command::Destroy { id: dead },
    ]);
    (app, own, buddy, far, enemy, neutral)
}

#[test]
fn next_unit_cycles_creation_order_filters_and_wraps_without_simulation_changes() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            let (mut app, own, buddy, far, enemy, neutral) = actors(size);
            app.view.zoom = zoom;
            app.selected.clear();
            app.pending.push(Command::Stop { id: own });
            if zoom == 1.0 {
                app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                    .into_iter()
                    .cycle()
                    .take(1600 * 1200 * 4)
                    .collect();
            }
            let before = saved(&app);
            for (step, id) in [own, buddy, far, own].into_iter().enumerate() {
                press_n(&mut app);
                assert_eq!(app.selected, BTreeSet::from([id]));
                assert_eq!(
                    app.order_feedback.message(Instant::now()),
                    Some(format!("Next unit ({}/3)", step % 3 + 1).as_str())
                );
                assert_eq!(app.view.zoom, zoom);
                let camera = (app.view.cam_x, app.view.cam_y);
                app.bound_camera(size);
                assert_eq!(
                    (app.view.cam_x, app.view.cam_y),
                    camera,
                    "N already applies existing bounds"
                );
                if size == (640, 480) && zoom == 1.0 {
                    let expected = if id == own {
                        (364.0, 80.0)
                    } else if id == buddy {
                        (394.0, 95.0)
                    } else {
                        (364.0, 720.0)
                    };
                    assert_eq!(camera, expected, "known projected centers and bottom clamp");
                }
                assert_eq!(app.pending, [Command::Stop { id: own }]);
                assert_eq!(saved(&app), before);
                if zoom == 1.0 {
                    proof(&app, &format!("{}-{step}", size.0));
                }
            }
            app.selected = BTreeSet::from([own, far, enemy, neutral, u32::MAX]);
            press_n(&mut app);
            assert_eq!(
                app.selected,
                BTreeSet::from([own]),
                "mixed selection advances after its last eligible id and wraps"
            );
            let building = app.session.as_ref().unwrap().sim.buildings()[0].id;
            app.selected = BTreeSet::from([enemy, neutral, building, u32::MAX]);
            press_n(&mut app);
            assert_eq!(
                app.selected,
                BTreeSet::from([own]),
                "ineligible seeds start at first owned unit"
            );
            assert_eq!(saved(&app), before);
        }
    }
    println!("Nine layout/zoom cases cycle all living owned types, including an offscreen harvester, in creation order with wrap/mixed/invalid seeds; exact camera cases, bounds, zoom, pending Stop and Session bytes are verified.");
}

#[test]
fn next_unit_cancels_old_gestures_modes_and_timing_but_keeps_ready_work_and_teams() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for mode in 0..4 {
            let (mut app, own, slot) = placement::ready(size, 1.0);
            let buddy = spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
            app.control = true;
            key(&mut app, KeyCode::Digit1, ElementState::Pressed);
            app.control = false;
            assert!(app.type_select_key(
                KeyCode::KeyT,
                ElementState::Pressed,
                false,
                Instant::now()
            ));
            assert!(app.type_press.is_some());
            match mode {
                0 => app.placing = Some(slot),
                1 => app.service_mode = ServiceMode::Repair,
                2 => app.service_mode = ServiceMode::Sell,
                _ => app.attack_move = true,
            }
            app.left_down = Some((100.0, 100.0));
            app.right_down = Some((110.0, 110.0));
            app.right_panning = true;
            app.radar_drag = true;
            app.camera.edge_armed = true;
            app.pending.push(Command::Stop { id: own });
            let before = saved(&app);
            press_n(&mut app);
            assert_eq!(app.selected, BTreeSet::from([buddy]));
            assert!(
                app.left_down.is_none()
                    && app.right_down.is_none()
                    && !app.right_panning
                    && !app.radar_drag
                    && !app.camera.edge_armed
            );
            assert!(
                !app.attack_move && app.placing.is_none() && app.service_mode == ServiceMode::None
            );
            assert!(app.type_press.is_none() && app.groups.message(Instant::now()).is_none());
            app.pointer_button(ElementState::Released, MouseButton::Left);
            app.pointer_button(ElementState::Released, MouseButton::Right);
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(app.session.as_ref().unwrap().ready_structure(), Some(slot));
            assert_eq!(saved(&app), before);
            app.selected.clear();
            key(&mut app, KeyCode::Digit1, ElementState::Pressed);
            assert_eq!(
                app.selected,
                BTreeSet::from([own]),
                "assigned team survives navigation"
            );
            assert_eq!(saved(&app), before);
        }
    }
    println!("Twelve layout/mode cases cancel stale pointer/command and type/team timing state, reject stale releases and preserve READY work, assigned teams, queued orders and encoded Sessions.");
}

#[test]
fn next_unit_guards_preserve_selection_camera_and_menu_text() {
    for guard in 0..12 {
        let (mut app, own, _, _, _, _) = actors((640, 480));
        match guard {
            2 => app.pointer.focused = false,
            3 => app.shift = true,
            4 => app.control = true,
            5 => app.super_key = true,
            6 => app.session.as_mut().unwrap().outcome = Outcome::Won,
            7 => app.session.as_mut().unwrap().outcome = Outcome::Lost,
            8 => app.map = None,
            9 => app.session = None,
            10 => app.menu.set(Page::Pause),
            _ => {}
        }
        let before = app.session.as_ref().map(|_| saved(&app));
        let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyN),
            if guard == 11 {
                ElementState::Released
            } else {
                ElementState::Pressed
            },
            None,
            guard == 0,
            guard == 1,
        );
        assert_eq!(app.selected, BTreeSet::from([own]), "guard {guard}");
        assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
        assert!(app.pending.is_empty());
        assert_eq!(app.session.as_ref().map(|_| saved(&app)), before);
    }
    let (mut app, _, _) = scene((640, 480));
    app.menu.set(Page::Save);
    app.menu.name.clear();
    let selected = app.selected.clone();
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::KeyN),
        ElementState::Pressed,
        Some("n"),
        false,
        false,
    );
    assert_eq!(app.menu.name, "n");
    assert_eq!(app.selected, selected);
    println!("Twelve release/repeat/focus/synthetic/modifier/missing/ended/menu guards preserve selection, camera and Session bytes; Save editor still receives N text.");
}

#[test]
fn next_unit_handles_singleton_and_empty_world_without_extra_state() {
    let (mut app, own, _) = scene((640, 480));
    let before = saved(&app);
    for _ in 0..3 {
        press_n(&mut app);
        assert_eq!(app.selected, BTreeSet::from([own]));
        assert_eq!(saved(&app), before);
    }
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Next unit (1/1)")
    );
    app.session
        .as_mut()
        .unwrap()
        .sim
        .step(&[Command::Destroy { id: own }]);
    let before = saved(&app);
    let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
    press_n(&mut app);
    assert!(app.selected.is_empty());
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("No units available")
    );
    assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
    assert_eq!(saved(&app), before);
    println!("Singleton cycles remain stable; an empty eligible world clears stale selection, keeps camera and reports bounded empty feedback without changing Session bytes.");
}

fn proof(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_NEXT_UNIT_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let size = app.camera_size();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose_at(&mut frame, size.0, size.1, Instant::now());
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    let s = app.session.as_ref().unwrap();
    std::fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
        "selected":app.selected,"camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],
        "notice":app.order_feedback.message(Instant::now()),"pending":format!("{:?}",app.pending),
        "tick":s.sim.tick(),"sim_hash":format!("{:016x}",s.sim.state_hash()),"snapshot_bytes":saved(app).len(),"retail_art":false
    })).unwrap()).unwrap();
}
