use super::*;

#[test]
fn production_feedback_maps_commands_and_expires() {
    let mut app = new_app(String::new(), String::new(), 1);
    let now = Instant::now();
    app.acknowledge_production_order(None, true, now);
    assert!(
        app.order_feedback.0.is_none(),
        "right-click without work stays silent"
    );
    app.acknowledge_production_order(Some(&Command::Stop { id: 1 }), false, now);
    assert!(
        app.order_feedback.0.is_none(),
        "unrelated commands stay silent"
    );
    app.acknowledge_production_order(None, false, now);
    assert_eq!(app.order_feedback.message(now), Some("Cannot build here"));
    assert!(app.order_feedback.0.as_ref().unwrap().marker.is_none());
    assert!(app.order_feedback.expire(now + LIFETIME));
    app.acknowledge_placement_start(now);
    assert_eq!(
        app.order_feedback.message(now),
        Some("Select a location to place the structure")
    );
    assert!(app.order_feedback.expire(now + LIFETIME));
}

#[test]
fn retail_sidebar_clicks_acknowledge_queue_hold_resume_and_cancel() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.session.as_mut().unwrap().sim.set_ai(None);
    app.map = Some(yr_render::render_map_background(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let session = app.session.as_ref().unwrap();
    let tank = session
        .buildables
        .iter()
        .position(|b| b.id == "MTNK")
        .unwrap();
    let plant = session
        .buildables
        .iter()
        .position(|b| b.id == "GAPOWR")
        .unwrap();
    let saved = yr_app::snapshot::encode(session).unwrap();

    app.sidebar_click(tank);
    assert!(matches!(app.pending.as_slice(), [Command::Produce { .. }]));
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(app.order_feedback.message(now), Some("Build order sent"));
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    proof(&app, "01-queued", now);
    app.tick_session();
    app.pending.clear();

    app.sidebar_right_click(tank);
    assert!(matches!(
        app.pending.as_slice(),
        [Command::HoldProduce { held: true, .. }]
    ));
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(app.order_feedback.message(now), Some("Production held"));
    proof(&app, "02-held", now);
    app.tick_session();
    app.pending.clear();

    app.sidebar_click(tank);
    assert!(matches!(
        app.pending.as_slice(),
        [Command::HoldProduce { held: false, .. }]
    ));
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(app.order_feedback.message(now), Some("Production resumed"));
    proof(&app, "03-resumed", now);
    app.tick_session();
    app.pending.clear();

    app.sidebar_right_click(tank);
    app.tick_session();
    app.pending.clear();
    app.sidebar_right_click(tank);
    assert!(matches!(
        app.pending.as_slice(),
        [Command::CancelProduce { .. }]
    ));
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(
        app.order_feedback.message(now),
        Some("Build order cancelled")
    );
    proof(&app, "04-cancelled", now);
    app.tick_session();
    app.pending.clear();
    assert!(app.session.as_ref().unwrap().slot_head(tank).is_none());

    // A second structure click while its factory queue is occupied is unavailable.
    app.sidebar_click(plant);
    assert!(matches!(app.pending.as_slice(), [Command::Produce { .. }]));
    app.sidebar_click(plant);
    assert_eq!(
        app.pending.len(),
        1,
        "occupied construction queue queues no copy"
    );
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(app.order_feedback.message(now), Some("Cannot build here"));
    proof(&app, "05-unavailable", now);
    app.tick_session();
    app.pending.clear();

    // READY pickup announces placement instead of a build order.
    for _ in 0..2000 {
        if app.session.as_ref().unwrap().ready_structure() == Some(plant) {
            break;
        }
        app.tick_session();
    }
    assert_eq!(app.session.as_ref().unwrap().ready_structure(), Some(plant));
    app.sidebar_click(plant);
    assert_eq!(app.placing, Some(plant));
    assert!(app.pending.is_empty());
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(
        app.order_feedback.message(now),
        Some("Select a location to place the structure")
    );
    proof(&app, "06-placement", now);
    let mut expired = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose_at(&mut expired, WIN_W, WIN_H, now + LIFETIME);
    assert!(app.order_feedback.message(now + LIFETIME).is_none());
    println!("EB1 sidebar production: queue/hold/resume/cancel/occupied/READY notices match queued commands; pre-step save bytes preserved");
}

fn proof(app: &App, name: &str, now: Instant) {
    let Some(dir) = std::env::var_os("OPENYR_PRODUCTION_ORDER_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose_at(&mut frame, WIN_W, WIN_H, now);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "tick": app.session.as_ref().unwrap().sim.tick(),
            "hash": format!("{:016x}", app.session.as_ref().unwrap().sim.state_hash()),
            "message": app.order_feedback.message(now),
            "pending": format!("{:?}", app.pending),
        }))
        .unwrap(),
    )
    .unwrap();
}
