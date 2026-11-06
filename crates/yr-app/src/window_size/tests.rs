use super::*;

#[test]
fn frames_fit_physical_and_logical_bounds_without_unbounded_allocations() {
    for physical in [
        (1, 1),
        (640, 480),
        (1000, 700),
        (1280, 800),
        (1600, 1000),
        (2560, 1600),
        (u32::MAX, u32::MAX),
    ] {
        for scale in [0.5, 1.0, 1.25, 1.5, 2.0, 3.0, f64::MIN_POSITIVE, f64::MAX] {
            let (w, h) = frame_for_surface(physical, scale).unwrap();
            assert!(
                w > 0 && h > 0 && w <= physical.0 && h <= physical.1 && w <= WIN_W && h <= WIN_H
            );
        }
    }
    assert_eq!(frame_for_surface((640, 480), 1.0), Some((640, 480)));
    assert_eq!(frame_for_surface((1000, 700), 1.0), Some((1000, 700)));
    assert_eq!(frame_for_surface((2560, 1600), 2.0), Some((1280, 800)));
    assert_eq!(frame_for_surface((1280, 960), 2.0), Some((640, 480)));
    assert_eq!(frame_for_surface((1600, 1000), 1.0), Some((1280, 800)));
    for physical in [(0, 480), (640, 0), (0, 0)] {
        assert_eq!(frame_for_surface(physical, 1.0), None);
    }
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(frame_for_surface((640, 480), scale), None);
    }
}

#[test]
fn compact_menu_buttons_and_slider_drags_use_the_same_frame_as_drawing() {
    use yr_app::menu::{setup::Field, Button, Layout, Page};
    for (physical, scale) in [
        ((640, 480), 1.0),
        ((1000, 700), 1.0),
        ((1280, 960), 2.0),
        ((2560, 1600), 2.0),
    ] {
        let mut app = new_app(String::new(), String::new(), 1);
        app.resize_window(physical, scale).unwrap();
        let size = app.camera_size();
        app.menu.front = true;
        app.menu.set(Page::Main);
        let layout = Layout::for_state(&app.menu, size.0, size.1);
        let i = app
            .menu
            .buttons()
            .iter()
            .position(|b| b.0 == Button::Quit)
            .unwrap();
        let (x, y, _, _) = layout.button(i, Button::Quit);
        app.pointer_moved((
            layout.origin.0 + f64::from(x + 10) * layout.scale,
            layout.origin.1 + f64::from(y + 10) * layout.scale,
        ));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        assert_eq!(app.menu.page, Page::Exit, "{size:?}");
        app.menu.set(Page::Skirmish);
        let layout = Layout::for_state(&app.menu, size.0, size.1);
        let at = |x, y| {
            (
                layout.origin.0 + f64::from(x) * layout.scale,
                layout.origin.1 + f64::from(y) * layout.scale,
            )
        };
        app.pointer_moved(at(410, 350));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_moved(at(477, 350));
        app.pointer_button(ElementState::Released, MouseButton::Left);
        assert_eq!(app.menu.setup.focus, Some(Field::Units));
        assert_eq!(app.menu.setup.value(Field::Units), 10);
        assert!(app.menu.setup.drag.is_none());
    }
}

#[test]
fn resize_preserves_zoom_and_cancels_old_gestures_even_when_minimized() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.view.zoom = 1.5;
    app.pointer_moved((300.0, 300.0));
    app.left_down = Some(app.cursor);
    app.resize_window((640, 480), 1.0).unwrap();
    assert_eq!(app.camera_size(), (640, 480));
    assert_eq!(app.view.zoom, 1.5);
    assert!(!app.pointer.inside && app.left_down.is_none());
    app.resize_window((0, 0), 1.0).unwrap();
    assert_eq!(app.camera_size(), (640, 480));
    app.resize_window((1920, 1200), 1.0).unwrap();
    assert_eq!(app.camera_size(), (1280, 800));
    assert_eq!(app.view.zoom, 1.5);
}

#[test]
fn retail_compact_start_loading_sidebar_selection_and_pause_share_dimensions() {
    use yr_app::menu::{Button, Layout, Page};
    let directory = std::env::var_os("OPENYR_GAMEDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
        });
    if !directory.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&directory).unwrap();
    let click = |app: &mut App, button: Button| {
        let size = app.camera_size();
        let layout = Layout::for_state(&app.menu, size.0, size.1);
        let i = app
            .menu
            .buttons()
            .iter()
            .position(|b| b.0 == button)
            .unwrap();
        let (x, y, _, _) = layout.button(i, button);
        app.pointer_moved((
            layout.origin.0 + f64::from(x + 10) * layout.scale,
            layout.origin.1 + f64::from(y + 10) * layout.scale,
        ));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_button(ElementState::Released, MouseButton::Left);
    };
    let proof = |app: &App, name: &str| {
        if let Some(out) = std::env::var_os("OPENYR_COMPACT_PROOF") {
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
        }
    };
    for size in [(640, 480), (1000, 700), (1280, 800)] {
        let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
        app.resize_window(size, 1.0).unwrap();
        app.view.zoom = 1.25;
        app.front_art = yr_app::frontend::Art::load(&fs);
        app.loading_art = yr_app::loading::Art::load(&fs);
        app.menu_art = yr_app::menu::Art::load(&fs);
        app.front_preview =
            Some(yr_app::frontend::load_preview(&app.gamedir, &app.map_name, 1).unwrap());
        app.menu.front = true;
        app.menu.set(Page::Main);
        proof(&app, "main");
        click(&mut app, Button::SinglePlayer);
        click(&mut app, Button::Skirmish);
        assert_eq!(app.menu.page, Page::Skirmish);
        proof(&app, "skirmish");
        click(&mut app, Button::Start);
        assert_eq!(app.menu.page, Page::Loading);
        proof(&app, "loading");
        let until = Instant::now() + Duration::from_secs(180);
        while app.load_job.is_some() {
            app.poll_menu_load();
            assert!(Instant::now() < until, "compact match load timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(app.menu.page, Page::Closed);
        assert_eq!(app.view.zoom, 1.25);
        let map = app.map.as_ref().unwrap();
        assert!(
            ((map.width as f32 / 2.0 - app.view.cam_x) * app.view.zoom
                - (size.0 - SIDEBAR_W) as f32 / 2.0)
                .abs()
                < 0.01
        );
        assert!(
            ((map.height as f32 / 2.0 - app.view.cam_y) * app.view.zoom - size.1 as f32 / 2.0)
                .abs()
                < 0.01
        );
        let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        let (x, y) = sidebar::tab_origin(size.0, Category::Vehicles);
        app.pointer_moved((f64::from(x + 10), f64::from(y + 10)));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        assert_eq!(app.sidebar_view.category, Category::Vehicles);
        let unit = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .find(|u| u.owner == yr_sim::OWNER_PLAYER)
            .unwrap()
            .clone();
        app.center_world_at(
            (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0),
            (size.0 - SIDEBAR_W, size.1),
        );
        app.pointer_moved((f64::from(size.0 - SIDEBAR_W) / 2.0, f64::from(size.1) / 2.0));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        assert!(app.selected.contains(&unit.id));
        proof(&app, "selected-match");
        app.escape_command_or_pause();
        assert_eq!(app.menu.page, Page::Pause);
        proof(&app, "pause");
        click(&mut app, Button::Resume);
        assert_eq!(app.menu.page, Page::Closed);
        assert!(app.pending.is_empty());
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            saved
        );
    }
}
