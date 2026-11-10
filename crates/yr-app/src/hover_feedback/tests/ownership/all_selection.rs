use super::*;
use type_selection::spawn;

fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}
fn press_p(app: &mut App) {
    key(app, KeyCode::KeyP, ElementState::Pressed);
    key(app, KeyCode::KeyP, ElementState::Released);
}

#[test]
fn select_all_covers_mixed_offscreen_and_harvester_units_without_changing_match() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            let (mut app, own, enemy) = scene(size);
            let buddy = spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
            let far = spawn(&mut app, 2, (31, 31), yr_sim::OWNER_PLAYER);
            let neutral = spawn(&mut app, 0, (4, 5), yr_sim::OWNER_NEUTRAL);
            let dead = spawn(&mut app, 0, (5, 5), yr_sim::OWNER_PLAYER);
            app.session.as_mut().unwrap().sim.step(&[
                Command::AssignRole {
                    id: far,
                    role: yr_sim::Role::Harvester,
                },
                Command::Destroy { id: dead },
            ]);
            let building = app
                .session
                .as_ref()
                .unwrap()
                .sim
                .buildings()
                .iter()
                .find(|b| b.owner == yr_sim::OWNER_PLAYER)
                .unwrap()
                .id;
            app.view.zoom = zoom;
            point_at(&mut app, own, size);
            app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                .into_iter()
                .cycle()
                .take(1600 * 1200 * 4)
                .collect();
            app.selected = BTreeSet::from([own, enemy, neutral, dead, building, u32::MAX]);
            app.pending.push(Command::Stop { id: own });
            let before = saved(&app);
            let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
            if zoom == 1.0 {
                proof(&app, &format!("{}-before", size.0));
            }
            press_p(&mut app);
            assert_eq!(app.selected, BTreeSet::from([own, buddy, far]));
            assert_eq!(
                app.order_feedback.message(Instant::now()),
                Some("Selected all (3 units)")
            );
            assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(saved(&app), before);
            if zoom == 1.0 {
                proof(&app, &format!("{}-after", size.0));
            }
            // No existing seed is needed, and repeated deliberate P is idempotent.
            app.selected.clear();
            press_p(&mut app);
            assert_eq!(app.selected, BTreeSet::from([own, buddy, far]));
            press_p(&mut app);
            assert_eq!(app.selected, BTreeSet::from([own, buddy, far]));
            assert_eq!(saved(&app), before);
        }
    }
    println!("Nine layout/zoom cases select mixed owned units and an offscreen harvester without a seed, prune enemy/neutral/dead/building/stale IDs, and preserve camera, pending orders and full Session bytes.");
}

#[test]
fn select_all_cleans_gestures_modes_and_timing_but_retains_ready_work_and_teams() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for mode in 0..4 {
            let (mut app, own, slot) = placement::ready(size, 1.0);
            app.control = true;
            key(&mut app, KeyCode::Digit1, ElementState::Pressed);
            app.control = false;
            key(&mut app, KeyCode::Digit1, ElementState::Pressed);
            assert!(app.groups.message(Instant::now()).is_some());
            if mode % 2 == 0 {
                assert!(app.type_select_key(
                    KeyCode::KeyT,
                    ElementState::Pressed,
                    false,
                    Instant::now()
                ));
                assert!(app.type_press.is_some());
            }
            match mode {
                0 => app.placing = Some(slot),
                1 => app.service_mode = ServiceMode::Repair,
                2 => app.service_mode = ServiceMode::Sell,
                _ => app.attack_move = true,
            }
            app.left_down = Some((100.0, 100.0));
            app.right_down = Some((100.0, 100.0));
            app.right_panning = true;
            app.radar_drag = true;
            app.camera.edge_armed = true;
            app.pending.push(Command::Stop { id: own });
            let before = saved(&app);
            let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
            press_p(&mut app);
            assert!(app.left_down.is_none() && app.right_down.is_none());
            assert!(!app.right_panning && !app.radar_drag && !app.camera.edge_armed);
            assert!(
                !app.attack_move && app.placing.is_none() && app.service_mode == ServiceMode::None
            );
            assert!(app.type_press.is_none());
            assert!(app.groups.message(Instant::now()).is_none());
            app.pointer_button(ElementState::Released, MouseButton::Left);
            app.pointer_button(ElementState::Released, MouseButton::Right);
            assert_eq!(app.selected, BTreeSet::from([own]));
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(app.session.as_ref().unwrap().ready_structure(), Some(slot));
            assert_eq!(saved(&app), before);
            key(&mut app, KeyCode::Digit1, ElementState::Pressed);
            assert_eq!(
                (app.view.cam_x, app.view.cam_y, app.view.zoom),
                camera,
                "P interrupts prior team recall timing"
            );
            app.selected.clear();
            key(&mut app, KeyCode::Digit1, ElementState::Pressed);
            assert_eq!(
                app.selected,
                BTreeSet::from([own]),
                "assigned team survives P"
            );
        }
    }
    println!("Twelve layout/mode cases cancel stale gestures and command modes, interrupt type/team timing, retain assigned teams and READY work, and preserve commands, camera and encoded Sessions.");
}

#[test]
fn select_all_guards_preserve_selection_and_menu_name_input() {
    for guard in 0..12 {
        let (mut app, own, _) = scene((640, 480));
        spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
        match guard {
            2 => app.pointer.focused = false,
            3 => app.control = true,
            4 => app.shift = true,
            5 => app.super_key = true,
            6 => app.session.as_mut().unwrap().outcome = Outcome::Won,
            7 => app.session.as_mut().unwrap().outcome = Outcome::Lost,
            8 => app.map = None,
            9 => app.session = None,
            10 => app.menu.set(Page::Pause),
            _ => {}
        }
        let before = app.session.as_ref().map(|_| saved(&app));
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyP),
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
        assert!(app.pending.is_empty());
        assert_eq!(app.session.as_ref().map(|_| saved(&app)), before);
    }
    let (mut app, _, _) = scene((640, 480));
    app.menu.set(Page::Save);
    app.menu.name.clear();
    let selected = app.selected.clone();
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::KeyP),
        ElementState::Pressed,
        Some("p"),
        false,
        false,
    );
    assert_eq!(app.menu.name, "p");
    assert_eq!(app.selected, selected);
    println!("Twelve release/repeat/focus/synthetic/modifier/menu/missing/ended guards preserve selection and Session bytes; Save editor still accepts P text.");
}

#[test]
fn select_all_handles_empty_and_single_unit_feedback() {
    let (mut app, own, _) = scene((640, 480));
    press_p(&mut app);
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Selected all (1 unit)")
    );
    app.session
        .as_mut()
        .unwrap()
        .sim
        .step(&[Command::Destroy { id: own }]);
    let before = saved(&app);
    press_p(&mut app);
    assert!(app.selected.is_empty());
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("No units available")
    );
    assert_eq!(saved(&app), before);
    println!(
        "Single-unit and empty-world selection have bounded feedback and no simulation mutation."
    );
}

fn proof(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_ALL_SELECTION_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let size = app.camera_size();
    let mut pixels = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose_at(&mut pixels, size.0, size.1, Instant::now());
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &pixels),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(dir.join(format!("{name}.json")), serde_json::to_vec_pretty(&serde_json::json!({
        "selected":app.selected,"camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],
        "pending":format!("{:?}",app.pending),"tick":session.sim.tick(),
        "sim_hash":format!("{:016x}",session.sim.state_hash()),"snapshot_bytes":saved(app).len(),
        "notice":app.order_feedback.message(Instant::now()),"retail_art":false
    })).unwrap()).unwrap();
}
