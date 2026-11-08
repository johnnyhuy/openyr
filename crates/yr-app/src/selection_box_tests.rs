use super::*;

#[test]
fn selection_box_does_not_paint_over_the_sidebar() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let mut app = new_app(String::new(), String::new(), 1);
        app.frame_size = size;
        app.map = Some(yr_render::RenderedMap {
            width: size.0,
            height: size.1,
            rgba: [30, 40, 30, 255]
                .into_iter()
                .cycle()
                .take((size.0 * size.1 * 4) as usize)
                .collect(),
            summary: "Synthetic selection canvas".into(),
            origin_x: 0,
            origin_y: 0,
            scale: 1,
        });
        let mut baseline = vec![0; (size.0 * size.1 * 4) as usize];
        app.compose(&mut baseline, size.0, size.1);
        app.pointer_moved((100.0, 120.0));
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        app.pointer_moved((f64::from(size.0 - 10), 300.0));
        let mut frame = baseline.clone();
        app.compose(&mut frame, size.0, size.1);
        let world = size.0 - SIDEBAR_W;
        for y in 0..size.1 {
            let start = ((y * size.0 + world) * 4) as usize;
            let end = ((y + 1) * size.0 * 4) as usize;
            assert_eq!(
                &frame[start..end],
                &baseline[start..end],
                "sidebar pixels at {size:?}, row {y}"
            );
        }
        assert!(frame.as_chunks::<4>().0.contains(&SELECT_COL));
        capture(&app, "01-over-sidebar", &frame);
        app.pointer_moved((f64::from(world - 20), 300.0));
        app.compose(&mut frame, size.0, size.1);
        capture(&app, "02-back-in-world", &frame);
        app.pointer_button(ElementState::Released, MouseButton::Left);
        app.compose(&mut frame, size.0, size.1);
        capture(&app, "03-release", &frame);
    }
}

#[test]
fn selection_box_picker_excludes_hidden_and_enemy_anchors_without_changing_sim() {
    let mut sim = yr_sim::Sim::new(42);
    let mut ids = Vec::new();
    for (x, owner) in [
        (5, yr_sim::OWNER_PLAYER),
        (7, yr_sim::OWNER_PLAYER),
        (4, yr_sim::OWNER_ENEMY),
    ] {
        let id = sim.next_id_peek();
        ids.push(id);
        sim.step(&[
            Command::Spawn {
                kind: 0,
                x: Fix::from_int(x),
                y: Fix::from_int(5),
                loco: yr_sim::terrain::Loco::Foot,
                speed: Fix::ONE,
                hp: 100,
                armor: 0,
                weapon: None,
            },
            Command::AssignOwner { id, owner },
        ]);
    }
    let saved = sim.save_bytes().unwrap();
    let hash = sim.state_hash();
    // Projection-only raster metadata; this fixture does not decode game artwork.
    let map = yr_render::RenderedMap {
        width: 12000,
        height: 8000,
        rgba: Vec::new(),
        summary: String::new(),
        origin_x: 6000,
        origin_y: 4000,
        scale: 1,
    };
    let (mx, my) = yr_app::interaction::ground_point(&map, None, (5.5, 5.5));
    let mut receipts = Vec::new();
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            let world = size.0 - SIDEBAR_W;
            let view = View {
                cam_x: mx - (world as f32 - 30.0 * zoom) / zoom,
                cam_y: my - 100.0 / zoom,
                zoom,
                dirty: false,
            };
            let bounds = selection_box::Bounds::new(
                (0.0, 0.0),
                (f64::from(size.0 - 1), f64::from(size.1 - 1)),
                size,
            )
            .unwrap();
            let picked: Vec<_> = bounds.ids(&map, &sim, &view).collect();
            assert_eq!(
                picked,
                [ids[0]],
                "visible owned unit only at {size:?}, zoom {zoom}"
            );
            let wx = f64::from(world) - 30.0 * f64::from(zoom);
            let ordinary =
                selection_box::Bounds::new((wx - 2.0, 98.0), (wx + 2.0, 102.0), size).unwrap();
            assert_eq!(
                ordinary.ids(&map, &sim, &view).collect::<Vec<_>>(),
                [ids[0]]
            );
            let reversed = selection_box::Bounds::new(
                (f64::from(size.0 - 1), f64::from(size.1 - 1)),
                (0.0, 0.0),
                size,
            )
            .unwrap();
            assert_eq!(reversed.ids(&map, &sim, &view).collect::<Vec<_>>(), picked);
            assert_eq!(sim.state_hash(), hash);
            assert_eq!(sim.save_bytes().unwrap(), saved);
            receipts.push(serde_json::json!({"size":size,"zoom":zoom,"picked":picked,"hidden_owned":ids[1],"enemy":ids[2],"hash":format!("{hash:016x}"),"snapshot_unchanged":true}));
        }
    }
    if let Some(dir) = std::env::var_os("OPENYR_SELECTION_BOX_PROOF") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("sim-picks.json"),
            serde_json::to_vec_pretty(&receipts).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn selection_box_bounds_reject_invalid_extents_and_keep_viewport_edges_half_open() {
    use selection_box::Bounds;
    for size in [(0, 0), (168, 480), (640, 0)] {
        assert!(Bounds::new((0.0, 0.0), (100.0, 100.0), size).is_none());
    }
    for point in [
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
        (f64::NEG_INFINITY, 1.0),
    ] {
        assert!(Bounds::new(point, (300.0, 300.0), (640, 480)).is_none());
        assert!(Bounds::new((100.0, 100.0), point, (640, 480)).is_none());
    }
    assert!(Bounds::new((480.0, 1.0), (600.0, 300.0), (640, 480)).is_none());
    assert!(Bounds::new((-100.0, -100.0), (-1.0, -1.0), (640, 480)).is_none());
    let bounds = Bounds::new((-f64::MAX, -f64::MAX), (f64::MAX, f64::MAX), (640, 480)).unwrap();
    assert!(bounds.contains((0.0, 0.0)));
    assert!(bounds.contains((471.75, 479.75)));
    for point in [
        (472.0, 100.0),
        (100.0, 480.0),
        (-0.1, 100.0),
        (f64::NAN, 1.0),
    ] {
        assert!(!bounds.contains(point));
    }
    let point = Bounds::new((100.0, 100.0), (100.0, 100.0), (640, 480)).unwrap();
    assert!(point.contains((100.0, 100.0)));
    assert!(!point.contains((100.1, 100.0)));
}

fn capture(app: &App, name: &str, frame: &[u8]) {
    if app.frame_size != (640, 480) {
        return;
    }
    let Some(dir) = std::env::var_os("OPENYR_SELECTION_BOX_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(640, 480, frame),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "world_width":640-SIDEBAR_W,"left_down":app.left_down,"cursor":app.cursor,
            "session_present":app.session.is_some(),"retail_art":false,
        }))
        .unwrap(),
    )
    .unwrap();
}
