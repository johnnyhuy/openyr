use super::*;
use type_selection::spawn;

fn f(app: &mut App) {
    key(app, KeyCode::KeyF, ElementState::Pressed);
    key(app, KeyCode::KeyF, ElementState::Released);
}
fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}
fn canvas(size: (u32, u32)) -> (App, u32, u32) {
    let (mut app, own, enemy) = scene(size);
    app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
        .into_iter()
        .cycle()
        .take(1600 * 1200 * 4)
        .collect();
    let map = app.map.as_mut().unwrap();
    for y in 0..1200 {
        for x in 0..1600 {
            if x % 60 == 0 || y % 30 == 0 {
                let at = (y * 1600 + x) * 4;
                map.rgba[at..at + 4].copy_from_slice(&[45, 55, 45, 255]);
            }
        }
    }
    app.pointer_moved((200.0, 200.0));
    (app, own, enemy)
}
fn assert_centered(app: &App, id: u32, size: (u32, u32)) {
    let sim = &app.session.as_ref().unwrap().sim;
    let u = sim.units().iter().find(|u| u.id == id).unwrap();
    let map = app.map.as_ref().unwrap();
    let (x, y) = yr_app::interaction::ground_point(
        map,
        sim.terrain(),
        (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0),
    );
    let clamp = |p: f32, extent: u32, pixels: u32| {
        let spare = extent as f32 - pixels as f32 / app.view.zoom;
        if spare <= 0.0 {
            spare * 0.5
        } else {
            p.clamp(0.0, spare)
        }
    };
    let expected = (
        clamp(
            x - (size.0 - SIDEBAR_W) as f32 / (2.0 * app.view.zoom),
            map.width,
            size.0 - SIDEBAR_W,
        ),
        clamp(
            y - size.1 as f32 / (2.0 * app.view.zoom),
            map.height,
            size.1,
        ),
    );
    assert!(
        (app.view.cam_x - expected.0).abs() < 0.01 && (app.view.cam_y - expected.1).abs() < 0.01
    );
}

#[test]
fn unit_follow_tracks_moving_subcell_ground_positions_without_changing_saved_state() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            let (mut app, own, enemy) = canvas(size);
            app.view.zoom = zoom;
            // A flat elevated grid permits movement while checking ground lift.
            let sim = &mut app.session.as_mut().unwrap().sim;
            let mut grid = sim.terrain().unwrap().clone();
            for y in 0..32 {
                for x in 0..32 {
                    grid.cell_mut(x, y).unwrap().height = 4;
                }
            }
            sim.set_terrain(grid);
            app.selected.insert(enemy);
            app.pending = vec![Command::Stop { id: own }];
            let selection = app.selected.clone();
            let before = saved(&app);
            f(&mut app);
            assert_eq!(app.camera.follow, Some(own));
            assert_eq!(
                app.order_feedback.message(Instant::now()),
                Some("Following unit")
            );
            assert_eq!(saved(&app), before);
            assert_centered(&app, own, size);
            let now = Instant::now();
            for step in 0..=12 {
                if step > 0 {
                    let commands = if step == 1 {
                        vec![Command::Move {
                            id: own,
                            x: Fix::from_int(4) + Fix::from_raw(128),
                            y: Fix::from_int(12) + Fix::from_raw(128),
                        }]
                    } else {
                        vec![]
                    };
                    app.session.as_mut().unwrap().sim.step(&commands);
                }
                let before = saved(&app);
                app.update_camera(now + Duration::from_millis(step * 16), size);
                assert_eq!(app.camera.follow, Some(own));
                assert_centered(&app, own, size);
                assert_eq!(app.view.zoom, zoom);
                assert_eq!(app.selected, selection);
                assert_eq!(app.pending, [Command::Stop { id: own }]);
                assert_eq!(saved(&app), before);
                if zoom == 1.0 && step % 3 == 0 {
                    capture(&app, size, step);
                }
            }
            let u = app
                .session
                .as_ref()
                .unwrap()
                .sim
                .units()
                .iter()
                .find(|u| u.id == own)
                .unwrap();
            assert_ne!(
                (u.x.raw(), u.y.raw()),
                (1024, 1024),
                "the fixture must actually move"
            );
            assert!(
                u.x.raw() % 256 != 0 || u.y.raw() % 256 != 0,
                "keep a subcell coordinate"
            );
            // A stationary update need not request a new composition.
            app.view.dirty = false;
            app.update_camera(now + Duration::from_secs(1), size);
            assert!(!app.view.dirty);
        }
    }
    println!("Nine layout/zoom sequences follow authoritative moving subcell ground anchors through thirteen redraw updates, with terrain lift, raster/oversized bounds and exact Session/selection/zoom/pending-order preservation.");
}

#[test]
fn unit_follow_stops_for_manual_input_context_and_invalid_targets() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for action in 0..20 {
            let (mut app, own, _) = canvas(size);
            app.control = true;
            key(&mut app, KeyCode::Digit1, ElementState::Pressed);
            app.control = false;
            app.pending = vec![Command::Stop { id: own }];
            f(&mut app);
            assert_eq!(app.camera.follow, Some(own));
            match action {
                0 => f(&mut app),
                1 => key(&mut app, KeyCode::ArrowRight, ElementState::Pressed),
                2 => {
                    app.menu.edge_scroll = true;
                    app.pointer_moved((1.0, 240.0));
                }
                3 => app.zoom_camera(2.0),
                4 => app.pointer_button(ElementState::Pressed, MouseButton::Left),
                5 => app.pointer_button(ElementState::Pressed, MouseButton::Right),
                6 => key(&mut app, KeyCode::KeyH, ElementState::Pressed),
                7 => key(&mut app, KeyCode::KeyN, ElementState::Pressed),
                8 => key(&mut app, KeyCode::Digit1, ElementState::Pressed),
                9 => app.focus_changed(false),
                10 => app.invalidate_pointer(),
                11 => app.open_menu(Page::Pause),
                12 => {
                    app.resize_window(size, 1.0).unwrap();
                }
                13 => app.map = None,
                14 => app.session = None,
                15 => app.session.as_mut().unwrap().outcome = Outcome::Won,
                16 => app
                    .session
                    .as_mut()
                    .unwrap()
                    .sim
                    .step(&[Command::Destroy { id: own }]),
                17 => app
                    .session
                    .as_mut()
                    .unwrap()
                    .sim
                    .step(&[Command::AssignOwner {
                        id: own,
                        owner: yr_sim::OWNER_ENEMY,
                    }]),
                18 => app.selected.clear(),
                _ => app.center_world_at((12.0, 12.0), (size.0 - SIDEBAR_W, size.1)),
            }
            let before = app.session.as_ref().map(|_| saved(&app));
            app.update_camera(Instant::now(), size);
            assert_eq!(app.camera.follow, None, "action {action} at {size:?}");
            app.update_camera(Instant::now() + Duration::from_millis(20), size);
            assert_eq!(app.camera.follow, None);
            assert_eq!(app.session.as_ref().map(|_| saved(&app)), before);
            assert_eq!(app.pending, [Command::Stop { id: own }]);
        }
    }
    println!("Sixty cancellation cases cover manual pan/edge/zoom/gestures/navigation, focus/leave/menu/resize, absent/ended matches and dead/unowned/deselected targets without follow resuming.");
}

#[test]
fn unit_follow_retains_keyboard_guards_and_filters_selection_without_replacing_it() {
    for guard in 0..11 {
        let (mut app, _, _) = canvas((640, 480));
        let (mut repeat, mut synthetic) = (false, false);
        match guard {
            0 => repeat = true,
            1 => synthetic = true,
            2 => app.focus_changed(false),
            3 => app.control = true,
            4 => app.shift = true,
            5 => app.super_key = true,
            6 => app.menu.set(Page::Save),
            7 => app.map = None,
            8 => app.session = None,
            9 => app.session.as_mut().unwrap().outcome = Outcome::Lost,
            _ => {
                key(&mut app, KeyCode::ArrowRight, ElementState::Pressed);
            }
        }
        let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
        let selected = app.selected.clone();
        let before = app.session.as_ref().map(|_| saved(&app));
        app.menu.name = "Keep".into();
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyF),
            ElementState::Pressed,
            Some("f"),
            repeat,
            synthetic,
        );
        assert_eq!(app.camera.follow, None);
        assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
        assert_eq!(app.selected, selected);
        assert_eq!(app.session.as_ref().map(|_| saved(&app)), before);
        if guard == 6 {
            assert_eq!(app.menu.name, "Keepf");
        }
    }
    let (mut app, own, enemy) = canvas((640, 480));
    let buddy = spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
    let neutral = spawn(&mut app, 0, (6, 4), yr_sim::OWNER_NEUTRAL);
    let building = app.session.as_ref().unwrap().sim.buildings()[0].id;
    let selection = BTreeSet::from([own, buddy, enemy, neutral, building, u32::MAX]);
    app.selected = selection.clone();
    let before = saved(&app);
    f(&mut app);
    assert_eq!(app.camera.follow, Some(own));
    assert_eq!(app.selected, selection);
    f(&mut app);
    assert_eq!(app.camera.follow, None);
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Follow stopped")
    );
    app.selected = [enemy, neutral, building, u32::MAX].into();
    f(&mut app);
    assert_eq!(app.camera.follow, None);
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Select a unit to follow")
    );
    assert_eq!(saved(&app), before);
    println!("Eleven keyboard guards retain menu text editing; mixed selection chooses the lowest living owned unit without replacing selection, while ineligible selection reports no target.");
}

#[test]
fn unit_follow_cleans_old_modes_and_keeps_ready_work_orders_and_teams() {
    let size = (640, 480);
    for mode in 0..4 {
        let (mut app, own, slot) = placement::ready(size, 1.0);
        app.control = true;
        key(&mut app, KeyCode::Digit1, ElementState::Pressed);
        app.control = false;
        app.left_down = Some((100.0, 100.0));
        app.right_down = Some((110.0, 110.0));
        app.right_panning = true;
        app.radar_drag = true;
        app.camera.edge_armed = true;
        match mode {
            0 => app.placing = Some(slot),
            1 => app.service_mode = ServiceMode::Repair,
            2 => app.service_mode = ServiceMode::Sell,
            _ => app.attack_move = true,
        }
        app.pending = vec![Command::Stop { id: own }];
        let before = saved(&app);
        f(&mut app);
        assert_eq!(app.camera.follow, Some(own));
        assert!(
            app.left_down.is_none()
                && app.right_down.is_none()
                && !app.right_panning
                && !app.radar_drag
                && !app.camera.edge_armed
        );
        assert!(app.placing.is_none() && app.service_mode == ServiceMode::None && !app.attack_move);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        app.pointer_button(ElementState::Released, MouseButton::Right);
        assert_eq!(app.selected, [own].into());
        assert_eq!(app.pending, [Command::Stop { id: own }]);
        assert_eq!(app.session.as_ref().unwrap().ready_structure(), Some(slot));
        assert_eq!(saved(&app), before);
        app.selected.clear();
        key(&mut app, KeyCode::Digit1, ElementState::Pressed);
        assert_eq!(app.selected, [own].into());
        assert_eq!(app.camera.follow, None);
        assert_eq!(saved(&app), before);
    }
    let (mut app, own, _) = canvas(size);
    let session = app.session.as_ref().unwrap();
    let slot = session
        .buildables
        .iter()
        .position(|b| b.id == "GAPOWR")
        .unwrap();
    app.sidebar_view.select(session.buildables[slot].category);
    let index = sidebar::slots(session, app.sidebar_view.category)
        .position(|s| s == slot)
        .unwrap();
    let origin = sidebar::slot_origin(size.0, index);
    app.pending = vec![Command::Stop { id: own }];
    let command = session.production_click(slot, false, &app.pending).unwrap();
    let before = saved(&app);
    f(&mut app);
    app.pointer_moved((f64::from(origin.0 + 10), f64::from(origin.1 + 10)));
    click(&mut app);
    assert_eq!(app.pending, [Command::Stop { id: own }, command]);
    assert_eq!(app.camera.follow, Some(own));
    assert_eq!(saved(&app), before);
    let (mut app, own, slot) = placement::ready(size, 1.0);
    f(&mut app);
    assert_eq!(app.camera.follow, Some(own));
    let session = app.session.as_ref().unwrap();
    let index = sidebar::slots(session, app.sidebar_view.category)
        .position(|s| s == slot)
        .unwrap();
    let origin = sidebar::slot_origin(size.0, index);
    let before = saved(&app);
    app.pointer_moved((f64::from(origin.0 + 10), f64::from(origin.1 + 10)));
    click(&mut app);
    assert_eq!(app.placing, Some(slot));
    assert_eq!(app.camera.follow, None);
    assert!(app.pending.is_empty());
    assert_eq!(saved(&app), before);
    println!("Four READY/mode cases preserve teams, production, queued Stop and Session bytes; actual sidebar production stays usable during follow, while READY placement pickup stops follow.");
}

fn capture(app: &App, size: (u32, u32), step: u64) {
    let Some(dir) = std::env::var_os("OPENYR_UNIT_FOLLOW_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut frame, size.0, size.1);
    let name = format!("{}-{step}", size.0);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    let s = app.session.as_ref().unwrap();
    let u = s
        .sim
        .units()
        .iter()
        .find(|u| Some(u.id) == app.camera.follow)
        .unwrap();
    std::fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
        "follow":app.camera.follow,"cell_raw":[u.x.raw(),u.y.raw()],"camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],"tick":s.sim.tick(),
        "sim_hash":format!("{:016x}",s.sim.state_hash()),"snapshot_bytes":saved(app).len(),"selected":app.selected,"pending":format!("{:?}",app.pending),"retail_art":false,
    })).unwrap()).unwrap();
}
