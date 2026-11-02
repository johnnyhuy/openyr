use super::*;
use yr_app::menu::{Button, Layout};
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
}

#[test]
fn retail_front_recovery_messages_wrap_scroll_and_return_through_actual_handlers() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
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
fn retail_front_menu_restores_the_saved_map_instead_of_the_default() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
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
    let named = yr_app::snapshot::encode(&saved).unwrap();
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
    assert_eq!(legacy.sim.state_hash(), expected);
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
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
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
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
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
    let map = app.map.as_ref().unwrap();
    assert!(
        (app.view.cam_x + (WIN_W - SIDEBAR_W) as f32 / (2.0 * app.view.zoom)
            - map.width as f32 / 2.0)
            .abs()
            < 0.01
    );
    assert!(
        (app.view.cam_y + WIN_H as f32 / (2.0 * app.view.zoom) - map.height as f32 / 2.0).abs()
            < 0.01
    );
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
