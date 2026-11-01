use super::*;

fn map(width: u32, height: u32) -> yr_render::RenderedMap {
    yr_render::RenderedMap {
        width,
        height,
        rgba: Vec::new(),
        summary: String::new(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    }
}

#[test]
fn pan_distance_is_independent_of_redraw_rate_zoom_and_diagonal() {
    for zoom in [0.25, 1.0, 1.25, 4.0] {
        for fps in [30, 60, 144] {
            for direction in [(1.0, 0.0), (-1.0, -1.0)] {
                let mut view = View {
                    cam_x: 0.0,
                    cam_y: 0.0,
                    zoom,
                    dirty: false,
                };
                let mut input = Input::default();
                let start = Instant::now();
                for frame in 0..=fps {
                    let elapsed = input.elapsed(
                        start + Duration::from_secs_f64(f64::from(frame) / f64::from(fps)),
                    );
                    view.pan(direction, elapsed);
                }
                let distance = view.cam_x.hypot(view.cam_y) * zoom;
                assert!(
                    (distance - 600.0).abs() < 0.01,
                    "{fps}Hz, zoom {zoom}, {direction:?}: {distance}"
                );
            }
        }
    }
    let mut input = Input::default();
    let start = Instant::now();
    assert_eq!(input.elapsed(start), Duration::ZERO);
    assert_eq!(input.elapsed(start + Duration::from_secs(20)), MAX_FRAME);
}

#[test]
fn camera_bounds_center_small_maps_and_exclude_sidebar_width() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.map = Some(map(1600, 900));
    for zoom in [0.25, 1.0, 1.25, 4.0] {
        app.view.zoom = zoom;
        for size in [(1280, 800), (800, 600), (1760, 1000)] {
            for target in [-100_000.0, 100_000.0] {
                app.view.cam_x = target;
                app.view.cam_y = target;
                app.bound_camera(size);
                let visible = ((size.0 - SIDEBAR_W) as f32 / zoom, size.1 as f32 / zoom);
                for (position, extent, span) in [
                    (app.view.cam_x, 1600.0, visible.0),
                    (app.view.cam_y, 900.0, visible.1),
                ] {
                    if span > extent {
                        assert_eq!(position + span * 0.5, extent * 0.5);
                    } else {
                        assert!(position >= 0.0 && position + span <= extent + 0.001);
                    }
                }
            }
        }
    }
    app.left_down = Some((12.0, 12.0));
    app.right_down = Some((12.0, 12.0));
    app.selected.insert(123);
    app.zoom_camera(0.01);
    assert_eq!(app.view.zoom, 0.25);
    assert!(app.left_down.is_none() && app.right_down.is_none());
    app.left_release();
    app.right_release();
    assert!(app.pending.is_empty());
    assert_eq!(app.selected, [123].into());
}

#[test]
fn edge_zones_reject_sidebar_outside_letterboxing_and_invalid_points() {
    let viewport = (1112, 800);
    assert_eq!(edge_direction((0.0, 0.0), viewport), (-1.0, -1.0));
    assert_eq!(edge_direction((1111.0, 799.0), viewport), (1.0, 1.0));
    assert_eq!(edge_direction((1100.0, 400.0), viewport), (1.0, 0.0));
    assert_eq!(edge_direction((1099.0, 400.0), viewport), (0.0, 0.0));
    for cursor in [
        (1112.0, 0.0),
        (1279.0, 799.0),
        (-1.0, 0.0),
        (0.0, -1.0),
        (0.0, 800.0),
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
    ] {
        assert_eq!(edge_direction(cursor, viewport), (0.0, 0.0));
    }
    assert_eq!(edge_direction((0.0, 0.0), (0, 0)), (0.0, 0.0));
    assert_eq!(edge_direction((0.0, 0.0), (12, 800)), (0.0, 0.0));
}

#[test]
fn retail_camera_handlers_preserve_match_state_and_guard_gestures() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    app.menu_art = yr_app::menu::Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let size = (1280, 800);
    let now = Instant::now();
    app.view.center_map(app.map.as_ref().unwrap(), (1112, 800));
    app.update_camera(now, size);
    proof(&app, "01-center-tick1");
    let center = (app.view.cam_x, app.view.cam_y);
    app.cursor = (1111.0, 400.0);
    app.pointer.inside = true;
    app.camera.edge_armed = true;
    app.update_camera(now + MAX_FRAME, size);
    assert_eq!(
        (app.view.cam_x, app.view.cam_y),
        center,
        "edge scrolling defaults off"
    );
    app.apply_menu_effect(yr_app::menu::Effect::EdgeScroll);
    assert!(app.menu.edge_scroll && !app.camera.edge_armed);
    app.camera.edge_armed = true;
    app.update_camera(now + MAX_FRAME * 2, size);
    assert!((app.view.cam_x - center.0 - 60.0).abs() < 0.001);
    proof(&app, "02-edge-right-tick1");
    let stopped = (app.view.cam_x, app.view.cam_y);
    for guard in 0..8 {
        app.left_down = (guard == 0).then_some((10.0, 10.0));
        app.right_down = (guard == 1).then_some((10.0, 10.0));
        app.right_panning = guard == 2;
        app.radar_drag = guard == 3;
        app.pointer.inside = guard != 4;
        app.pointer.focused = guard != 5;
        app.menu.set(if guard == 6 {
            yr_app::menu::Page::Pause
        } else {
            yr_app::menu::Page::Closed
        });
        app.camera.edge_armed = guard != 7;
        app.update_camera(now + MAX_FRAME * (guard + 3), size);
        assert_eq!((app.view.cam_x, app.view.cam_y), stopped, "guard {guard}");
    }
    app.menu.set(yr_app::menu::Page::Closed);
    app.pointer.focused = true;
    app.pointer.inside = true;
    app.left_down = Some((10.0, 10.0));
    app.keys.insert(KeyCode::ArrowLeft);
    app.update_camera(now + MAX_FRAME * 11, size);
    assert!(app.left_down.is_none());
    app.left_release();
    assert!(!app.camera.edge_armed && app.pending.is_empty());
    app.focus_changed(false);
    assert!(app.keys.is_empty());
    app.focus_changed(true);
    app.view.cam_x = -10_000.0;
    app.view.cam_y = -10_000.0;
    proof(&app, "03-unbounded-request-tick1");
    app.update_camera(now + MAX_FRAME * 12, size);
    assert_eq!((app.view.cam_x, app.view.cam_y), (0.0, 0.0));
    proof(&app, "04-bounded-origin-tick1");
    app.zoom_camera(0.25);
    proof(&app, "05-zoom-out-tick1");
    app.open_menu(yr_app::menu::Page::Controls);
    proof(&app, "06-options-tick1");
    // Exercise the actual hit/release path in both supported options surfaces.
    for front in [false, true] {
        app.menu.front = front;
        if front {
            app.front_art = yr_app::frontend::Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
        }
        app.menu.set(yr_app::menu::Page::Controls);
        let layout = yr_app::menu::Layout::for_state(&app.menu, size.0, size.1);
        let index = app
            .menu
            .buttons()
            .iter()
            .position(|b| b.0 == yr_app::menu::Button::EdgeScroll)
            .unwrap();
        let (x, y, _, _) = layout.button(index, yr_app::menu::Button::EdgeScroll);
        app.cursor = (
            layout.origin.0 + f64::from(x + 8) * layout.scale,
            layout.origin.1 + f64::from(y + 8) * layout.scale,
        );
        let before = app.menu.edge_scroll;
        app.menu_mouse(ElementState::Pressed, MouseButton::Left);
        app.menu_mouse(ElementState::Released, MouseButton::Left);
        assert_ne!(app.menu.edge_scroll, before);
        if front {
            proof(&app, "07-front-options-tick1");
        }
    }
    app.menu.front = false;
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    assert!(app.pending.is_empty());
    println!("EB1 camera handlers: edge/key pan, focus/modal/gesture guards, zoom/bounds and complete match save invariance passed");
}

fn proof(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_CAMERA_INPUT_PROOF") else {
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
    std::fs::write(dir.join(format!("{name}.json")), serde_json::to_vec_pretty(&serde_json::json!({
        "tick":app.session.as_ref().unwrap().sim.tick(), "hash":format!("{:016x}",app.session.as_ref().unwrap().sim.state_hash()),
        "camera":[app.view.cam_x,app.view.cam_y], "zoom":app.view.zoom, "edge_scroll":app.menu.edge_scroll,
        "map_size":[app.map.as_ref().unwrap().width,app.map.as_ref().unwrap().height], "viewport":[WIN_W-SIDEBAR_W,WIN_H]
    })).unwrap()).unwrap();
}
