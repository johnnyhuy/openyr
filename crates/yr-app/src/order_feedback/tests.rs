use super::*;

#[test]
fn notice_expiry_and_inactive_keyboard_input_do_not_submit_commands() {
    let now = Instant::now();
    let mut feedback = Feedback::default();
    feedback.record(Kind::Blocked, &[], (-1.0, 3.0), now);
    assert_eq!(feedback.message(now), Some("No selected unit can act here"));
    assert!(feedback.message(now - Duration::from_millis(1)).is_none());
    assert!(feedback.message(now + LIFETIME).is_none());
    assert!(feedback.expire(now + LIFETIME));
    assert!(!feedback.expire(now + LIFETIME));
    let mut app = new_app(String::new(), String::new(), 1);
    for code in [KeyCode::KeyS, KeyCode::KeyH] {
        assert!(app.unit_order_key(code, ElementState::Pressed, false, now));
        assert!(app.pending.is_empty() && app.order_feedback.message(now).is_none());
    }
}

#[test]
fn retail_tactical_orders_have_visible_feedback_without_changing_saved_state() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let sim = &app.session.as_ref().unwrap().sim;
    let unit = sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some_and(|w| w.damage > 0))
        .unwrap()
        .clone();
    let enemy = sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_ENEMY)
        .unwrap()
        .clone();
    let plant = sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        .unwrap()
        .clone();
    let own_cell = (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0);
    center(&mut app, own_cell, (WIN_W, WIN_H));
    app.cursor = point(&app, own_cell);
    app.left_down = Some(app.cursor);
    app.left_release();
    assert_eq!(app.selected, [unit.id].into());
    assert!(app.order_feedback.0.is_none());
    proof(&app, "01-selected-tick1", Instant::now());
    let cell = (-5..=5)
        .flat_map(|dy| {
            (-5..=5).map(move |dx| (unit.x.to_int_floor() + dx, unit.y.to_int_floor() + dy))
        })
        .find(|&cell| {
            yr_app::interaction::at(
                app.session.as_ref().unwrap(),
                &app.selected,
                cell,
                false,
                ServiceMode::None,
            )
            .cursor
                == Kind::Move
        })
        .unwrap();
    let destination = (cell.0 as f32 + 0.5, cell.1 as f32 + 0.5);
    app.cursor = point(&app, destination);
    let expected = app.world_action(app.cursor).unwrap().commands;
    app.left_down = Some(app.cursor);
    app.left_release();
    assert_eq!(app.pending, expected);
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(
        app.order_feedback.message(now),
        Some("Move order sent (1 unit)")
    );
    proof(&app, "02-move-tick1", now);
    let commands = app.pending.clone();
    app.pending.clear();
    app.attack_move = true;
    app.left_down = Some(app.cursor);
    app.left_release();
    assert!(!app.attack_move);
    assert!(matches!(app.pending.as_slice(), [Command::AttackMove { id, .. }] if *id == unit.id));
    let now = app.order_feedback.0.as_ref().unwrap().at;
    proof(&app, "03-attack-move-tick1", now);
    app.pending.clear();
    let (x, y) = plant.centre();
    let target = (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0);
    center(&mut app, target, (WIN_W, WIN_H));
    app.cursor = point(&app, target);
    app.selected.extend([enemy.id, u32::MAX]);
    app.left_down = Some(app.cursor);
    app.left_release();
    assert!(
        matches!(app.pending.as_slice(), [Command::Attack {id, target}] if *id == unit.id && *target == plant.id)
    );
    app.selected = [unit.id].into();
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(
        app.order_feedback.message(now),
        Some("Attack order sent (1 unit)")
    );
    proof(&app, "04-attack-tick1", now);
    let mut visible = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose_at(&mut visible, WIN_W, WIN_H, now);
    let at = ((WIN_H / 2 * WIN_W + (WIN_W - SIDEBAR_W) / 2 - 12) * 4) as usize;
    assert_eq!(
        &visible[at..at + 4],
        &ENEMY_COL,
        "marker is visible over live building art"
    );
    let mut expired = vec![0; visible.len()];
    app.compose_at(&mut expired, WIN_W, WIN_H, now + LIFETIME);
    assert_ne!(visible, expired);
    for y in 0..WIN_H {
        let at = ((y * WIN_W + WIN_W - SIDEBAR_W) * 4) as usize;
        assert_eq!(
            &visible[at..at + (SIDEBAR_W * 4) as usize],
            &expired[at..at + (SIDEBAR_W * 4) as usize]
        );
    }
    app.pending.clear();
    // Markers use world projection but keep UI pixel dimensions at every zoom/layout.
    for zoom in [0.25, 1.0, 4.0] {
        app.view.zoom = zoom;
        for size in [(800, 600), (1280, 800), (1760, 1000)] {
            center(&mut app, target, size);
            let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
            app.draw_order_marker(&mut frame, size, now);
            let bounds = pixel_bounds(&frame, size);
            let (cx, cy) = ((size.0 - SIDEBAR_W) as i32 / 2, size.1 as i32 / 2);
            assert_eq!(bounds, Some((cx - 14, cy - 7, cx + 14, cy + 7)));
        }
    }
    app.view.zoom = 1.0;
    center(&mut app, target, (WIN_W, WIN_H));
    app.cursor = point(&app, (-1.5, -1.5));
    app.left_click(app.cursor);
    assert!(app.pending.is_empty());
    let now = app.order_feedback.0.as_ref().unwrap().at;
    assert_eq!(
        app.order_feedback.message(now),
        Some("No selected unit can act here")
    );
    // Use a visible absent cell to show unavailable-action feedback.
    let grid = app.session.as_ref().unwrap().sim.terrain().unwrap();
    let origin = grid.origin();
    let hole = (0..grid.height())
        .flat_map(|dy| {
            (0..grid.width())
                .map(move |dx| ((origin.0 + dx as i32) as u16, (origin.1 + dy as i32) as u16))
        })
        .find(|&(x, y)| {
            if grid.cell(x, y).is_some() || grid.cell(x + 1, y).is_none() {
                return false;
            }
            let map = app.map.as_ref().unwrap();
            let pixel = yr_app::interaction::ground_point(
                map,
                Some(grid),
                (f32::from(x) + 0.5, f32::from(y) + 0.5),
            );
            let picked = yr_app::interaction::cell_at(map, Some(grid), pixel);
            grid.cell(picked.0.floor() as u16, picked.1.floor() as u16)
                .is_none()
        })
        .unwrap();
    let hole = (f32::from(hole.0) + 0.5, f32::from(hole.1) + 0.5);
    center(&mut app, hole, (WIN_W, WIN_H));
    app.cursor = point(&app, hole);
    app.left_click(app.cursor);
    assert!(app.pending.is_empty());
    proof(
        &app,
        "05-unavailable-tick1",
        app.order_feedback.0.as_ref().unwrap().at,
    );
    center(&mut app, own_cell, (WIN_W, WIN_H));
    app.selected = [unit.id, enemy.id, u32::MAX].into();
    let now = Instant::now();
    for guard in 0..4 {
        app.pointer.focused = guard != 0;
        app.menu.set(if guard == 1 {
            yr_app::menu::Page::Pause
        } else {
            yr_app::menu::Page::Closed
        });
        app.unit_order_key(
            KeyCode::KeyS,
            if guard == 2 {
                ElementState::Released
            } else {
                ElementState::Pressed
            },
            guard == 3,
            now,
        );
        assert!(app.pending.is_empty(), "guard {guard}");
    }
    app.unit_order_key(KeyCode::KeyS, ElementState::Pressed, false, now);
    assert!(matches!(app.pending.as_slice(), [Command::Stop { id }] if *id == unit.id));
    app.selected = [unit.id].into();
    assert_eq!(
        app.order_feedback.message(now),
        Some("Stop order sent (1 unit)")
    );
    proof(&app, "06-stop-tick1", now);
    app.pending.clear();
    app.unit_order_key(KeyCode::KeyH, ElementState::Pressed, false, now);
    assert!(app.pending.is_empty());
    assert_eq!(
        app.order_feedback.message(now),
        Some("Select a harvester to harvest")
    );
    let harvester = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.role == yr_sim::Role::Harvester)
        .unwrap()
        .id;
    app.selected.insert(harvester);
    app.left_down = Some(app.cursor);
    app.unit_order_key(KeyCode::KeyH, ElementState::Pressed, false, now);
    assert!(matches!(app.pending.as_slice(), [Command::Harvest {id}] if *id == harvester));
    app.pending.clear();
    app.left_release();
    assert!(app.pending.is_empty(), "key cancels stale pointer release");
    proof(&app, "07-harvest-tick1", now);
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    app.view.dirty = false;
    app.view.dirty |= app.order_feedback.expire(now + LIFETIME);
    assert!(app.view.dirty && app.order_feedback.0.is_none());
    proof(&app, "08-expired-tick1", now + LIFETIME);
    for reset in 0..4 {
        app.unit_order_key(KeyCode::KeyS, ElementState::Pressed, false, now);
        app.pending.clear();
        match reset {
            0 => {
                app.focus_changed(false);
                app.focus_changed(true);
            }
            1 => {
                app.open_menu(yr_app::menu::Page::Pause);
                app.menu.set(yr_app::menu::Page::Closed);
            }
            2 => {
                app.cancel_mode_or_selection();
            }
            _ => {
                app.select_box((0.0, 0.0), (1.0, 1.0), false);
            }
        }
        assert!(app.order_feedback.0.is_none());
    }
    // Acknowledging and painting do not alter the eventual simulation result.
    let session = app.session.as_mut().unwrap();
    let mut direct = yr_sim::Sim::from_save_bytes(&session.sim.save_bytes().unwrap()).unwrap();
    direct.step(&commands);
    app.pending = commands;
    app.tick_session();
    assert_eq!(
        app.session.as_ref().unwrap().sim.state_hash(),
        direct.state_hash()
    );
    check_target_tracking(&mut app);
    println!("EB1 tactical orders: resolver-identical commands; move/attack-move/attack/blocked feedback; Stop/Harvest ownership and repeat/focus/modal guards; nine zoom/layout marker checks; full save bytes and subsequent sim hash preserved");
}

fn check_target_tracking(app: &mut App) {
    let mut sim = yr_sim::Sim::new(42);
    sim.set_terrain(yr_sim::TerrainGrid::from_tiles(
        0,
        0,
        8,
        8,
        (0..8).flat_map(|y| (0..8).map(move |x| (x, y, 0, 13, 0))),
    ));
    let id = sim.next_id_peek();
    sim.step(&[Command::Spawn {
        kind: 0,
        x: Fix::from_int(2),
        y: Fix::from_int(2),
        loco: yr_sim::terrain::Loco::Track,
        speed: Fix::ONE,
        hp: 100,
        armor: 0,
        weapon: None,
    }]);
    app.session.as_mut().unwrap().sim = sim;
    let now = Instant::now();
    app.order_feedback.record(
        Kind::Attack,
        &[Command::Attack {
            id: 999,
            target: id,
        }],
        (2.0, 2.0),
        now,
    );
    center(app, (2.0, 2.0), (WIN_W, WIN_H));
    let mut before = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.draw_order_marker(&mut before, (WIN_W, WIN_H), now);
    app.session.as_mut().unwrap().sim.step(&[Command::Move {
        id,
        x: Fix::from_int(5),
        y: Fix::from_int(2),
    }]);
    let unit = &app.session.as_ref().unwrap().sim.units()[0];
    assert_ne!(unit.x, Fix::from_int(2));
    let position = point(
        app,
        (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0),
    );
    let mut after = vec![0; before.len()];
    app.draw_order_marker(&mut after, (WIN_W, WIN_H), now);
    assert_ne!(
        before, after,
        "target marker follows current world position"
    );
    let (x0, y0, x1, y1) = pixel_bounds(&after, (WIN_W, WIN_H)).unwrap();
    assert_eq!(
        ((x0 + x1) / 2, (y0 + y1) / 2),
        (position.0 as i32, position.1 as i32)
    );
    app.session
        .as_mut()
        .unwrap()
        .sim
        .step(&[Command::Destroy { id }]);
    after.fill(0);
    app.draw_order_marker(&mut after, (WIN_W, WIN_H), now);
    assert!(
        pixel_bounds(&after, (WIN_W, WIN_H)).is_none(),
        "lost target leaves no stale marker"
    );
}

fn center(app: &mut App, cell: (f32, f32), size: (u32, u32)) {
    app.center_world_at(cell, (size.0 - SIDEBAR_W, size.1));
}

fn point(app: &App, cell: (f32, f32)) -> (f64, f64) {
    let (x, y) = yr_app::interaction::ground_point(
        app.map.as_ref().unwrap(),
        app.session.as_ref().unwrap().sim.terrain(),
        cell,
    );
    (
        f64::from((x - app.view.cam_x) * app.view.zoom),
        f64::from((y - app.view.cam_y) * app.view.zoom),
    )
}

fn pixel_bounds(frame: &[u8], size: (u32, u32)) -> Option<(i32, i32, i32, i32)> {
    frame
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| p[3] != 0)
        .fold(None, |bounds, (i, _)| {
            let (x, y) = ((i % size.0 as usize) as i32, (i / size.0 as usize) as i32);
            Some(
                bounds.map_or((x, y, x, y), |(x0, y0, x1, y1): (i32, i32, i32, i32)| {
                    (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
                }),
            )
        })
}

fn proof(app: &App, name: &str, now: Instant) {
    let Some(dir) = std::env::var_os("OPENYR_ORDER_PROOF") else {
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
            "tick":app.session.as_ref().unwrap().sim.tick(),
            "hash":format!("{:016x}", app.session.as_ref().unwrap().sim.state_hash()),
            "message":app.order_feedback.message(now), "pending":format!("{:?}",app.pending),
            "camera":[app.view.cam_x,app.view.cam_y], "zoom":app.view.zoom
        }))
        .unwrap(),
    )
    .unwrap();
}
