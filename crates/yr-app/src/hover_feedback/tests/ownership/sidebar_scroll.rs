use super::*;

fn fixture(size: (u32, u32)) -> App {
    let (mut app, own, _) = scene(size);
    let session = app.session.as_mut().unwrap();
    // Extra authored cards exercise long lists without advancing production.
    let cards = session.buildables.clone();
    for _ in 0..6 {
        session.buildables.extend(cards.iter().cloned());
    }
    app.pending.push(Command::Stop { id: own });
    app.map.as_mut().unwrap().rgba = [30, 45, 30, 255]
        .into_iter()
        .cycle()
        .take(1600 * 1200 * 4)
        .collect();
    app
}

fn arrow(size: (u32, u32), down: bool) -> (f64, f64) {
    (
        f64::from(size.0 - SIDEBAR_W + if down { 90 } else { 30 }),
        f64::from(size.1 - 14),
    )
}

fn row(app: &App) -> usize {
    app.sidebar_view
        .first_row(app.session.as_ref().unwrap(), app.frame_size.1)
}

fn frame(app: &App) -> Vec<u8> {
    let (width, height) = app.frame_size;
    let mut pixels = vec![0; (width * height * 4) as usize];
    app.compose(&mut pixels, width, height);
    pixels
}

fn capture(app: &App, name: &str) {
    if app.frame_size != (640, 302) {
        return;
    }
    let Some(path) = std::env::var_os("OPENYR_SCROLL_HOLD_PROOF") else {
        return;
    };
    let path = PathBuf::from(path);
    std::fs::create_dir_all(&path).unwrap();
    let size = app.frame_size;
    std::fs::write(
        path.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame(app)),
    )
    .unwrap();
    println!(
        "{name}: authored scene, tick {}, first row {}, held {}",
        app.session.as_ref().unwrap().sim.tick(),
        row(app),
        app.sidebar_view.pointer_pressed()
    );
}

#[test]
fn sidebar_scroll_hold_delays_repeats_and_never_catches_up_or_switches_arrows() {
    for size in [(640, 302), (800, 352), (1280, 480)] {
        let mut app = fixture(size);
        let session = app.session.as_ref().unwrap();
        let before = yr_app::snapshot::encode(session).unwrap();
        let selected = app.selected.clone();
        let pending = app.pending.clone();
        let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
        let now = Instant::now();
        let down = arrow(size, true);
        app.pointer_moved(down);
        // Pin the native press clock to check exact delay/interval boundaries.
        assert!(app
            .sidebar_view
            .press_scroll(app.session.as_ref().unwrap(), size, down, now));
        assert_eq!(row(&app), 1);
        capture(&app, "01-pressed-0ms");
        let held = frame(&app);
        let mut released = app.sidebar_view;
        released.clear_press();
        let held_view = std::mem::replace(&mut app.sidebar_view, released);
        assert_ne!(frame(&app), held, "held arrow has visible feedback");
        app.sidebar_view = held_view;
        for (ms, expected) in [
            (349, 1),
            (350, 2),
            (429, 2),
            (430, 3),
            (30_000, 4),
            (30_000, 4),
        ] {
            app.repeat_sidebar_scroll(now + Duration::from_millis(ms));
            assert_eq!(row(&app), expected, "{size:?} at {ms}ms");
        }
        capture(&app, "02-delayed-redraw-30000ms");
        // Moving to the other arrow pauses the original direction, without
        // changing it or accumulating missed repeats.
        app.cursor = arrow(size, false);
        app.repeat_sidebar_scroll(now + Duration::from_secs(40));
        assert_eq!(row(&app), 4);
        app.cursor = down;
        app.repeat_sidebar_scroll(now + Duration::from_secs(40));
        assert_eq!(row(&app), 4);
        app.repeat_sidebar_scroll(now + Duration::from_millis(40_350));
        assert_eq!(row(&app), 5);
        // A release cannot issue a production or world command.
        app.pointer_button(ElementState::Released, MouseButton::Left);
        app.repeat_sidebar_scroll(now + Duration::from_secs(80));
        assert_eq!(row(&app), 5);
        capture(&app, "03-released");
        assert_eq!(app.selected, selected);
        assert_eq!(app.pending, pending);
        assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            before
        );
    }
    println!("Three layouts retain exact initial/350ms/80ms boundaries, one-row delayed redraws, origin-only leave/reentry and visible pressed feedback; Session bytes, selection, camera and prior commands remain exact.");
}

#[test]
fn sidebar_scroll_hold_bounds_cancellation_and_fresh_input_recovery() {
    for route in 0..8 {
        let size = (640, 352);
        let mut app = fixture(size);
        let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        app.menu.edge_scroll = true;
        app.view.cam_x = 300.;
        app.view.cam_y = 200.;
        app.pointer_moved(arrow(size, true));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        let now = Instant::now();
        let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
        for pos in [(470., 200.), arrow(size, false)] {
            app.pointer_moved(pos);
            assert!(app.edge_cursor(size).is_none());
            app.pointer_wheel(MouseScrollDelta::LineDelta(0., -1.));
            app.repeat_sidebar_scroll(now + Duration::from_secs(60));
            app.update_camera(now, size);
            assert_eq!(row(&app), 1);
            assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
            assert!(app.sidebar_view.pointer_pressed());
        }
        match route {
            0 => app.pointer_button(ElementState::Released, MouseButton::Left),
            1 => app.invalidate_pointer(),
            2 => {
                app.focus_changed(false);
                app.focus_changed(true);
            }
            3 => {
                app.open_menu(Page::Pause);
                app.menu.set(Page::Closed);
            }
            4 => app.pointer_button(ElementState::Pressed, MouseButton::Right),
            5 => {
                key(&mut app, KeyCode::KeyW, ElementState::Pressed);
            }
            6 => {
                app.resize_window((640, 352), 1.).unwrap();
            }
            _ => {
                key(&mut app, KeyCode::ArrowLeft, ElementState::Pressed);
                key(&mut app, KeyCode::ArrowLeft, ElementState::Released);
            }
        }
        assert!(!app.sidebar_view.pointer_pressed(), "route {route}");
        app.pointer_moved(arrow(size, true));
        let first = row(&app);
        app.repeat_sidebar_scroll(now + Duration::from_secs(120));
        assert_eq!(row(&app), first);
        app.pointer_button(ElementState::Released, MouseButton::Right);
        app.sidebar_view.select(sidebar::Category::Structures);
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        assert!(
            app.sidebar_view.pointer_pressed(),
            "fresh arrow press after route {route}"
        );
        // Hold down to the end, then up to the beginning. Disabled endpoints
        // keep the gesture until release but cannot advance out of bounds.
        for seconds in 1..=30 {
            app.repeat_sidebar_scroll(now + Duration::from_secs(120 + seconds));
        }
        let bottom = row(&app);
        assert!(bottom > 1);
        app.repeat_sidebar_scroll(now + Duration::from_secs(200));
        assert_eq!(row(&app), bottom);
        assert!(app.sidebar_view.pointer_pressed());
        app.pointer_button(ElementState::Released, MouseButton::Left);
        app.pointer_moved(arrow(size, false));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        assert_eq!(row(&app), bottom - 1);
        for seconds in 1..=30 {
            app.repeat_sidebar_scroll(now + Duration::from_secs(200 + seconds));
        }
        assert_eq!(row(&app), 0);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        assert!(
            !app.sidebar_view.pointer_pressed(),
            "disabled up arrow stays inert"
        );
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            before
        );
    }
    println!("Eight cancellation routes discard scroll holds without rearming; fresh presses recover, both endpoints clamp, and held gestures suppress edge/wheel actions without changing complete Session bytes.");
}
