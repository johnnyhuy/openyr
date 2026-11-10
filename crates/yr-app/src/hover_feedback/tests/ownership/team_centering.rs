use super::*;

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];
fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}
fn assign(app: &mut App, code: KeyCode) {
    app.control = true;
    key(app, code, ElementState::Pressed);
    key(app, code, ElementState::Released);
    app.control = false;
}
fn double_recall(app: &mut App, code: KeyCode) {
    let at = Instant::now();
    assert!(app.group_key(code, ElementState::Pressed, false, at));
    assert!(app.group_key(
        code,
        ElementState::Pressed,
        false,
        at + Duration::from_millis(50)
    ));
}
fn projected(app: &App, id: u32) -> (f32, f32) {
    let session = app.session.as_ref().unwrap();
    let u = session.sim.units().iter().find(|u| u.id == id).unwrap();
    yr_app::interaction::ground_point(
        app.map.as_ref().unwrap(),
        session.sim.terrain(),
        (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0),
    )
}

#[test]
fn team_centering_matches_configured_viewport_for_all_slots_and_zoom() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            for mixed in [false, true] {
                for code in DIGITS {
                    let (mut app, own, _) = scene(size);
                    let buddy = type_selection::spawn(&mut app, 1, (6, 5), yr_sim::OWNER_PLAYER);
                    app.selected = if mixed {
                        BTreeSet::from([own, buddy])
                    } else {
                        BTreeSet::from([own])
                    };
                    let expected = app.selected.clone();
                    assign(&mut app, code);
                    app.selected.clear();
                    app.view.zoom = zoom;
                    app.pending.push(Command::Stop { id: own });
                    let before = saved(&app);
                    double_recall(&mut app, code);
                    assert_eq!(app.selected, expected);
                    let a = projected(&app, own);
                    let b = projected(&app, if mixed { buddy } else { own });
                    // Check the drawn anchor midpoint in screen coordinates,
                    // independently of the handler's camera request arithmetic.
                    let screen = (
                        ((a.0 + b.0) * 0.5 - app.view.cam_x) * zoom,
                        ((a.1 + b.1) * 0.5 - app.view.cam_y) * zoom,
                    );
                    assert!(
                        (screen.0 - (size.0 - SIDEBAR_W) as f32 / 2.0).abs() < 0.001,
                        "horizontal {size:?} zoom={zoom} mixed={mixed} {code:?}: {screen:?}"
                    );
                    assert!(
                        (screen.1 - size.1 as f32 / 2.0).abs() < 0.001,
                        "vertical {size:?} zoom={zoom} mixed={mixed} {code:?}: {screen:?}"
                    );
                    assert_eq!(app.view.zoom, zoom);
                    assert_eq!(app.pending, [Command::Stop { id: own }]);
                    assert_eq!(saved(&app), before);
                    app.selected.clear();
                    key(&mut app, code, ElementState::Pressed);
                    assert_eq!(app.selected, expected, "stored team survives centering");
                    assert_eq!(saved(&app), before);
                }
            }
        }
    }
    println!("162 size/zoom/single-or-mixed/slot cases put the projected team midpoint at the configured world viewport center, preserving zoom, team membership, queued Stop and full Session bytes.");
}

#[test]
fn team_centering_default_frame_retains_existing_coordinates() {
    let (mut app, own, _) = scene((1280, 800));
    assert_eq!(projected(&app, own), (600.0, 320.0));
    assign(&mut app, KeyCode::Digit1);
    let before = saved(&app);
    double_recall(&mut app, KeyCode::Digit1);
    assert_eq!((app.view.cam_x, app.view.cam_y), (44.0, -80.0));
    assert_eq!(saved(&app), before);
    println!("Default 1280x800 raw centering request remains (44,-80) for anchor (600,320), with Session bytes unchanged.");
}

#[test]
fn team_centering_compositor_uses_frame_changed_without_a_pixels_surface() {
    for (size, expected) in [
        ((640, 480), (364.0, 80.0)),
        ((800, 600), (284.0, 20.0)),
        ((1280, 800), (44.0, -80.0)),
    ] {
        let (mut app, own, _) = scene((1280, 800));
        app.resize_window(size, 1.0).unwrap();
        assert_eq!(app.camera_size(), size);
        assert!(app.pixels.is_none());
        app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(1600 * 1200 * 4)
            .collect();
        assign(&mut app, KeyCode::Digit1);
        app.pending.push(Command::Stop { id: own });
        let before = saved(&app);
        double_recall(&mut app, KeyCode::Digit1);
        proof(&app, size);
        assert_eq!(
            (app.view.cam_x, app.view.cam_y),
            expected,
            "configured size {size:?}"
        );
        assert_eq!(app.selected, BTreeSet::from([own]));
        assert_eq!(app.pending, [Command::Stop { id: own }]);
        assert_eq!(saved(&app), before);
    }
    println!("Three shared resize/group/compositor cases retain configured dimensions with no Pixels surface; captures are raw requests before the unchanged redraw bounds policy.");
}

fn proof(app: &App, size: (u32, u32)) {
    let Some(dir) = std::env::var_os("OPENYR_TEAM_CENTERING_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose_at(&mut frame, size.0, size.1, Instant::now());
    std::fs::write(
        dir.join(format!("{}.png", size.0)),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(dir.join(format!("{}.json", size.0)),serde_json::to_vec_pretty(&serde_json::json!({
        "frame":size,"camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],"selected":app.selected,
        "projected_anchor":projected(app,*app.selected.first().unwrap()),
        "pending":format!("{:?}",app.pending),"tick":session.sim.tick(),
        "sim_hash":format!("{:016x}",session.sim.state_hash()),"snapshot_bytes":saved(app).len(),
        "logical_double_press_ms":50,"raw_request_before_redraw_bounds":true,"retail_art":false
    })).unwrap()).unwrap();
}
