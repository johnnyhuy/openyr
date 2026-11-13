use super::*;
use winit::keyboard::ModifiersState;

fn press(app: &mut App, code: KeyCode, repeat: bool, text: Option<&str>) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        text,
        repeat,
        false,
    );
}

#[test]
fn alt_world_routes_preserve_match_files_modes_and_camera() {
    let dir = PreferenceDirectory::new();
    let save = dir.0.join("protected.oyrsave");
    let prefs = dir.0.join("preferences.json");
    let original = b"existing saved match stays byte exact";
    let preferences = br#"{"muted":false,"zoom":100,"edge_scroll":false}"#;
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for extra in [
            ModifiersState::empty(),
            ModifiersState::CONTROL,
            ModifiersState::SHIFT,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ] {
            for code in [
                KeyCode::KeyS,
                KeyCode::KeyA,
                KeyCode::KeyK,
                KeyCode::KeyL,
                KeyCode::KeyM,
                KeyCode::F5,
                KeyCode::F9,
                KeyCode::Escape,
                KeyCode::Equal,
                KeyCode::Minus,
                KeyCode::ArrowRight,
                KeyCode::Digit1,
                KeyCode::KeyT,
                KeyCode::KeyP,
                KeyCode::KeyN,
                KeyCode::KeyF,
                KeyCode::KeyH,
                KeyCode::KeyQ,
            ] {
                std::fs::write(&save, original).unwrap();
                std::fs::write(&prefs, preferences).unwrap();
                let (mut app, own, _) = scene(size);
                app.save_path = save.clone();
                app.preferences_path = Some(prefs.clone());
                app.pending.push(Command::Move {
                    id: own,
                    x: Fix::from_int(12),
                    y: Fix::from_int(4),
                });
                let pending = app.pending.clone();
                app.attack_move = true;
                app.left_down = Some((100.0, 100.0));
                let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
                let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                app.modifiers_changed(ModifiersState::ALT | extra);
                press(&mut app, code, false, None);
                assert_eq!(app.menu.page, Page::Closed, "{code:?} {extra:?}");
                assert_eq!(app.pending, pending, "{code:?} {extra:?}");
                assert!(
                    app.attack_move
                        && app.placing.is_none()
                        && app.service_mode == ServiceMode::None
                );
                assert_eq!(app.left_down, Some((100.0, 100.0)));
                assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
                assert_eq!(app.selected, BTreeSet::from([own]));
                assert!(!app.keys.contains(&code));
                assert!(app.order_feedback.message(Instant::now()).is_none());
                assert_eq!(std::fs::read(&save).unwrap(), original);
                assert_eq!(std::fs::read(&prefs).unwrap(), preferences);
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    before
                );
            }
        }
    }
}

#[test]
fn alt_world_repeats_need_fresh_input_and_option_text_keeps_menu_routing() {
    for code in [
        KeyCode::ArrowRight,
        KeyCode::Equal,
        KeyCode::Minus,
        KeyCode::F9,
    ] {
        let (mut app, _, _) = scene((640, 480));
        let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
        app.modifiers_changed(ModifiersState::ALT);
        press(&mut app, code, false, None);
        app.modifiers_changed(ModifiersState::empty());
        press(&mut app, code, true, None);
        assert_eq!(app.menu.page, Page::Closed);
        assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
        assert_eq!(app.keyboard_pan(), (0.0, 0.0));
        app.keyboard_input(
            PhysicalKey::Code(code),
            ElementState::Released,
            None,
            false,
            false,
        );
        press(&mut app, code, false, None);
        match code {
            KeyCode::F9 => assert!(matches!(app.menu.page, Page::Confirm(_))),
            KeyCode::ArrowRight => assert_eq!(app.keyboard_pan(), (1.0, 0.0)),
            _ => assert_ne!(app.view.zoom, camera.2),
        }
    }
    let (mut app, _, _) = scene((800, 600));
    app.open_menu(Page::Save);
    app.menu.name.clear();
    app.modifiers_changed(ModifiersState::ALT);
    press(&mut app, KeyCode::KeyA, false, Some("native option text"));
    assert_eq!(app.menu.name, "native option text");
    app.focus_changed(false);
    assert!(!app.alt_key && !app.super_key && !app.shift && !app.control);
}
