use super::*;

fn fixture(size: (u32, u32)) -> App {
    let mut app = new_app(String::new(), String::new(), 1);
    app.frame_size = size;
    app.selected.insert(17);
    app.pending.push(yr_sim::Command::Stop { id: 17 });
    app.pointer_moved((100.0, 120.0));
    app
}

#[test]
fn right_press_cancels_left_and_radar_gestures_before_all_match_routes() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for route in 0..5 {
            let mut app = fixture(size);
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            app.pointer_moved((300.0, 300.0));
            let target = if route == 1 {
                (f64::from(size.0 - 50), 300.0)
            } else {
                app.cursor
            };
            app.pointer_moved(target);
            app.radar_drag = true;
            if route == 2 {
                app.service_mode = ServiceMode::Repair;
            }
            if route == 3 {
                app.service_mode = ServiceMode::Sell;
            }
            if route == 4 {
                app.placing = Some(0);
            }
            app.view.dirty = false;
            app.pointer_button(ElementState::Pressed, MouseButton::Right);
            assert!(
                app.left_down.is_none() && !app.radar_drag,
                "route {route}, {size:?}"
            );
            assert!(app.view.dirty);
            assert_eq!(app.selected, BTreeSet::from([17]));
            app.pointer_button(ElementState::Released, MouseButton::Right);
            let selection = app.selected.clone();
            app.pointer_moved((350.0, 340.0));
            app.pointer_button(ElementState::Released, MouseButton::Left);
            assert_eq!(app.selected, selection);
            assert_eq!(app.pending, [yr_sim::Command::Stop { id: 17 }]);
            assert!(app.left_down.is_none() && !app.radar_drag && app.placing.is_none());
            assert_eq!(app.service_mode, ServiceMode::None);
        }
    }
    println!("15 size/route cases: right press cancels unfinished left/radar gestures; stale releases preserve queued commands.");
}

#[test]
fn world_right_gesture_owns_conflicting_left_presses_until_release() {
    for pan in [false, true] {
        for left_first in [false, true] {
            let mut app = fixture((640, 480));
            app.map = Some(yr_render::RenderedMap {
                width: 2000,
                height: 2000,
                rgba: Vec::new(),
                summary: String::new(),
                origin_x: 0,
                origin_y: 0,
                scale: 1,
            });
            app.view.cam_x = 500.0;
            app.view.cam_y = 500.0;
            app.pointer_button(ElementState::Pressed, MouseButton::Right);
            if pan {
                app.pointer_moved((130.0, 140.0));
            }
            let camera = (app.view.cam_x, app.view.cam_y);
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            assert!(app.left_down.is_none());
            assert!(app.right_down.is_some());
            assert_eq!(app.right_panning, pan);
            if left_first {
                app.pointer_button(ElementState::Released, MouseButton::Left);
                app.pointer_button(ElementState::Released, MouseButton::Right);
            } else {
                app.pointer_button(ElementState::Released, MouseButton::Right);
                app.pointer_button(ElementState::Released, MouseButton::Left);
            }
            assert_eq!((app.view.cam_x, app.view.cam_y), camera);
            assert_eq!(app.selected.contains(&17), pan);
            assert_eq!(app.pending, [yr_sim::Command::Stop { id: 17 }]);
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            assert!(
                app.left_down.is_some(),
                "a fresh left gesture is available after release"
            );
            app.invalidate_pointer();
            app.pointer_moved((200.0, 220.0));
            app.pointer_button(ElementState::Released, MouseButton::Left);
            assert!(app.left_down.is_none());
        }
    }
    println!("Both release orders preserve right-click/pan semantics; conflicting left presses stay inert and fresh presses recover.");
}

fn capture(app: &App, name: &str, frame: &[u8]) {
    let Some(dir) = std::env::var_os("OPENYR_CANCEL_GESTURE_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(app.frame_size.0, app.frame_size.1, frame),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "synthetic_scene": true, "session_present": app.session.is_some(),
            "left_down": app.left_down, "right_down": app.right_down, "selected": app.selected,
            "radar_drag": app.radar_drag, "pending": format!("{:?}", app.pending),
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn right_cancellation_removes_the_rubber_band_and_stale_left_release_stays_inert() {
    let mut app = fixture((640, 480));
    app.map = Some(yr_render::RenderedMap {
        width: 640,
        height: 480,
        rgba: [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(640 * 480 * 4)
            .collect(),
        summary: "Synthetic gesture canvas".into(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    });
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_moved((300.0, 300.0));
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    assert!(frame.as_chunks::<4>().0.contains(&SELECT_COL));
    capture(&app, "01-box-drag", &frame);
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    app.compose(&mut frame, 640, 480);
    capture(&app, "02-right-pressed", &frame);
    let after_press = frame.clone();
    app.pointer_button(ElementState::Released, MouseButton::Right);
    app.compose(&mut frame, 640, 480);
    capture(&app, "03-right-released", &frame);
    app.pointer_moved((350.0, 340.0));
    app.pointer_button(ElementState::Released, MouseButton::Left);
    app.compose(&mut frame, 640, 480);
    capture(&app, "04-late-left-release", &frame);
    assert!(!after_press.as_chunks::<4>().0.contains(&SELECT_COL));
    assert_eq!(frame, after_press);
    assert!(app.left_down.is_none() && app.selected.is_empty());
    assert_eq!(app.pending, [yr_sim::Command::Stop { id: 17 }]);
    println!("Shared compositor: right press removes the unfinished rubber band; right/late-left releases cannot restore it.");
}
