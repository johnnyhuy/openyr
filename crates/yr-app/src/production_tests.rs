use super::*;

#[test]
fn retail_sidebar_holds_resumes_cancels_and_restores_paid_work() {
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
    // Isolate production spending from harvesting and incidental combat.
    let remove: Vec<_> = session
        .sim
        .units()
        .iter()
        .map(|u| Command::Remove { id: u.id })
        .collect();
    session.step(&remove);
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
    let (cx, cy) = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::WarFactory)
        .unwrap()
        .centre();
    app.center_world_at(
        (cx.raw() as f32 / 256.0, cy.raw() as f32 / 256.0),
        (WIN_W - SIDEBAR_W, WIN_H),
    );
    app.sidebar_view.select(sidebar::Category::Vehicles);
    app.sidebar_click(tank);
    app.tick_session();
    for _ in 0..100 {
        app.tick_session();
    }
    assert!(app.session.as_ref().unwrap().slot_head(tank).unwrap().paid > 0);
    proof(&mut app, "01-building", false);
    app.sidebar_right_click(tank);
    assert!(matches!(
        app.pending.as_slice(),
        [Command::HoldProduce { held: true, .. }]
    ));
    app.tick_session();
    let session = app.session.as_ref().unwrap();
    let head = *session.slot_head(tank).unwrap();
    assert!(head.held);
    let credits = session.sim.credits();
    let bytes = yr_app::snapshot::encode(session).unwrap();
    proof(&mut app, "02-held", true);
    for _ in 0..50 {
        app.tick_session();
    }
    let session = app.session.as_ref().unwrap();
    assert_eq!(session.slot_head(tank), Some(&head));
    assert_eq!(session.sim.credits(), credits);
    proof(&mut app, "03-still-held", false);
    let mut restored = app.load_session().unwrap();
    yr_app::snapshot::restore(&mut restored, &bytes).unwrap();
    assert!(restored.slot_head(tank).unwrap().held);
    assert_eq!(yr_app::snapshot::encode(&restored).unwrap(), bytes);
    app.session = Some(restored);
    proof(&mut app, "04-loaded-held", false);
    app.sidebar_click(tank);
    assert!(matches!(
        app.pending.as_slice(),
        [Command::HoldProduce { held: false, .. }]
    ));
    app.tick_session();
    assert_eq!(
        app.session.as_ref().unwrap().slot_status(tank).0,
        1,
        "resume cannot enqueue a copy"
    );
    assert!(
        app.session
            .as_ref()
            .unwrap()
            .slot_head(tank)
            .unwrap()
            .progress
            > head.progress
    );
    proof(&mut app, "05-resumed", false);
    app.sidebar_right_click(tank);
    app.tick_session();
    let session = app.session.as_ref().unwrap();
    let refund = session.slot_head(tank).unwrap().paid;
    let credits = session.sim.credits();
    app.sidebar_right_click(tank);
    app.tick_session();
    let session = app.session.as_ref().unwrap();
    assert!(session.slot_head(tank).is_none());
    assert_eq!(session.sim.credits(), credits + refund);
    proof(&mut app, "06-cancelled", false);
    // Rapid clicks before the next tick see earlier pending queue intent.
    app.sidebar_click(tank);
    app.sidebar_click(tank);
    app.tick_session();
    app.sidebar_right_click(tank);
    app.sidebar_right_click(tank);
    assert!(matches!(
        app.pending.as_slice(),
        [
            Command::HoldProduce { held: true, .. },
            Command::CancelProduce { .. }
        ]
    ));
    app.tick_session();
    assert_eq!(app.session.as_ref().unwrap().slot_status(tank).0, 1);
    assert!(!app.session.as_ref().unwrap().slot_head(tank).unwrap().held);
    app.sidebar_right_click(tank);
    app.sidebar_click(tank);
    assert!(matches!(
        app.pending.as_slice(),
        [
            Command::HoldProduce { held: true, .. },
            Command::HoldProduce { held: false, .. }
        ]
    ));
    app.tick_session();
    assert_eq!(app.session.as_ref().unwrap().slot_status(tank).0, 1);
    app.sidebar_right_click(tank);
    app.tick_session();
    app.sidebar_right_click(tank);
    app.tick_session();
    // Construction uses the same hold/resume route before READY placement.
    app.sidebar_view.select(sidebar::Category::Structures);
    app.sidebar_click(plant);
    app.sidebar_click(plant);
    assert_eq!(
        app.pending.len(),
        1,
        "pending construction occupies its queue"
    );
    app.tick_session();
    app.sidebar_right_click(plant);
    app.tick_session();
    assert!(app.session.as_ref().unwrap().slot_head(plant).unwrap().held);
    proof(&mut app, "07-structure-held", false);
    app.sidebar_click(plant);
    app.tick_session();
    assert!(!app.session.as_ref().unwrap().slot_head(plant).unwrap().held);
    for _ in 0..1000 {
        if app.session.as_ref().unwrap().ready_structure() == Some(plant) {
            break;
        }
        app.tick_session();
    }
    assert_eq!(app.session.as_ref().unwrap().ready_structure(), Some(plant));
    app.sidebar_click(plant);
    assert_eq!(app.placing, Some(plant));
    assert!(app.pending.is_empty());
    proof(&mut app, "08-ready", false);
    app.sidebar_right_click(plant);
    app.tick_session();
    assert!(app.placing.is_none());
    assert!(app.session.as_ref().unwrap().slot_head(plant).is_none());
}

fn proof(app: &mut App, name: &str, save: bool) {
    let session = app.session.as_ref().unwrap();
    let before = yr_app::snapshot::encode(session).unwrap();
    let tick = session.sim.tick();
    let credits = session.sim.credits();
    let production = session.sim.production().to_vec();
    let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose(&mut frame, WIN_W, WIN_H);
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        before
    );
    let Some(out) = std::env::var_os("OPENYR_PRODUCTION_HOLD_PROOF").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let file = format!("{name}-t{tick:05}");
    std::fs::write(
        out.join(format!("{file}.png")),
        yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
    )
    .unwrap();
    std::fs::write(
        out.join(format!("{file}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "tick": tick, "credits": credits, "orders": production,
        }))
        .unwrap(),
    )
    .unwrap();
    if save {
        std::fs::write(out.join("held.oyrsave"), before).unwrap();
    }
    println!("{file}: credits {credits}, orders {production:?}");
}
