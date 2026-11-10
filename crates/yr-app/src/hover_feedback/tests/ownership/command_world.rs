use super::*;

fn event(app: &mut App, code: KeyCode, repeat: bool) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        None,
        repeat,
        false,
    );
}
fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}

#[test]
fn command_world_chords_preserve_orders_modes_and_pointer_gestures() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for chord in 0..4 {
            for code in [
                KeyCode::KeyS,
                KeyCode::KeyA,
                KeyCode::KeyK,
                KeyCode::KeyL,
                KeyCode::Escape,
            ] {
                let (mut app, own, _) = scene(size);
                point_at(&mut app, own, size);
                if chord == 0 && code == KeyCode::KeyS {
                    app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                        .into_iter()
                        .cycle()
                        .take(1600 * 1200 * 4)
                        .collect();
                }
                app.pending.push(Command::Move {
                    id: own,
                    x: Fix::from_int(12),
                    y: Fix::from_int(4),
                });
                let pending = app.pending.clone();
                app.attack_move = true;
                app.left_down = Some((100.0, 100.0));
                let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
                let before = saved(&app);
                if chord == 0 && code == KeyCode::KeyS {
                    proof(&app, &format!("{}-before", size.0));
                }
                app.super_key = true;
                app.control = chord & 1 != 0;
                app.shift = chord & 2 != 0;
                event(&mut app, code, false);
                if chord == 0 && code == KeyCode::KeyS {
                    proof(&app, &format!("{}-after", size.0));
                }
                assert_eq!(
                    app.pending, pending,
                    "{size:?} {code:?} chord {chord} cannot submit an order"
                );
                assert!(
                    app.attack_move
                        && app.placing.is_none()
                        && app.service_mode == ServiceMode::None
                );
                assert_eq!(app.left_down, Some((100.0, 100.0)));
                assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
                assert_eq!(app.selected, BTreeSet::from([own]));
                assert!(app.order_feedback.message(Instant::now()).is_none());
                assert!(!app.keys.contains(&code));
                assert_eq!(saved(&app), before);
            }
        }
    }
    println!("60 layout/key/Command(+Control/Shift) cases preserve pending Move, armed mode, pointer gesture, feedback, camera, selection and encoded Session bytes.");
}

#[test]
fn command_world_storage_menu_and_camera_routes_stay_inert() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for code in [
            KeyCode::KeyM,
            KeyCode::F5,
            KeyCode::F9,
            KeyCode::Equal,
            KeyCode::Minus,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ] {
            let dir = PreferenceDirectory::new();
            let (mut app, own, _) = scene(size);
            app.save_path = dir.0.join("protected.oyrsave");
            let prefs = dir.0.join("preferences.json");
            let save_bytes = b"existing saved match stays byte exact";
            let pref_bytes = br#"{"muted":false,"zoom":100,"edge_scroll":false}"#;
            std::fs::write(&app.save_path, save_bytes).unwrap();
            std::fs::write(&prefs, pref_bytes).unwrap();
            app.preferences_path = Some(prefs.clone());
            app.pending.push(Command::Stop { id: own });
            let before = saved(&app);
            let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
            let muted = app.menu.muted;
            app.super_key = true;
            event(&mut app, code, false);
            assert_eq!(app.menu.page, Page::Closed, "{code:?}");
            assert_eq!(app.menu.muted, muted, "{code:?}");
            assert_eq!(
                (app.view.cam_x, app.view.cam_y, app.view.zoom),
                camera,
                "{code:?}"
            );
            assert_eq!(app.keyboard_pan(), (0.0, 0.0), "{code:?}");
            assert!(!app.keys.contains(&code));
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(std::fs::read(&app.save_path).unwrap(), save_bytes);
            assert_eq!(std::fs::read(&prefs).unwrap(), pref_bytes);
            assert_eq!(saved(&app), before);
        }
    }
    println!("27 actual Command mute/save/load/zoom/pan cases preserve protected save/preference files, menu state, camera, orders and full Session bytes.");
}

#[test]
fn command_world_repeats_need_release_or_a_fresh_plain_press() {
    for code in [KeyCode::ArrowRight, KeyCode::Equal, KeyCode::Minus] {
        let (mut app, _, _) = scene((640, 480));
        let before = saved(&app);
        let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
        app.super_key = true;
        event(&mut app, code, false);
        app.super_key = false;
        event(&mut app, code, true);
        assert_eq!(
            (app.view.cam_x, app.view.cam_y, app.view.zoom),
            camera,
            "old {code:?} repeat cannot become plain input after Command release"
        );
        assert_eq!(app.keyboard_pan(), (0.0, 0.0));
        key(&mut app, code, ElementState::Released);
        event(&mut app, code, false);
        if code == KeyCode::ArrowRight {
            assert_eq!(app.keyboard_pan(), (1.0, 0.0));
        } else {
            assert_ne!(app.view.zoom, camera.2);
        }
        assert_eq!(saved(&app), before);
    }
    // A repeated plain camera key also stops owning camera motion once rejected
    // as Command input, and cannot resume from an old OS repeat.
    let (mut app, _, _) = scene((640, 480));
    event(&mut app, KeyCode::ArrowRight, false);
    assert_eq!(app.keyboard_pan(), (1.0, 0.0));
    app.super_key = true;
    event(&mut app, KeyCode::ArrowRight, true);
    assert_eq!(app.keyboard_pan(), (0.0, 0.0));
    app.super_key = false;
    event(&mut app, KeyCode::ArrowRight, true);
    assert_eq!(app.keyboard_pan(), (0.0, 0.0));
    event(&mut app, KeyCode::ArrowRight, false);
    assert_eq!(app.keyboard_pan(), (1.0, 0.0));
    println!("Command-origin pan/zoom repeats stay suppressed after modifier release; key-up or a fresh plain press restores normal input, including an already-held pan key.");
}

#[test]
fn command_world_menu_editing_and_documented_plain_commands_still_work() {
    let (mut app, own, _) = scene((640, 480));
    event(&mut app, KeyCode::KeyA, false);
    assert!(app.attack_move);
    event(&mut app, KeyCode::KeyS, false);
    assert_eq!(app.pending, [Command::Stop { id: own }]);
    assert!(!app.attack_move);
    event(&mut app, KeyCode::KeyK, false);
    assert_eq!(app.service_mode, ServiceMode::Repair);
    event(&mut app, KeyCode::KeyL, false);
    assert_eq!(app.service_mode, ServiceMode::Sell);
    app.session
        .as_mut()
        .unwrap()
        .sim
        .step(&[Command::AssignRole {
            id: own,
            role: yr_sim::Role::Harvester,
        }]);
    app.control = true;
    event(&mut app, KeyCode::KeyH, false);
    app.control = false;
    assert!(matches!(app.pending.last(),Some(Command::Harvest{id}) if *id==own));
    let pending = app.pending.clone();
    app.menu.set(Page::Save);
    app.menu.name = "Original".into();
    app.super_key = true;
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some("a"),
        false,
        false,
    );
    assert!(app.menu.name_selected);
    key(&mut app, KeyCode::KeyA, ElementState::Released);
    app.super_key = false;
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::KeyS),
        ElementState::Pressed,
        Some("s"),
        false,
        false,
    );
    assert_eq!(app.menu.name, "s");
    assert_eq!(app.pending, pending, "menu text cannot become Stop");
    println!("Plain A/S/K/L, documented Ctrl+H and menu Command+A replacement remain available; menu text submits no world order.");
}

fn proof(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_COMMAND_WORLD_PROOF") else {
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
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "pending":format!("{:?}",app.pending),"attack_move":app.attack_move,
            "left_down":app.left_down,"camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],
            "selected":app.selected,"notice":app.order_feedback.message(Instant::now()),
            "tick":s.sim.tick(),"sim_hash":format!("{:016x}",s.sim.state_hash()),
            "snapshot_bytes":saved(app).len(),"retail_art":false
        }))
        .unwrap(),
    )
    .unwrap();
}
