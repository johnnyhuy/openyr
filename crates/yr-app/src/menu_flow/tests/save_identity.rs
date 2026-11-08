use super::*;

fn save_app(dir: &std::path::Path) -> App {
    let mut app = new_app(String::new(), String::new(), 1);
    app.save_path = dir.join("quicksave.oyrsave");
    app.frame_size = (640, 480);
    app.open_menu(Page::Save);
    app
}

fn capture(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_SAVE_IDENTITY_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (width, height) = app.frame_size;
    let mut frame = vec![0; (width * height * 4) as usize];
    app.compose(&mut frame, width, height);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(width, height, &frame),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "session_present": app.session.is_some(), "retail_art": false,
            "page": format!("{:?}", app.menu.page), "display_name": app.menu.name,
            "selected": app.menu.selected, "selected_path": app.menu.save_slot_path,
            "pending": format!("{:?}", app.pending),
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn selecting_a_save_keeps_the_actual_path_through_confirmation_and_decline() {
    for basename in [
        format!("{}-chosen.oyrsave", "W".repeat(80)),
        " spaced .oyrsave".into(),
        ".hidden.oyrsave".into(),
    ] {
        let temp = Scratch::new();
        let chosen = temp.0.join(&basename);
        let sibling_name = if basename.starts_with(' ') {
            "spaced.oyrsave".to_string()
        } else {
            format!("{}.oyrsave", "W".repeat(48))
        };
        let sibling = temp.0.join(sibling_name);
        std::fs::write(&chosen, b"chosen bytes").unwrap();
        std::fs::write(&sibling, b"protected sibling bytes").unwrap();
        let mut app = save_app(&temp.0);
        let index = app
            .menu
            .slots
            .iter()
            .position(|slot| slot.path == chosen)
            .unwrap();
        app.menu.select_slot(index);
        assert!(app
            .menu
            .buttons()
            .iter()
            .any(|&(button, enabled)| button == Button::Save && enabled));
        if basename.starts_with('W') {
            capture(&app, "01-selected");
        }
        for attempt in 0..2 {
            app.apply_menu_effect(Effect::RequestSave);
            if attempt == 0 && basename.starts_with('W') {
                capture(&app, "02-confirm-selected");
            }
            assert_eq!(app.menu.page, Page::Confirm(Intent::Save(chosen.clone())));
            assert_eq!(
                app.menu.activate(Button::Accept),
                Effect::Commit(Intent::Save(chosen.clone()))
            );
            app.menu_key_code(KeyCode::Escape, None, false);
            assert_eq!(app.menu.page, Page::Save);
            if attempt == 0 && basename.starts_with('W') {
                capture(&app, "03-declined");
            }
            assert_eq!(std::fs::read(&chosen).unwrap(), b"chosen bytes");
            assert_eq!(std::fs::read(&sibling).unwrap(), b"protected sibling bytes");
        }
        assert!(app.pending.is_empty() && app.load_job.is_none());
    }
    println!("Selected long/spaced basenames retain their exact confirmation and Commit target; decline preserves sibling bytes.");
}

#[test]
fn accepted_name_edits_clear_selected_identity_but_ignored_input_does_not() {
    let temp = Scratch::new();
    let chosen = temp.0.join(format!("{}-chosen.oyrsave", "W".repeat(80)));
    let sibling = temp.0.join("Replacement.oyrsave");
    let prefix = temp.0.join(format!("{}.oyrsave", "W".repeat(48)));
    std::fs::write(&chosen, b"chosen bytes").unwrap();
    std::fs::write(&sibling, b"protected sibling bytes").unwrap();
    std::fs::write(&prefix, b"protected prefix bytes").unwrap();
    let mut app = save_app(&temp.0);
    let index = app
        .menu
        .slots
        .iter()
        .position(|slot| slot.path == chosen)
        .unwrap();
    app.menu.select_slot(index);
    app.menu_key_code(KeyCode::KeyN, Some("\n\t/\\:"), false);
    app.menu_key_code(KeyCode::KeyZ, Some("Z"), false); // Full 48-character label: no accepted edit.
    assert_eq!(app.menu.selected_save_path(), Some(chosen.as_path()));
    app.control = true;
    app.menu_key_code(KeyCode::KeyA, Some("a"), false);
    app.menu_key_code(KeyCode::KeyC, Some("c"), false);
    app.control = false;
    assert_eq!(app.menu.selected_save_path(), Some(chosen.as_path()));
    app.menu_key_code(KeyCode::KeyR, Some("Replacement"), false);
    assert!(app.menu.save_slot_path.is_none() && app.menu.selected.is_none());
    app.apply_menu_effect(Effect::RequestSave);
    assert_eq!(app.menu.page, Page::Confirm(Intent::Save(sibling.clone())));
    capture(&app, "04-edited-confirm");
    assert_eq!(
        app.menu.activate(Button::Accept),
        Effect::Commit(Intent::Save(sibling.clone()))
    );
    app.menu_key_code(KeyCode::Escape, None, false);
    assert_eq!(app.menu.name, "Replacement");
    assert_eq!(std::fs::read(&chosen).unwrap(), b"chosen bytes");
    assert_eq!(std::fs::read(&sibling).unwrap(), b"protected sibling bytes");
    app.menu.select_slot(index);
    app.control = true;
    app.menu_key_code(KeyCode::KeyA, Some("a"), false);
    app.control = false;
    // Re-entering an identical display label is still an explicit new-name edit.
    app.menu_key_code(KeyCode::KeyW, Some(&"W".repeat(48)), false);
    app.apply_menu_effect(Effect::RequestSave);
    assert_eq!(app.menu.page, Page::Confirm(Intent::Save(prefix.clone())));
    assert_eq!(
        app.menu.activate(Button::Accept),
        Effect::Commit(Intent::Save(prefix.clone()))
    );
    app.menu_key_code(KeyCode::Escape, None, false);
    assert_eq!(std::fs::read(&chosen).unwrap(), b"chosen bytes");
    assert_eq!(std::fs::read(&prefix).unwrap(), b"protected prefix bytes");
    app.menu.select_slot(index);
    app.menu_key_code(KeyCode::Backspace, None, false);
    assert!(app.menu.save_slot_path.is_none() && app.menu.selected.is_none());
    app.menu.select_slot(index);
    app.menu.set(Page::Pause);
    app.menu.set(Page::Save);
    assert!(
        app.menu.save_slot_path.is_none()
            && app.menu.selected.is_none()
            && app.menu.name.is_empty()
    );
    app.menu.select_slot(index);
    app.apply_menu_effect(Effect::RequestSave);
    app.open_menu(Page::Save); // A real list refresh must not retain old selection identity.
    assert!(app.menu.save_slot_path.is_none() && app.menu.selected.is_none());
    assert_eq!(app.menu.name, "Match 0");
    println!("Only accepted edits create a fresh save name (even an identical label); ignored input keeps identity and unrelated pages/list refreshes discard it.");
}

#[test]
fn stale_selected_targets_do_not_fall_back_to_a_sibling_or_outside_path() {
    for replacement in 0..5 {
        let temp = Scratch::new();
        let chosen = temp.0.join(format!("{}-chosen.oyrsave", "W".repeat(80)));
        let sibling = temp.0.join(format!("{}.oyrsave", "W".repeat(48)));
        let protected = temp.0.join("protected.oyrsave");
        std::fs::write(&chosen, b"chosen bytes").unwrap();
        std::fs::write(&sibling, b"sibling bytes").unwrap();
        std::fs::write(&protected, b"outside target bytes").unwrap();
        let mut app = save_app(&temp.0);
        let index = app
            .menu
            .slots
            .iter()
            .position(|slot| slot.path == chosen)
            .unwrap();
        app.menu.select_slot(index);
        std::fs::remove_file(&chosen).unwrap();
        match replacement {
            0 => {}
            1 => std::fs::create_dir(&chosen).unwrap(),
            #[cfg(unix)]
            2 => std::os::unix::fs::symlink(&protected, &chosen).unwrap(),
            #[cfg(not(unix))]
            2 => {}
            3 => {
                app.menu.slots[index].path = protected.clone();
                app.menu.save_slot_path = Some(protected.clone());
                app.save_path = temp.0.join("different-folder").join("quicksave.oyrsave");
            }
            _ => app.menu.slots.clear(),
        }
        app.apply_menu_effect(Effect::RequestSave);
        assert!(matches!(app.menu.page, Page::Message { .. }));
        app.menu_key_code(KeyCode::Escape, None, false);
        assert_eq!(app.menu.page, Page::Save);
        assert!(
            app.menu.save_slot_path.is_none()
                && app.menu.selected.is_none()
                && app.menu.name.is_empty()
        );
        assert_eq!(std::fs::read(&sibling).unwrap(), b"sibling bytes");
        assert_eq!(std::fs::read(&protected).unwrap(), b"outside target bytes");
        assert!(app.pending.is_empty() && app.load_job.is_none());
    }
    println!("Missing/directory/symlink/outside selected targets recover without reconstructing a sibling path or changing protected bytes.");
}

#[cfg(target_os = "linux")]
#[test]
fn a_non_utf8_selected_name_remains_a_path_not_a_lossy_label() {
    use std::os::unix::ffi::OsStringExt;
    let temp = Scratch::new();
    let mut name = vec![0xff; 24];
    name.extend_from_slice(b".oyrsave");
    let chosen = temp.0.join(std::ffi::OsString::from_vec(name));
    std::fs::write(&chosen, b"raw-name bytes").unwrap();
    let mut app = save_app(&temp.0);
    app.menu.select_slot(0);
    assert!(app
        .menu
        .buttons()
        .iter()
        .any(|&(button, enabled)| button == Button::Save && enabled));
    app.apply_menu_effect(Effect::RequestSave);
    assert_eq!(app.menu.page, Page::Confirm(Intent::Save(chosen.clone())));
    assert_eq!(
        app.menu.activate(Button::Accept),
        Effect::Commit(Intent::Save(chosen.clone()))
    );
    app.menu_key_code(KeyCode::Escape, None, false);
    assert_eq!(std::fs::read(chosen).unwrap(), b"raw-name bytes");
    println!("Linux non-UTF-8 basenames retain the exact selected Path/OsStr through confirmation and Commit intent.");
}

#[test]
fn mismatched_in_memory_selection_cannot_derive_or_retain_an_overwrite_target() {
    for mutation in 0..3 {
        let temp = Scratch::new();
        let chosen = temp.0.join(format!("{}-chosen.oyrsave", "W".repeat(80)));
        let sibling = temp.0.join("Replacement.oyrsave");
        std::fs::write(&chosen, b"chosen bytes").unwrap();
        std::fs::write(&sibling, b"protected sibling bytes").unwrap();
        let mut app = save_app(&temp.0);
        let index = app
            .menu
            .slots
            .iter()
            .position(|slot| slot.path == chosen)
            .unwrap();
        app.menu.select_slot(index);
        match mutation {
            0 => app.menu.name = "Replacement".into(),
            1 => app.menu.slots[index].path = sibling.clone(),
            _ => {
                app.menu.set(Page::Confirm(Intent::Save(sibling.clone())));
                assert!(
                    app.menu.save_slot_path.is_none()
                        && app.menu.selected.is_none()
                        && app.menu.name.is_empty()
                );
                app.menu_key_code(KeyCode::Escape, None, false);
                assert_eq!(app.menu.page, Page::Save);
            }
        }
        assert!(!app
            .menu
            .buttons()
            .iter()
            .any(|&(button, enabled)| button == Button::Save && enabled));
        if mutation < 2 {
            app.apply_menu_effect(Effect::RequestSave);
            assert!(matches!(app.menu.page, Page::Message { .. }));
            app.menu_key_code(KeyCode::Escape, None, false);
        }
        assert!(
            app.menu.save_slot_path.is_none()
                && app.menu.selected.is_none()
                && app.menu.name.is_empty()
        );
        assert_eq!(std::fs::read(chosen).unwrap(), b"chosen bytes");
        assert_eq!(std::fs::read(sibling).unwrap(), b"protected sibling bytes");
    }
    println!("Direct editor/slot mismatch and unrelated Save confirmations cannot reuse the old selected identity or derive a sibling target.");
}
