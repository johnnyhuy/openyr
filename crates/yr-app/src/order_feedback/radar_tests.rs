use super::*;

#[test]
fn radar_notices_preserve_the_existing_command_sequence_and_clear_on_navigation() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.selected.extend([7, 9]);
    for attack_move in [false, true] {
        app.attack_move = attack_move;
        app.view.dirty = false;
        app.radar_click((12.75, 20.25));
        let (x, y) = App::cell_leptons((12.75, 20.25));
        let expected: Vec<_> = [7, 9]
            .into_iter()
            .map(|id| {
                if attack_move {
                    Command::AttackMove { id, x, y }
                } else {
                    Command::Move { id, x, y }
                }
            })
            .collect();
        assert_eq!(app.pending, expected);
        let notice = app.order_feedback.0.as_ref().unwrap();
        assert_eq!(
            notice.text,
            if attack_move {
                "Attack-move order sent (2 units)"
            } else {
                "Move order sent (2 units)"
            }
        );
        assert!(matches!(notice.marker, Some((Target::Ground(x,y),_)) if x==12.5 && y==20.5));
        assert!(app.order_feedback.message(notice.at + LIFETIME).is_none());
        assert!(app.view.dirty && !app.attack_move);
        app.pending.clear();
    }
    app.selected.clear();
    app.radar_click((8.5, 9.5));
    assert!(app.radar_drag && app.pending.is_empty() && app.order_feedback.0.is_none());
}

#[test]
fn retail_radar_input_acknowledges_orders_with_unchanged_commands_and_save_bytes() {
    let directory = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    let mut session = app.load_session().unwrap();
    session.sim.set_ai(None);
    let own = session
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some())
        .unwrap()
        .id;
    let slot = session
        .buildables
        .iter()
        .position(|b| b.id == "AMRADR")
        .unwrap();
    session.step(&[session.produce(slot).unwrap()]);
    while session.ready_structure().is_none() {
        session.step(&[]);
        assert!(session.sim.tick() < 2000);
    }
    let grid = session.sim.terrain().unwrap();
    let spot = (0..grid.height())
        .find_map(|y| {
            (0..grid.width())
                .map(|x| grid.cell_at_local(x, y))
                .find(|&cell| session.can_place(slot, cell))
        })
        .unwrap();
    session.step(&[session.place(slot, spot).unwrap()]);
    assert!(!session.radar.ready());
    app.session = Some(session);
    app.selected = [own].into();
    app.pointer_moved((1200.0, 100.0));
    click(&mut app, MouseButton::Left);
    assert!(
        app.pending.is_empty() && app.order_feedback.0.is_none(),
        "opening radar ignores orders"
    );
    while !app.session.as_ref().unwrap().radar.ready() {
        app.session.as_mut().unwrap().step(&[]);
        assert!(app.session.as_ref().unwrap().sim.tick() < 3000);
    }
    app.map = Some(yr_render::render_map_background(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let mut receipts = Vec::new();
    for size in [(640, 480), (1000, 700), (1280, 800)] {
        app.resize_window(size, 1.0).unwrap();
        let (pos, cell) = destination(&app, size, own);
        for zoom in [0.5, 1.0, 2.0] {
            app.view.zoom = zoom;
            app.center_world_at(cell, (size.0 - SIDEBAR_W, size.1));
            for attack_move in [false, true] {
                app.selected = [own].into();
                app.attack_move = attack_move;
                let camera = (app.view.cam_x, app.view.cam_y);
                let selection = app.selected.clone();
                let (x, y) = App::cell_leptons(cell);
                let expected = if attack_move {
                    Command::AttackMove { id: own, x, y }
                } else {
                    Command::Move { id: own, x, y }
                };
                app.pointer_moved(pos);
                click(&mut app, MouseButton::Left);
                assert_eq!(app.pending, [expected]);
                assert_eq!(app.selected, selection);
                assert_eq!((app.view.cam_x, app.view.cam_y), camera);
                assert_eq!(app.view.zoom, zoom);
                let at = app.order_feedback.0.as_ref().unwrap().at;
                assert_eq!(
                    app.order_feedback.message(at),
                    Some(if attack_move {
                        "Attack-move order sent (1 unit)"
                    } else {
                        "Move order sent (1 unit)"
                    })
                );
                if zoom == 1.0 {
                    proof(
                        &app,
                        size,
                        if attack_move { "attack-move" } else { "move" },
                        at,
                    );
                }
                app.pending.clear();
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    saved
                );
                click(&mut app, MouseButton::Right);
                assert!(app.order_feedback.0.is_none() && app.selected.is_empty());
                // Empty selection navigates without displaying an order acknowledgement.
                click(&mut app, MouseButton::Left);
                assert!(app.order_feedback.0.is_none() && app.pending.is_empty());
                app.radar_drag = false;
            }
        }
        receipts.push(serde_json::json!({"size":size,"position":pos,"cell":cell}));
    }
    app.selected = [own].into();
    let size = app.camera_size();
    let (pos, _) = destination(&app, size, own);
    app.focus_changed(false);
    app.pointer_moved(pos);
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    app.focus_changed(true);
    app.pointer_moved(pos);
    app.open_menu(yr_app::menu::Page::Pause);
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    app.menu.set(yr_app::menu::Page::Closed);
    app.resize_window((640, 480), 1.0).unwrap();
    click(&mut app, MouseButton::Left);
    assert!(app.pending.is_empty() && app.order_feedback.0.is_none());
    if let Some(out) = std::env::var_os("OPENYR_RADAR_ORDER_PROOF") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        yr_app::snapshot::save(app.session.as_ref().unwrap(), &out.join("active.oyrsave")).unwrap();
        let sim = &app.session.as_ref().unwrap().sim;
        let unit = sim.units().iter().find(|u| u.id == own).unwrap();
        let ground = yr_app::interaction::ground_point(
            app.map.as_ref().unwrap(),
            sim.terrain(),
            (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0),
        );
        std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"layouts":receipts,"tick":sim.tick(),"hash":format!("{:016x}",sim.state_hash()),"unit_id":own,"unit_ground":ground,"map_size":[app.map.as_ref().unwrap().width,app.map.as_ref().unwrap().height]})).unwrap()).unwrap();
    }
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    // Submit the same order with and without UI feedback, then compare full state.
    let size = app.camera_size();
    let (pos, cell) = destination(&app, size, own);
    app.selected = [own].into();
    app.pointer_moved(pos);
    click(&mut app, MouseButton::Left);
    let commands = app.pending.clone();
    let (x, y) = App::cell_leptons(cell);
    assert_eq!(commands, [Command::Move { id: own, x, y }]);
    let mut reference = app.load_session().unwrap();
    yr_app::snapshot::restore(&mut reference, &saved).unwrap();
    app.tick_session();
    reference.step(&commands);
    // The live tick consumes presentation events after stepping the session.
    reference.sim.drain_events();
    assert!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
            == yr_app::snapshot::encode(&reference).unwrap(),
        "feedback must preserve the full snapshot after equivalent event consumption"
    );
    let provider = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && usize::from(b.kind) == slot)
        .unwrap()
        .id;
    app.session
        .as_mut()
        .unwrap()
        .step(&[Command::Destroy { id: provider }]);
    assert!(!app.session.as_ref().unwrap().radar.ready());
    app.order_feedback.clear();
    app.pointer_moved(pos);
    click(&mut app, MouseButton::Left);
    assert!(
        app.pending.is_empty() && app.order_feedback.0.is_none(),
        "unpowered radar ignores orders"
    );
}

fn click(app: &mut App, button: MouseButton) {
    app.pointer_button(ElementState::Pressed, button);
    app.pointer_button(ElementState::Released, button);
}
fn destination(app: &App, size: (u32, u32), id: u32) -> ((f64, f64), (f32, f32)) {
    let session = app.session.as_ref().unwrap();
    let unit = session.sim.units().iter().find(|u| u.id == id).unwrap();
    (50..154)
        .find_map(|y| {
            ((size.0 - SIDEBAR_W + 14)..(size.0 - 14)).find_map(|x| {
                let pos = (f64::from(x), f64::from(y));
                let cell = session.radar.target(size.0, pos)?;
                let distance = (cell.0 - unit.x.raw() as f32 / 256.0).abs()
                    + (cell.1 - unit.y.raw() as f32 / 256.0).abs();
                let (lx, ly) = App::cell_leptons(cell);
                ((4.0..8.0).contains(&distance) && session.sim.can_move_to(id, lx, ly))
                    .then_some((pos, cell))
            })
        })
        .unwrap()
}
fn proof(app: &App, size: (u32, u32), name: &str, now: Instant) {
    if let Some(out) = std::env::var_os("OPENYR_RADAR_ORDER_PROOF") {
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
