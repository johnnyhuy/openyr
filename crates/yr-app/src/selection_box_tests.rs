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
    let (mx, my) = yr_app::interaction::ground_point(&map, None, (5.0, 5.0));
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

#[test]
fn box_selection_follows_the_drawn_subcell_unit_anchor() {
    let mut receipts = Vec::new();
    for raw in [
        (5 * 256 + 64, 6 * 256 + 192),
        (5 * 256 + 128, 6 * 256 + 128),
        (5 * 256, 6 * 256),
    ] {
        for height in [0, 4] {
            let (sim, id) = subcell_sim(raw, height, Fix::ONE);
            let saved = sim.save_bytes().unwrap();
            let hash = sim.state_hash();
            for scale in [1, 2, 4] {
                let map = projection_map(scale);
                let cell = (raw.0 as f32 / 256.0, raw.1 as f32 / 256.0);
                // This is the exact ground coordinate used by the unit renderer.
                let point = yr_app::interaction::ground_point(&map, sim.terrain(), cell);
                let snapped = yr_app::interaction::ground_point(
                    &map,
                    sim.terrain(),
                    (cell.0.floor() + 0.5, cell.1.floor() + 0.5),
                );
                for size in [(640, 480), (800, 600), (1280, 800)] {
                    for zoom in [0.25, 1.0, 1.25, 4.0] {
                        let center = (f64::from(size.0 - SIDEBAR_W) / 2.0, f64::from(size.1) / 2.0);
                        let view = View {
                            cam_x: point.0 - center.0 as f32 / zoom,
                            cam_y: point.1 - center.1 as f32 / zoom,
                            zoom,
                            dirty: false,
                        };
                        let bounds = selection_box::Bounds::new(
                            (center.0 - 4.0, center.1 - 4.0),
                            (center.0 + 4.0, center.1 + 4.0),
                            size,
                        )
                        .unwrap();
                        let picked = bounds.ids(&map, &sim, &view).collect::<Vec<_>>();
                        assert_eq!(picked,[id],"raw={raw:?}, height={height}, scale={scale}, size={size:?}, zoom={zoom}");
                        let old = (
                            f64::from((snapped.0 - view.cam_x) * zoom),
                            f64::from((snapped.1 - view.cam_y) * zoom),
                        );
                        let separated =
                            (old.0 - center.0).abs() > 8.0 || (old.1 - center.1).abs() > 8.0;
                        if separated {
                            let stale = selection_box::Bounds::new(
                                (old.0 - 4.0, old.1 - 4.0),
                                (old.0 + 4.0, old.1 + 4.0),
                                size,
                            )
                            .unwrap();
                            assert!(
                                stale.ids(&map, &sim, &view).next().is_none(),
                                "the old cell center is not the drawn unit"
                            );
                        }
                        assert_eq!(sim.state_hash(), hash);
                        assert_eq!(sim.save_bytes().unwrap(), saved);
                        receipts.push(serde_json::json!({"raw":raw,"height":height,"scale":scale,"size":size,"zoom":zoom,"picked":picked,"drawn_center":center,"cell_center":old,"cell_center_separate":separated,"hash":format!("{hash:016x}"),"snapshot_unchanged":true}));
                    }
                }
            }
        }
    }
    assert_eq!(receipts.len(), 216);
    write_subcell_receipts("subcell-matrix.json", &receipts);
    println!("216 size/scale/zoom/height/position cases pick the drawn anchor and preserve Sim bytes/hash.");
}

#[test]
fn box_selection_tracks_actual_sim_movement_without_mutating_it() {
    let (mut sim, id) = subcell_sim((5 * 256 + 128, 6 * 256 + 128), 4, Fix::from_raw(32));
    let map = projection_map(1);
    let point = yr_app::interaction::ground_point(&map, sim.terrain(), (5.5, 6.5));
    let view = View {
        cam_x: point.0 - 200.0,
        cam_y: point.1 - 200.0,
        zoom: 1.0,
        dirty: false,
    };
    let mut receipts = Vec::new();
    let start = sim.units()[0].x;
    for step in 0..8 {
        if step == 0 {
            sim.step(&[Command::Move {
                id,
                x: Fix::from_raw(9 * 256 + 128),
                y: Fix::from_raw(6 * 256 + 128),
            }]);
        } else {
            sim.step(&[]);
        }
        let unit = &sim.units()[0];
        let raw = (unit.x.raw(), unit.y.raw());
        let point = yr_app::interaction::ground_point(
            &map,
            sim.terrain(),
            (raw.0 as f32 / 256.0, raw.1 as f32 / 256.0),
        );
        let win = (
            f64::from(point.0 - view.cam_x),
            f64::from(point.1 - view.cam_y),
        );
        let saved = sim.save_bytes().unwrap();
        let hash = sim.state_hash();
        let bounds = selection_box::Bounds::new(
            (win.0 - 4.0, win.1 - 4.0),
            (win.0 + 4.0, win.1 + 4.0),
            (640, 480),
        )
        .unwrap();
        assert_eq!(bounds.ids(&map, &sim, &view).collect::<Vec<_>>(), [id]);
        assert_eq!(sim.state_hash(), hash);
        assert_eq!(sim.save_bytes().unwrap(), saved);
        receipts.push(serde_json::json!({"tick":sim.tick(),"raw":raw,"drawn":win,"selected":id,"hash":format!("{hash:016x}"),"snapshot_unchanged":true}));
    }
    assert!(sim.units()[0].x > start, "the fixture must actually move");
    write_subcell_receipts("moving-unit.json", &receipts);
    println!("Eight actual Sim movement ticks retain accurate box picks and unchanged post-tick Sim bytes/hash.");
}

fn subcell_sim(raw: (i32, i32), height: u8, speed: Fix) -> (yr_sim::Sim, u32) {
    let mut sim = yr_sim::Sim::new(42);
    let mut terrain = yr_sim::TerrainGrid::new(0, 0, 16, 16);
    for x in 0..16 {
        for y in 0..16 {
            terrain.cell_mut(x, y).unwrap().height = height;
        }
    }
    sim.set_terrain(terrain);
    let id = sim.next_id_peek();
    sim.step(&[
        Command::Spawn {
            kind: 0,
            x: Fix::from_raw(raw.0),
            y: Fix::from_raw(raw.1),
            loco: yr_sim::terrain::Loco::Foot,
            speed,
            hp: 100,
            armor: 0,
            weapon: None,
        },
        Command::AssignOwner {
            id,
            owner: yr_sim::OWNER_PLAYER,
        },
    ]);
    (sim, id)
}

fn projection_map(scale: u32) -> yr_render::RenderedMap {
    yr_render::RenderedMap {
        width: 12000,
        height: 8000,
        rgba: Vec::new(),
        summary: String::new(),
        origin_x: 6000,
        origin_y: 4000,
        scale,
    }
}

fn write_subcell_receipts(name: &str, receipts: &[serde_json::Value]) {
    if let Some(dir) = std::env::var_os("OPENYR_SUBCELL_SELECTION_PROOF") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), serde_json::to_vec_pretty(receipts).unwrap()).unwrap();
    }
}
