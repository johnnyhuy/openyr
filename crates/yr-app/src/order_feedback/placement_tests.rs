use super::*;

#[test]
fn placement_notice_requires_an_action_or_mode_and_expires() {
    let mut app = new_app(String::new(), String::new(), 1);
    let now = Instant::now();
    app.acknowledge_placement_order(None, (2.25, 3.75), now);
    assert!(app.order_feedback.0.is_none());
    app.placing = Some(0);
    app.acknowledge_placement_order(Some(&Command::Stop { id: 1 }), (2.25, 3.75), now);
    assert!(app.order_feedback.0.is_none());
    app.acknowledge_placement_order(None, (2.25, 3.75), now);
    assert_eq!(app.order_feedback.message(now), Some("Cannot place here"));
    assert!(
        matches!(app.order_feedback.0.as_ref().unwrap().marker,Some((Target::Ground(x,y),ENEMY_COL)) if x==2.5 && y==3.5)
    );
    assert!(app.order_feedback.expire(now + LIFETIME));
}

#[test]
fn retail_placement_notices_preserve_ready_queue_commands_and_complete_snapshots() {
    let directory = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    let mut session = app.load_session().unwrap();
    session.sim.set_ai(None);
    let slot = session
        .buildables
        .iter()
        .position(|b| b.id == "GAPOWR")
        .unwrap();
    let occupied = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && session.kinds[b.kind as usize] == "GAPOWR")
        .unwrap()
        .cell;
    let selected = session
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER)
        .unwrap()
        .id;
    session.step(&[session.produce(slot).unwrap()]);
    while session.ready_structure() != Some(slot) {
        session.step(&[]);
        assert!(session.sim.tick() < 2000);
    }
    let valid = valid_origin(&session, slot);
    let ready = yr_app::snapshot::encode(&session).unwrap();
    app.session = Some(session);
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    app.selected = [selected].into();
    if let Some(out) = std::env::var_os("OPENYR_PLACEMENT_ORDER_PROOF") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        yr_app::snapshot::save(app.session.as_ref().unwrap(), &out.join("ready.oyrsave")).unwrap();
        let geometry = |origin| {
            let cell = cursor_cell(&app, slot, origin);
            yr_app::interaction::ground_point(
                app.map.as_ref().unwrap(),
                app.session.as_ref().unwrap().sim.terrain(),
                cell,
            )
        };
        std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"slot":slot,"valid_origin":valid,"invalid_origin":occupied,"valid_ground":geometry(valid),"invalid_ground":geometry(occupied),"map_size":[app.map.as_ref().unwrap().width,app.map.as_ref().unwrap().height],"tick":app.session.as_ref().unwrap().sim.tick(),"hash":format!("{:016x}",app.session.as_ref().unwrap().sim.state_hash())})).unwrap()).unwrap();
    }
    for size in [(640, 480), (1000, 700), (1280, 800)] {
        app.resize_window(size, 1.0).unwrap();
        for zoom in [0.5, 1.0, 2.0] {
            yr_app::snapshot::restore(app.session.as_mut().unwrap(), &ready).unwrap();
            app.view.zoom = zoom;
            app.placing = None;
            pickup(&mut app, slot, size);
            assert_eq!(app.placing, Some(slot));
            point_at(&mut app, slot, occupied, size);
            assert_eq!(app.placement_target(), Some((slot, occupied, false)));
            let queued = Command::Stop { id: selected };
            app.pending.push(queued);
            let camera = (app.view.cam_x, app.view.cam_y);
            click(&mut app, MouseButton::Left);
            assert_eq!(app.pending, [queued]);
            assert_eq!(app.placing, Some(slot));
            assert_eq!((app.view.cam_x, app.view.cam_y), camera);
            assert_eq!(app.view.zoom, zoom);
            assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == ready);
            let now = app.order_feedback.0.as_ref().unwrap().at;
            assert_eq!(app.order_feedback.message(now), Some("Cannot place here"));
            if zoom == 1.0 {
                proof(&app, size, "invalid", now);
            }
            app.view.dirty = false;
            click(&mut app, MouseButton::Right);
            assert!(app.placing.is_none() && app.order_feedback.0.is_none() && app.view.dirty);
            assert_eq!(app.pending, [queued]);
            assert_eq!(app.selected, [selected].into());
            pickup(&mut app, slot, size);
            point_at(&mut app, slot, valid, size);
            assert_eq!(app.placement_target(), Some((slot, valid, true)));
            let expected = app.session.as_ref().unwrap().place(slot, valid).unwrap();
            click(&mut app, MouseButton::Left);
            assert_eq!(app.pending, [queued, expected]);
            assert!(app.placing.is_none());
            assert_eq!(app.selected, [selected].into());
            assert_eq!(app.view.zoom, zoom);
            assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == ready);
            let now = app.order_feedback.0.as_ref().unwrap().at;
            assert_eq!(
                app.order_feedback.message(now),
                Some("Placement order sent")
            );
            let (w, h) = app.session.as_ref().unwrap().buildables[slot]
                .footprint()
                .unwrap();
            assert!(matches!(app.order_feedback.0.as_ref().unwrap().marker,
                Some((Target::Ground(x,y),PLAYER_COL)) if x==f32::from(valid.0)+f32::from(w)/2.0 && y==f32::from(valid.1)+f32::from(h)/2.0));
            if zoom == 1.0 {
                proof(&app, size, "valid", now);
            }
            let mut reference = app.load_session().unwrap();
            yr_app::snapshot::restore(&mut reference, &ready).unwrap();
            app.tick_session();
            reference.step(&[queued, expected]);
            reference.sim.drain_events();
            assert!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
                    == yr_app::snapshot::encode(&reference).unwrap()
            );
            assert!(app.session.as_ref().unwrap().ready_structure().is_none());
            // A stale mode with no ready item must not announce a queued placement.
            let other = valid_origin(app.session.as_ref().unwrap(), slot);
            app.placing = Some(slot);
            point_at(&mut app, slot, other, size);
            assert!(app.placement_target().unwrap().2);
            let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            click(&mut app, MouseButton::Left);
            assert!(app.pending.is_empty());
            assert_eq!(
                app.order_feedback.message(Instant::now()),
                Some("Cannot place here")
            );
            assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == before);
        }
    }
    yr_app::snapshot::restore(app.session.as_mut().unwrap(), &ready).unwrap();
    let size = app.camera_size();
    app.placing = Some(slot);
    point_at(&mut app, slot, valid, size);
    app.focus_changed(false);
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    app.focus_changed(true);
    pickup(&mut app, slot, size);
    point_at(&mut app, slot, valid, size);
    app.open_menu(yr_app::menu::Page::Pause);
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    app.menu.set(yr_app::menu::Page::Closed);
    pickup(&mut app, slot, size);
    point_at(&mut app, slot, valid, size);
    app.resize_window((640, 480), 1.0).unwrap();
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    app.placing = Some(usize::MAX);
    app.pointer_moved((100.0, 100.0));
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty());
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Cannot place here")
    );
    assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == ready);
}

fn valid_origin(session: &Session, slot: usize) -> (u16, u16) {
    let grid = session.sim.terrain().unwrap();
    (0..grid.height())
        .find_map(|y| {
            (0..grid.width())
                .map(|x| grid.cell_at_local(x, y))
                .find(|&cell| session.can_place(slot, cell))
        })
        .unwrap()
}
fn cursor_cell(app: &App, slot: usize, origin: (u16, u16)) -> (f32, f32) {
    let (w, h) = app.session.as_ref().unwrap().buildables[slot]
        .footprint()
        .unwrap();
    (
        f32::from(origin.0) + f32::from((w - 1) / 2) + 0.5,
        f32::from(origin.1) + f32::from((h - 1) / 2) + 0.5,
    )
}
fn point_at(app: &mut App, slot: usize, origin: (u16, u16), size: (u32, u32)) {
    let cell = cursor_cell(app, slot, origin);
    app.center_world_at(cell, (size.0 - SIDEBAR_W, size.1));
    app.pointer_moved((f64::from(size.0 - SIDEBAR_W) / 2.0, f64::from(size.1) / 2.0));
}
fn pickup(app: &mut App, slot: usize, size: (u32, u32)) {
    app.sidebar_view.select(sidebar::Category::Structures);
    let index = sidebar::slots(app.session.as_ref().unwrap(), sidebar::Category::Structures)
        .position(|s| s == slot)
        .unwrap();
    let (x, y) = sidebar::slot_origin(size.0, index);
    app.pointer_moved((f64::from(x + 20), f64::from(y + 20)));
    click(app, MouseButton::Left);
}
fn click(app: &mut App, button: MouseButton) {
    app.pointer_button(ElementState::Pressed, button);
    app.pointer_button(ElementState::Released, button);
}
fn proof(app: &App, size: (u32, u32), name: &str, now: Instant) {
    if let Some(out) = std::env::var_os("OPENYR_PLACEMENT_ORDER_PROOF") {
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
