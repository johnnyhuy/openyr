use super::*;

fn fixture(size: (u32, u32), zoom: f32, texture: bool) -> App {
    let mut app = new_app(String::new(), String::new(), 1);
    app.frame_size = size;
    app.view.zoom = zoom;
    app.view.cam_x = 1000.0;
    app.view.cam_y = 1000.0;
    let extent = if texture { 1536 } else { 8192 };
    let mut rgba = Vec::new();
    if texture {
        for y in 0..extent {
            for x in 0..extent {
                let color = if x % 32 == 0 || y % 32 == 0 {
                    [20, 24, 32, 255]
                } else if (1148..1156).contains(&x) || (1148..1156).contains(&y) {
                    [240, 100, 60, 255]
                } else {
                    [
                        40 + (x / 32 % 8) as u8 * 16,
                        50 + (y / 32 % 8) as u8 * 16,
                        100,
                        255,
                    ]
                };
                rgba.extend_from_slice(&color);
            }
        }
    }
    app.map = Some(yr_render::RenderedMap {
        width: extent,
        height: extent,
        rgba,
        summary: "Synthetic camera grid".into(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    });
    app.selected.insert(17);
    app.pending.push(yr_sim::Command::Stop { id: 17 });
    app.pointer_moved((300.0, 250.0));
    app
}

fn proof(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_SMALL_DRAG_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (width, height) = app.frame_size;
    let mut frame = vec![0; (width * height * 4) as usize];
    app.compose(&mut frame, width, height);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(width, height, &frame),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "synthetic_scene": true, "session_present": app.session.is_some(),
            "size": app.frame_size, "zoom": app.view.zoom,
            "camera": [app.view.cam_x, app.view.cam_y], "pointer": [app.cursor.0, app.cursor.1],
            "right_panning": app.right_panning, "selected": app.selected,
            "pending": format!("{:?}", app.pending),
        }))
        .unwrap(),
    )
    .unwrap();
}

fn exercise(size: (u32, u32), zoom: f32, direction: (i32, i32), capture: bool) {
    let mut app = fixture(
        size,
        zoom,
        capture && std::env::var_os("OPENYR_SMALL_DRAG_PROOF").is_some(),
    );
    let pending = app.pending.clone();
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    if capture {
        proof(&app, "01-pressed");
    }
    for step in 1..=24 {
        app.pointer_moved((
            300.0 + f64::from(step * direction.0),
            250.0 + f64::from(step * direction.1),
        ));
        if capture && step == 3 {
            proof(&app, "02-threshold-crossed");
        }
    }
    if capture {
        proof(&app, "03-dragged");
    }
    app.pointer_button(ElementState::Released, MouseButton::Right);
    if capture {
        proof(&app, "04-released");
    }
    assert_eq!(
        (app.view.cam_x, app.view.cam_y),
        (
            1000.0 - 24.0 * direction.0 as f32 / zoom,
            1000.0 - 24.0 * direction.1 as f32 / zoom,
        ),
        "{size:?}, zoom {zoom}, direction {direction:?}"
    );
    assert_eq!(app.selected, BTreeSet::from([17]));
    assert_eq!(app.pending, pending);
    assert!(app.right_down.is_none() && !app.right_panning);
}

#[test]
fn small_motion_drags_pan_without_losing_threshold_pixels_or_selection() {
    exercise((640, 480), 1.0, (1, 1), true);
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            for direction in [(1, 0), (0, 1), (1, 1)] {
                if (size, zoom, direction) != ((640, 480), 1.0, (1, 1)) {
                    exercise(size, zoom, direction, false);
                }
            }
        }
    }
    println!("27 size/zoom/direction cases: one-pixel events pan by their full displacement and preserve selection/queued commands.");
}

#[test]
fn click_jitter_uses_press_displacement_and_out_and_back_remains_a_drag() {
    let mut app = fixture((640, 480), 1.0, false);
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    for offset in [(1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)] {
        app.pointer_moved((300.0 + offset.0, 250.0 + offset.1));
        assert!(!app.right_panning);
    }
    assert_eq!((app.view.cam_x, app.view.cam_y), (1000.0, 1000.0));
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert!(app.selected.is_empty());
    app.selected.insert(17);
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    for step in (0..=12).chain((0..12).rev()) {
        app.pointer_moved((300.0 + f64::from(step), 250.0));
    }
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert_eq!((app.view.cam_x, app.view.cam_y), (1000.0, 1000.0));
    assert!(app.selected.contains(&17));
    println!("Jitter inside the click threshold stays a click; a completed out-and-back drag preserves selection.");
}

#[test]
fn small_drags_respect_bounds_and_invalidation_rejects_stale_releases() {
    let mut app = fixture((640, 480), 1.0, false);
    app.view.cam_x = 0.0;
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    for step in 1..=12 {
        app.pointer_moved((300.0 + f64::from(step), 250.0));
    }
    assert!(app.right_panning);
    assert_eq!(app.view.cam_x, 0.0);
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert!(app.selected.contains(&17));
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    app.pointer_moved((313.0, 250.0));
    app.invalidate_pointer();
    app.pointer_moved((340.0, 250.0));
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert_eq!(app.view.cam_x, 0.0);
    assert!(app.selected.contains(&17));
    assert_eq!(app.pending, [yr_sim::Command::Stop { id: 17 }]);
    println!("Bounded small drags remain drags; pointer invalidation prevents stale release cancellation or camera motion.");
}
