use super::*;

fn key(app: &mut App, code: KeyCode) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        None,
        false,
        false,
    );
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Released,
        None,
        false,
        false,
    );
}
fn button_position(app: &App, button: Button) -> (f64, f64) {
    let layout = Layout::for_state(&app.menu, app.frame_size.0, app.frame_size.1);
    let index = app
        .menu
        .buttons()
        .iter()
        .position(|b| b.0 == button)
        .unwrap();
    let (x, y, _, _) = layout.button(index, button);
    (
        layout.origin.0 + f64::from(x + 10) * layout.scale,
        layout.origin.1 + f64::from(y + 10) * layout.scale,
    )
}

#[test]
fn menu_keyboard_tracking_interrupt_prevents_stale_confirmation_delete() {
    for front in [false, true] {
        for size in [(640, 480), (800, 600), (1280, 800)] {
            let temp = Scratch::new();
            let path = temp.0.join("keep.oyrsave");
            std::fs::write(&path, b"protected fixture bytes").unwrap();
            let mut app = new_app(String::new(), String::new(), 1);
            app.frame_size = size;
            app.save_path = temp.0.join("active.oyrsave");
            app.menu.front = front;
            app.open_menu(Page::Delete);
            app.menu.select_slot(0);
            app.apply_menu_effect(Effect::RequestDelete);
            key(&mut app, KeyCode::ArrowUp);
            assert_eq!(app.menu.focus, 0);
            app.pointer_moved(button_position(&app, Button::Accept));
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            assert_eq!(app.menu.pressed, Some(Button::Accept));
            capture(&app, "00-held");
            key(&mut app, KeyCode::Tab);
            assert_eq!(app.menu.focus, 1);
            capture(&app, "01-keyboard-focus");
            app.pointer_button(ElementState::Released, MouseButton::Left);
            assert_eq!(std::fs::read(&path).unwrap(), b"protected fixture bytes");
            assert!(matches!(app.menu.page, Page::Confirm(_)));
            assert!(app.menu.pressed.is_none());
            capture(&app, "02-stale-release");
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            app.pointer_button(ElementState::Released, MouseButton::Left);
            assert!(!path.exists(), "a fresh deliberate click still deletes");
            assert!(matches!(app.menu.page, Page::Message { .. }));
            assert!(app.pending.is_empty());
        }
    }
}

fn local(app: &App, x: i32, y: i32) -> (f64, f64) {
    let l = Layout::for_state(&app.menu, app.frame_size.0, app.frame_size.1);
    (
        l.origin.0 + f64::from(x) * l.scale,
        l.origin.1 + f64::from(y) * l.scale,
    )
}

#[test]
fn menu_keyboard_tracking_interrupt_stops_slider_and_map_thumb_drags() {
    use yr_app::menu::setup::{Field, Setup};
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for field in [Field::Credits, Field::Units] {
            let mut app = new_app(String::new(), String::new(), 1);
            app.frame_size = size;
            app.menu.front = true;
            app.menu.set(Page::Skirmish);
            let x = app.menu.setup.thumb_x(field) + 3;
            let y = Setup::y(field) + 10;
            app.pointer_moved(local(&app, x, y));
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            assert!(app.menu.setup.drag.is_some());
            let value = app.menu.setup.value(field);
            key(&mut app, KeyCode::Tab);
            assert!(app.menu.setup.drag.is_none());
            app.pointer_moved(local(&app, x - 40, y));
            assert_eq!(app.menu.setup.value(field), value);
            app.pointer_button(ElementState::Released, MouseButton::Left);
            app.pointer_moved(local(&app, x, y));
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            app.pointer_moved(local(&app, x - 40, y));
            assert_ne!(app.menu.setup.value(field), value, "fresh drag still works");
            app.pointer_button(ElementState::Released, MouseButton::Left);
        }
        let mut app = new_app(String::new(), String::new(), 1);
        app.frame_size = size;
        app.menu.front = true;
        app.menu.set(Page::ChooseMap);
        app.menu.maps.entries = (0..60)
            .map(|i| yr_app::menu::maps::MapChoice {
                asset: format!("MAP{i}.MAP"),
                title: format!("Map {i}"),
            })
            .collect();
        app.menu.maps.ready = Some(0);
        app.menu.maps.top = 10;
        let y = app.menu.maps.thumb().0 + 10;
        app.pointer_moved(local(&app, 525, y));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        assert!(app.menu.maps.drag.is_some());
        key(&mut app, KeyCode::Tab);
        assert!(app.menu.maps.drag.is_none());
        app.pointer_moved(local(&app, 525, y + 90));
        assert_eq!(app.menu.maps.top, 10);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        app.pointer_moved(local(&app, 525, y));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_moved(local(&app, 525, y + 90));
        assert!(app.menu.maps.top > 10, "fresh thumb drag still works");
        app.pointer_button(ElementState::Released, MouseButton::Left);
        assert_eq!(app.menu.maps.selected, 0);
        assert!(app.pending.is_empty());
    }
}

#[test]
fn menu_keyboard_tracking_preserves_popup_navigation_and_synthetic_key_guards() {
    use yr_app::menu::setup::Field;
    let mut app = new_app(String::new(), String::new(), 1);
    app.menu.front = true;
    app.menu.set(Page::Skirmish);
    app.menu.setup.press(Field::Country(0), 287);
    let initial = app.menu.setup.value(Field::Country(0));
    key(&mut app, KeyCode::ArrowDown);
    assert!(app.menu.setup.popup.is_some());
    key(&mut app, KeyCode::Enter);
    assert!(app.menu.setup.popup.is_none());
    assert_ne!(app.menu.setup.value(Field::Country(0)), initial);
    app.menu.set(Page::Exit);
    app.pointer_moved(button_position(&app, Button::Accept));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        None,
        false,
        true,
    );
    assert_eq!(app.menu.pressed, Some(Button::Accept));
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
        None,
        true,
        false,
    );
    assert_eq!(app.menu.pressed, Some(Button::Accept));
    key(&mut app, KeyCode::Tab);
    assert!(app.menu.pressed.is_none());
    app.pointer_button(ElementState::Released, MouseButton::Left);
    assert!(!app.quit_requested);
}

fn capture(app: &App, name: &str) {
    if !app.menu.front || app.frame_size != (640, 480) {
        return;
    }
    let Some(dir) = std::env::var_os("OPENYR_KEYBOARD_TRACKING_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (w, h) = app.frame_size;
    let mut frame = vec![0; (w * h * 4) as usize];
    app.compose(&mut frame, w, h);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(w, h, &frame),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "pressed": format!("{:?}",app.menu.pressed), "focus":app.menu.focus,
            "page": format!("{:?}",app.menu.page), "session_present":app.session.is_some(),
            "retail_art":false,"file_preserved":app.menu.slots[0].path.is_file(),
        }))
        .unwrap(),
    )
    .unwrap();
}
