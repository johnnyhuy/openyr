use super::*;
use winit::keyboard::ModifiersState;

fn press(app: &mut App, code: KeyCode, repeat: bool) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        None,
        repeat,
        false,
    );
}

#[test]
fn late_desktop_modifier_stops_pan_and_requires_fresh_input() {
    let mut cases = 0;
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for code in [
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ] {
            for desktop in [
                ModifiersState::ALT,
                ModifiersState::SUPER,
                ModifiersState::ALT | ModifiersState::SUPER,
            ] {
                for extra in [
                    ModifiersState::empty(),
                    ModifiersState::CONTROL,
                    ModifiersState::SHIFT,
                    ModifiersState::CONTROL | ModifiersState::SHIFT,
                ] {
                    let (mut app, own, _) = scene(size);
                    app.view.cam_x = 200.0;
                    app.view.cam_y = 200.0;
                    app.pending.push(Command::Stop { id: own });
                    let pending = app.pending.clone();
                    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                    let now = Instant::now();
                    app.update_camera(now, size);
                    press(&mut app, code, false);
                    assert_ne!(app.keyboard_pan(), (0.0, 0.0));
                    let before = (app.view.cam_x, app.view.cam_y, app.view.zoom);
                    app.modifiers_changed(desktop | extra);
                    assert_eq!(
                        app.keyboard_pan(),
                        (0.0, 0.0),
                        "late {desktop:?} {extra:?} {code:?}"
                    );
                    assert!(app.suppressed_keys.contains(&code));
                    app.update_camera(now + Duration::from_millis(20), size);
                    assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), before);
                    app.modifiers_changed(extra);
                    press(&mut app, code, true);
                    app.update_camera(now + Duration::from_millis(40), size);
                    assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), before);
                    assert_eq!(app.pending, pending);
                    assert_eq!(app.selected, BTreeSet::from([own]));
                    assert_eq!(
                        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                        saved
                    );
                    key(&mut app, code, ElementState::Released);
                    press(&mut app, code, false);
                    app.update_camera(now + Duration::from_millis(60), size);
                    assert_ne!((app.view.cam_x, app.view.cam_y), (before.0, before.1));
                    key(&mut app, code, ElementState::Released);
                    assert_eq!(app.keyboard_pan(), (0.0, 0.0));
                    cases += 1;
                }
            }
        }
    }
    println!("{cases} late-modifier cases stop held pan, suppress stale repeats, preserve exact Session bytes, and restore fresh arrows.");
}

#[test]
fn ordinary_modifiers_keep_held_pan_and_menu_editing_keeps_ownership() {
    for modifiers in [
        ModifiersState::empty(),
        ModifiersState::CONTROL,
        ModifiersState::SHIFT,
        ModifiersState::CONTROL | ModifiersState::SHIFT,
    ] {
        let (mut app, _, _) = scene((800, 600));
        press(&mut app, KeyCode::ArrowRight, false);
        app.modifiers_changed(modifiers);
        assert_eq!(app.keyboard_pan(), (1.0, 0.0));
        app.open_menu(Page::Save);
        app.menu.name = "Checkpoint".into();
        app.modifiers_changed(ModifiersState::SUPER);
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyA),
            ElementState::Pressed,
            Some("a"),
            false,
            false,
        );
        assert!(app.menu.name_selected);
        app.modifiers_changed(ModifiersState::ALT);
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyE),
            ElementState::Pressed,
            Some("é"),
            false,
            false,
        );
        assert_eq!(app.menu.name, "é");
        assert!(app.keys.is_empty());
    }
}
