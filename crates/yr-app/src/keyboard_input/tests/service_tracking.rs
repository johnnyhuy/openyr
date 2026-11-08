use super::*;

fn key(app: &mut App, code: KeyCode, repeat: bool, synthetic: bool) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        None,
        repeat,
        synthetic,
    );
    if !repeat {
        app.keyboard_input(
            PhysicalKey::Code(code),
            ElementState::Released,
            None,
            false,
            false,
        );
    }
}

#[test]
fn service_mode_pan_cleanup_restores_a_visible_fresh_box_gesture() {
    for code in [KeyCode::KeyK, KeyCode::KeyL] {
        let mut app = new_app(String::new(), String::new(), 1);
        app.frame_size = (640, 480);
        app.map = Some(yr_render::RenderedMap {
            width: 1200,
            height: 1000,
            rgba: [30, 40, 30, 255]
                .into_iter()
                .cycle()
                .take(1200 * 1000 * 4)
                .collect(),
            summary: "Synthetic service input canvas".into(),
            origin_x: 0,
            origin_y: 0,
            scale: 1,
        });
        app.view.cam_x = 200.0;
        app.view.cam_y = 200.0;
        app.pointer_moved((100.0, 120.0));
        app.pointer_button(ElementState::Pressed, MouseButton::Right);
        app.pointer_moved((150.0, 150.0));
        assert!(app.right_panning);
        app.selected.insert(17);
        app.pending.push(Command::Stop { id: 17 });
        capture(&app, code, "00-pan");
        let camera = (app.view.cam_x, app.view.cam_y);
        key(&mut app, code, false, false);
        capture(&app, code, "01-service-active");
        assert!(!app.right_panning);
        key(&mut app, code, false, false);
        assert_eq!(app.service_mode, ServiceMode::None);
        app.pointer_moved((200.0, 180.0));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_moved((300.0, 280.0));
        let mut frame = vec![0; 640 * 480 * 4];
        app.compose(&mut frame, 640, 480);
        assert!(
            frame.as_chunks::<4>().0.contains(&SELECT_COL),
            "fresh box must be visible after {code:?} interrupts a pan"
        );
        capture(&app, code, "02-fresh-box");
        let before = frame.clone();
        app.pointer_button(ElementState::Released, MouseButton::Right);
        app.compose(&mut frame, 640, 480);
        assert_eq!(frame, before, "old right release cannot hide the new box");
        assert_eq!((app.view.cam_x, app.view.cam_y), camera);
        assert_eq!(app.selected, [17].into());
        assert_eq!(app.pending, [Command::Stop { id: 17 }]);
        capture(&app, code, "03-stale-right-release");
    }
}

#[test]
fn service_mode_pan_cleanup_preserves_mode_toggles_selection_and_queued_intents() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for initial in [ServiceMode::None, ServiceMode::Repair, ServiceMode::Sell] {
            for (code, requested) in [
                (KeyCode::KeyK, ServiceMode::Repair),
                (KeyCode::KeyL, ServiceMode::Sell),
            ] {
                for gesture in 0..4 {
                    let mut app = new_app(String::new(), String::new(), 1);
                    app.frame_size = size;
                    app.pointer_moved((300.0, 300.0));
                    app.selected.insert(17);
                    app.pending.push(Command::Stop { id: 17 });
                    app.service_mode = initial;
                    app.placing = Some(3);
                    app.attack_move = true;
                    match gesture {
                        0 => app.left_down = Some((100.0, 120.0)),
                        1 | 2 => {
                            app.right_down = Some((100.0, 120.0));
                            app.right_panning = gesture == 2;
                        }
                        _ => app.radar_drag = true,
                    }
                    app.camera.edge_armed = true;
                    let expected = if initial == requested {
                        ServiceMode::None
                    } else {
                        requested
                    };
                    key(&mut app, code, false, false);
                    assert_eq!(app.service_mode, expected);
                    assert!(app.left_down.is_none() && app.right_down.is_none());
                    assert!(!app.right_panning && !app.radar_drag && !app.camera.edge_armed);
                    assert!(app.pointer.inside && app.pointer.focused);
                    assert_eq!(app.cursor, (300.0, 300.0));
                    assert!(app.placing.is_none() && !app.attack_move);
                    app.pointer_button(ElementState::Released, MouseButton::Left);
                    app.pointer_button(ElementState::Released, MouseButton::Right);
                    assert_eq!(app.service_mode, expected);
                    assert_eq!(app.selected, [17].into());
                    assert_eq!(app.pending, [Command::Stop { id: 17 }]);
                }
            }
        }
    }
}

#[test]
fn service_mode_pan_cleanup_obeys_repeat_synthetic_focus_and_menu_guards() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.pointer_moved((100.0, 120.0));
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    app.pointer_moved((150.0, 150.0));
    assert!(app.right_panning);
    key(&mut app, KeyCode::KeyK, true, false);
    key(&mut app, KeyCode::KeyK, false, true);
    assert!(app.right_panning && app.right_down.is_some());
    assert_eq!(app.service_mode, ServiceMode::None);
    app.focus_changed(false);
    key(&mut app, KeyCode::KeyK, false, false);
    assert_eq!(app.service_mode, ServiceMode::None);
    app.focus_changed(true);
    app.menu.set(Page::Pause);
    key(&mut app, KeyCode::KeyL, false, false);
    assert_eq!(app.menu.page, Page::Pause);
    assert_eq!(app.service_mode, ServiceMode::None);
    app.menu.set(Page::Closed);
    key(&mut app, KeyCode::KeyL, false, false);
    assert_eq!(app.service_mode, ServiceMode::Sell);
    key(&mut app, KeyCode::KeyL, true, false);
    assert_eq!(app.service_mode, ServiceMode::Sell);
    key(&mut app, KeyCode::KeyL, false, false);
    assert_eq!(app.service_mode, ServiceMode::None);
}

fn capture(app: &App, code: KeyCode, name: &str) {
    if code != KeyCode::KeyK {
        return;
    }
    let Some(dir) = std::env::var_os("OPENYR_SERVICE_TRACKING_PROOF") else {
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
    std::fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
        "mode":format!("{:?}",app.service_mode),"right_panning":app.right_panning,
        "left_down":app.left_down,"right_down":app.right_down,"radar_drag":app.radar_drag,
        "camera":[app.view.cam_x,app.view.cam_y],"selected":app.selected,
        "pending":format!("{:?}",app.pending),"session_present":app.session.is_some(),"retail_art":false,
    })).unwrap()).unwrap();
}
