use super::*;
use yr_app::menu::Button;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "openyr-menu-flow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn click(app: &mut App, button: Button) {
    let index = app
        .menu
        .buttons()
        .iter()
        .position(|b| b.0 == button)
        .unwrap();
    let layout = Layout::for_state(&app.menu, WIN_W, WIN_H);
    let (x, y, _, _) = layout.button(index, button);
    app.cursor = (
        layout.origin.0 + f64::from(x + 10) * layout.scale,
        layout.origin.1 + f64::from(y + 10) * layout.scale,
    );
    app.menu_mouse(ElementState::Pressed, MouseButton::Left);
    app.menu_mouse(ElementState::Released, MouseButton::Left);
}
fn proof(app: &App, name: &str) {
    let Some(out) = std::env::var_os("OPENYR_MENU_PROOF") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let session = app.session.as_ref().unwrap();
    let hash = session.sim.state_hash();
    let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose(&mut frame, WIN_W, WIN_H);
    std::fs::write(
        out.join(format!("{name}.png")),
        yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
    )
    .unwrap();
    std::fs::write(
        out.join(format!("{name}.txt")),
        format!(
            "page={:?}\ntick={}\nhash={hash:016x}\nzoom={}\nmuted={}\npending={}\n",
            app.menu.page,
            session.sim.tick(),
            app.menu.zoom,
            app.menu.muted,
            app.pending.len()
        ),
    )
    .unwrap();
    assert_eq!(
        hash,
        session.sim.state_hash(),
        "drawing never changes the match"
    );
}
fn finish_load(app: &mut App) {
    // Retail reconstruction competes with corpus/capture tests. Match the
    // front-menu loader's bounded budget while retaining every state check.
    let deadline = Instant::now() + Duration::from_secs(180);
    while app.load_job.is_some() {
        app.poll_menu_load();
        assert!(Instant::now() < deadline, "match load timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn retail_saved_load_success_is_visible_only_after_committing_the_match() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let temp = Scratch::new();
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let path = temp.0.join("Checkpoint 07.oyrsave");
    std::fs::write(&path, &saved).unwrap();
    app.start_menu_load(Some(path.clone()));
    assert!(app.order_feedback.message(Instant::now()).is_none());
    finish_load(&mut app);
    assert!(!app.menu.visible());
    assert_eq!(app.save_path, path);
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Match loaded (tick 1): Checkpoint 07")
    );
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    assert!(app.selected.is_empty() && app.pending.is_empty());
    let now = Instant::now();
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (w, h) = size;
        app.frame_size = size;
        let mut with = vec![0; (w * h * 4) as usize];
        app.compose_at(&mut with, w, h, now);
        let notice = std::mem::take(&mut app.order_feedback);
        let mut without = vec![0; with.len()];
        app.compose_at(&mut without, w, h, now);
        let differences: Vec<_> = with
            .as_chunks::<4>()
            .0
            .iter()
            .zip(without.as_chunks::<4>().0)
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some((i as u32 % w, i as u32 / w)))
            .collect();
        assert!(
            !differences.is_empty(),
            "load confirmation must actually paint"
        );
        assert!(differences
            .iter()
            .all(|&(x, y)| x < w - SIDEBAR_W && y < 64));
        if let Some(out) = std::env::var_os("OPENYR_LOAD_SUCCESS_PROOF") {
            let out = PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            for (label, frame) in [("notice", &with), ("cleared", &without)] {
                std::fs::write(
                    out.join(format!("{w}x{h}-{label}.png")),
                    yr_render::png::encode_rgba(w, h, frame),
                )
                .unwrap();
            }
            std::fs::write(out.join("fixture.oyrsave"), &saved).unwrap();
        }
        app.order_feedback = notice;
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            saved
        );
    }
    app.open_menu(Page::Pause);
    assert!(app.order_feedback.message(Instant::now()).is_none());
    // A failed load cannot report success or replace the retained match.
    let corrupt = temp.0.join("Damaged.oyrsave");
    std::fs::write(&corrupt, b"damaged").unwrap();
    app.start_menu_load(Some(corrupt));
    finish_load(&mut app);
    assert!(matches!(app.menu.page, Page::Message { .. }));
    assert!(app.order_feedback.message(Instant::now()).is_none());
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    app.start_menu_load(Some(path.clone()));
    app.apply_menu_effect(Effect::CancelLoad);
    finish_load(&mut app);
    assert!(app.order_feedback.message(Instant::now()).is_none());
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    // Restart/new setup uses the same prepared-match path without a saved slot.
    app.start_menu_load(None);
    finish_load(&mut app);
    assert!(app.order_feedback.message(Instant::now()).is_none());
    app.menu.front = true;
    app.menu.set(Page::Load);
    app.apply_menu_effect(Effect::Commit(Intent::Load(path)));
    finish_load(&mut app);
    assert!(!app.menu.front && !app.menu.visible());
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Match loaded (tick 1): Checkpoint 07")
    );
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    println!("Pause/front saved load confirmation painted at three sizes after commit; failure/cancel/restart stayed silent; complete restored save bytes unchanged.");
}

#[test]
fn retail_long_save_name_saves_under_complete_filename_with_unchanged_bytes() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let temp = Scratch::new();
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.save_path = temp.0.join("quicksave.oyrsave");
    app.preferences_path = Some(temp.0.join("interface.json"));
    app.session = Some(app.load_session().unwrap());
    app.menu_art = yr_app::menu::Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
    let expected = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    app.open_menu(Page::Pause);
    click(&mut app, Button::Save);
    assert_eq!(app.menu.page, Page::Save);
    app.menu.name.clear();
    let long_a = format!("{}A", "W".repeat(47));
    app.menu_key_code(KeyCode::KeyW, Some(long_a.as_str()), false);
    assert_eq!(app.menu.name, long_a);
    app.menu_key_code(KeyCode::Backspace, None, false);
    assert_eq!(app.menu.name, "W".repeat(47));
    app.menu_key_code(KeyCode::KeyB, Some("B"), false);
    assert!(app.menu.name.ends_with('B'));
    app.menu_key_code(KeyCode::Backspace, None, false);
    app.menu_key_code(KeyCode::KeyA, Some("A"), false);
    assert_eq!(app.menu.name, long_a);
    click(&mut app, Button::Save);
    let saved = temp.0.join(format!("{long_a}.oyrsave"));
    assert!(saved.is_file(), "save must use the complete stored name");
    assert_eq!(std::fs::read(&saved).unwrap(), expected);
    assert_eq!(app.menu.name, long_a, "saving keeps the full stored name");
    assert!(app.menu.slots.iter().any(|slot| slot.name == long_a));
    let short = temp.0.join("short.oyrsave");
    app.save_menu_path(short.clone());
    assert_eq!(
        std::fs::read(&short).unwrap(),
        expected,
        "filename choice must not change match bytes"
    );
    println!("Long save name saved under its complete filename with unchanged match bytes.");
}

#[test]
fn retail_quicksave_clears_modal_input_on_success_and_failure() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let temp = Scratch::new();
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.session.as_mut().unwrap().sim.set_ai(None);
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.menu_art = yr_app::menu::Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
    let id = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.hp > 0)
        .unwrap()
        .id;
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let blocked = temp.0.join("not-a-directory");
    std::fs::write(&blocked, b"keep").unwrap();
    for failed in [false, true] {
        for mode in ["attack-move", "repair", "sell", "placement"] {
            app.menu.set(Page::Closed);
            app.pointer.focused = true;
            app.selected = [id].into();
            app.control = true;
            app.group_key(
                KeyCode::Digit1,
                ElementState::Pressed,
                false,
                Instant::now(),
            );
            assert!(app.groups.message(Instant::now()).is_some());
            app.pending = vec![Command::Stop { id }];
            let pending = app.pending.clone();
            app.attack_move = mode == "attack-move";
            app.placing = (mode == "placement").then_some(0);
            app.service_mode = match mode {
                "repair" => ServiceMode::Repair,
                "sell" => ServiceMode::Sell,
                _ => ServiceMode::None,
            };
            app.shift = true;
            app.control = true;
            app.keys = [KeyCode::ArrowRight, KeyCode::KeyA].into();
            app.left_down = Some((200.0, 200.0));
            app.right_down = Some((200.0, 200.0));
            app.right_panning = true;
            app.radar_drag = true;
            let old_target = temp.0.join("previous.oyrsave");
            app.save_path = old_target.clone();
            let path = if failed {
                blocked.join("save.oyrsave")
            } else {
                temp.0.join(format!("{mode}.oyrsave"))
            };
            // This is the same entry point as the live F5 handler.
            app.save_menu_path(path.clone());
            assert!(matches!(app.menu.page, Page::Message { .. }));
            assert!(
                !app.attack_move && app.placing.is_none() && app.service_mode == ServiceMode::None,
                "quicksave must cancel {mode} on both save outcomes"
            );
            assert!(!app.shift && !app.control && app.keys.is_empty());
            assert!(app.left_down.is_none() && app.right_down.is_none());
            assert!(!app.right_panning && !app.radar_drag);
            assert!(app.groups.message(Instant::now()).is_none());
            assert_eq!(app.selected, [id].into());
            assert_eq!(app.pending, pending);
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                saved
            );
            assert!(!app.advance_clock(Instant::now() + Duration::from_secs(3600)));
            if failed {
                assert_eq!(app.save_path, old_target);
                assert_eq!(std::fs::read(&blocked).unwrap(), b"keep");
            } else {
                assert_eq!(app.save_path, path);
                assert_eq!(std::fs::read(&path).unwrap(), saved);
            }
            if let Some(out) = std::env::var_os("OPENYR_QUICKSAVE_PROOF") {
                let out = PathBuf::from(out);
                std::fs::create_dir_all(&out).unwrap();
                if !failed && mode == "attack-move" {
                    std::fs::write(out.join("fixture.oyrsave"), &saved).unwrap();
                }
                for (w, h) in [(640, 480), (1280, 800)] {
                    let mut frame = vec![0; (w * h * 4) as usize];
                    app.compose(&mut frame, w, h);
                    std::fs::write(
                        out.join(format!("{w}x{h}-{mode}-failed-{failed}.png")),
                        yr_render::png::encode_rgba(w, h, &frame),
                    )
                    .unwrap();
                }
            }
            app.menu_key_code(KeyCode::Escape, None, false);
            assert_eq!(app.menu.page, if failed { Page::Save } else { Page::Pause });
            if failed {
                app.menu_key_code(KeyCode::Escape, None, false);
            }
            app.menu_key_code(KeyCode::Escape, None, false);
            assert!(!app.menu.visible());
            app.left_release();
            app.right_release();
            assert_eq!(app.pending, pending, "stale releases cannot add an order");
            assert_eq!(app.selected, [id].into());
            app.selected.clear();
            app.group_key(
                KeyCode::Digit1,
                ElementState::Pressed,
                false,
                Instant::now(),
            );
            assert_eq!(
                app.selected,
                [id].into(),
                "assigned teams survive menu entry"
            );
        }
    }
    println!("Quicksave success/failure clears four modes and held gestures, preserves selection/teams, pending orders and full save bytes; Escape recovery and stale releases passed.");
}

#[test]
fn retail_pause_recovery_scrolling_keeps_the_match_and_routes_inputs() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let temp = Scratch::new();
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.menu_art = yr_app::menu::Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let original_target = temp.0.join("quicksave.oyrsave");
    app.save_path = original_target.clone();
    let mut blocked = temp.0.clone();
    for _ in 0..5 {
        blocked.push("diagnostic-folder-with-a-long-name-".repeat(3));
    }
    blocked.push("directory.oyrsave");
    std::fs::create_dir_all(&blocked).unwrap();
    app.save_menu_path(blocked.clone());
    let Page::Message { text, .. } = &app.menu.page else {
        panic!("failed save must expose a recovery message")
    };
    assert!(text.len() > 400);
    assert_eq!(app.save_path, original_target);
    // Exercise an overflowing diagnostic independently of OS error wording.
    let diagnostic = format!(
        "{text}\n\n{}\nEND: select another save or return to the match.",
        "The current match is still in memory. ".repeat(250)
    );
    for size in [(640, 480), (800, 600), (1280, 800)] {
        app.frame_size = size;
        app.menu.message(diagnostic.clone(), Page::Save);
        app.pointer_moved((100.0, 100.0));
        capture_recovery(&app, "first");
        app.menu_key_code(KeyCode::ArrowDown, None, false);
        assert_eq!(app.menu.message_scroll, 1);
        app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
        assert!(app.menu.message_scroll > 1);
        app.menu_key_code(KeyCode::PageDown, None, true);
        app.menu_key_code(KeyCode::End, None, false);
        let end = app.menu.message_scroll;
        assert!(end > 10);
        capture_recovery(&app, "last");
        app.menu_key_code(KeyCode::End, None, true);
        assert_eq!(app.menu.message_scroll, end);
        app.menu_key_code(KeyCode::PageUp, None, false);
        assert!(app.menu.message_scroll < end);
        app.menu_key_code(KeyCode::Home, None, false);
        assert_eq!(app.menu.message_scroll, 0);
        app.menu_key_code(KeyCode::ArrowUp, None, true);
        assert_eq!(app.menu.message_scroll, 0);
        assert!(!app.advance_clock(Instant::now() + Duration::from_secs(3600)));
        app.menu_key_code(KeyCode::Escape, None, false);
        assert_eq!(app.menu.page, Page::Save);
        assert!(app.pending.is_empty());
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            saved
        );
    }
    assert!(blocked.is_dir());
    println!("Pause recovery: real >400-byte save error, full diagnostic navigation at three layouts, Escape recovery and complete saved-match bytes preserved.");
}

fn capture_recovery(app: &App, name: &str) {
    let Some(out) = std::env::var_os("OPENYR_RECOVERY_PROOF") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let (w, h) = app.camera_size();
    let mut frame = vec![0; (w * h * 4) as usize];
    app.compose(&mut frame, w, h);
    std::fs::write(
        out.join(format!("{w}x{h}-{name}.png")),
        yr_render::png::encode_rgba(w, h, &frame),
    )
    .unwrap();
    std::fs::write(
        out.join(format!("{w}x{h}-{name}.txt")),
        format!(
            "scroll={}\npage={:?}\n",
            app.menu.message_scroll, app.menu.page
        ),
    )
    .unwrap();
}

#[test]
fn preferences_are_bounded_and_errors_remain_recoverable() {
    let temp = Scratch::new();
    let path = temp.0.join("interface.json");
    let mut app = new_app(String::new(), String::new(), 1);
    app.read_preferences_file(path.clone());
    app.view.zoom = 1.5;
    app.apply_menu_effect(Effect::Mute);
    app.apply_menu_effect(Effect::EdgeScroll);
    let mut reopened = new_app(String::new(), String::new(), 1);
    reopened.read_preferences_file(path.clone());
    assert!(reopened.menu.muted);
    assert_eq!(reopened.view.zoom, 1.5);
    assert!(reopened.menu.edge_scroll);
    // Existing preferences predate the optional scrolling field.
    std::fs::write(&path, br#"{"muted":false,"zoom":125}"#).unwrap();
    reopened.read_preferences_file(path.clone());
    assert!(!reopened.menu.edge_scroll);
    assert_eq!(reopened.view.zoom, 1.25);
    std::fs::write(&path, br#"{"muted":true,"zoom":150,"edge_scroll":true}"#).unwrap();
    reopened.read_preferences_file(path.clone());
    for bytes in [
        br#"{"muted":false,"zoom":0}"#.to_vec(),
        vec![b' '; 16 * 1024 + 1],
        br#"{"muted":false,"zoom":100,"extra":1}"#.to_vec(),
        br#"{"muted":false,"zoom":100,"edge_scroll":1}"#.to_vec(),
    ] {
        std::fs::write(&path, bytes).unwrap();
        reopened.read_preferences_file(path.clone());
        assert!(matches!(reopened.menu.page, Page::Message { .. }));
        let effect = reopened.menu.escape();
        reopened.apply_menu_effect(effect);
        assert_eq!(reopened.menu.page, Page::Pause);
        assert_eq!(reopened.view.zoom, 1.5);
    }
    app.save_path = PathBuf::from("relative.oyrsave");
    assert_eq!(app.save_directory(), std::path::Path::new("."));
}

#[test]
fn escape_cancels_commands_before_pause_and_modal_clicks_do_not_become_orders() {
    let temp = Scratch::new();
    let mut app = new_app(String::new(), String::new(), 1);
    app.save_path = temp.0.join("quicksave.oyrsave");
    app.selected.insert(7);
    for mode in [0, 1, 2] {
        match mode {
            0 => app.service_mode = ServiceMode::Repair,
            1 => app.placing = Some(1),
            _ => app.attack_move = true,
        }
        app.left_down = Some((10.0, 10.0));
        app.escape_command_or_pause();
        assert!(!app.menu.visible());
        assert!(app.left_down.is_none());
        assert_eq!(app.selected.len(), 1);
    }
    app.keys.insert(KeyCode::ArrowRight);
    app.shift = true;
    app.control = true;
    app.right_down = Some((10.0, 10.0));
    app.escape_command_or_pause();
    assert_eq!(app.menu.page, Page::Pause);
    assert!(app.keys.is_empty() && !app.shift && !app.control && app.right_down.is_none());
    app.cursor = (500.0, 500.0);
    app.menu_mouse(ElementState::Pressed, MouseButton::Left);
    app.menu_mouse(ElementState::Released, MouseButton::Left);
    app.menu_key_code(KeyCode::KeyA, Some("a"), false);
    app.menu_key_code(KeyCode::KeyS, Some("s"), false);
    assert!(app.pending.is_empty() && !app.attack_move);
    click(&mut app, Button::Controls);
    app.focus_changed(false);
    app.menu_mouse(ElementState::Released, MouseButton::Left);
    assert_eq!(app.menu.page, Page::Controls);
    click(&mut app, Button::Back);
    click(&mut app, Button::Resume);
    assert!(!app.menu.visible());
    assert!(app.keys.is_empty() && app.left_down.is_none());
}

#[test]
fn delete_slots_requires_confirmation_and_refreshes_scrolling_in_both_flows() {
    for front in [false, true] {
        let temp = Scratch::new();
        let mut app = new_app(String::new(), String::new(), 1);
        app.save_path = temp.0.join("Match 11.oyrsave");
        app.menu.front = front;
        for i in 0..12 {
            std::fs::write(temp.0.join(format!("Match {i:02}.oyrsave")), [i]).unwrap();
        }
        std::fs::write(temp.0.join("keep.txt"), b"keep").unwrap();
        app.open_menu(if front {
            Page::SinglePlayer
        } else {
            Page::Pause
        });
        // Single Player normally receives its refreshed slots from Main.
        if front {
            app.menu.slots = yr_app::menu::slots(&temp.0).unwrap();
            click(&mut app, Button::Load);
        }
        click(&mut app, Button::Delete);
        assert_eq!(app.menu.page, Page::Delete);
        click(&mut app, Button::Delete);
        assert_eq!(app.menu.page, Page::Delete, "no selection cannot delete");
        for _ in 0..12 {
            app.menu_key_code(KeyCode::ArrowDown, None, false);
        }
        assert_eq!(app.menu.selected, Some(11));
        assert_eq!(app.menu.scroll, 2);
        app.menu_key_code(KeyCode::Enter, None, false);
        assert_eq!(
            app.menu.page,
            Page::Confirm(Intent::Delete(app.save_path.clone()))
        );
        assert_eq!(
            app.menu.focus, 1,
            "confirmation defaults to keeping the save"
        );
        app.menu_key_code(KeyCode::Enter, None, true);
        assert!(matches!(app.menu.page, Page::Confirm(Intent::Delete(_))));
        app.menu_key_code(KeyCode::Enter, None, false);
        assert_eq!(app.menu.page, Page::Delete);
        assert_eq!(std::fs::read(&app.save_path).unwrap(), [11]);
        click(&mut app, Button::Delete);
        app.menu_key_code(KeyCode::Escape, None, false);
        assert_eq!(app.menu.page, Page::Delete);
        assert!(app.save_path.is_file());
        click(&mut app, Button::Delete);
        click(&mut app, Button::Accept);
        assert!(matches!(app.menu.page, Page::Message { .. }));
        assert!(!app.save_path.exists());
        assert_eq!(app.menu.selected, Some(10));
        assert_eq!(app.menu.scroll, 1);
        assert_eq!(app.menu.slots.len(), 11);
        assert_eq!(
            std::fs::read(temp.0.join("Match 10.oyrsave")).unwrap(),
            [10]
        );
        assert_eq!(std::fs::read(temp.0.join("keep.txt")).unwrap(), b"keep");
        click(&mut app, Button::Back);
        // Mouse slot selection uses the same scaled list geometry in both shells.
        let layout = Layout::for_state(&app.menu, WIN_W, WIN_H);
        app.cursor = (
            layout.origin.0 + 50.0 * layout.scale,
            layout.origin.1 + 155.0 * layout.scale,
        );
        app.menu_mouse(ElementState::Pressed, MouseButton::Left);
        app.menu_mouse(ElementState::Released, MouseButton::Left);
        assert_eq!(app.menu.selected, Some(1));
        while !app.menu.slots.is_empty() {
            click(&mut app, Button::Delete);
            click(&mut app, Button::Accept);
            click(&mut app, Button::Back);
        }
        assert_eq!(app.menu.selected, None);
        assert_eq!(app.menu.scroll, 0);
        assert!(!app.menu.buttons()[0].1);
        click(&mut app, Button::Delete);
        assert_eq!(app.menu.page, Page::Delete);
        click(&mut app, Button::Back);
        assert_eq!(app.menu.page, if front { Page::Load } else { Page::Pause });
        assert!(app.pending.is_empty());
        assert!(app.load_job.is_none());
        assert_eq!(app.save_path, temp.0.join("Match 11.oyrsave"));
    }
}

#[test]
fn delete_slots_rejects_stale_symlink_directory_and_outside_targets() {
    let temp = Scratch::new();
    let outside = Scratch::new();
    let mut app = new_app(String::new(), String::new(), 1);
    app.save_path = temp.0.join("active.oyrsave");
    let path = temp.0.join("selected.oyrsave");
    std::fs::write(&path, b"save").unwrap();
    app.open_menu(Page::Delete);
    app.menu.select_slot(0);
    click(&mut app, Button::Delete);
    std::fs::remove_file(&path).unwrap();
    click(&mut app, Button::Accept);
    assert!(
        matches!(&app.menu.page, Page::Message { text, .. } if text.starts_with("Could not delete"))
    );
    assert!(app.menu.slots.is_empty());
    click(&mut app, Button::Back);
    assert_eq!(app.menu.page, Page::Delete);

    let protected = outside.0.join("protected.oyrsave");
    std::fs::write(&protected, b"protected").unwrap();
    let unrelated = temp.0.join("keep.txt");
    std::fs::write(&unrelated, b"unrelated").unwrap();
    for invalid in [&protected, &unrelated] {
        app.menu.slots = vec![yr_app::menu::Slot {
            name: "invalid".into(),
            path: invalid.clone(),
        }];
        app.delete_menu_path(invalid.clone());
        assert!(
            matches!(&app.menu.page, Page::Message { text, .. } if text.starts_with("Could not delete"))
        );
        assert!(invalid.is_file());
    }
    for directory in [true, false] {
        std::fs::write(&path, b"save").unwrap();
        app.open_menu(Page::Delete);
        app.menu.select_slot(0);
        click(&mut app, Button::Delete);
        std::fs::remove_file(&path).unwrap();
        if directory {
            std::fs::create_dir(&path).unwrap();
        } else {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&protected, &path).unwrap();
        }
        click(&mut app, Button::Accept);
        assert!(
            matches!(&app.menu.page, Page::Message { text, .. } if text.starts_with("Could not delete"))
        );
        assert!(app.menu.slots.is_empty());
        assert_eq!(std::fs::read(&protected).unwrap(), b"protected");
        if directory {
            std::fs::remove_dir(&path).unwrap();
        } else {
            #[cfg(unix)]
            std::fs::remove_file(&path).unwrap();
        }
    }
}

#[test]
fn retail_delete_saved_matches_keeps_live_state_and_supports_front_menus() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let capture = |app: &App, name: &str| {
        if let Some(out) = std::env::var_os("OPENYR_DELETE_PROOF") {
            let out = PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
            app.compose(&mut frame, WIN_W, WIN_H);
            std::fs::write(
                out.join(format!("{name}.png")),
                yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
            )
            .unwrap();
            std::fs::write(
                out.join(format!("{name}.txt")),
                format!(
                    "page={:?}\nfront={}\nslots={}\nselected={:?}\ntick={:?}\nhash={:?}\n",
                    app.menu.page,
                    app.menu.front,
                    app.menu.slots.len(),
                    app.menu.selected,
                    app.session.as_ref().map(|s| s.sim.tick()),
                    app.session
                        .as_ref()
                        .map(|s| format!("{:016x}", s.sim.state_hash()))
                ),
            )
            .unwrap();
        }
    };
    let temp = Scratch::new();
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.save_path = temp.0.join("Checkpoint 01.oyrsave");
    app.session = Some(app.load_session().unwrap());
    app.menu_art = yr_app::menu::Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
    let bytes = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let hash = app.session.as_ref().unwrap().sim.state_hash();
    let active = app.save_path.clone();
    let other = temp.0.join("Checkpoint 02.oyrsave");
    std::fs::write(&active, &bytes).unwrap();
    std::fs::write(&other, &bytes).unwrap();
    app.open_menu(Page::Pause);
    capture(&app, "01-pause-delete-enabled-t1");
    click(&mut app, Button::Delete);
    app.menu_key_code(KeyCode::ArrowDown, None, false);
    capture(&app, "02-delete-slot-t1");
    click(&mut app, Button::Delete);
    capture(&app, "03-delete-confirm-t1");
    app.menu_key_code(KeyCode::Escape, None, false);
    assert_eq!(std::fs::read(&active).unwrap(), bytes);
    click(&mut app, Button::Delete);
    click(&mut app, Button::Accept);
    capture(&app, "04-delete-success-t1");
    assert!(!active.exists());
    assert_eq!(std::fs::read(&other).unwrap(), bytes);
    click(&mut app, Button::Back);
    click(&mut app, Button::Delete);
    std::fs::remove_file(&other).unwrap();
    click(&mut app, Button::Accept);
    capture(&app, "05-delete-error-t1");
    click(&mut app, Button::Back);
    capture(&app, "06-delete-empty-t1");
    assert!(app.menu.slots.is_empty());
    assert!(!app.advance_clock(app.last_tick + Duration::from_secs(3600)));
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        bytes
    );
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
    assert_eq!(
        app.save_path, active,
        "deleting a save keeps the future save target"
    );
    click(&mut app, Button::Back);
    click(&mut app, Button::Resume);
    assert_eq!(app.accum, Duration::ZERO);
    app.save_menu_path(app.save_path.clone());
    assert_eq!(
        std::fs::read(&active).unwrap(),
        bytes,
        "the running match can recreate its save"
    );

    app.enter_front();
    assert!(app.session.is_none());
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Load);
    capture(&app, "07-front-saved-matches");
    click(&mut app, Button::Delete);
    app.menu_key_code(KeyCode::ArrowDown, None, false);
    capture(&app, "08-front-delete-slot");
    click(&mut app, Button::Delete);
    capture(&app, "09-front-delete-confirm");
    app.menu_key_code(KeyCode::Enter, None, false);
    assert!(
        active.is_file(),
        "front confirmation defaults to keeping the save"
    );
    click(&mut app, Button::Delete);
    click(&mut app, Button::Accept);
    capture(&app, "10-front-delete-success");
    click(&mut app, Button::Back);
    capture(&app, "11-front-delete-empty");
    click(&mut app, Button::Back);
    assert_eq!(app.menu.page, Page::Load);
    assert!(app.menu.slots.is_empty());
    click(&mut app, Button::Back);
    assert_eq!(app.menu.page, Page::SinglePlayer);
    assert!(app.session.is_none() && app.pending.is_empty() && app.load_job.is_none());
    println!("Deleted confirmed retail saves in pause and front menus; cancellation preserved bytes; stale-file recovery and empty lists passed; live tick 1 hash {hash:016x} and complete save bytes unchanged; recreated active save at the retained target.");
}

#[test]
fn retail_menu_pause_save_overwrite_async_load_cancel_restart_and_results() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let temp = Scratch::new();
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.save_path = temp.0.join("quicksave.oyrsave");
    app.preferences_path = Some(temp.0.join("interface.json"));
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    app.menu_art = yr_app::menu::Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
    app.pointer.inside = true;
    let initial_hash = app.session.as_ref().unwrap().sim.state_hash();
    proof(&app, "01-world-t1");
    app.escape_command_or_pause();
    app.cursor = (-1.0, -1.0);
    proof(&app, "02-pause-normal-t1");
    let layout = Layout::new(WIN_W, WIN_H);
    app.cursor = (layout.origin.0 + 890.0, layout.origin.1 + 235.0);
    proof(&app, "03-pause-hover-t1");
    app.menu_mouse(ElementState::Pressed, MouseButton::Left);
    proof(&app, "04-pause-pressed-t1");
    app.menu_mouse(ElementState::Released, MouseButton::Left);
    assert_eq!(app.menu.page, Page::Controls);
    click(&mut app, Button::Mute);
    click(&mut app, Button::ZoomIn);
    assert!(app.menu.muted && app.menu.zoom == 125);
    proof(&app, "05-controls-t1");
    click(&mut app, Button::Back);
    assert!(!app.advance_clock(app.last_tick + Duration::from_secs(3600)));
    app.tick_session();
    assert_eq!(initial_hash, app.session.as_ref().unwrap().sim.state_hash());
    click(&mut app, Button::Save);
    app.menu.name.clear();
    app.menu_key_code(KeyCode::KeyM, Some("Mission One"), false);
    proof(&app, "06-save-empty-list-t1");
    click(&mut app, Button::Save);
    assert_eq!(
        app.menu.slots.len(),
        1,
        "new saves are immediately loadable"
    );
    let saved = app.save_path.clone();
    assert!(saved.is_file());
    proof(&app, "07-save-success-t1");
    click(&mut app, Button::Back);
    click(&mut app, Button::Save);
    click(&mut app, Button::Save);
    assert!(matches!(app.menu.page, Page::Confirm(Intent::Save(_))));
    proof(&app, "08-overwrite-confirm-t1");
    click(&mut app, Button::Decline);
    click(&mut app, Button::Save);
    click(&mut app, Button::Accept);
    click(&mut app, Button::Back);
    click(&mut app, Button::Resume);
    assert_eq!(app.accum, Duration::ZERO);
    app.advance_clock(app.last_tick + TICK);
    assert_eq!(
        app.session.as_ref().unwrap().sim.tick(),
        2,
        "resume has no catch-up burst"
    );
    let tick2_hash = app.session.as_ref().unwrap().sim.state_hash();
    app.save_path = temp.0.join("other-target.oyrsave");
    app.open_menu(Page::Pause);
    click(&mut app, Button::Load);
    app.menu_key_code(KeyCode::ArrowDown, None, false);
    assert_eq!(app.menu.selected, Some(0));
    proof(&app, "09-load-slot-t2");
    click(&mut app, Button::Load);
    proof(&app, "10-discard-confirm-t2");
    click(&mut app, Button::Decline);
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), tick2_hash);
    click(&mut app, Button::Load);
    click(&mut app, Button::Accept);
    proof(&app, "11-loading-t2");
    app.selected.insert(7);
    app.pending.push(Command::Stop { id: 7 });
    finish_load(&mut app);
    assert!(!app.menu.visible());
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), initial_hash);
    assert_eq!(app.save_path, saved);
    assert!(app.pending.is_empty() && app.keys.is_empty() && app.selected.is_empty());
    proof(&app, "12-restored-world-t1");
    let corrupt = temp.0.join("damaged.oyrsave");
    std::fs::write(&corrupt, b"damaged").unwrap();
    app.start_menu_load(Some(corrupt));
    finish_load(&mut app);
    assert!(matches!(app.menu.page, Page::Message { .. }));
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), initial_hash);
    proof(&app, "13-load-error-t1");
    click(&mut app, Button::Back);
    app.start_menu_load(Some(saved));
    click(&mut app, Button::Back);
    finish_load(&mut app);
    assert_eq!(
        app.menu.page,
        Page::Pause,
        "cancelled worker cannot replace or resume the match"
    );
    proof(&app, "14-load-cancelled-t1");
    click(&mut app, Button::Abort);
    proof(&app, "15-abort-t1");
    click(&mut app, Button::Restart);
    finish_load(&mut app);
    assert!(!app.menu.visible());
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), initial_hash);
    let blocked_parent = temp.0.join("not-a-directory");
    std::fs::write(&blocked_parent, b"keep").unwrap();
    app.save_menu_path(blocked_parent.join("unwritable.oyrsave"));
    assert!(matches!(app.menu.page, Page::Message { .. }));
    assert_eq!(std::fs::read(&blocked_parent).unwrap(), b"keep");
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), initial_hash);
    proof(&app, "18-save-error-t1");
    app.session.as_mut().unwrap().outcome = Outcome::Won;
    app.open_menu(Page::Result(true));
    proof(&app, "16-won-t1");
    assert_eq!(app.menu.escape(), Effect::None);
    app.session.as_mut().unwrap().outcome = Outcome::Lost;
    app.open_menu(Page::Result(false));
    proof(&app, "17-lost-t1");
    click(&mut app, Button::Quit);
    assert!(app.quit_requested);
    println!("Menu flow verified: frozen tick 1, single resumed tick 2, restored hash {initial_hash:016x}, cancelled load retained pause, restart retained seed.");
}
