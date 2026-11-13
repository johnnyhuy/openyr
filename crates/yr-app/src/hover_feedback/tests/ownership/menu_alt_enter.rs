use super::*;
use winit::keyboard::ModifiersState;
use yr_app::menu::Intent;

fn event(app: &mut App, code: KeyCode, state: ElementState, repeat: bool, text: Option<&str>) {
    app.keyboard_input(PhysicalKey::Code(code), state, text, repeat, false);
}

#[test]
fn alt_menu_enter_preserves_confirmed_save_files_and_plain_enter_still_commits() {
    let dir = PreferenceDirectory::new();
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for front in [false, true] {
            for operation in 0..3 {
                if front && operation == 0 {
                    continue;
                }
                for code in [KeyCode::Enter, KeyCode::NumpadEnter] {
                    for extra in [
                        ModifiersState::empty(),
                        ModifiersState::CONTROL,
                        ModifiersState::SHIFT,
                        ModifiersState::SUPER,
                    ] {
                        let (mut app, own, _) = scene(size);
                        let before =
                            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                        let path = dir.0.join("protected.oyrsave");
                        let original = b"existing save contents must survive a desktop chord";
                        std::fs::write(&path, original).unwrap();
                        app.save_path = dir.0.join("active.oyrsave");
                        app.menu.slots = yr_app::menu::slots(&dir.0).unwrap();
                        let intent = match operation {
                            0 => Intent::Save(path.clone()),
                            1 => Intent::Delete(path.clone()),
                            _ => Intent::Load(path.clone()),
                        };
                        let page = Page::Confirm(intent);
                        app.menu.front = front;
                        app.menu.set(page.clone());
                        app.menu.focus = 0;
                        app.pending.push(Command::Stop { id: own });
                        app.modifiers_changed(ModifiersState::ALT | extra);
                        event(&mut app, code, ElementState::Pressed, false, Some("\r"));
                        assert_eq!(app.menu.page, page);
                        assert_eq!(app.menu.focus, 0);
                        assert!(app.load_job.is_none());
                        assert_eq!(std::fs::read(&path).unwrap(), original);
                        assert_eq!(app.selected, BTreeSet::from([own]));
                        assert_eq!(app.pending, [Command::Stop { id: own }]);
                        assert_eq!(
                            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                            before
                        );
                        assert!(app.suppressed_keys.contains(&code));
                        app.modifiers_changed(ModifiersState::empty());
                        event(&mut app, code, ElementState::Pressed, true, Some("\r"));
                        assert_eq!(app.menu.page, page);
                        assert_eq!(std::fs::read(&path).unwrap(), original);
                        event(&mut app, code, ElementState::Released, false, None);
                        if operation < 2 {
                            event(&mut app, code, ElementState::Pressed, false, Some("\r"));
                            assert!(matches!(app.menu.page, Page::Message { .. }));
                            if operation == 0 {
                                assert_eq!(std::fs::read(&path).unwrap(), before);
                            } else {
                                assert!(!path.exists());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn alt_menu_enter_keeps_navigation_inert_and_option_text_usable() {
    for page in [
        Page::Main,
        Page::SinglePlayer,
        Page::Skirmish,
        Page::ChooseMap,
        Page::Pause,
        Page::Controls,
        Page::Save,
        Page::Load,
        Page::Delete,
    ] {
        let (mut app, _, _) = scene((800, 600));
        app.menu.front = matches!(
            page,
            Page::Main | Page::SinglePlayer | Page::Skirmish | Page::ChooseMap
        );
        app.menu.set(page.clone());
        let focus = app.menu.focus;
        app.modifiers_changed(ModifiersState::ALT);
        event(
            &mut app,
            KeyCode::Enter,
            ElementState::Pressed,
            false,
            Some("\r"),
        );
        assert_eq!(app.menu.page, page);
        assert_eq!(app.menu.focus, focus);
        assert!(app.load_job.is_none());
    }
    let (mut app, _, _) = scene((800, 600));
    app.open_menu(Page::Save);
    app.menu.name.clear();
    app.modifiers_changed(ModifiersState::ALT);
    event(
        &mut app,
        KeyCode::KeyA,
        ElementState::Pressed,
        false,
        Some("\u{00e6}"),
    );
    event(
        &mut app,
        KeyCode::Space,
        ElementState::Pressed,
        false,
        Some(" "),
    );
    assert_eq!(app.menu.name, "\u{00e6} ");
}
