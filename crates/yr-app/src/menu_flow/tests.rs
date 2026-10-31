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
    let layout = Layout::new(WIN_W, WIN_H);
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
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.load_job.is_some() {
        app.poll_menu_load();
        assert!(Instant::now() < deadline, "match load timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn preferences_are_bounded_and_errors_remain_recoverable() {
    let temp = Scratch::new();
    let path = temp.0.join("interface.json");
    let mut app = new_app(String::new(), String::new(), 1);
    app.read_preferences_file(path.clone());
    app.view.zoom = 1.5;
    app.apply_menu_effect(Effect::Mute);
    let mut reopened = new_app(String::new(), String::new(), 1);
    reopened.read_preferences_file(path.clone());
    assert!(reopened.menu.muted);
    assert_eq!(reopened.view.zoom, 1.5);
    for bytes in [
        br#"{"muted":false,"zoom":0}"#.to_vec(),
        vec![b' '; 16 * 1024 + 1],
        br#"{"muted":false,"zoom":100,"extra":1}"#.to_vec(),
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
