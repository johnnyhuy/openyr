use super::*;
use sidebar::Category;

fn canvas(size: (u32, u32), zoom: f32) -> (App, u32, u32) {
    let (mut app, own, enemy) = scene(size);
    let map = app.map.as_mut().unwrap();
    map.rgba = (0..map.height)
        .flat_map(|y| {
            (0..map.width).flat_map(move |x| {
                if (x / 60 + y / 30) % 2 == 0 {
                    [30, 45, 30, 255]
                } else {
                    [45, 60, 40, 255]
                }
            })
        })
        .collect();
    app.view.zoom = zoom;
    (app, own, enemy)
}

fn press_tab(app: &mut App) {
    let origin = sidebar::tab_origin(app.camera_size().0, Category::Defences);
    app.pointer_moved((f64::from(origin.0 + 10), f64::from(origin.1 + 10)));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    assert_eq!(app.sidebar_view.category, Category::Defences);
}

fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}

#[test]
fn sidebar_tab_ownership_suppresses_world_feedback_until_release() {
    for size in [(640, 480), (1280, 800)] {
        for zoom in [0.5, 1.0, 2.0] {
            for hostile in [false, true] {
                let (mut app, own, enemy) = canvas(size, zoom);
                let target = if hostile { enemy } else { own };
                point_at(&mut app, target, size);
                let cursor = app.cursor;
                let kind = app.cursor_kind(size);
                assert_eq!(kind, if hostile { Kind::Attack } else { Kind::Select });
                assert_eq!(app.hover_target(size).unwrap().id, target);
                app.pending.push(Command::Stop { id: own });
                let before = saved(&app);
                let selected = app.selected.clone();
                let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
                press_tab(&mut app);
                app.pointer_moved(cursor);
                capture(&app, size, zoom, hostile, "held");
                assert_eq!(app.cursor_kind(size), Kind::Pointer);
                assert!(app.hover_target(size).is_none());
                app.pointer_button(ElementState::Released, MouseButton::Left);
                assert_eq!(app.cursor_kind(size), kind);
                assert_eq!(app.hover_target(size).unwrap().id, target);
                assert_eq!(app.pending, [Command::Stop { id: own }]);
                assert_eq!(app.selected, selected);
                assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
                assert_eq!(saved(&app), before);
                capture(&app, size, zoom, hostile, "released");
                click(&mut app);
                if hostile {
                    assert!(
                        matches!(app.pending.last(), Some(Command::Attack {id, target}) if *id == own && *target == enemy)
                    );
                } else {
                    assert_eq!(app.selected, [own].into());
                }
                assert_eq!(saved(&app), before);
            }
        }
    }
    println!("12 held-tab select/attack cases suppress actionable feedback, release without commands, restore normal hover and accept fresh clicks; Session bytes, camera and prior intents remain exact.");
}

#[test]
fn sidebar_tab_ownership_blocks_edge_pan_and_wheel_without_cancelling_press() {
    for size in [(640, 480), (1280, 800)] {
        for zoom in [1.0, 1.5, 2.0] {
            let (mut app, own, _) = canvas(size, zoom);
            app.menu.edge_scroll = true;
            app.view.cam_x = 300.;
            app.view.cam_y = 200.;
            app.bound_camera(size);
            let now = Instant::now();
            app.pointer_moved((200., 200.));
            app.update_camera(now, size);
            app.pending.push(Command::Stop { id: own });
            let before = saved(&app);
            let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
            press_tab(&mut app);
            let edge = (f64::from(size.0 - SIDEBAR_W - 2), f64::from(size.1) / 2.);
            app.pointer_moved(edge);
            assert!(
                app.edge_cursor(size).is_none(),
                "held tab must own edge input"
            );
            app.update_camera(now + Duration::from_millis(100), size);
            assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
            for position in [edge, (f64::from(size.0 - 20), 300.)] {
                app.pointer_moved(position);
                for delta in [
                    MouseScrollDelta::LineDelta(0., 1.),
                    MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0., 50.)),
                ] {
                    app.pointer_wheel(delta);
                    assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
                    app.pointer_moved(edge);
                    assert!(
                        app.edge_cursor(size).is_none(),
                        "wheel must not cancel ownership"
                    );
                    app.pointer_moved(position);
                }
            }
            app.pointer_moved(edge);
            app.pointer_button(ElementState::Released, MouseButton::Left);
            assert!(app.edge_cursor(size).is_some());
            app.update_camera(now + Duration::from_millis(200), size);
            assert!(
                app.view.cam_x > camera.0,
                "fresh released edge intent resumes panning"
            );
            app.pointer_moved((200., 200.));
            app.pointer_wheel(MouseScrollDelta::LineDelta(0., 1.));
            assert!(
                app.view.zoom > zoom,
                "fresh released wheel input resumes zoom"
            );
            assert_eq!(app.selected, [own].into());
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(saved(&app), before);
        }
    }
    println!("Six layout/zoom edge cases and 24 world/sidebar wheel cases preserve held ownership; released input resumes pan/zoom without changing selection, orders or Session bytes.");
}

fn capture(app: &App, size: (u32, u32), zoom: f32, hostile: bool, phase: &str) {
    if size.0 != 640 || zoom != 1.0 || !hostile {
        return;
    }
    let Some(path) = std::env::var_os("OPENYR_SIDEBAR_OWNERSHIP_PROOF") else {
        return;
    };
    let path = PathBuf::from(path);
    std::fs::create_dir_all(&path).unwrap();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut frame, size.0, size.1);
    std::fs::write(
        path.join(format!("{phase}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    std::fs::write(path.join(format!("{phase}.json")),serde_json::to_vec_pretty(&serde_json::json!({"cursor":format!("{:?}",app.cursor_kind(size)),"hover":app.hover_target(size).map(|t|t.id),"selected":app.selected,"pending":format!("{:?}",app.pending),"camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],"sim_hash":format!("{:016x}",app.session.as_ref().unwrap().sim.state_hash()),"snapshot_bytes":saved(app).len(),"retail_art":false})).unwrap()).unwrap();
}
