use super::*;

#[test]
fn teams_replace_extend_and_prune_without_crossing_slots() {
    let mut groups = ControlGroups::default();
    let mut selected = [1, 2, 99].into();
    let mut owned = [1, 2, 3].into();
    let now = Instant::now();
    for index in 0..9 {
        groups.activate(index, &mut selected, &owned, true, false, now);
    }
    assert_eq!(selected, [1, 2].into());
    selected = [3].into();
    groups.activate(1, &mut selected, &owned, true, false, now);
    groups.activate(0, &mut selected, &owned, false, false, now);
    assert_eq!(selected, [1, 2].into());
    groups.activate(1, &mut selected, &owned, false, true, now);
    assert_eq!(selected, [1, 2, 3].into());
    assert!(
        groups.recalled.is_none(),
        "additive recall never arms centering"
    );
    owned.remove(&1);
    owned.remove(&2);
    groups.activate(0, &mut selected, &owned, false, false, now);
    assert_eq!(
        selected,
        [3].into(),
        "empty recall keeps the surviving selection"
    );
    assert!(groups.members[0].is_empty());
    assert!(
        groups.members[8].is_empty(),
        "prune every team after ownership/death changes"
    );
    assert_eq!(groups.message(now), Some("Team 1 is empty"));
    selected.clear();
    groups.activate(1, &mut selected, &owned, true, false, now);
    assert!(groups.members[1].is_empty());
    assert_eq!(groups.message(now), Some("Team 2 cleared"));
    assert!(groups.message(now + NOTICE_TIME).is_none());
}

#[test]
fn centering_requires_two_deliberate_same_team_recalls_within_the_native_window() {
    let mut groups = ControlGroups::default();
    let owned = [1, 2].into();
    let mut selected = [1].into();
    let now = Instant::now();
    groups.activate(0, &mut selected, &owned, true, false, now);
    groups.activate(1, &mut selected, &owned, true, false, now);
    assert!(
        !groups
            .activate(0, &mut selected, &owned, false, false, now)
            .center
    );
    assert!(
        groups
            .activate(0, &mut selected, &owned, false, false, now + DOUBLE_PRESS)
            .center
    );
    assert!(
        !groups
            .activate(
                0,
                &mut selected,
                &owned,
                false,
                false,
                now + DOUBLE_PRESS * 3
            )
            .center
    );
    assert!(
        !groups
            .activate(
                1,
                &mut selected,
                &owned,
                false,
                false,
                now + DOUBLE_PRESS * 3
            )
            .center
    );
    groups.interrupt();
    assert!(
        !groups
            .activate(
                1,
                &mut selected,
                &owned,
                false,
                false,
                now + DOUBLE_PRESS * 3
            )
            .center
    );
    selected.insert(2);
    assert!(
        !groups
            .activate(
                1,
                &mut selected,
                &owned,
                false,
                false,
                now + DOUBLE_PRESS * 3
            )
            .center
    );
    for (key, index) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .into_iter()
    .zip(0..9)
    {
        assert_eq!(slot(key), Some(index));
    }
    assert_eq!(slot(KeyCode::Digit0), None);
}

#[test]
fn retail_team_keys_preserve_match_state_and_drive_selection_orders_and_camera() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let ids: Vec<_> = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some_and(|w| w.damage > 0))
        .map(|u| u.id)
        .take(3)
        .collect();
    assert_eq!(ids.len(), 3);
    let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    app.selected = ids[..2].iter().copied().collect();
    app.center_group();
    app.control = true;
    let now = Instant::now();
    key(&mut app, KeyCode::Digit1, false, now);
    proof(&app, "01-assigned-tick1");
    app.control = false;
    app.selected.clear();
    app.attack_move = true;
    app.placing = Some(0);
    app.service_mode = ServiceMode::Sell;
    app.left_down = Some((10.0, 10.0));
    app.right_down = Some((20.0, 20.0));
    app.right_panning = true;
    app.radar_drag = true;
    let camera = (app.view.cam_x, app.view.cam_y);
    key(&mut app, KeyCode::Digit1, true, now);
    assert!(app.selected.is_empty(), "OS repeat is not recall");
    key(&mut app, KeyCode::Digit1, false, now);
    assert_eq!(app.selected, ids[..2].iter().copied().collect());
    assert_eq!(
        (app.view.cam_x, app.view.cam_y),
        camera,
        "one press only selects"
    );
    assert!(
        app.left_down.is_none()
            && app.right_down.is_none()
            && !app.right_panning
            && !app.radar_drag
    );
    assert!(!app.attack_move && app.placing.is_none() && app.service_mode == ServiceMode::None);
    app.left_release();
    app.right_release();
    assert!(
        app.pending.is_empty(),
        "a stale mouse release cannot issue an order"
    );
    for zoom in [0.25, 1.0, 4.0] {
        app.view.zoom = zoom;
        app.groups.recalled = Some((0, now));
        key(
            &mut app,
            KeyCode::Digit1,
            false,
            now + Duration::from_millis(100),
        );
        let points: Vec<_> = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .filter(|u| app.selected.contains(&u.id))
            .map(|u| {
                yr_app::interaction::ground_point(
                    app.map.as_ref().unwrap(),
                    app.session.as_ref().unwrap().sim.terrain(),
                    (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0),
                )
            })
            .collect();
        let minmax = |axis: usize| {
            points
                .iter()
                .map(|&(x, y)| if axis == 0 { x } else { y })
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
                    (lo.min(v), hi.max(v))
                })
        };
        let (x0, x1) = minmax(0);
        let (y0, y1) = minmax(1);
        assert!(
            (((x0 + x1) * 0.5 - app.view.cam_x) * zoom - (WIN_W - SIDEBAR_W) as f32 * 0.5).abs()
                < 0.01
        );
        assert!((((y0 + y1) * 0.5 - app.view.cam_y) * zoom - WIN_H as f32 * 0.5).abs() < 0.01);
    }
    app.view.zoom = 1.0;
    let recenter = Instant::now();
    key(&mut app, KeyCode::Digit1, false, recenter);
    key(
        &mut app,
        KeyCode::Digit1,
        false,
        recenter + Duration::from_millis(100),
    );
    proof(&app, "02-recalled-centered-tick1");
    let second = [ids[2]].into();
    app.selected = second;
    app.control = true;
    key(&mut app, KeyCode::Digit2, false, Instant::now());
    app.control = false;
    app.shift = true;
    key(&mut app, KeyCode::Digit1, false, Instant::now());
    assert_eq!(app.selected, ids.iter().copied().collect());
    app.shift = false;
    proof(&app, "03-additive-tick1");
    let group = app.groups.members[0].clone();
    app.open_menu(yr_app::menu::Page::Pause);
    app.control = true;
    app.selected.clear();
    key(&mut app, KeyCode::Digit1, false, Instant::now());
    assert_eq!(
        app.groups.members[0], group,
        "modal number input cannot reassign"
    );
    app.menu.set(yr_app::menu::Page::Closed);
    app.focus_changed(false);
    key(&mut app, KeyCode::Digit1, false, Instant::now());
    assert!(app.selected.is_empty());
    app.focus_changed(true);
    key(&mut app, KeyCode::Digit1, false, Instant::now());
    assert_eq!(app.selected, group);
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        before,
        "team keys and all proof redraws preserve the complete match save"
    );
    // Actual world-order resolver consumes the recalled selection.
    let enemy = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        .unwrap();
    let action = yr_app::interaction::at(
        app.session.as_ref().unwrap(),
        &app.selected,
        (i32::from(enemy.cell.0), i32::from(enemy.cell.1)),
        false,
        false,
        ServiceMode::None,
    );
    assert!(!action.commands.is_empty());
    assert!(action
        .commands
        .iter()
        .all(|c| matches!(c, Command::Attack{id,..} if app.selected.contains(id))));
    // Real removal and reassignment invalidate every remembered team member.
    app.session.as_mut().unwrap().sim.step(&[
        Command::Remove { id: ids[0] },
        Command::AssignOwner {
            id: ids[1],
            owner: yr_sim::OWNER_ENEMY,
        },
    ]);
    let changed = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    key(&mut app, KeyCode::Digit1, false, Instant::now());
    assert!(app.selected.is_empty() && app.groups.members[0].is_empty());
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        changed
    );
    proof(&app, "04-survivors-empty-tick2");
    let session = app.session.as_ref().unwrap();
    for (width, height) in [(1, 1), (168, 8), (192, 32), (320, 64), (1280, 80)] {
        let mut frame = vec![17; (width * height * 4) as usize];
        yr_app::interaction::draw_feedback(&mut frame, width, height, session, &"W界".repeat(1000));
        for y in 0..height {
            for x in width.saturating_sub(SIDEBAR_W)..width {
                let at = ((y * width + x) * 4) as usize;
                assert_eq!(
                    &frame[at..at + 4],
                    &[17; 4],
                    "feedback stays out of the sidebar"
                );
            }
        }
    }
    yr_app::interaction::draw_feedback(&mut [0; 3], 320, 64, session, "invalid framebuffer");
    // A completed replacement must clear groups even when unit IDs coincide.
    let (job, send, _) = menu_flow::LoadJob::channel(yr_app::menu::Page::Pause);
    send.send(Ok(menu_flow::Prepared::Match {
        map_name: "EB1.MMX".into(),
        session: Box::new(app.session.take().unwrap()),
        map: app.map.take().unwrap(),
        save_path: None,
        sprites: app.sprites.take().map(Box::new),
    }))
    .unwrap();
    app.load_job = Some(job);
    app.poll_menu_load();
    assert!(app.groups.members.iter().all(BTreeSet::is_empty));
    assert!(app.groups.notice.is_none() && app.selected.is_empty());
    app.selected.insert(ids[2]);
    app.control = true;
    key(&mut app, KeyCode::Digit3, false, Instant::now());
    assert!(!app.groups.members[2].is_empty());
    app.enter_front();
    assert!(app.groups.members.iter().all(BTreeSet::is_empty));
    println!("Retail EB1 team assignment/recall/addition, repeat/modal/focus guards, selection orders, camera zooms, replacement isolation and full save invariance passed.");
}

fn key(app: &mut App, key: KeyCode, repeat: bool, now: Instant) {
    assert!(app.group_key(key, ElementState::Pressed, repeat, now));
}

fn proof(app: &App, name: &str) {
    let Some(path) = std::env::var_os("OPENYR_GROUP_PROOF") else {
        return;
    };
    let path = PathBuf::from(path);
    std::fs::create_dir_all(&path).unwrap();
    let mut pixels = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose(&mut pixels, WIN_W, WIN_H);
    std::fs::write(
        path.join(format!("{name}.png")),
        yr_render::png::encode_rgba(WIN_W, WIN_H, &pixels),
    )
    .unwrap();
    std::fs::write(
        path.join(format!("{name}.txt")),
        format!(
            "tick={}\nselected={:?}\nmessage={:?}\nhash={:016x}\n",
            app.session.as_ref().unwrap().sim.tick(),
            app.selected,
            app.groups.message(Instant::now()),
            app.session.as_ref().unwrap().sim.state_hash()
        ),
    )
    .unwrap();
}
