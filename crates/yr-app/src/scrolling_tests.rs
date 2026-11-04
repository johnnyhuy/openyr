use super::*;

#[test]
fn retail_sidebar_scrolling_reaches_every_slot_and_routes_production_without_zoom() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let session = app.session.as_mut().unwrap();
    session.sim.set_ai(None);
    check_viewports(session);
    let before = yr_app::snapshot::encode(session).unwrap();
    let zoom = app.view.zoom;
    let size = (800, 352); // Two complete cameo rows above the bottom controls.
    app.pointer.inside = true;
    let slots: Vec<_> = sidebar::slots(session, Category::Structures).collect();
    assert!(slots.len() > 4);
    app.cursor = (f64::from(size.0 - SIDEBAR_W + 90), 240.0);
    capture(&mut app, size, "01-top");
    let mut reached = BTreeSet::new();
    loop {
        let session = app.session.as_ref().unwrap();
        let row = app.sidebar_view.first_row(session, size.1);
        for (i, &slot) in slots.iter().skip(row * 2).take(4).enumerate() {
            let (x, y) = sidebar::slot_origin(size.0, i);
            for (dx, dy) in [(0, 0), (59, 47)] {
                assert_eq!(
                    app.sidebar_slot_at((f64::from(x + dx), f64::from(y + dy)), size),
                    Some(slot)
                );
            }
            reached.insert(slot);
        }
        for (x, y) in [(22, 228), (83, 228), (23, 276), (23, 326), (85, 328)] {
            assert_eq!(
                app.sidebar_slot_at((f64::from(size.0 - SIDEBAR_W + x), f64::from(y)), size),
                None
            );
        }
        app.match_wheel(MouseScrollDelta::LineDelta(0.0, -1.0), size);
        if app
            .sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1)
            == row
        {
            break;
        }
    }
    assert_eq!(reached, slots.iter().copied().collect());
    capture(&mut app, size, "02-bottom");
    let bottom = app
        .sidebar_view
        .first_row(app.session.as_ref().unwrap(), size.1);
    assert!(bottom > 0);
    app.match_wheel(MouseScrollDelta::LineDelta(0.0, -f32::MAX), size);
    assert_eq!(
        app.sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1),
        bottom
    );
    assert_eq!(app.view.zoom, zoom);
    // Wheel input stays inside the panel even over its non-scrolling header.
    app.cursor.1 = 20.0;
    app.match_wheel(MouseScrollDelta::LineDelta(0.0, -1.0), size);
    assert_eq!(app.view.zoom, zoom);
    // The enabled up button moves exactly one row and never queues a build.
    app.cursor = (f64::from(size.0 - SIDEBAR_W + 30), f64::from(size.1 - 20));
    app.sidebar_left_press(size);
    assert_eq!(
        app.sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1),
        bottom - 1
    );
    assert!(app.pending.is_empty());
    // Actual tab handling preserves each category's position.
    for category in [Category::Vehicles, Category::Structures] {
        let (x, y) = sidebar::tab_origin(size.0, category);
        app.cursor = (f64::from(x + 2), f64::from(y + 2));
        app.sidebar_left_press(size);
        assert_eq!(app.sidebar_view.category, category);
    }
    assert_eq!(
        app.sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1),
        bottom - 1
    );
    // Enlarging the framebuffer clamps both drawing and input to the top.
    let session = app.session.as_ref().unwrap();
    assert_eq!(app.sidebar_view.first_row(session, 800), 0);
    assert_eq!(
        sidebar::slot_at(800, 800, session, app.sidebar_view, (655.0, 228.0)),
        Some(slots[0])
    );
    assert_eq!(yr_app::snapshot::encode(session).unwrap(), before);

    // A scrolled card uses its original id for actual production handlers.
    app.sidebar_view.select(Category::Vehicles);
    let session = app.session.as_ref().unwrap();
    let vehicles: Vec<_> = sidebar::slots(session, Category::Vehicles).collect();
    assert!(vehicles.len() > 2);
    let compact = (800, 302); // One complete row.
    app.sidebar_view.scroll(session, compact.1, 1);
    let slot = vehicles[2];
    // An unavailable entry must still hover but must not enqueue a different id.
    app.cursor = (655.0, 240.0);
    assert_eq!(app.sidebar_slot_at(app.cursor, compact), Some(slot));
    assert!(!session.slot_available(slot), "SREF needs the Battle Lab");
    app.sidebar_left_press(compact);
    assert!(app.pending.is_empty());
    // CMIN is the fourth vehicle, reachable only after scrolling this view.
    let miner = vehicles[3];
    assert_eq!(app.session.as_ref().unwrap().buildables[miner].id, "CMIN");
    app.cursor = (717.0, 240.0);
    app.sidebar_left_press(compact);
    app.tick_session();
    assert!(app.session.as_ref().unwrap().slot_head(miner).is_some());
    app.sidebar_right_press(compact);
    app.tick_session();
    assert!(app.session.as_ref().unwrap().slot_head(miner).unwrap().held);
    capture(&mut app, compact, "03-scrolled-held");
    app.sidebar_left_press(compact);
    app.tick_session();
    assert!(!app.session.as_ref().unwrap().slot_head(miner).unwrap().held);
    // READY work survives scrolling out of view and reappears for placement.
    app.sidebar_view.select(Category::Structures);
    app.sidebar_view
        .scroll(app.session.as_ref().unwrap(), compact.1, i32::MIN);
    app.cursor = (655.0, 240.0);
    let plant = slots[0];
    app.sidebar_left_press(compact);
    app.tick_session();
    for _ in 0..1000 {
        if app.session.as_ref().unwrap().ready_structure() == Some(plant) {
            break;
        }
        app.tick_session();
    }
    assert_eq!(app.session.as_ref().unwrap().ready_structure(), Some(plant));
    let ready_bytes = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    app.match_wheel(MouseScrollDelta::LineDelta(0.0, -f32::MAX), compact);
    assert_ne!(app.sidebar_slot_at(app.cursor, compact), Some(plant));
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        ready_bytes
    );
    app.match_wheel(MouseScrollDelta::LineDelta(0.0, f32::MAX), compact);
    app.sidebar_left_press(compact);
    assert_eq!(app.placing, Some(plant));
    assert!(app.pending.is_empty());
    capture(&mut app, compact, "04-ready");
    app.placing = None;
    // Fractional pixel motion accumulates; focus cleanup discards old input.
    app.sidebar_view
        .scroll(app.session.as_ref().unwrap(), size.1, i32::MIN);
    for _ in 0..5 {
        app.match_wheel(
            MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, -10.0)),
            size,
        );
    }
    assert_eq!(
        app.sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1),
        1
    );
    app.sidebar_view
        .scroll(app.session.as_ref().unwrap(), size.1, i32::MIN);
    for _ in 0..4 {
        app.match_wheel(
            MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, -10.0)),
            size,
        );
    }
    assert_eq!(
        app.sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1),
        0
    );
    app.cancel_pointer_action();
    app.match_wheel(
        MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, -10.0)),
        size,
    );
    assert_eq!(
        app.sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1),
        0
    );
    app.match_wheel(MouseScrollDelta::LineDelta(0.0, -1.0), size);
    assert_eq!(
        app.sidebar_view
            .first_row(app.session.as_ref().unwrap(), size.1),
        1
    );
    for delta in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0] {
        app.match_wheel(MouseScrollDelta::LineDelta(0.0, delta), size);
    }
    for guard in 0..5 {
        app.pointer.focused = guard != 0;
        app.pointer.inside = guard != 1;
        app.left_down = (guard == 2).then_some((0.0, 0.0));
        app.right_down = (guard == 3).then_some((0.0, 0.0));
        app.radar_drag = guard == 4;
        app.match_wheel(MouseScrollDelta::LineDelta(0.0, -1.0), size);
        assert_eq!(
            app.sidebar_view
                .first_row(app.session.as_ref().unwrap(), size.1),
            1
        );
    }
    app.pointer.focused = true;
    app.radar_drag = false;
    app.cursor = (100.0, 100.0);
    app.match_wheel(MouseScrollDelta::LineDelta(0.0, 1.0), size);
    assert!(app.view.zoom > zoom, "world wheel still zooms");
}

fn capture(app: &mut App, size: (u32, u32), name: &str) {
    let session = app.session.as_ref().unwrap();
    let before = yr_app::snapshot::encode(session).unwrap();
    let tick = session.sim.tick();
    let row = app.sidebar_view.first_row(session, size.1);
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut frame, size.0, size.1);
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        before
    );
    if let Some(out) = std::env::var_os("OPENYR_SIDEBAR_SCROLL_PROOF").map(PathBuf::from) {
        std::fs::create_dir_all(&out).unwrap();
        let path = out.join(format!("{name}-t{tick:05}-row{row}.png"));
        std::fs::write(&path, yr_render::png::encode_rgba(size.0, size.1, &frame)).unwrap();
        println!("{}: tick {tick}, first row {row}", path.display());
    }
}

fn check_viewports(session: &Session) {
    for category in Category::ALL {
        let ids: Vec<_> = sidebar::slots(session, category).collect();
        let mut reference = vec![0; (SIDEBAR_W * 800 * 4) as usize];
        sidebar::draw(&mut reference, SIDEBAR_W, 800, session, category, None);
        for (height, rows) in [(0, 0), (301, 0), (302, 1), (351, 1), (352, 2), (800, 10)] {
            for width in [SIDEBAR_W, 800] {
                let mut view = sidebar::View::from(category);
                let mut reached = BTreeSet::new();
                loop {
                    let first = view.first_row(session, height);
                    let mut frame = vec![0; (width * height * 4) as usize];
                    sidebar::draw(&mut frame, width, height, session, view, None);
                    for (i, &slot) in ids.iter().skip(first * 2).take(rows * 2).enumerate() {
                        let (sx, sy) = sidebar::slot_origin(width, i);
                        let (rx, ry) = sidebar::slot_origin(SIDEBAR_W, first * 2 + i);
                        assert_eq!(
                            sidebar::slot_at(
                                width,
                                height,
                                session,
                                view,
                                (f64::from(sx), f64::from(sy))
                            ),
                            Some(slot)
                        );
                        // Each displayed cameo, dim/clock/text included, matches
                        // its complete unscrolled panel crop at the original id.
                        for y in 0..48 {
                            let at = (((sy + y) as u32 * width + sx as u32) * 4) as usize;
                            let expected = (((ry + y) as u32 * SIDEBAR_W + rx as u32) * 4) as usize;
                            assert_eq!(
                                &frame[at..at + 240],
                                &reference[expected..expected + 240],
                                "{category:?}, slot {slot}, height {height}"
                            );
                        }
                        reached.insert(slot);
                    }
                    if !view.scroll(session, height, 1) {
                        break;
                    }
                }
                if rows > 0 {
                    assert_eq!(reached, ids.iter().copied().collect());
                } else {
                    assert!(reached.is_empty());
                    assert_eq!(
                        sidebar::slot_at(width, height, session, view, (23.0, 228.0)),
                        None
                    );
                }
                let last = view.first_row(session, height);
                assert!(!view.scroll(session, height, i32::MAX));
                assert_eq!(view.first_row(session, height), last);
                view.scroll(session, height, i32::MIN);
                assert_eq!(view.first_row(session, height), 0);
                // The disabled up arrow never returns a scroll action.
                assert_eq!(
                    view.scroll_at(
                        session,
                        width,
                        height,
                        (f64::from(width - SIDEBAR_W + 30), f64::from(height) - 20.0)
                    ),
                    None
                );
            }
        }
    }
}
