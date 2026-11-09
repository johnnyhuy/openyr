use super::*;
use sidebar::Category;

const TABS: [(KeyCode, Category); 4] = [
    (KeyCode::KeyQ, Category::Structures),
    (KeyCode::KeyW, Category::Defences),
    (KeyCode::KeyE, Category::Infantry),
    (KeyCode::KeyR, Category::Vehicles),
];

fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}

fn frame(app: &App, now: Instant) -> Vec<u8> {
    let size = app.camera_size();
    let mut pixels = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose_at(&mut pixels, size.0, size.1, now);
    pixels
}

#[test]
fn category_keys_match_mouse_tab_composition_and_preserve_match() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for (code, category) in TABS {
            let (mut keyboard, own, _) = scene(size);
            let (mut mouse, _, _) = scene(size);
            for app in [&mut keyboard, &mut mouse] {
                app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                    .into_iter()
                    .cycle()
                    .take(1600 * 1200 * 4)
                    .collect();
                app.sidebar_view.select(if category == Category::Vehicles {
                    Category::Infantry
                } else {
                    Category::Vehicles
                });
                app.pending.push(Command::Stop { id: own });
                let (x, y) = sidebar::tab_origin(size.0, category);
                app.pointer_moved((f64::from(x + 10), f64::from(y + 10)));
            }
            let before = saved(&keyboard);
            let camera = (keyboard.view.cam_x, keyboard.view.cam_y, keyboard.view.zoom);
            key(&mut keyboard, code, ElementState::Pressed);
            key(&mut keyboard, code, ElementState::Released);
            click(&mut mouse);
            assert_eq!(keyboard.sidebar_view.category, category);
            assert_eq!(keyboard.selected, mouse.selected);
            assert_eq!(keyboard.pending, mouse.pending);
            assert_eq!(
                (keyboard.view.cam_x, keyboard.view.cam_y, keyboard.view.zoom),
                camera
            );
            assert_eq!(saved(&keyboard), before);
            let now = Instant::now();
            let pixels = frame(&keyboard, now);
            assert_eq!(pixels, frame(&mouse, now), "{size:?} {category:?}");
            if size.0 == 640 {
                if let Some(dir) = std::env::var_os("OPENYR_CATEGORY_KEYS_PROOF") {
                    let dir = PathBuf::from(dir);
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(
                        dir.join(format!("{}.png", category.file_name())),
                        yr_render::png::encode_rgba(size.0, size.1, &pixels),
                    )
                    .unwrap();
                    std::fs::write(dir.join(format!("{}.json", category.file_name())),
                        serde_json::to_vec_pretty(&serde_json::json!({
                            "key":format!("{code:?}"), "category":category.label(),
                            "selected":keyboard.selected, "pending":format!("{:?}",keyboard.pending),
                            "tick":keyboard.session.as_ref().unwrap().sim.tick(),
                            "sim_hash":format!("{:016x}",keyboard.session.as_ref().unwrap().sim.state_hash()),
                            "snapshot_bytes":before.len(), "mouse_pixels_equal":true,
                            "retail_art":false, "size":size
                        })).unwrap()).unwrap();
                }
            }
        }
    }
    println!("12 actual Q/W/E/R dispatch cases match complete mouse-tab compositor frames and preserve selection, camera, pending commands and encoded Session bytes.");
}

#[test]
fn category_keys_cancel_old_gestures_and_placement_without_losing_ready_work() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for (code, category) in TABS {
            for mode in 0..4 {
                let (mut app, own, slot) = placement::ready(size, 1.0);
                app.sidebar_view.select(Category::Vehicles);
                match mode {
                    0 => app.placing = Some(slot),
                    1 => app.service_mode = ServiceMode::Repair,
                    2 => app.service_mode = ServiceMode::Sell,
                    _ => app.attack_move = true,
                }
                app.left_down = Some((100.0, 100.0));
                app.right_down = Some((100.0, 100.0));
                app.right_panning = true;
                app.radar_drag = true;
                app.camera.edge_armed = true;
                app.pending.push(Command::Stop { id: own });
                let pending = app.pending.clone();
                let selected = app.selected.clone();
                let before = saved(&app);
                key(&mut app, code, ElementState::Pressed);
                assert_eq!(app.sidebar_view.category, category);
                assert!(app.left_down.is_none() && app.right_down.is_none());
                assert!(!app.right_panning && !app.radar_drag && !app.camera.edge_armed);
                assert!(app.placing.is_none());
                assert_eq!(app.service_mode, ServiceMode::None);
                assert_eq!(app.attack_move, mode == 3, "mouse-tab mode policy");
                assert!(!app.keys.contains(&code));
                app.pointer_button(ElementState::Released, MouseButton::Left);
                app.pointer_button(ElementState::Released, MouseButton::Right);
                assert_eq!(app.pending, pending);
                assert_eq!(app.selected, selected);
                assert_eq!(app.session.as_ref().unwrap().ready_structure(), Some(slot));
                assert_eq!(saved(&app), before);
            }
        }
    }
    println!("48 layout/key/mode cases cancel stale gestures and service/placement state, retain attack-move like mouse tabs, preserve READY work and reject stale releases without changing saves or queued orders.");
}

#[test]
fn category_keys_respect_guards_and_leave_menu_text_available() {
    for (code, category) in TABS {
        for guard in 0..11 {
            let (mut app, _, _) = scene((640, 480));
            let initial = if category == Category::Vehicles {
                Category::Infantry
            } else {
                Category::Vehicles
            };
            app.sidebar_view.select(initial);
            match guard {
                2 => app.pointer.focused = false,
                3 => app.control = true,
                4 => app.shift = true,
                5 => app.super_key = true,
                6 => app.session.as_mut().unwrap().outcome = Outcome::Won,
                7 => app.session.as_mut().unwrap().outcome = Outcome::Lost,
                8 => app.map = None,
                9 => app.session = None,
                10 => app.menu.set(Page::Pause),
                _ => {}
            }
            let before = app.session.as_ref().map(|_| saved(&app));
            app.keyboard_input(
                PhysicalKey::Code(code),
                ElementState::Pressed,
                None,
                guard == 0,
                guard == 1,
            );
            assert_eq!(app.sidebar_view.category, initial, "{code:?} guard {guard}");
            assert_eq!(app.session.as_ref().map(|_| saved(&app)), before);
        }
    }
    let (mut app, _, _) = scene((640, 480));
    app.menu.set(Page::Save);
    app.menu.name.clear();
    let category = app.sidebar_view.category;
    for (code, text) in [
        (KeyCode::KeyQ, "q"),
        (KeyCode::KeyW, "w"),
        (KeyCode::KeyE, "e"),
        (KeyCode::KeyR, "r"),
    ] {
        app.keyboard_input(
            PhysicalKey::Code(code),
            ElementState::Pressed,
            Some(text),
            false,
            false,
        );
        key(&mut app, code, ElementState::Released);
    }
    assert_eq!(app.menu.name, "qwer");
    assert_eq!(app.sidebar_view.category, category);
    println!("44 key/guard cases reject repeats, synthetic/unfocused/modifier input, ended or absent matches and pause menus; Save editor still accepts qwer.");
}

#[test]
fn category_keys_retain_rows_and_reset_fractional_wheel_input() {
    let (mut app, _, _) = scene((640, 302));
    let before = saved(&app);
    app.sidebar_view.select(Category::Structures);
    let session = app.session.as_ref().unwrap();
    assert!(sidebar::slots(session, Category::Structures).count() > 2);
    assert!(app.sidebar_view.scroll(session, 302, 1));
    assert!(!app.sidebar_view.wheel(session, 302, -0.6));
    key(&mut app, KeyCode::KeyR, ElementState::Pressed);
    key(&mut app, KeyCode::KeyQ, ElementState::Pressed);
    let session = app.session.as_ref().unwrap();
    assert_eq!(app.sidebar_view.first_row(session, 302), 1);
    assert!(
        !app.sidebar_view.wheel(session, 302, -0.6),
        "old fraction must not survive category changes"
    );
    assert!(app.sidebar_view.wheel(session, 302, -0.4));
    assert_eq!(app.sidebar_view.first_row(session, 302), 0);
    assert_eq!(saved(&app), before);
    println!("Compact category roundtrip preserves its row and clears old wheel fractions without changing Session bytes.");
}
