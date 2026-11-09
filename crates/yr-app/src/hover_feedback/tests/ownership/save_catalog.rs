use super::*;
use yr_app::menu::Intent;

#[test]
fn save_catalog_keeps_a_new_named_save_reachable_after_sixty_four_slots() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, _, _) = scene(size);
        let directory = PreferenceDirectory::new();
        let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        for index in 0..64 {
            std::fs::write(directory.0.join(format!("Slot {index:03}.oyrsave")), &saved).unwrap();
        }
        app.save_path = directory.0.join("quicksave.oyrsave");
        app.open_menu(Page::Save);
        app.menu.name.clear();
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyZ),
            ElementState::Pressed,
            Some("Z latest"),
            false,
            false,
        );
        key(&mut app, KeyCode::KeyZ, ElementState::Released);
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.menu.page, Page::Message { .. }));
        let latest = directory.0.join("Z latest.oyrsave");
        assert_eq!(std::fs::read(&latest).unwrap(), saved);
        assert_eq!(app.save_path, latest);
        if size == (640, 480) {
            proof(&app, "00-save-success");
        }
        for front in [false, true] {
            app.menu.front = front;
            for page in [Page::Load, Page::Delete] {
                app.open_menu(page.clone());
                assert_eq!(app.menu.slots.len(), 65);
                press_key(&mut app, KeyCode::End);
                assert_eq!(app.menu.selected, Some(64));
                assert_eq!(app.menu.scroll, 55);
                assert_eq!(app.menu.slots[64].path, latest);
                if size == (640, 480) && !front && page == Page::Load {
                    proof(&app, "01-last-slot-visible");
                }
                press_key(&mut app, KeyCode::PageUp);
                assert_eq!(app.menu.selected, Some(54));
                press_key(&mut app, KeyCode::PageDown);
                assert_eq!(app.menu.selected, Some(64));
                press_key(&mut app, KeyCode::Enter);
                assert_eq!(
                    app.menu.page,
                    Page::Confirm(if page == Page::Load {
                        Intent::Load(latest.clone())
                    } else {
                        Intent::Delete(latest.clone())
                    })
                );
                assert_eq!(app.menu.focus, usize::from(page == Page::Delete));
                if size == (640, 480) && !front {
                    proof(
                        &app,
                        if page == Page::Load {
                            "02-load-confirmation"
                        } else {
                            "03-delete-default-no"
                        },
                    );
                }
                press_key(&mut app, KeyCode::Escape);
                assert_eq!(app.menu.page, page);
            }
        }
        app.menu.front = false;
        app.open_menu(Page::Save);
        assert_eq!(app.menu.slots.len(), 65);
        app.pointer_moved((100.0, 100.0));
        app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -100.0));
        assert_eq!(app.menu.scroll, 55);
        let layout = yr_app::menu::Layout::for_state(&app.menu, size.0, size.1);
        let point = (
            layout.origin.0 + 60.0 * layout.scale,
            layout.origin.1 + (150.0 + 9.0 * 24.0 + 10.0) * layout.scale,
        );
        app.pointer_moved(point);
        assert_eq!(layout.slot(&app.menu, point), Some(64));
        click(&mut app);
        assert_eq!(app.menu.save_slot_path, Some(latest.clone()));
        assert_eq!(app.menu.selected, Some(64));
        assert_eq!(std::fs::read(&latest).unwrap(), saved);
        for index in 0..64 {
            assert_eq!(
                std::fs::read(directory.0.join(format!("Slot {index:03}.oyrsave"))).unwrap(),
                saved
            );
        }
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            saved
        );
        assert!(app.pending.is_empty());
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 65);
    }
    println!("Real Save creates the 65th named file; Load/Delete endpoint/page keys and confirmations plus Save pointer selection retain its exact path at three layouts, preserving every file and encoded Session.");
}

#[test]
fn save_catalog_retains_the_directory_scan_bound_and_no_follow_filtering() {
    let directory = PreferenceDirectory::new();
    // More valid files than the directory-entry scan budget: returned metadata
    // remains bounded even after lifting the smaller display-catalogue limit.
    for index in 0..4097 {
        std::fs::write(directory.0.join(format!("Slot {index:04}.oyrsave")), []).unwrap();
    }
    let slots = yr_app::menu::slots(&directory.0).unwrap();
    assert_eq!(slots.len(), 4096);
    assert!(slots.windows(2).all(|pair| pair[0].path < pair[1].path));
    let filtered = PreferenceDirectory::new();
    for index in 0..65 {
        std::fs::write(filtered.0.join(format!("File {index:03}.oyrsave")), []).unwrap();
    }
    std::fs::create_dir(filtered.0.join("directory.oyrsave")).unwrap();
    std::fs::write(filtered.0.join("unrelated.txt"), b"keep").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        filtered.0.join("File 000.oyrsave"),
        filtered.0.join("linked.oyrsave"),
    )
    .unwrap();
    assert_eq!(yr_app::menu::slots(&filtered.0).unwrap().len(), 65);
    assert_eq!(
        std::fs::read(filtered.0.join("unrelated.txt")).unwrap(),
        b"keep"
    );
    println!("Metadata catalogue keeps the existing 4096-entry directory scan bound, exact-path sorting and regular-file/no-follow filtering.");
}

fn proof(app: &App, name: &str) {
    let Some(path) = std::env::var_os("OPENYR_SAVE_CATALOG_PROOF") else {
        return;
    };
    let path = PathBuf::from(path);
    std::fs::create_dir_all(&path).unwrap();
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    std::fs::write(
        path.join(format!("{name}.png")),
        yr_render::png::encode_rgba(640, 480, &frame),
    )
    .unwrap();
    std::fs::write(path.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
        "page":format!("{:?}",app.menu.page),"slot_count":app.menu.slots.len(),
        "selected":app.menu.selected,"scroll":app.menu.scroll,"focus":app.menu.focus,
        "last_slot":app.menu.slots.last().map(|slot|slot.path.file_name().unwrap().to_string_lossy()),
        "snapshot_bytes":yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap().len(),
        "tick":app.session.as_ref().unwrap().sim.tick(),
        "sim_hash":format!("{:016x}",app.session.as_ref().unwrap().sim.state_hash()),"retail_art":false
    })).unwrap()).unwrap();
}
