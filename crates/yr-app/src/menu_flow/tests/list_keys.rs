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

#[test]
fn saved_list_keys_page_and_reveal_exact_confirmation_targets() {
    for front in [false, true] {
        for size in [(640, 480), (800, 600), (1280, 800)] {
            for page in [Page::Load, Page::Delete] {
                let temp = Scratch::new();
                for i in 0..25 {
                    std::fs::write(temp.0.join(format!("Match {i:02}.oyrsave")), [i]).unwrap();
                }
                let mut app = new_app(String::new(), String::new(), 1);
                app.save_path = temp.0.join("active.oyrsave");
                app.frame_size = size;
                app.menu.front = front;
                app.open_menu(page.clone());
                key(&mut app, KeyCode::End);
                assert_eq!(app.menu.selected, Some(24));
                assert_eq!(app.menu.scroll, 15);
                capture(&app, "00-end");
                key(&mut app, KeyCode::PageUp);
                assert_eq!(app.menu.selected, Some(14));
                assert!(app.menu.scroll <= 14 && 14 < app.menu.scroll + 10);
                capture(&app, "01-page-up");
                key(&mut app, KeyCode::Home);
                assert_eq!(app.menu.selected, Some(0));
                assert_eq!(app.menu.scroll, 0);
                capture(&app, "02-home");
                key(&mut app, KeyCode::PageDown);
                assert_eq!(app.menu.selected, Some(10));
                assert_eq!(app.menu.scroll, 1);
                capture(&app, "03-page-down");
                let expected = temp.0.join("Match 10.oyrsave");
                key(&mut app, KeyCode::Enter);
                assert_eq!(
                    app.menu.page,
                    Page::Confirm(if page == Page::Load {
                        Intent::Load(expected)
                    } else {
                        Intent::Delete(expected)
                    })
                );
                assert_eq!(app.menu.focus, usize::from(page == Page::Delete));
                key(&mut app, KeyCode::Escape);
                assert_eq!(app.menu.page, page);
                for i in 0..25 {
                    assert_eq!(
                        std::fs::read(temp.0.join(format!("Match {i:02}.oyrsave"))).unwrap(),
                        [i]
                    );
                }
                assert!(app.pending.is_empty());
            }
        }
    }
}

#[test]
fn saved_list_keys_handle_short_empty_and_unselected_lists() {
    for count in [0usize, 1, 7, 10, 11] {
        for page in [Page::Load, Page::Delete] {
            let mut app = new_app(String::new(), String::new(), 1);
            app.menu.set(page.clone());
            app.menu.slots = (0..count)
                .map(|i| yr_app::menu::Slot {
                    name: format!("Slot {i}"),
                    path: format!("Slot {i}.oyrsave").into(),
                })
                .collect();
            key(&mut app, KeyCode::PageDown);
            assert_eq!(app.menu.selected, if count == 0 { None } else { Some(0) });
            key(&mut app, KeyCode::End);
            assert_eq!(app.menu.selected, count.checked_sub(1));
            assert_eq!(app.menu.scroll, count.saturating_sub(10));
            key(&mut app, KeyCode::PageDown);
            assert_eq!(app.menu.selected, count.checked_sub(1));
            key(&mut app, KeyCode::Home);
            assert_eq!(app.menu.selected, if count == 0 { None } else { Some(0) });
            key(&mut app, KeyCode::PageUp);
            assert_eq!(app.menu.scroll, 0);
            assert_eq!(app.menu.page, page);
        }
    }
}

#[test]
fn saved_list_keys_obey_focus_guards_and_leave_save_editor_alone() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.menu.set(Page::Load);
    app.menu.slots = (0..25)
        .map(|i| yr_app::menu::Slot {
            name: format!("Slot {i}"),
            path: format!("Slot {i}.oyrsave").into(),
        })
        .collect();
    app.menu.select_slot(2);
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::End),
        ElementState::Pressed,
        None,
        false,
        true,
    );
    assert_eq!(app.menu.selected, Some(2));
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::End),
        ElementState::Pressed,
        None,
        true,
        false,
    );
    assert_eq!(app.menu.selected, Some(2));
    app.focus_changed(false);
    key(&mut app, KeyCode::End);
    assert_eq!(app.menu.selected, Some(2));
    app.focus_changed(true);
    key(&mut app, KeyCode::End);
    assert_eq!(app.menu.selected, Some(24));
    key(&mut app, KeyCode::Tab);
    assert_eq!(app.menu.selected, Some(24));
    app.menu.set(Page::Save);
    app.menu.name = "checkpoint".into();
    app.menu.select_slot(2);
    let name = app.menu.name.clone();
    let path = app.menu.save_slot_path.clone();
    for code in [
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
    ] {
        key(&mut app, code);
        assert_eq!(app.menu.name, name);
        assert_eq!(app.menu.selected, Some(2));
        assert_eq!(app.menu.save_slot_path, path);
    }
}

fn capture(app: &App, name: &str) {
    if !app.menu.front || app.frame_size != (640, 480) || app.menu.page != Page::Load {
        return;
    }
    let Some(dir) = std::env::var_os("OPENYR_LIST_KEYS_PROOF") else {
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
            "selected":app.menu.selected, "scroll":app.menu.scroll,
            "path": app.menu.selected.map(|i| &app.menu.slots[i].path),
            "session_present":app.session.is_some(), "retail_art":false,
        }))
        .unwrap(),
    )
    .unwrap();
}
