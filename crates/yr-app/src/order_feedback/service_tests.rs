use super::*;

#[test]
fn service_feedback_requires_a_service_action_and_expires() {
    let mut app = new_app(String::new(), String::new(), 1);
    let now = Instant::now();
    app.acknowledge_service_order(None, (2.5, 3.5), now);
    assert!(app.order_feedback.0.is_none());
    app.service_mode = ServiceMode::Sell;
    app.acknowledge_service_order(Some(&Command::Stop { id: 1 }), (2.5, 3.5), now);
    assert!(app.order_feedback.0.is_none());
    app.acknowledge_service_order(None, (2.75, 3.25), now);
    assert_eq!(app.order_feedback.message(now), Some("Cannot sell here"));
    assert!(matches!(app.order_feedback.0.as_ref().unwrap().marker,
        Some((Target::Ground(x,y), ENEMY_COL)) if x==2.5 && y==3.5));
    assert!(app.order_feedback.expire(now + LIFETIME));
}

#[test]
fn retail_service_notices_match_guarded_clicks_and_preserve_commands_and_saves() {
    let directory = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.session.as_mut().unwrap().sim.set_ai(None);
    app.map = Some(yr_render::render_map_background(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let session = app.session.as_ref().unwrap();
    let plant = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && session.kinds[b.kind as usize] == "GAPOWR")
        .unwrap()
        .id;
    let enemy = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        .unwrap()
        .id;
    let selected = session
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER)
        .unwrap()
        .id;
    app.selected = [selected].into();
    let full = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    point_at(&mut app, plant, (1280, 800));
    app.toggle_service(ServiceMode::Repair);
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty());
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Cannot repair here")
    );
    assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == full);
    // Controlled native fixture, using the public validated snapshot format.
    let sim = &mut app.session.as_mut().unwrap().sim;
    let bytes = sim.save_bytes().unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
    for building in json["buildings"].as_array_mut().unwrap() {
        if building["id"] == plant {
            building["hp"] = (building["max_hp"].as_i64().unwrap() / 2).into();
        }
    }
    let mut bytes = bytes[..12].to_vec();
    bytes.extend(serde_json::to_vec(&json).unwrap());
    *sim = yr_sim::Sim::from_save_bytes(&bytes).unwrap();
    let damaged = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    if let Some(out) = std::env::var_os("OPENYR_SERVICE_ORDER_PROOF") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        yr_app::snapshot::save(app.session.as_ref().unwrap(), &out.join("damaged.oyrsave"))
            .unwrap();
        let session = app.session.as_ref().unwrap();
        let b = session
            .sim
            .buildings()
            .iter()
            .find(|b| b.id == plant)
            .unwrap();
        let (x, y) = b.centre();
        let ground = yr_app::interaction::ground_point(
            app.map.as_ref().unwrap(),
            session.sim.terrain(),
            (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
        );
        std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"plant_id":plant,"ground":ground,"tick":session.sim.tick(),"hash":format!("{:016x}",session.sim.state_hash()),"map_size":[app.map.as_ref().unwrap().width,app.map.as_ref().unwrap().height]})).unwrap()).unwrap();
    }
    for size in [(640, 480), (1000, 700), (1280, 800)] {
        app.resize_window(size, 1.0).unwrap();
        for zoom in [0.5, 1.0, 2.0] {
            app.view.zoom = zoom;
            for mode in [ServiceMode::Repair, ServiceMode::Sell] {
                yr_app::snapshot::restore(app.session.as_mut().unwrap(), &damaged).unwrap();
                app.service_mode = ServiceMode::None;
                point_at(&mut app, plant, size);
                app.toggle_service(mode);
                let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                let expected = app.world_action(app.cursor).unwrap().commands;
                assert_eq!(expected.len(), 1);
                let camera = (app.view.cam_x, app.view.cam_y);
                click(&mut app, MouseButton::Left);
                assert_eq!(app.pending, expected);
                assert_eq!(app.selected, [selected].into());
                assert_eq!((app.view.cam_x, app.view.cam_y), camera);
                assert_eq!(app.view.zoom, zoom);
                assert_eq!(app.service_mode, mode);
                assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == before);
                let now = app.order_feedback.0.as_ref().unwrap().at;
                assert_eq!(
                    app.order_feedback.message(now),
                    Some(if mode == ServiceMode::Repair {
                        "Repair order sent"
                    } else {
                        "Sell order sent"
                    })
                );
                if zoom == 1.0 {
                    proof(
                        &app,
                        size,
                        if mode == ServiceMode::Repair {
                            "repair"
                        } else {
                            "sell"
                        },
                        now,
                    );
                }
                // Right cancellation clears the notice/mode, without dropping queued work.
                app.view.dirty = false;
                click(&mut app, MouseButton::Right);
                assert!(app.order_feedback.0.is_none() && app.view.dirty);
                assert_eq!(app.service_mode, ServiceMode::None);
                assert_eq!(app.pending, expected);
                assert_eq!(app.selected, [selected].into());
                let mut reference = app.load_session().unwrap();
                yr_app::snapshot::restore(&mut reference, &before).unwrap();
                app.tick_session();
                reference.step(&expected);
                reference.sim.drain_events();
                assert!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
                        == yr_app::snapshot::encode(&reference).unwrap()
                );
            }
            // A repair already running resolves to the ordinary cancellation command.
            yr_app::snapshot::restore(app.session.as_mut().unwrap(), &damaged).unwrap();
            let command = app.session.as_ref().unwrap().repair(plant).unwrap();
            app.session.as_mut().unwrap().step(&[command]);
            let active = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            point_at(&mut app, plant, size);
            app.toggle_service(ServiceMode::Repair);
            click(&mut app, MouseButton::Left);
            assert_eq!(
                app.pending,
                [Command::Repair {
                    id: plant,
                    spec: None
                }]
            );
            let now = app.order_feedback.0.as_ref().unwrap().at;
            assert_eq!(
                app.order_feedback.message(now),
                Some("Cancel repair order sent")
            );
            assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == active);
            if zoom == 1.0 {
                proof(&app, size, "cancel-repair", now);
            }
            app.pending.clear();
            // Right-clicking empty sidebar chrome also cancels the service notice.
            app.pointer_moved((f64::from(size.0 - 5), 20.0));
            app.view.dirty = false;
            click(&mut app, MouseButton::Right);
            assert!(
                app.order_feedback.0.is_none()
                    && app.service_mode == ServiceMode::None
                    && app.view.dirty
            );
            // Enemy targets remain unavailable for both service modes.
            for mode in [ServiceMode::Repair, ServiceMode::Sell] {
                point_at(&mut app, enemy, size);
                app.toggle_service(mode);
                click(&mut app, MouseButton::Left);
                assert!(app.pending.is_empty());
                let now = app.order_feedback.0.as_ref().unwrap().at;
                assert_eq!(
                    app.order_feedback.message(now),
                    Some(if mode == ServiceMode::Repair {
                        "Cannot repair here"
                    } else {
                        "Cannot sell here"
                    })
                );
                if zoom == 1.0 {
                    proof(
                        &app,
                        size,
                        if mode == ServiceMode::Repair {
                            "unavailable-repair"
                        } else {
                            "unavailable-sell"
                        },
                        now,
                    );
                }
                app.toggle_service(mode);
            }
        }
    }
    yr_app::snapshot::restore(app.session.as_mut().unwrap(), &damaged).unwrap();
    let size = app.camera_size();
    point_at(&mut app, plant, size);
    app.toggle_service(ServiceMode::Repair);
    app.focus_changed(false);
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    app.focus_changed(true);
    point_at(&mut app, plant, size);
    app.toggle_service(ServiceMode::Repair);
    app.open_menu(yr_app::menu::Page::Pause);
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    app.menu.set(yr_app::menu::Page::Closed);
    point_at(&mut app, plant, size);
    app.toggle_service(ServiceMode::Repair);
    app.resize_window((640, 480), 1.0).unwrap();
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == damaged);
}

fn point_at(app: &mut App, id: u32, size: (u32, u32)) {
    let b = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .buildings()
        .iter()
        .find(|b| b.id == id)
        .unwrap();
    let (x, y) = b.centre();
    app.center_world_at(
        (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
        (size.0 - SIDEBAR_W, size.1),
    );
    app.pointer_moved((f64::from(size.0 - SIDEBAR_W) / 2.0, f64::from(size.1) / 2.0));
}
fn click(app: &mut App, button: MouseButton) {
    app.pointer_button(ElementState::Pressed, button);
    app.pointer_button(ElementState::Released, button);
}
fn proof(app: &App, size: (u32, u32), name: &str, now: Instant) {
    if let Some(out) = std::env::var_os("OPENYR_SERVICE_ORDER_PROOF") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        for (phase, time) in [("visible", now), ("expired", now + LIFETIME)] {
            let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
            app.compose_at(&mut frame, size.0, size.1, time);
            std::fs::write(
                out.join(format!("{}x{}-{name}-{phase}.png", size.0, size.1)),
                yr_render::png::encode_rgba(size.0, size.1, &frame),
            )
            .unwrap();
        }
    }
}
