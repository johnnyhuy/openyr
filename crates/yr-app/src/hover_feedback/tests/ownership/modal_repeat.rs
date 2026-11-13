use super::*;

fn repeat(app: &mut App, code: KeyCode, text: Option<&str>) {
    app.keyboard_input(
        PhysicalKey::Code(code),
        ElementState::Pressed,
        text,
        true,
        false,
    );
}

#[test]
fn modal_transition_requires_a_fresh_press_before_save_name_edits() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, own, _) = scene(size);
        let folder = PreferenceDirectory::new();
        app.save_path = folder.0.join("quicksave.oyrsave");
        yr_app::snapshot::save(app.session.as_ref().unwrap(), &app.save_path).unwrap();
        let bytes = std::fs::read(&app.save_path).unwrap();
        let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        key(&mut app, KeyCode::KeyZ, ElementState::Pressed);
        key(&mut app, KeyCode::Backspace, ElementState::Pressed);
        app.open_menu(Page::Save);
        app.menu.name = "Keep this name".into();
        repeat(&mut app, KeyCode::KeyZ, Some("z"));
        repeat(&mut app, KeyCode::Backspace, None);
        assert_eq!(app.menu.name, "Keep this name");
        // Check each repeat separately so append/backspace cannot cancel out.
        repeat(&mut app, KeyCode::KeyZ, Some("z"));
        assert_eq!(
            app.menu.name, "Keep this name",
            "held world text cannot enter a new editor"
        );
        capture(&app, &format!("modal-{}-ignored-repeat", size.0));
        key(&mut app, KeyCode::KeyZ, ElementState::Released);
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyZ),
            ElementState::Pressed,
            Some("z"),
            false,
            false,
        );
        repeat(&mut app, KeyCode::KeyZ, Some("z"));
        assert_eq!(
            app.menu.name, "Keep this namezz",
            "normal editing repeats stay available after a fresh press"
        );
        capture(&app, &format!("modal-{}-fresh-edit", size.0));
        assert_eq!(app.selected, BTreeSet::from([own]));
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            before
        );
        assert_eq!(std::fs::read(&app.save_path).unwrap(), bytes);
        assert!(app.pending.is_empty());
    }
    println!("Held world letters/backspace do not enter Save; fresh presses and same-editor repeats work at three layouts, preserving selection, saved files and encoded Sessions.");
}

fn press_menu_button(app: &mut App, button: yr_app::menu::Button) {
    let size = app.camera_size();
    let layout = yr_app::menu::Layout::for_state(&app.menu, size.0, size.1);
    let index = app
        .menu
        .buttons()
        .iter()
        .position(|b| b.0 == button)
        .unwrap();
    let (x, y, _, _) = layout.button(index, button);
    app.pointer_moved((
        layout.origin.0 + f64::from(x + 5) * layout.scale,
        layout.origin.1 + f64::from(y + 5) * layout.scale,
    ));
    click(app);
}

#[test]
fn menu_keys_do_not_rearm_world_pan_or_zoom_after_keyboard_or_pointer_resume() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for code in [
            KeyCode::ArrowUp,
            KeyCode::ArrowRight,
            KeyCode::Equal,
            KeyCode::Minus,
        ] {
            for pointer in [false, true] {
                let (mut app, own, _) = scene(size);
                app.pending.push(Command::Stop { id: own });
                let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                app.open_menu(Page::Pause);
                key(&mut app, code, ElementState::Pressed);
                if pointer {
                    press_menu_button(&mut app, yr_app::menu::Button::Resume);
                } else {
                    key(&mut app, KeyCode::Escape, ElementState::Pressed);
                }
                assert_eq!(app.menu.page, Page::Closed);
                let zoom = app.view.zoom;
                repeat(&mut app, code, None);
                assert_eq!(app.keyboard_pan(), (0.0, 0.0), "{code:?} pointer={pointer}");
                assert_eq!(
                    app.view.zoom, zoom,
                    "held zoom key cannot change the resumed view"
                );
                assert!(app.keys.is_empty());
                key(&mut app, code, ElementState::Released);
                key(&mut app, code, ElementState::Pressed);
                if matches!(code, KeyCode::Equal | KeyCode::Minus) {
                    assert_ne!(app.view.zoom, zoom);
                } else {
                    assert_ne!(app.keyboard_pan(), (0.0, 0.0));
                }
                assert_eq!(app.pending, [Command::Stop { id: own }]);
                assert_eq!(app.selected, BTreeSet::from([own]));
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    before
                );
            }
        }
    }
    println!("24 native menu-resume cases: held pan/zoom repeats are inert; fresh presses work; valid pending orders, selection and Session bytes survive both keyboard and pointer resume.");
}

#[test]
fn dropdown_close_suppresses_the_arrow_that_was_held_inside_it() {
    use yr_app::menu::setup::Field;
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, _, _) = scene(size);
        let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        app.menu.front = true;
        app.open_menu(Page::Skirmish);
        let layout = yr_app::menu::Layout::for_state(&app.menu, size.0, size.1);
        app.pointer_moved((
            layout.origin.0 + 300.0 * layout.scale,
            layout.origin.1 + 70.0 * layout.scale,
        ));
        click(&mut app);
        assert_eq!(app.menu.setup.popup.unwrap().field, Field::Country(0));
        key(&mut app, KeyCode::ArrowDown, ElementState::Pressed);
        // Choose the second country by pointer while Down remains held.
        app.pointer_moved((
            layout.origin.0 + 300.0 * layout.scale,
            layout.origin.1 + 117.0 * layout.scale,
        ));
        click(&mut app);
        assert!(app.menu.setup.popup.is_none());
        let chosen = app.menu.setup.config.players[0].country;
        repeat(&mut app, KeyCode::ArrowDown, None);
        assert_eq!(app.menu.setup.config.players[0].country, chosen);
        capture(&app, &format!("modal-{}-chosen-country", size.0));
        key(&mut app, KeyCode::ArrowDown, ElementState::Released);
        key(&mut app, KeyCode::ArrowDown, ElementState::Pressed);
        assert_ne!(app.menu.setup.config.players[0].country, chosen);
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            before
        );
        assert!(app.pending.is_empty());
    }
    println!("Native dropdown pointer selection survives a held navigation arrow; fresh arrows still adjust the field at three layouts without mutating the running Session.");
}

#[test]
fn focus_and_prepared_match_commit_do_not_revive_held_keys() {
    use crate::menu_flow::{LoadJob, Prepared};
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, _, _) = scene(size);
        key(&mut app, KeyCode::ArrowRight, ElementState::Pressed);
        app.focus_changed(false);
        app.focus_changed(true);
        repeat(&mut app, KeyCode::ArrowRight, None);
        assert_eq!(app.keyboard_pan(), (0.0, 0.0));
        key(&mut app, KeyCode::ArrowRight, ElementState::Released);
        let (job, sender, _) = LoadJob::channel(Page::Pause);
        app.load_job = Some(job);
        app.open_menu(Page::Loading);
        key(&mut app, KeyCode::ArrowRight, ElementState::Pressed);
        let fresh = session();
        let bytes = yr_app::snapshot::encode(&fresh).unwrap();
        sender
            .send(Ok(Prepared::Match {
                session: Box::new(fresh),
                map: app.map.take().unwrap(),
                map_name: "AUTHORED.MAP".into(),
                save_path: None,
                sprites: None,
            }))
            .ok()
            .unwrap();
        app.modifiers_changed(ModifiersState::ALT);
        app.poll_menu_load();
        assert!(!app.alt_key);
        assert_eq!(app.menu.page, Page::Closed);
        repeat(&mut app, KeyCode::ArrowRight, None);
        assert_eq!(app.keyboard_pan(), (0.0, 0.0));
        key(&mut app, KeyCode::ArrowRight, ElementState::Released);
        key(&mut app, KeyCode::ArrowRight, ElementState::Pressed);
        assert_ne!(app.keyboard_pan(), (0.0, 0.0));
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            bytes
        );
        assert!(app.pending.is_empty());
    }
    println!("Focus return without synthetic events and controlled prepared-match commits keep old repeats inert at three layouts; fresh presses work and committed Session bytes are exact. Retail reconstruction is not exercised.");
}
