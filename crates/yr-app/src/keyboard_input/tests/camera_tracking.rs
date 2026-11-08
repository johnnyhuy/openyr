use super::*;

fn press(app: &mut App, code: KeyCode, repeat: bool, synthetic: bool) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        None,
        repeat,
        synthetic,
    );
}

fn canvas(size: (u32, u32)) -> App {
    let mut app = new_app(String::new(), String::new(), 1);
    app.frame_size = size;
    app.map = Some(yr_render::RenderedMap {
        width: 1600,
        height: 1200,
        rgba: [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(1600 * 1200 * 4)
            .collect(),
        summary: "Synthetic camera input canvas".into(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    });
    app.view.cam_x = 200.0;
    app.view.cam_y = 200.0;
    app.selected.insert(17);
    app.pending.push(Command::Stop { id: 17 });
    app
}

#[test]
fn camera_key_cancels_box_before_redraw_and_stale_release() {
    let mut app = canvas((640, 480));
    app.pointer_moved((100.0, 120.0));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_moved((200.0, 220.0));
    capture(&app, "00-box");
    press(&mut app, KeyCode::ArrowRight, false, false);
    capture(&app, "01-key-before-redraw");
    // The release must be harmless even when no redraw has run since key-down.
    app.pointer_button(ElementState::Released, MouseButton::Left);
    capture(&app, "02-stale-release");
    assert_eq!(
        app.selected,
        [17].into(),
        "arrow key must cancel before a stale release can finish box selection"
    );
    assert_eq!(app.pending, [Command::Stop { id: 17 }]);
    assert_eq!((app.view.cam_x, app.view.cam_y), (200.0, 200.0));
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    assert!(!frame.as_chunks::<4>().0.contains(&SELECT_COL));
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Released,
        None,
        false,
        false,
    );
    app.pointer_moved((250.0, 240.0));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_moved((350.0, 340.0));
    app.compose(&mut frame, 640, 480);
    assert!(
        frame.as_chunks::<4>().0.contains(&SELECT_COL),
        "fresh gesture remains usable after key-up"
    );
    capture(&app, "03-fresh-box");
}

#[test]
fn camera_keys_cancel_all_pointer_tracking_without_waiting_for_redraw() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for code in [
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ] {
            for gesture in 0..4 {
                let mut app = canvas(size);
                app.pointer_moved((100.0, 120.0));
                match gesture {
                    0 => {
                        app.pointer_button(ElementState::Pressed, MouseButton::Left);
                        app.pointer_moved((200.0, 220.0));
                    }
                    1 | 2 => {
                        app.pointer_button(ElementState::Pressed, MouseButton::Right);
                        if gesture == 2 {
                            app.pointer_moved((120.0, 140.0));
                        }
                    }
                    _ => app.radar_drag = true,
                }
                app.camera.edge_armed = true;
                app.attack_move = true;
                app.placing = Some(3);
                app.service_mode = ServiceMode::Repair;
                let camera = (app.view.cam_x, app.view.cam_y);
                press(&mut app, code, false, false);
                assert!(app.keys.contains(&code));
                assert!(app.left_down.is_none() && app.right_down.is_none());
                assert!(!app.right_panning && !app.radar_drag && !app.camera.edge_armed);
                assert!(app.pointer.inside && app.pointer.focused);
                app.pointer_button(ElementState::Released, MouseButton::Left);
                app.pointer_button(ElementState::Released, MouseButton::Right);
                assert_eq!(app.selected, [17].into());
                assert_eq!(app.pending, [Command::Stop { id: 17 }]);
                assert_eq!((app.view.cam_x, app.view.cam_y), camera);
                assert!(app.attack_move && app.placing == Some(3));
                assert_eq!(app.service_mode, ServiceMode::Repair);
            }
        }
    }
}

#[test]
fn camera_key_tracking_cleanup_keeps_keyboard_dispatch_guards() {
    let mut app = canvas((640, 480));
    app.pointer_moved((100.0, 120.0));
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    press(&mut app, KeyCode::ArrowRight, false, true);
    press(&mut app, KeyCode::ArrowRight, true, false);
    assert!(
        app.right_down.is_some(),
        "focus-sync and its repeat stay inert"
    );
    press(&mut app, KeyCode::KeyQ, false, false);
    assert!(
        app.right_down.is_some(),
        "unrelated key keeps existing policy"
    );
    press(&mut app, KeyCode::ArrowRight, false, false);
    assert!(app.right_down.is_none());
    app.focus_changed(false);
    press(&mut app, KeyCode::ArrowLeft, false, false);
    assert!(!app.keys.contains(&KeyCode::ArrowLeft));
    app.focus_changed(true);
    app.menu.set(Page::Pause);
    app.menu.focus = 0;
    press(&mut app, KeyCode::ArrowDown, false, false);
    assert!(!app.keys.contains(&KeyCode::ArrowDown));
    assert_eq!(app.menu.focus, 1, "menu arrow navigation still runs");
}

fn capture(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_CAMERA_KEY_PROOF") else {
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
    std::fs::write(dir.join(format!("{name}.json")), serde_json::to_vec_pretty(&serde_json::json!({
        "left_down": app.left_down, "right_down": app.right_down, "right_panning": app.right_panning,
        "radar_drag": app.radar_drag, "edge_armed": app.camera.edge_armed,
        "selected": app.selected, "pending": format!("{:?}", app.pending),
        "camera": [app.view.cam_x, app.view.cam_y], "session_present": app.session.is_some(),
        "retail_art": false, "redraw_handler_called": false,
    })).unwrap()).unwrap();
}
