use super::*;

fn prepared(size: (u32, u32)) -> (App, u32) {
    let (mut app, own, _) = scene(size);
    for cell in [(5, 4), (31, 31)] {
        let sim = &mut app.session.as_mut().unwrap().sim;
        let id = sim.next_id_peek();
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: Fix::from_int(cell.0),
                y: Fix::from_int(cell.1),
                loco: yr_sim::terrain::Loco::Track,
                speed: Fix::ZERO,
                hp: 100,
                armor: 0,
                weapon: None,
            },
            Command::AssignOwner {
                id,
                owner: yr_sim::OWNER_PLAYER,
            },
        ]);
    }
    let far = app.session.as_ref().unwrap().sim.units().last().unwrap().id;
    point_at(&mut app, own, size);
    app.pending.push(Command::Stop { id: own });
    app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
        .into_iter()
        .cycle()
        .take(1600 * 1200 * 4)
        .collect();
    (app, far)
}
fn type_at(app: &mut App, at: Instant) {
    assert!(app.type_select_key(KeyCode::KeyT, ElementState::Pressed, false, at));
}
fn event(app: &mut App, code: KeyCode, repeat: bool, synthetic: bool) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        None,
        repeat,
        synthetic,
    );
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Released,
        None,
        false,
        synthetic,
    );
}
fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}

#[test]
fn fresh_keys_between_t_presses_restart_viewport_selection() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for code in [
            KeyCode::KeyH,
            KeyCode::KeyS,
            KeyCode::KeyA,
            KeyCode::ArrowRight,
            KeyCode::Equal,
            KeyCode::KeyQ,
        ] {
            let (mut app, far) = prepared(size);
            let at = Instant::now();
            let before = saved(&app);
            type_at(&mut app, at);
            assert!(!app.selected.contains(&far));
            event(&mut app, code, false, false);
            let pending = app.pending.clone();
            let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
            type_at(&mut app, at + Duration::from_millis(50));
            if code == KeyCode::KeyH {
                proof(
                    &app,
                    &format!("sequence-{}", size.0),
                    at + Duration::from_millis(60),
                );
            }
            assert!(
                !app.selected.contains(&far),
                "T, {code:?}, T must start a new viewport sequence"
            );
            assert_eq!(app.pending, pending);
            assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
            assert_eq!(saved(&app), before);
        }
    }
    println!("18 native key sequences: camera, order, mode, zoom and unrelated fresh keys interrupt double-T; selection submits no extra orders and preserves camera/Session bytes.");
}

#[test]
fn accepted_pointer_and_wheel_input_interrupt_type_sequence() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for action in 0..4 {
            let (mut app, far) = prepared(size);
            let at = Instant::now();
            let before = saved(&app);
            type_at(&mut app, at);
            match action {
                0 => app.pointer_button(ElementState::Pressed, MouseButton::Left),
                1 => app.pointer_button(ElementState::Pressed, MouseButton::Right),
                2 => app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, 1.0)),
                _ => {
                    app.pointer_moved((f64::from(size.0 - 20), 250.0));
                    app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
                }
            }
            let pending = app.pending.clone();
            let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
            type_at(&mut app, at + Duration::from_millis(50));
            assert!(
                !app.selected.contains(&far),
                "pointer/wheel action {action} interrupts T sequence"
            );
            app.pointer_button(ElementState::Released, MouseButton::Left);
            app.pointer_button(ElementState::Released, MouseButton::Right);
            assert_eq!(app.pending, pending);
            assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
            assert_eq!(saved(&app), before);
        }
    }
    println!("12 native accepted pointer/wheel sequences interrupt double-T; later stale releases cannot add orders and saved Session bytes stay exact.");
}

#[test]
fn rejected_modified_t_cannot_complete_a_later_plain_t_pair() {
    for modifier in 0..3 {
        let (mut app, far) = prepared((640, 480));
        let at = Instant::now();
        let before = saved(&app);
        type_at(&mut app, at);
        match modifier {
            0 => app.control = true,
            1 => app.shift = true,
            _ => app.super_key = true,
        }
        event(&mut app, KeyCode::KeyT, false, false);
        app.control = false;
        app.shift = false;
        app.super_key = false;
        type_at(&mut app, at + Duration::from_millis(50));
        assert!(!app.selected.contains(&far));
        assert_eq!(saved(&app), before);
    }
    println!("Tracked Control/Shift/Command T chords interrupt the previous plain-T window without selecting or issuing commands.");
}

#[test]
fn ignored_events_and_pointer_motion_keep_consecutive_t_working() {
    for action in 0..7 {
        let (mut app, far) = prepared((640, 480));
        let at = Instant::now();
        let before = saved(&app);
        type_at(&mut app, at);
        match action {
            0 => event(&mut app, KeyCode::KeyH, true, false),
            1 => event(&mut app, KeyCode::KeyH, false, true),
            2 => app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, f32::NAN)),
            3 => app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, 0.0)),
            4 => {
                app.invalidate_pointer();
                app.pointer_button(ElementState::Pressed, MouseButton::Left);
            }
            5 => app.pointer_moved((app.cursor.0 + 1.0, app.cursor.1 + 1.0)),
            _ => {
                assert!(app.type_select_key(
                    KeyCode::KeyT,
                    ElementState::Pressed,
                    true,
                    at + Duration::from_millis(20)
                ));
            }
        }
        assert!(!app.selected.contains(&far));
        let pending = app.pending.clone();
        type_at(&mut app, at + Duration::from_millis(50));
        assert!(
            app.selected.contains(&far),
            "guarded/motion event {action} does not consume a deliberate T pair"
        );
        assert_eq!(app.pending, pending);
        assert_eq!(saved(&app), before);
    }
    println!("Seven positive guard cases retain global expansion for a deliberate T pair after repeats, focus-sync input, invalid/zero wheels, rejected clicks or pointer motion.");
}

fn proof(app: &App, name: &str, now: Instant) {
    let Some(dir) = std::env::var_os("OPENYR_TYPE_INTERRUPTION_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let size = app.camera_size();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose_at(&mut frame, size.0, size.1, now);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({"selected":app.selected,"camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],"pending":format!("{:?}",app.pending),"tick":session.sim.tick(),"sim_hash":format!("{:016x}",session.sim.state_hash()),"snapshot_bytes":saved(app).len(),"logical_second_press_ms":50,"logical_capture_ms":60,"retail_art":false})).unwrap()).unwrap();
}
