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
    app.map = Some(yr_render::render_map_background(&app.gamedir, &app.map_name, 1).unwrap());
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
        "map_size":[app.map.as_ref().unwrap().width,app.map.as_ref().unwrap().height], "viewport":[WIN_W-SIDEBAR_W,WIN_H],
        "cursor_position":[app.cursor.0,app.cursor.1], "cursor_kind":format!("{:?}",app.cursor_kind((WIN_W, WIN_H)))
    })).unwrap()).unwrap();
}

#[test]
fn scroll_directions_follow_bounds_for_every_zoom_and_viewport() {
    let map = map(1600, 900);
    for zoom in [0.25, 1.0, 1.25, 4.0] {
        for viewport in [(1112, 800), (632, 600), (1592, 1000)] {
            let mut view = View {
                cam_x: 400.0,
                cam_y: 300.0,
                zoom,
                dirty: false,
            };
            view.constrain(&map, viewport);
            for requested in Direction::ALL {
                let available = view.available_scroll(requested, &map, viewport);
                let before = (view.cam_x, view.cam_y);
                let mut moved = View { ..view };
                if let Some(direction) = available {
                    let (x, y) = direction.axes();
                    moved.pan((f32::from(x), f32::from(y)), MAX_FRAME);
                }
                moved.constrain(&map, viewport);
                let delta = (moved.cam_x - before.0, moved.cam_y - before.1);
                let sign = |value: f32| match value.partial_cmp(&0.0).unwrap() {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                let actual = Direction::from_axes(sign(delta.0), sign(delta.1));
                assert_eq!(available, actual, "{zoom} {viewport:?} {requested:?}");
            }
            view.cam_x = 0.0;
            view.cam_y = 0.0;
            view.constrain(&map, viewport);
            assert_eq!(
                view.available_scroll(Direction::NorthWest, &map, viewport),
                None
            );
            assert_eq!(
                view.available_scroll(Direction::NorthEast, &map, viewport),
                ((viewport.0 as f32 / zoom) < map.width as f32).then_some(Direction::East)
            );
        }
    }
}

#[test]
fn retail_edge_cursors_share_motion_guards_and_preserve_match_bytes() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map_background(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    app.menu.edge_scroll = true;
    app.pointer.inside = true;
    app.camera.edge_armed = true;
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let size = (WIN_W, WIN_H);
    let viewport = (size.0 - SIDEBAR_W, size.1);
    let edge_point = |direction: Direction| {
        let (x, y) = direction.axes();
        let axis = |a: i8, extent: u32| match a {
            -1 => 0.0,
            1 => f64::from(extent - 1),
            _ => f64::from(extent / 2),
        };
        (axis(x, viewport.0), axis(y, viewport.1))
    };
    for blocked in [false, true] {
        for direction in Direction::ALL {
            app.view.center_map(app.map.as_ref().unwrap(), viewport);
            app.cursor = edge_point(direction);
            if blocked {
                let (x, y) = direction.axes();
                app.view.cam_x = f32::from(x) * 100_000.0;
                app.view.cam_y = f32::from(y) * 100_000.0;
                app.bound_camera(size);
            }
            assert_eq!(app.cursor_kind(size), Kind::Scroll(direction, blocked));
            // The actual live press/release handler cannot issue an edge order,
            // place a structure or act on a repair/sale target under the arrow.
            for mode in [ServiceMode::None, ServiceMode::Repair, ServiceMode::Sell] {
                app.service_mode = mode;
                app.placing = Some(0);
                app.left_press();
                app.left_release();
                assert!(app.left_down.is_none() && app.pending.is_empty());
                assert_eq!(app.placing, Some(0));
            }
            app.placing = None;
            app.service_mode = ServiceMode::None;
            app.menu.edge_scroll = false;
            proof(
                &app,
                &format!("before-{direction:?}-blocked-{blocked}-tick1"),
            );
            app.menu.edge_scroll = true;
            proof(
                &app,
                &format!("scroll-{direction:?}-blocked-{blocked}-tick1"),
            );
            let before = (app.view.cam_x, app.view.cam_y);
            app.camera.frame_at = None;
            let now = Instant::now();
            app.update_camera(now, size);
            app.update_camera(now + MAX_FRAME, size);
            assert_eq!(before == (app.view.cam_x, app.view.cam_y), blocked);
        }
    }
    app.view.cam_x = 100.0;
    app.view.cam_y = 0.0;
    app.cursor = edge_point(Direction::NorthEast);
    assert_eq!(app.cursor_kind(size), Kind::Scroll(Direction::East, false));
    proof(&app, "scroll-partial-corner-East-tick1");
    let now = Instant::now();
    app.camera.frame_at = None;
    app.update_camera(now, size);
    app.update_camera(now + MAX_FRAME, size);
    assert!((app.view.cam_x - 160.0).abs() < 0.001);
    assert_eq!(app.view.cam_y, 0.0);
    // Once the pointer leaves the edge zone, ordinary press/selection resumes.
    app.cursor = (400.0, 300.0);
    assert!(app.edge_cursor(size).is_none());
    app.left_press();
    assert_eq!(app.left_down, Some(app.cursor));
    app.cancel_pointer_action();
    app.camera.edge_armed = true;
    app.view.center_map(app.map.as_ref().unwrap(), viewport);
    app.cursor = edge_point(Direction::East);
    for guard in 0..13 {
        app.left_down = (guard == 0).then_some(app.cursor);
        app.right_down = (guard == 1).then_some(app.cursor);
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
        app.menu.edge_scroll = guard != 8;
        app.keys.clear();
        if guard == 9 {
            app.keys.insert(KeyCode::ArrowUp);
        }
        app.session.as_mut().unwrap().outcome = if guard == 10 {
            Outcome::Won
        } else {
            Outcome::Ongoing
        };
        app.cursor = match guard {
            11 => (f64::from(WIN_W - 1), 400.0),
            12 => (f64::NAN, 400.0),
            _ => edge_point(Direction::East),
        };
        assert_eq!(app.edge_cursor(size), None, "guard {guard}");
    }
    app.session.as_mut().unwrap().outcome = Outcome::Ongoing;
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    println!("EB1: all 16 scroll cursor states, matching movement, guarded clicks, focus/menu/input guards and unchanged complete match bytes passed");
}
