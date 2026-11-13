use super::*;
use yr_app::menu::{Button, Layout};
fn retail_directory() -> PathBuf {
    std::env::var_os("OPENYR_GAMEDIR").map_or_else(
        || {
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
        },
        PathBuf::from,
    )
}
fn click(app: &mut App, button: Button) {
    let i = app
        .menu
        .buttons()
        .iter()
        .position(|b| b.0 == button)
        .unwrap();
    let l = Layout::for_state(&app.menu, WIN_W, WIN_H);
    let (x, y, _, _) = l.button(i, button);
    app.cursor = (
        l.origin.0 + f64::from(x + 12) * l.scale,
        l.origin.1 + f64::from(y + 12) * l.scale,
    );
    app.menu_mouse(ElementState::Pressed, MouseButton::Left);
    app.menu_mouse(ElementState::Released, MouseButton::Left);
}
fn wait(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(180);
    while app.load_job.is_some() {
        app.poll_menu_load();
        assert!(Instant::now() < deadline, "front loader timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn proof(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_FRONT_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose(&mut frame, WIN_W, WIN_H);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.txt")),
        format!(
            "page={:?}\nfront={}\nsim={:?}\nmovie={:?}\n",
            app.menu.page,
            app.menu.front,
            app.session
                .as_ref()
                .map(|s| (s.sim.tick(), s.sim.state_hash())),
            app.movie
                .as_ref()
                .and_then(|m| m.current.as_ref())
                .map(|f| (f.index, f.sequence))
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.setup.txt")),
        format!(
            "menu_options={:?}\nsession={:?}\n",
            app.menu.setup.options,
            app.session.as_ref().map(|s| (
                s.start_options,
                s.sim.credits(),
                s.sim.credits_of(yr_sim::OWNER_ENEMY),
                s.sim
                    .units()
                    .iter()
                    .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
                    .count(),
                s.sim
                    .units()
                    .iter()
                    .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
                    .count()
            ))
        ),
    )
    .unwrap();
}

#[test]
fn retail_main_menu_cancel_and_immediate_retry_enters_the_requested_match() {
    use yr_app::menu::LoadStage;
    let directory = retail_directory();
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.enter_front();
    app.menu_key_code(KeyCode::Tab, None, false);
    assert_eq!(app.menu.buttons()[app.menu.focus].0, Button::SinglePlayer);
    proof(&app, "retry-00-main-keyboard-focus");
    app.menu_key_code(KeyCode::Enter, None, false);
    assert_eq!(app.menu.page, Page::SinglePlayer);
    click(&mut app, Button::Skirmish);
    assert_eq!(app.menu.loading.stage, LoadStage::Preview);
    proof(&app, "retry-01-preview-loading");
    click(&mut app, Button::Back);
    click(&mut app, Button::Skirmish);
    assert_eq!(app.menu.page, Page::Loading);
    assert_eq!(app.menu.loading.stage, LoadStage::Waiting);
    proof(&app, "retry-02-preview-waiting");
    wait(&mut app);
    assert_eq!(app.menu.page, Page::Skirmish);
    assert!(app.session.is_none());
    proof(&app, "retry-03-skirmish");
    click(&mut app, Button::Start);
    click(&mut app, Button::Back);
    assert_eq!(app.menu.page, Page::Skirmish);
    app.menu
        .setup
        .set(yr_app::menu::setup::Field::Credits, 7000);
    click(&mut app, Button::Start);
    assert_eq!(app.menu.loading.stage, LoadStage::Waiting);
    proof(&app, "retry-04-match-waiting");
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut last_stage = None;
    while app.load_job.is_some() {
        app.poll_menu_load();
        if app.menu.page == Page::Loading && last_stage != Some(app.menu.loading.stage) {
            last_stage = Some(app.menu.loading.stage);
            proof(&app, &format!("retry-05-{:?}", app.menu.loading.stage));
            // Loading still accepts keyboard navigation and cancel input.
            app.menu_key_code(KeyCode::Tab, None, false);
            assert!(app.session.is_none());
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.menu.page, Page::Closed);
    assert!(!app.menu.front && app.sprites.is_some());
    let session = app.session.as_ref().unwrap();
    assert_eq!(session.sim.credits(), 7000);
    assert_eq!(session.sim.tick(), 1);
    proof(&app, "retry-06-started-match");
}

#[test]
fn retail_skirmish_settings_start_save_restore_and_restart_both_participants() {
    use yr_app::{menu::setup::Field, session::StartOptions};
    let directory = retail_directory();
    if !directory.is_dir() {
        return;
    }
    let temp = std::env::temp_dir().join(format!("openyr-setup-options-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.save_path = temp.join("Settings.oyrsave");
    app.preferences_path = Some(temp.join("interface.json"));
    app.enter_front();
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    app.menu_key_code(KeyCode::Tab, None, false);
    assert_eq!(app.menu.setup.focus, Some(Field::Credits));
    app.menu_key_code(KeyCode::Home, None, false);
    app.menu_key_code(KeyCode::Tab, None, false);
    assert_eq!(app.menu.setup.focus, Some(Field::Units));
    app.menu_key_code(KeyCode::Home, None, false);
    assert_eq!(
        (
            app.menu.setup.value(Field::Credits),
            app.menu.setup.value(Field::Units)
        ),
        (5000, 0)
    );
    proof(&app, "setup-minimum-controls");
    let layout = Layout::for_state(&app.menu, WIN_W, WIN_H);
    // A real mouse press selects a value; grabbing its thumb preserves it.
    app.cursor = (
        layout.origin.0 + 450.0 * layout.scale,
        layout.origin.1 + 350.0 * layout.scale,
    );
    app.menu_mouse(ElementState::Pressed, MouseButton::Left);
    app.menu_mouse(ElementState::Released, MouseButton::Left);
    assert_eq!(app.menu.setup.value(Field::Units), 6);
    let options = StartOptions {
        credits: Some(5000),
        units: Some(6),
        ..Default::default()
    };
    assert_eq!(app.menu.setup.options, options);
    let thumb = app.menu.setup.thumb_x(Field::Units);
    app.menu.setup.press(Field::Units, thumb + 6);
    app.menu.setup.drag_to(thumb + 6);
    assert_eq!(
        app.menu.setup.value(Field::Units),
        6,
        "grabbing a thumb cannot change its value"
    );
    app.cancel_pointer_action();
    assert!(app.menu.setup.drag.is_none());
    proof(&app, "setup-configured-controls");
    click(&mut app, Button::Start);
    wait(&mut app);
    let assert_setup = |app: &App| {
        let s = app.session.as_ref().unwrap();
        assert_eq!(s.start_options, options);
        assert_eq!(s.sim.credits(), 5000);
        assert_eq!(s.sim.credits_of(yr_sim::OWNER_ENEMY), 5000);
        for owner in [yr_sim::OWNER_PLAYER, yr_sim::OWNER_ENEMY] {
            assert_eq!(s.sim.units().iter().filter(|u| u.owner == owner).count(), 6);
        }
    };
    assert_setup(&app);
    let initial_hash = app.session.as_ref().unwrap().sim.state_hash();
    proof(&app, "setup-configured-match-tick1");
    app.escape_command_or_pause();
    click(&mut app, Button::Save);
    app.menu.name = "Settings".into();
    click(&mut app, Button::Save);
    let path = app.save_path.clone();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 3);
    app.menu_key_code(KeyCode::Escape, None, false);
    click(&mut app, Button::Abort);
    click(&mut app, Button::MainMenu);
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Load);
    let index = app.menu.slots.iter().position(|s| s.path == path).unwrap();
    app.menu.select_slot(index);
    click(&mut app, Button::Load);
    click(&mut app, Button::Accept);
    wait(&mut app);
    assert_setup(&app);
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), initial_hash);
    proof(&app, "setup-restored-match-tick1");
    let mut bad = bytes;
    let length = usize::from(u16::from_le_bytes(bad[37..39].try_into().unwrap()));
    bad[39 + length] = 4;
    let bad_path = temp.join("Broken.oyrsave");
    std::fs::write(&bad_path, bad).unwrap();
    app.open_menu(Page::Load);
    let index = app
        .menu
        .slots
        .iter()
        .position(|s| s.path == bad_path)
        .unwrap();
    app.menu.select_slot(index);
    click(&mut app, Button::Load);
    click(&mut app, Button::Accept);
    wait(&mut app);
    assert!(matches!(app.menu.page, Page::Message { .. }));
    assert_setup(&app);
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), initial_hash);
    proof(&app, "setup-invalid-save-preserves-match");
    click(&mut app, Button::Back);
    click(&mut app, Button::Abort);
    click(&mut app, Button::Restart);
    wait(&mut app);
    assert_setup(&app);
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), initial_hash);
    proof(&app, "setup-restarted-match-tick1");
    app.enter_front();
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    assert_eq!(app.menu.setup.options, options);
    proof(&app, "setup-returned-controls");
    let legacy_options = StartOptions {
        shared_forces: false,
        ..Default::default()
    };
    let old = yr_app::session::load_with_options(
        directory.to_str().unwrap(),
        "EB1.MMX",
        42,
        legacy_options,
    )
    .unwrap();
    assert_eq!(
        old.sim.state_hash(),
        0x4601c68d2468f7dc,
        "fresh legacy-profile EB1 includes corrected map health and headings"
    );
    let path = temp.join("Legacy.oyrsave");
    yr_app::snapshot::save(&old, &path).unwrap();
    let legacy = yr_app::snapshot::load_from_menu(directory.to_str().unwrap(), "EB1.MMX", &path)
        .unwrap()
        .0;
    assert_eq!(legacy.sim.state_hash(), old.sim.state_hash());
    assert_eq!(
        yr_app::snapshot::encode(&legacy).unwrap(),
        std::fs::read(&path).unwrap(),
        "the current legacy profile round-trips complete saved bytes"
    );
    assert_eq!(legacy.start_options, legacy_options);
    assert_eq!(
        legacy
            .sim
            .units()
            .iter()
            .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
            .count(),
        3
    );
    if let Some(folder) = std::env::var_os("OPENYR_LEGACY_SETUP_PROOF") {
        for name in ["EB1-v1.oyrsave", "EB1-v2.oyrsave"] {
            let path = PathBuf::from(&folder).join(name);
            let bytes = std::fs::read(&path).unwrap();
            let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
            let payload = match version {
                1 => 37,
                2 => 39 + usize::from(u16::from_le_bytes(bytes[37..39].try_into().unwrap())),
                _ => panic!("expected a historical version-1/2 fixture"),
            };
            let raw = yr_sim::Sim::from_save_bytes(&bytes[payload..]).unwrap();
            assert_eq!(raw.state_hash(), 0x57dac47c915b074a);
            assert_eq!(raw.save_bytes().unwrap(), bytes[payload..]);
            let error =
                yr_app::snapshot::load_from_menu(directory.to_str().unwrap(), "EB1.MMX", &path)
                    .err()
                    .expect("pre-correction EB1 setup must be rejected");
            assert!(error.contains("map, rules or engine setup"), "{error}");
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            println!("Pre-correction {name} rejected; source file and raw snapshot retained.");
        }
    }
    println!("Configured credits=5000 units=6 for both owners; tick1 hash={initial_hash:016x}; fresh legacy-profile EB1 seed42 hash=4601c68d2468f7dc; saved state retained.");
    drop(app);
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn retail_front_recovery_messages_wrap_scroll_and_return_through_actual_handlers() {
    let dir = retail_directory();
    if !dir.is_dir() {
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.enter_front();
    for (name, text) in [
        ("recovery-short", "Unable to load this match.".into()),
        (
            "recovery-multiline",
            "The saved match is incompatible.\n\nReturn to setup and try another saved match in OpenYR.".into(),
        ),
        (
            "recovery-long",
            format!(
                "{}\nThe current match is safe. Return to setup to continue in OpenYR.",
                "This saved match uses a different map or rules setup. OpenYR could not load it. ".repeat(35)
            ),
        ),
        ("recovery-token", format!("Missing asset:\n{}\nReturn to setup.", "W界é".repeat(400))),
    ] {
        app.menu.message(text.clone(), Page::Load);
        proof(&app, name);
        app.menu_key_code(KeyCode::End, None, false);
        let end = app.menu.message_scroll;
        if name == "recovery-long" || name == "recovery-token" {
            assert!(end > 0);
            proof(&app, &format!("{name}-end"));
            app.menu_key_code(KeyCode::ArrowUp, None, false);
            assert_eq!(app.menu.message_scroll, end - 1);
            app.menu_scroll(1);
            assert_eq!(app.menu.message_scroll, end);
            app.menu_key_code(KeyCode::PageUp, None, true);
            assert_eq!(app.menu.message_scroll, end.saturating_sub(10));
            app.menu_key_code(KeyCode::PageDown, None, true);
            assert_eq!(app.menu.message_scroll, end);
        } else {
            assert_eq!(end, 0);
        }
        app.menu_key_code(KeyCode::Home, None, false);
        assert_eq!(app.menu.message_scroll, 0);
        app.menu_key_code(KeyCode::ArrowDown, None, true);
        assert_eq!(app.menu.message_scroll, end.min(1));
        app.menu_key_code(KeyCode::Tab, None, false);
        assert!(app.menu.keyboard_focus);
        assert!(matches!(&app.menu.page, Page::Message { text: actual, .. } if actual == &text));
        assert!(app.session.is_none() && app.pending.is_empty());
        assert!(!app.advance_clock(app.last_tick + Duration::from_secs(3600)));
        if name == "recovery-long" {
            app.menu_key_code(KeyCode::Escape, None, false);
        } else if name == "recovery-multiline" {
            app.menu_key_code(KeyCode::Enter, None, false);
        } else {
            click(&mut app, Button::Back);
        }
        assert_eq!(app.menu.page, Page::Load);
        assert_eq!(app.menu.message_scroll, 0);
    }
}

#[test]
fn retail_battle_map_chooser_previews_cancels_and_commits_the_selected_map() {
    let directory = retail_directory();
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.enter_front();
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    let accepted = app.front_preview.as_ref().unwrap().rgba.clone();
    assert_eq!(app.menu.maps.entries.len(), 91);
    for (index, asset) in [
        (0, "XMP29U2.MAP"),
        (12, "XMP08T2.MAP"),
        (26, "XKiller.MAP"),
        (32, "XEB1.MAP"),
        (72, "XMP26S6.MAP"),
        (83, "XMP32S8.MAP"),
    ] {
        assert_eq!(app.menu.maps.entries[index].asset, asset);
    }
    let settle = |app: &mut App| {
        let deadline = Instant::now() + Duration::from_secs(180);
        while app.chooser_job.is_some() || app.menu.maps.ready != Some(app.menu.maps.selected) {
            app.poll_map_chooser();
            assert!(app.menu.maps.error.is_none(), "{:?}", app.menu.maps.error);
            assert!(Instant::now() < deadline, "chooser preview did not settle");
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    click(&mut app, Button::ChooseMap);
    assert_eq!(app.menu.page, Page::ChooseMap);
    assert!(
        !app.menu
            .buttons()
            .iter()
            .find(|b| b.0 == Button::UseMap)
            .unwrap()
            .1
    );
    // Navigate while the initial preview is still pending.
    app.menu_key_code(KeyCode::End, None, false);
    app.menu_key_code(KeyCode::Home, None, false);
    app.menu_key_code(KeyCode::PageDown, None, false);
    for _ in 0..6 {
        app.menu_key_code(KeyCode::ArrowUp, None, false);
    }
    settle(&mut app);
    assert_eq!(app.menu.maps.selected, 12);
    assert_eq!(
        (
            app.chooser_preview.as_ref().unwrap().width,
            app.chooser_preview.as_ref().unwrap().height
        ),
        (70, 67)
    );
    assert_eq!(app.map_name, "EB1.MMX");
    proof(&app, "chooser-lake");
    app.menu_key_code(KeyCode::Escape, None, false);
    assert_eq!(app.menu.page, Page::Skirmish);
    assert_eq!(app.front_preview.as_ref().unwrap().rgba, accepted);
    assert_eq!(app.map_name, "EB1.MMX");
    click(&mut app, Button::ChooseMap);
    app.menu_key_code(KeyCode::End, None, false);
    app.menu_key_code(KeyCode::PageUp, None, false);
    settle(&mut app);
    assert_eq!(app.menu.maps.selected, 72);
    proof(&app, "chooser-bering");
    click(&mut app, Button::UseMap);
    assert_eq!(app.menu.page, Page::Skirmish);
    assert_eq!(app.map_name, "XMP26S6.MAP");
    assert_eq!(app.menu.map_name, app.map_name);
    assert_eq!(
        (
            app.front_preview.as_ref().unwrap().width,
            app.front_preview.as_ref().unwrap().height
        ),
        (236, 86)
    );
    assert!(app.session.is_none());
    proof(&app, "skirmish-selected-bering");
    click(&mut app, Button::Start);
    wait(&mut app);
    assert_eq!(app.menu.page, Page::Closed);
    assert_eq!(app.map_name, "XMP26S6.MAP");
    assert!(app.session.is_some() && app.map.is_some() && app.sprites.is_some());
    proof(&app, "chooser-bering-match-tick1");
    app.enter_front();
    assert_eq!(app.map_name, "XMP26S6.MAP");
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    click(&mut app, Button::ChooseMap);
    settle(&mut app);
    assert_eq!(app.menu.maps.selected, 72);

    // Cancel a pending different preview and drain its obsolete result.
    app.menu_key_code(KeyCode::Home, None, false);
    assert!(app.chooser_job.is_some());
    app.menu_key_code(KeyCode::Escape, None, false);
    let deadline = Instant::now() + Duration::from_secs(180);
    while app.chooser_job.is_some() {
        app.poll_map_chooser();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.menu.page, Page::Skirmish);
    assert_eq!(app.map_name, "XMP26S6.MAP");
    assert!(app.chooser_preview.is_none());

    // A failed map keeps the accepted map, disables Use Map and permits retry.
    click(&mut app, Button::ChooseMap);
    app.menu.maps.entries[0].asset = "OPENYR_MISSING_CHOOSER.MAP".into();
    app.menu.maps.top = 0;
    let layout = Layout::for_state(&app.menu, WIN_W, WIN_H);
    app.cursor = (
        layout.origin.0 + 345.0 * layout.scale,
        layout.origin.1 + 132.0 * layout.scale,
    );
    app.menu_mouse(ElementState::Pressed, MouseButton::Left);
    app.menu_mouse(ElementState::Released, MouseButton::Left);
    assert_eq!(app.menu.maps.selected, 0);
    while app.menu.maps.error.is_none() {
        app.poll_map_chooser();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!app.menu.buttons().contains(&(Button::UseMap, true)));
    assert_eq!(app.map_name, "XMP26S6.MAP");
    proof(&app, "chooser-preview-error");
    app.menu_key_code(KeyCode::PageDown, None, false);
    settle(&mut app);
    assert_eq!(app.menu.maps.selected, 18);
    app.menu.maps.drag = Some((200, 0));
    app.cancel_pointer_action();
    assert!(app.menu.maps.drag.is_none());
    app.menu_key_code(KeyCode::Tab, None, false);
    assert!(!app.menu.maps.list_focus && app.menu.keyboard_focus);
    app.menu_key_code(KeyCode::Escape, None, false);
    assert_eq!(app.menu.page, Page::Skirmish);
    assert_eq!(app.map_name, "XMP26S6.MAP");
}

#[test]
fn retail_front_menu_restores_the_saved_map_instead_of_the_default() {
    let dir = retail_directory();
    if !dir.is_dir() {
        return;
    }
    let temp = std::env::temp_dir().join(format!("openyr-saved-map-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let path = temp.join("Alamo.oyrsave");
    let mut saved = yr_app::session::load(dir.to_str().unwrap(), "XMP29U2.MAP", 91).unwrap();
    for _ in 0..3 {
        saved.step(&[]);
    }
    let expected = saved.sim.state_hash();
    let expected_tick = saved.sim.tick();
    yr_app::snapshot::save(&saved, &path).unwrap();
    assert!(yr_app::snapshot::load(dir.to_str().unwrap(), "EB1.MMX", &path).is_err());
    // Reconstruct an actual envelope-1/2 profile rather than stripping the
    // new shared-force metadata from a different initial setup.
    let mut old = yr_app::session::load_with_options(
        dir.to_str().unwrap(),
        "XMP29U2.MAP",
        42,
        yr_app::session::StartOptions {
            shared_forces: false,
            ..Default::default()
        },
    )
    .unwrap();
    for _ in 0..3 {
        old.step(&[]);
    }
    let old_hash = old.sim.state_hash();
    let named = yr_app::snapshot::encode(&old).unwrap();
    assert_eq!(u32::from_le_bytes(named[8..12].try_into().unwrap()), 2);
    let name_len = usize::from(u16::from_le_bytes(named[37..39].try_into().unwrap()));
    let mut legacy = named[..37].to_vec();
    legacy[8..12].copy_from_slice(&1u32.to_le_bytes());
    legacy.extend_from_slice(&named[39 + name_len..]);
    let legacy_path = temp.join("Legacy.oyrsave");
    std::fs::write(&legacy_path, legacy).unwrap();
    let (legacy, legacy_map) =
        yr_app::snapshot::load_from_menu(dir.to_str().unwrap(), "XMP29U2.MAP", &legacy_path)
            .unwrap();
    assert_eq!(legacy_map, "XMP29U2.MAP");
    assert_eq!(legacy.sim.state_hash(), old_hash);
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.save_path = path.clone();
    app.enter_front();
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Load);
    let index = app.menu.slots.iter().position(|s| s.path == path).unwrap();
    app.menu.select_slot(index);
    click(&mut app, Button::Load);
    click(&mut app, Button::Accept);
    wait(&mut app);
    proof(&app, "saved-map-restored");
    assert_eq!(app.menu.page, Page::Closed, "saved map should load");
    assert_eq!(app.map_name, "XMP29U2.MAP");
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), expected);
    assert_eq!(app.session.as_ref().unwrap().sim.tick(), expected_tick);
    assert!(app.map.is_some() && app.sprites.is_some() && app.movie.is_none());

    let second_path = temp.join("City.oyrsave");
    let second = yr_app::session::load(dir.to_str().unwrap(), "EB1.MMX", 92).unwrap();
    let second_hash = second.sim.state_hash();
    yr_app::snapshot::save(&second, &second_path).unwrap();
    let bad_path = temp.join("Broken.oyrsave");
    std::fs::write(&bad_path, b"truncated").unwrap();
    let changed_reference = |name: &str| {
        let mut bytes = named[..37].to_vec();
        bytes.extend((name.len() as u16).to_le_bytes());
        bytes.extend(name.bytes());
        bytes.extend_from_slice(&named[39 + name_len..]);
        bytes
    };
    let missing_path = temp.join("Missing-map.oyrsave");
    std::fs::write(&missing_path, changed_reference("OPENYR_MISSING_MAP.MAP")).unwrap();
    let wrong_path = temp.join("Wrong-map.oyrsave");
    std::fs::write(&wrong_path, changed_reference("EB1.MMX")).unwrap();
    let request = |app: &mut App, path: &std::path::Path| {
        app.open_menu(Page::Pause);
        click(app, Button::Load);
        let index = app.menu.slots.iter().position(|s| s.path == path).unwrap();
        app.menu.select_slot(index);
        click(app, Button::Load);
        click(app, Button::Accept);
    };
    let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    request(&mut app, &second_path);
    assert_eq!(app.map_name, "XMP29U2.MAP");
    click(&mut app, Button::Back);
    wait(&mut app);
    assert_eq!(app.menu.page, Page::Pause);
    assert_eq!(app.map_name, "XMP29U2.MAP");
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        before
    );
    for path in [&bad_path, &missing_path, &wrong_path] {
        request(&mut app, path);
        wait(&mut app);
        assert!(matches!(app.menu.page, Page::Message { .. }));
        assert_eq!(app.map_name, "XMP29U2.MAP");
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            before
        );
    }
    request(&mut app, &second_path);
    wait(&mut app);
    assert_eq!(app.menu.page, Page::Closed);
    assert_eq!(app.map_name, "EB1.MMX");
    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), second_hash);
    proof(&app, "pause-saved-map-restored");
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn retail_front_art_reload_replaces_cached_pause_menu_art() {
    let dir = retail_directory();
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.menu.front = false;
    app.menu.set(Page::Pause);
    let render = |art: &yr_app::menu::Art| {
        let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
        yr_app::menu::draw(&mut frame, WIN_W, WIN_H, art, &app.menu, app.cursor);
        frame
    };
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let expected = render(&yr_app::menu::Art::load(&fs));
    assert_ne!(render(&app.menu_art), expected);
    app.load_front_art().unwrap();
    let mut actual = vec![0; (WIN_W * WIN_H * 4) as usize];
    yr_app::menu::draw(
        &mut actual,
        WIN_W,
        WIN_H,
        &app.menu_art,
        &app.menu,
        app.cursor,
    );
    assert_eq!(actual, expected);
}
#[test]
fn retail_front_menu_preview_start_save_return_load_cancel_options_and_exit() {
    let dir = retail_directory();
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let temp = std::env::temp_dir().join(format!("openyr-front-flow-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.save_path = temp.join("named.oyrsave");
    app.preferences_path = Some(temp.join("interface.json"));
    app.enter_front();
    assert!(app.menu.front && app.session.is_none());
    assert_eq!(app.menu.page, Page::Main);
    let deadline = Instant::now() + Duration::from_secs(120);
    while app.movie.as_ref().unwrap().current.is_none() {
        app.movie.as_mut().unwrap().poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    proof(&app, "01-main-movie-frame0");
    assert!(!app.advance_clock(app.last_tick + Duration::from_secs(3600)));
    for button in [Button::Internet, Button::Network, Button::Movies] {
        click(&mut app, button);
        assert_eq!(app.menu.page, Page::Main);
    }
    click(&mut app, Button::SinglePlayer);
    proof(&app, "02-single-player");
    click(&mut app, Button::Campaign);
    assert_eq!(app.menu.page, Page::SinglePlayer);
    click(&mut app, Button::Skirmish);
    proof(&app, "03-preview-loading");
    wait(&mut app);
    assert_eq!(app.menu.page, Page::Skirmish);
    assert!(app.session.is_none());
    assert_eq!(
        (
            app.front_preview.as_ref().unwrap().width,
            app.front_preview.as_ref().unwrap().height
        ),
        (118, 61)
    );
    assert!(app.front_preview.as_ref().unwrap().authored);
    proof(&app, "04-skirmish-real-preview");
    click(&mut app, Button::Start);
    proof(&app, "05-match-loading");
    app.keys.insert(KeyCode::ArrowRight);
    app.shift = true;
    app.control = true;
    wait(&mut app);
    assert!(app.keys.is_empty() && !app.shift && !app.control);
    assert!(!app.menu.front && !app.menu.visible() && app.movie.is_none());
    let hash = app.session.as_ref().unwrap().sim.state_hash();
    assert_eq!(app.session.as_ref().unwrap().seed(), 42);
    proof(&app, "06-match-tick1");
    app.save_menu_path(app.save_path.clone());
    let e = app.menu.escape();
    app.apply_menu_effect(e);
    click(&mut app, Button::Abort);
    proof(&app, "07-abort-return-choice");
    click(&mut app, Button::MainMenu);
    assert!(app.session.is_none() && app.map.is_none() && app.pending.is_empty());
    assert_eq!(app.menu.page, Page::Main);
    proof(&app, "08-returned-main");
    click(&mut app, Button::Controls);
    click(&mut app, Button::Mute);
    click(&mut app, Button::ZoomIn);
    assert!(app.menu.muted && app.menu.zoom == 125);
    proof(&app, "09-front-options");
    assert!(app.menu.buttons().contains(&(Button::Folder, true)));
    click(&mut app, Button::Back);
    click(&mut app, Button::SinglePlayer);
    app.menu_key_code(KeyCode::Enter, None, false);
    assert_eq!(app.menu.page, Page::Load);
    proof(&app, "10-front-saved-match");
    app.menu.select_slot(0);
    click(&mut app, Button::Load);
    proof(&app, "11-front-load-confirmation");
    click(&mut app, Button::Accept);
    wait(&mut app);
    assert!(!app.menu.front && app.session.as_ref().unwrap().sim.state_hash() == hash);
    let session = app.session.as_ref().unwrap();
    let yard = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::ConstructionYard)
        .unwrap();
    let (x, y) = yard.centre();
    let point = yr_app::interaction::ground_point(
        app.map.as_ref().unwrap(),
        session.sim.terrain(),
        (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
    );
    assert!(
        (0.0..(WIN_W - SIDEBAR_W) as f32).contains(&((point.0 - app.view.cam_x) * app.view.zoom))
    );
    assert!((0.0..WIN_H as f32).contains(&((point.1 - app.view.cam_y) * app.view.zoom)));
    app.open_menu(Page::Result(true));
    proof(&app, "12-result-return-choice");
    click(&mut app, Button::MainMenu);
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Skirmish);
    click(&mut app, Button::Back);
    wait(&mut app);
    assert_eq!(
        app.menu.page,
        Page::SinglePlayer,
        "cancelled preview cannot open setup later"
    );
    proof(&app, "13-preview-cancelled");
    app.save_path = temp.join("bad.oyrsave");
    std::fs::write(&app.save_path, b"bad").unwrap();
    app.open_menu(Page::Load);
    let index = app
        .menu
        .slots
        .iter()
        .position(|s| s.path == app.save_path)
        .unwrap();
    app.menu.select_slot(index);
    click(&mut app, Button::Load);
    click(&mut app, Button::Accept);
    wait(&mut app);
    assert!(
        app.session.is_none() && app.menu.front && matches!(app.menu.page, Page::Message { .. })
    );
    proof(&app, "14-front-load-error");
    click(&mut app, Button::Back);
    assert_eq!(app.menu.page, Page::Load);
    click(&mut app, Button::Back);
    click(&mut app, Button::MainMenu);
    let mut bad = b"BIKi".to_vec();
    for value in [48u32, 1, 4, 1, 8, 8, 15, 1, 0, 0, 53, 56] {
        bad.extend(value.to_le_bytes());
    }
    bad.extend([0; 4]);
    app.movie = Some(yr_app::movie::Movie::new(bad).unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    while !matches!(app.menu.page, Page::Message { .. }) {
        app.poll_front_presentation();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    proof(&app, "16-movie-error-recovery");
    click(&mut app, Button::Back);
    assert_eq!(app.menu.page, Page::Main);
    click(&mut app, Button::Quit);
    proof(&app, "15-exit-confirmation");
    click(&mut app, Button::Decline);
    assert!(!app.quit_requested);
    click(&mut app, Button::Quit);
    click(&mut app, Button::Accept);
    assert!(app.quit_requested);
    drop(app);
    std::fs::remove_dir_all(temp).unwrap();
    println!("Front flow verified: no session before Start, source movie, real preview, hash {hash:016x} restored, return/cancel/errors retained correct front page.");
}

#[test]
fn initial_camera_center_excludes_the_sidebar_and_accounts_for_zoom() {
    let map = yr_render::RenderedMap {
        width: 3780,
        height: 2115,
        rgba: Vec::new(),
        summary: String::new(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    };
    for zoom in [0.25, 1.0, 1.25, 4.0] {
        let mut view = View {
            cam_x: 0.0,
            cam_y: 0.0,
            zoom,
            dirty: false,
        };
        view.center_map(&map, (WIN_W - SIDEBAR_W, WIN_H));
        assert!((view.cam_x + (WIN_W - SIDEBAR_W) as f32 / (2.0 * zoom) - 1890.0).abs() < 0.01);
        assert!((view.cam_y + WIN_H as f32 / (2.0 * zoom) - 1057.5).abs() < 0.01);
    }
}

#[test]
fn retail_skirmish_heading_tracks_dialog_refreshes() {
    let directory = retail_directory();
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "XMP29U2.MAP".into(), 1);
    app.enter_front();
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    let mut mismatches = Vec::new();
    let mut check = |app: &App, name: &str, expected: u64| {
        let mut frame = vec![0; 800 * 600 * 4];
        app.compose(&mut frame, 800, 600);
        let hash = (0..37)
            .flat_map(|y| {
                frame[(y * 800 + 632) * 4..(y * 800 + 800) * 4]
                    .iter()
                    .copied()
            })
            .fold(0xcbf29ce484222325u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
            });
        if hash != expected {
            mismatches.push((name.to_owned(), hash, expected));
        }
        if let Some(path) = std::env::var_os("OPENYR_FRONT_PROOF") {
            let path = PathBuf::from(path);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join(format!("{name}.png")),
                yr_render::png::encode_rgba(800, 600, &frame),
            )
            .unwrap();
            std::fs::write(
                path.join(format!("{name}.txt")),
                format!(
                    "page={:?}\nfront={}\nphase={}\n",
                    app.menu.page, app.menu.front, app.front_phase
                ),
            )
            .unwrap();
        }
    };
    let settle = |app: &mut App| {
        let deadline = Instant::now() + Duration::from_secs(180);
        while app.chooser_job.is_some() || app.menu.maps.ready != Some(app.menu.maps.selected) {
            app.poll_map_chooser();
            assert!(app.menu.maps.error.is_none(), "{:?}", app.menu.maps.error);
            assert!(Instant::now() < deadline, "chooser preview did not settle");
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    const INITIAL: u64 = 0xaf5382b7a6dc6151;
    const REFRESHED: u64 = 0x86ad3335e6790da9;
    check(&app, "heading-initial", INITIAL);
    // Exercise the actual window focus handler without changing the map.
    app.focus_changed(false);
    app.focus_changed(true);
    check(&app, "heading-after-focus", REFRESHED);
    click(&mut app, Button::Back);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    check(&app, "heading-reentry", INITIAL);
    click(&mut app, Button::ChooseMap);
    settle(&mut app);
    check(&app, "heading-chooser-entry", 0x03bbbbb008bd0f95);
    click(&mut app, Button::Back);
    check(&app, "heading-after-cancel", REFRESHED);
    assert_eq!(app.map_name, "XMP29U2.MAP");
    click(&mut app, Button::ChooseMap);
    settle(&mut app);
    click(&mut app, Button::UseMap);
    check(&app, "heading-after-same-map", REFRESHED);
    assert_eq!(app.map_name, "XMP29U2.MAP");
    click(&mut app, Button::ChooseMap);
    let layout = Layout::for_state(&app.menu, WIN_W, WIN_H);
    app.cursor = (
        layout.origin.0 + 350.0 * layout.scale,
        layout.origin.1 + 361.0 * layout.scale,
    );
    app.menu_mouse(ElementState::Pressed, MouseButton::Left);
    app.menu_mouse(ElementState::Released, MouseButton::Left);
    settle(&mut app);
    assert_eq!(app.menu.maps.selected, 12);
    check(&app, "heading-chooser-lake", 0x970ff46dc361ae9d);
    click(&mut app, Button::UseMap);
    check(&app, "heading-after-map-change", REFRESHED);
    assert_eq!(app.map_name, "XMP08T2.MAP");
    click(&mut app, Button::ChooseMap);
    app.menu_key_code(KeyCode::Home, None, false);
    settle(&mut app);
    check(&app, "heading-chooser-reset", 0x970ff46dc361ae9d);
    click(&mut app, Button::UseMap);
    check(&app, "heading-after-map-restore", REFRESHED);
    assert_eq!(app.map_name, "XMP29U2.MAP");
    click(&mut app, Button::Back);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    check(&app, "heading-reentry-after-map", INITIAL);
    app.focus_changed(false);
    app.focus_changed(true);
    check(&app, "heading-focus-verified", REFRESHED);
    assert!(
        mismatches.is_empty(),
        "original heading lifecycle: {mismatches:?}"
    );
}

#[test]
fn retail_participant_controls_reach_loading_match_and_restart() {
    use yr_app::{
        match_config::{Country, GameSpeed},
        menu::setup::Field,
    };
    let directory = retail_directory();
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.enter_front();
    click(&mut app, Button::SinglePlayer);
    click(&mut app, Button::Skirmish);
    wait(&mut app);
    let field_click = |app: &mut App, field: Field| {
        let layout = Layout::for_state(&app.menu, WIN_W, WIN_H);
        let (x, y, _, _) = field.rect();
        app.cursor = (
            layout.origin.0 + f64::from(x + 8) * layout.scale,
            layout.origin.1 + f64::from(y + 8) * layout.scale,
        );
        app.menu_mouse(ElementState::Pressed, MouseButton::Left);
        app.menu_mouse(ElementState::Released, MouseButton::Left);
    };
    field_click(&mut app, Field::Name(0));
    app.menu_key_code(KeyCode::KeyJ, Some("Johnny"), false);
    app.menu_key_code(KeyCode::Enter, None, false);
    assert_eq!(
        app.menu.page,
        Page::Skirmish,
        "editing a name must not start the match"
    );
    field_click(&mut app, Field::Country(0));
    proof(&app, "participants-01-country-popup");
    // The retail Countries list puts Russia at index 8.
    for _ in 0..8 {
        app.menu_key_code(KeyCode::ArrowDown, None, false);
    }
    app.menu_key_code(KeyCode::Enter, None, false);
    assert_eq!(app.menu.setup.config.players[0].country, Country::Russia);
    field_click(&mut app, Field::Color(0));
    app.menu_key_code(KeyCode::ArrowDown, None, false);
    app.menu_key_code(KeyCode::Enter, None, false);
    assert_eq!(app.menu.setup.config.players[0].color, 2);
    field_click(&mut app, Field::Team(0));
    app.menu_key_code(KeyCode::ArrowDown, None, false);
    app.menu_key_code(KeyCode::Enter, None, false);
    field_click(&mut app, Field::ShortGame);
    field_click(&mut app, Field::Speed);
    app.menu_key_code(KeyCode::ArrowDown, None, false);
    app.menu_key_code(KeyCode::Enter, None, false);
    assert_eq!(app.menu.setup.config.speed, GameSpeed::Fast);
    field_click(&mut app, Field::Country(1));
    // Closing a popup over Start must consume the click.
    click(&mut app, Button::Start);
    assert_eq!(app.menu.page, Page::Skirmish);
    assert!(app.load_job.is_none());
    assert!(app.menu.setup.popup.is_none());
    let config = app.menu.setup.config.clone();
    proof(&app, "participants-02-configured-setup");
    click(&mut app, Button::Start);
    assert_eq!(app.menu.loading.config, config);
    proof(&app, "participants-03-country-loading");
    wait(&mut app);
    assert_eq!(app.session.as_ref().unwrap().match_config, config);
    assert!(app
        .session
        .as_ref()
        .unwrap()
        .buildables
        .iter()
        .any(|b| b.id == "HTNK"));
    proof(&app, "participants-04-soviet-match");
    app.reset_menu_clock();
    let tick = app.session.as_ref().unwrap().sim.tick();
    let now = app.last_tick + Duration::from_millis(100);
    assert!(app.advance_clock(now));
    assert_eq!(app.session.as_ref().unwrap().sim.tick(), tick + 3);
    app.escape_command_or_pause();
    click(&mut app, Button::Abort);
    click(&mut app, Button::Restart);
    wait(&mut app);
    assert_eq!(app.session.as_ref().unwrap().match_config, config);
    assert_eq!(app.session.as_ref().unwrap().sim.tick(), 1);
    proof(&app, "participants-05-restarted-match");
}

mod movie_failure;
