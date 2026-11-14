use super::*;
use winit::keyboard::ModifiersState;
use yr_app::menu::Intent;

fn desktop_chords() -> impl Iterator<Item = ModifiersState> {
    [
        ModifiersState::ALT,
        ModifiersState::SUPER,
        ModifiersState::ALT | ModifiersState::SUPER,
    ]
    .into_iter()
    .flat_map(|desktop| {
        [
            ModifiersState::empty(),
            ModifiersState::CONTROL,
            ModifiersState::SHIFT,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ]
        .into_iter()
        .map(move |extra| desktop | extra)
    })
}

fn event(app: &mut App, code: KeyCode, state: ElementState, repeat: bool, text: Option<&str>) {
    app.keyboard_input(PhysicalKey::Code(code), state, text, repeat, false);
}

#[test]
fn desktop_menu_enter_preserves_confirmed_save_files_and_plain_enter_still_commits() {
    let dir = PreferenceDirectory::new();
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for front in [false, true] {
            for operation in 0..3 {
                if front && operation == 0 {
                    continue;
                }
                for code in [KeyCode::Enter, KeyCode::NumpadEnter] {
                    for modifiers in desktop_chords() {
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
                        app.modifiers_changed(modifiers);
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
fn desktop_menu_enter_keeps_navigation_inert_and_option_text_usable() {
    for modifiers in desktop_chords() {
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
            app.modifiers_changed(modifiers);
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

#[test]
fn desktop_menu_enter_command_delete_compositor_regression() {
    let dir = PreferenceDirectory::new();
    let mut deleted = 0;
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, own, _) = scene(size);
        app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(1600 * 1200 * 4)
            .collect();
        let path = dir.0.join("protected.oyrsave");
        let original = b"protected saved match";
        std::fs::write(&path, original).unwrap();
        app.save_path = dir.0.join("active.oyrsave");
        app.menu.slots = yr_app::menu::slots(&dir.0).unwrap();
        app.menu.set(Page::Confirm(Intent::Delete(path.clone())));
        app.menu.focus = 0;
        app.pending.push(Command::Stop { id: own });
        let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        let prefix = format!("command-enter-{}x{}", size.0, size.1);
        capture(&app, &format!("{prefix}-00-before"));
        app.modifiers_changed(ModifiersState::SUPER);
        event(
            &mut app,
            KeyCode::Enter,
            ElementState::Pressed,
            false,
            Some("\r"),
        );
        capture(&app, &format!("{prefix}-01-chord"));
        let preserved = std::fs::read(&path).is_ok_and(|bytes| bytes == original);
        deleted += usize::from(!preserved);
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            saved
        );
        assert_eq!(app.selected, BTreeSet::from([own]));
        assert_eq!(app.pending, [Command::Stop { id: own }]);
        println!("{size:?}: Command+Enter preserves protected file = {preserved}; full Session and queued orders unchanged");
        if preserved {
            app.modifiers_changed(ModifiersState::empty());
            event(
                &mut app,
                KeyCode::Enter,
                ElementState::Released,
                false,
                None,
            );
            event(
                &mut app,
                KeyCode::Enter,
                ElementState::Pressed,
                false,
                Some("\r"),
            );
            assert!(!path.exists());
            capture(&app, &format!("{prefix}-02-plain"));
        }
    }
    assert_eq!(
        deleted, 0,
        "Command+Enter must not commit permanent deletion"
    );
}

#[test]
fn desktop_menu_enter_keeps_command_name_editing_and_plain_setup_acceptance() {
    use yr_app::menu::setup::Field;
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for setup in [false, true] {
            let (mut app, _, _) = scene(size);
            app.menu.front = setup;
            app.menu
                .set(if setup { Page::Skirmish } else { Page::Save });
            if setup {
                app.menu.setup.focus = Some(Field::Name(0));
            } else {
                app.menu.name = "Replace me".into();
            }
            app.modifiers_changed(ModifiersState::SUPER);
            event(
                &mut app,
                KeyCode::KeyA,
                ElementState::Pressed,
                false,
                Some("a"),
            );
            assert!(if setup {
                app.menu.setup.name_selected
            } else {
                app.menu.name_selected
            });
            app.modifiers_changed(ModifiersState::empty());
            event(
                &mut app,
                KeyCode::KeyZ,
                ElementState::Pressed,
                false,
                Some("Zed"),
            );
            if setup {
                assert_eq!(app.menu.setup.config.players[0].name, "Zed");
                app.menu.setup.focus = Some(Field::Country(0));
                app.menu.setup.press(Field::Country(0), 287);
                assert!(app.menu.setup.popup.is_some());
                for modifiers in desktop_chords() {
                    app.modifiers_changed(modifiers);
                    event(
                        &mut app,
                        KeyCode::NumpadEnter,
                        ElementState::Pressed,
                        false,
                        None,
                    );
                    assert!(app.menu.setup.popup.is_some());
                    event(
                        &mut app,
                        KeyCode::NumpadEnter,
                        ElementState::Released,
                        false,
                        None,
                    );
                }
                app.modifiers_changed(ModifiersState::empty());
                event(
                    &mut app,
                    KeyCode::NumpadEnter,
                    ElementState::Pressed,
                    false,
                    None,
                );
                assert!(app.menu.setup.popup.is_none());
            } else {
                assert_eq!(app.menu.name, "Zed");
            }
        }
    }
}
