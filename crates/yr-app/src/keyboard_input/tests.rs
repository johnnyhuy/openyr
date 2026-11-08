use super::*;
use yr_app::menu::{Intent, Page};

fn key(app: &mut App, code: KeyCode, state: ElementState, text: Option<&str>, synthetic: bool) {
    app.keyboard_input(PhysicalKey::Code(code), state, text, false, synthetic);
}

#[test]
fn synthesized_enter_cannot_delete_a_save_but_a_deliberate_press_can() {
    let dir = std::env::temp_dir().join(format!(
        "openyr-focus-save-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _scratch = Scratch(dir.clone());
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("keep.oyrsave");
    let original = b"saved match bytes must survive synthetic Enter";
    std::fs::write(&path, original).unwrap();
    let mut app = new_app(String::new(), String::new(), 1);
    app.save_path = dir.join("active.oyrsave");
    app.menu.slots = yr_app::menu::slots(&dir).unwrap();
    app.menu.set(Page::Confirm(Intent::Delete(path.clone())));
    app.menu.focus = 0;
    app.focus_changed(false);
    app.focus_changed(true);
    key(
        &mut app,
        KeyCode::Enter,
        ElementState::Pressed,
        Some("\r"),
        true,
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(app.menu.page, Page::Confirm(Intent::Delete(path.clone())));
    key(
        &mut app,
        KeyCode::Enter,
        ElementState::Pressed,
        Some("\r"),
        false,
    );
    assert!(!path.exists());
    assert!(matches!(app.menu.page, Page::Message { .. }));
    println!("Synthetic focus-return Enter preserves save bytes; deliberate Enter deletes the selected file.");
}

#[test]
fn synthesized_or_unfocused_presses_cannot_open_load_or_arm_world_input() {
    for (focused, synthetic) in [(true, true), (false, false), (false, true)] {
        for code in [
            KeyCode::F9,
            KeyCode::F5,
            KeyCode::KeyA,
            KeyCode::KeyK,
            KeyCode::KeyL,
            KeyCode::ArrowRight,
            KeyCode::Escape,
        ] {
            let mut app = new_app(String::new(), String::new(), 1);
            app.focus_changed(focused);
            let page = app.menu.page.clone();
            key(&mut app, code, ElementState::Pressed, None, synthetic);
            assert_eq!(app.menu.page, page, "{code:?}");
            assert!(!app.attack_move && app.service_mode == ServiceMode::None);
            assert!(app.keys.is_empty() && app.pending.is_empty());
        }
    }
    let mut app = new_app(String::new(), String::new(), 1);
    key(
        &mut app,
        KeyCode::KeyA,
        ElementState::Pressed,
        Some("a"),
        false,
    );
    assert!(app.attack_move && app.keys.contains(&KeyCode::KeyA));
    key(&mut app, KeyCode::F9, ElementState::Pressed, None, false);
    assert!(matches!(app.menu.page, Page::Confirm(Intent::Load(_))));
    println!("Synthetic/unfocused world presses are inert; focused A and F9 retain their actions.");
}

#[test]
fn guarded_presses_do_not_edit_or_navigate_menus_and_releases_still_clear_keys() {
    for (focused, synthetic) in [(true, true), (false, false), (false, true)] {
        let mut app = new_app(String::new(), String::new(), 1);
        app.menu.set(Page::Save);
        app.menu.name = "Keep".into();
        app.focus_changed(focused);
        let focus = app.menu.focus;
        key(
            &mut app,
            KeyCode::KeyZ,
            ElementState::Pressed,
            Some("z"),
            synthetic,
        );
        key(
            &mut app,
            KeyCode::Backspace,
            ElementState::Pressed,
            None,
            synthetic,
        );
        key(
            &mut app,
            KeyCode::Tab,
            ElementState::Pressed,
            None,
            synthetic,
        );
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyZ),
            ElementState::Pressed,
            Some("z"),
            true,
            false,
        );
        assert_eq!(app.menu.name, "Keep");
        assert_eq!(app.menu.focus, focus);
        app.keys.insert(KeyCode::ArrowLeft);
        key(
            &mut app,
            KeyCode::ArrowLeft,
            ElementState::Released,
            None,
            synthetic,
        );
        assert!(app.keys.is_empty());
        app.focus_changed(true);
        key(
            &mut app,
            KeyCode::KeyZ,
            ElementState::Pressed,
            Some("z"),
            false,
        );
        assert_eq!(app.menu.name, "Keepz");
    }
    println!("Guarded menu input preserves names/focus; releases clear held keys even while modal or unfocused.");
}

#[test]
fn focus_return_repeats_stay_suppressed_until_release_or_a_fresh_press() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.selected.insert(17);
    app.focus_changed(false);
    app.focus_changed(true);
    for code in [KeyCode::ArrowRight, KeyCode::KeyA] {
        key(&mut app, code, ElementState::Pressed, None, true);
        app.keyboard_input(
            PhysicalKey::Code(code),
            ElementState::Pressed,
            None,
            true,
            false,
        );
        assert!(app.keys.is_empty() && !app.attack_move);
        assert!(app.suppressed_keys.contains(&code));
        key(&mut app, code, ElementState::Released, None, true);
        assert!(!app.suppressed_keys.contains(&code));
        key(&mut app, code, ElementState::Pressed, None, false);
        assert!(app.keys.contains(&code));
        key(&mut app, code, ElementState::Released, None, false);
    }
    assert!(app.attack_move && app.pending.is_empty());
    assert_eq!(app.selected, BTreeSet::from([17]));
    println!("Focus-return repeats cannot pan or arm attack-move; fresh presses work and selection remains intact.");
}

mod attack_move;

mod service_tracking;
