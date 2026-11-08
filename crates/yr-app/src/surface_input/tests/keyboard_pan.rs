use super::*;

fn key(app: &mut App, code: KeyCode, state: ElementState) {
    app.keyboard_input(PhysicalKey::Code(code), state, None, false, false);
}

fn app(size: (u32, u32)) -> App {
    let mut app = new_app(String::new(), String::new(), 1);
    app.frame_size = size;
    app.selected.insert(17);
    app.pending.push(Command::Stop { id: 17 });
    app.view.cam_x = 200.0;
    app.view.cam_y = 200.0;
    app
}

#[test]
fn held_camera_key_rejects_new_world_gestures_before_redraw() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for code in [
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ] {
            for mode in 0..5 {
                let mut app = app(size);
                key(&mut app, code, ElementState::Pressed);
                match mode {
                    1 => app.service_mode = ServiceMode::Repair,
                    2 => app.service_mode = ServiceMode::Sell,
                    3 => app.placing = Some(3),
                    4 => app.attack_move = true,
                    _ => {}
                }
                let service = app.service_mode;
                let placing = app.placing;
                let attack = app.attack_move;
                app.pointer_moved((100.0, 120.0));
                app.pointer_button(ElementState::Pressed, MouseButton::Right);
                app.pointer_button(ElementState::Released, MouseButton::Right);
                assert_eq!(
                    app.selected,
                    [17].into(),
                    "held {code:?} must own the world before redraw"
                );
                assert_eq!(app.service_mode, service);
                assert_eq!(app.placing, placing);
                assert_eq!(app.attack_move, attack);
                app.pointer_button(ElementState::Pressed, MouseButton::Left);
                app.pointer_moved((200.0, 220.0));
                assert!(app.left_down.is_none() && app.right_down.is_none());
                assert!(!app.right_panning && !app.radar_drag);
                app.pointer_button(ElementState::Released, MouseButton::Left);
                app.pointer_button(ElementState::Pressed, MouseButton::Right);
                app.pointer_moved((250.0, 270.0));
                assert!(app.right_down.is_none() && !app.right_panning);
                app.pointer_button(ElementState::Released, MouseButton::Right);
                assert_eq!((app.view.cam_x, app.view.cam_y), (200.0, 200.0));
                assert_eq!(app.selected, [17].into());
                assert_eq!(app.pending, [Command::Stop { id: 17 }]);
            }
        }
    }
}

#[test]
fn held_camera_key_box_feedback_returns_after_key_up() {
    let mut app = app((640, 480));
    app.map = Some(yr_render::RenderedMap {
        width: 1600,
        height: 1200,
        rgba: [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(1600 * 1200 * 4)
            .collect(),
        summary: "Synthetic keyboard-pan ownership canvas".into(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    });
    key(&mut app, KeyCode::ArrowRight, ElementState::Pressed);
    capture(&app, "00-held-arrow");
    app.pointer_moved((100.0, 120.0));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_moved((200.0, 220.0));
    capture(&app, "01-ignored-box");
    assert!(!has_box(&app));
    app.pointer_button(ElementState::Released, MouseButton::Left);
    key(&mut app, KeyCode::ArrowRight, ElementState::Released);
    capture(&app, "02-key-up");
    assert_eq!(app.selected, [17].into());
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_moved((300.0, 320.0));
    assert!(has_box(&app));
    capture(&app, "03-fresh-box");
}

#[test]
fn keyboard_pan_ownership_uses_net_direction_and_keeps_sidebar_and_menu_routes() {
    for (a, b) in [
        (KeyCode::ArrowLeft, KeyCode::ArrowRight),
        (KeyCode::ArrowUp, KeyCode::ArrowDown),
    ] {
        let mut app = app((640, 480));
        key(&mut app, a, ElementState::Pressed);
        key(&mut app, b, ElementState::Pressed);
        app.pointer_moved((100.0, 120.0));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        assert!(
            app.left_down.is_some(),
            "opposing arrows have no pan direction"
        );
        app.pointer_button(ElementState::Released, MouseButton::Left);
        key(&mut app, b, ElementState::Released);
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        assert!(
            app.left_down.is_none(),
            "one remaining arrow owns the world"
        );
        app.service_mode = ServiceMode::Repair;
        app.pointer_moved((639.0, 200.0));
        app.pointer_button(ElementState::Pressed, MouseButton::Right);
        assert_eq!(
            app.service_mode,
            ServiceMode::None,
            "existing sidebar right-press route still runs"
        );
        app.open_menu(Page::Pause);
        key(&mut app, KeyCode::ArrowDown, ElementState::Pressed);
        assert!(app.keys.is_empty());
        assert_eq!(app.menu.focus, 1);
    }
}

fn has_box(app: &App) -> bool {
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    frame.as_chunks::<4>().0.contains(&SELECT_COL)
}

fn capture(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_KEYBOARD_PAN_PROOF") else {
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
        "keys": format!("{:?}", app.keys), "left_down": app.left_down, "right_down": app.right_down,
        "right_panning": app.right_panning, "selected": app.selected, "pending": format!("{:?}", app.pending),
        "camera": [app.view.cam_x, app.view.cam_y], "session_present": app.session.is_some(),
        "retail_art": false, "redraw_handler_called": false,
    })).unwrap()).unwrap();
}
