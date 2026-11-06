use super::*;

#[test]
fn brackets_clip_to_world_keep_fixed_corners_and_reject_invalid_frames() {
    let size = (640, 480);
    let blank = vec![0; (size.0 * size.1 * 4) as usize];
    for half in [14.0, 28.0, 56.0] {
        let mut frame = blank.clone();
        draw_brackets(&mut frame, size, (200.0, 200.0), half, PLAYER_COL);
        assert_eq!(
            frame
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| **p == PLAYER_COL)
                .count(),
            44
        );
    }
    for center in [(-1.0, -1.0), (471.0, 479.0), (480.0, 200.0)] {
        let mut frame = blank.clone();
        draw_brackets(&mut frame, size, center, 28.0, PLAYER_COL);
        for y in 0..size.1 {
            let start = ((y * size.0 + size.0 - SIDEBAR_W) * 4) as usize;
            assert!(frame[start..((y + 1) * size.0 * 4) as usize]
                .iter()
                .all(|v| *v == 0));
        }
    }
    for (center, half) in [
        ((f32::NAN, 0.0), 20.0),
        ((0.0, f32::INFINITY), 20.0),
        ((0.0, 0.0), f32::MAX),
        ((f32::MAX, 0.0), 20.0),
        ((0.0, 0.0), -1.0),
    ] {
        let mut frame = blank.clone();
        draw_brackets(&mut frame, size, center, half, PLAYER_COL);
        assert_eq!(frame, blank);
    }
    let mut invalid = vec![0; 9];
    draw_brackets(&mut invalid, size, (200.0, 200.0), 20.0, PLAYER_COL);
    assert_eq!(invalid, vec![0; 9]);
}

#[test]
fn retail_hover_previews_match_clicks_and_guard_lifecycle_without_changing_saves() {
    let directory = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    let sim = &app.session.as_ref().unwrap().sim;
    let own = sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some_and(|w| w.damage > 0))
        .unwrap()
        .id;
    let enemy = sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_ENEMY)
        .unwrap()
        .id;
    let building = sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        .unwrap()
        .id;
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    if let Some(out) = std::env::var_os("OPENYR_HOVER_PROOF") {
        let map = app.map.as_ref().unwrap();
        let sim = &app.session.as_ref().unwrap().sim;
        let targets: Vec<_> = [own, enemy, building].into_iter().map(|id| {
            let (x,y) = sim.units().iter().find(|u| u.id==id).map(|u| (u.x,u.y))
                .or_else(|| sim.buildings().iter().find(|b| b.id==id).map(|b| b.centre())).unwrap();
            let cell = (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0);
            serde_json::json!({"id":id,"cell":cell,"ground":yr_app::interaction::ground_point(map,sim.terrain(),cell)})
        }).collect();
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("native-geometry.json"), serde_json::to_vec_pretty(&serde_json::json!({"map_size":[map.width,map.height],"targets":targets,"tick":sim.tick(),"hash":format!("{:016x}",sim.state_hash())})).unwrap()).unwrap();
    }

    for size in [(640, 480), (1000, 700), (1280, 800)] {
        app.resize_window(size, 1.0).unwrap();
        for zoom in [0.5, 1.0, 2.0] {
            app.view.zoom = zoom;
            app.selected.clear();
            point_at(&mut app, own, size);
            assert_eq!(
                app.hover_target(size),
                Some(Target {
                    id: own,
                    color: PLAYER_COL
                })
            );
            if zoom == 1.0 {
                proof(&mut app, size, "select");
            }
            app.pointer_button(ElementState::Pressed, MouseButton::Left);
            assert!(app.hover_target(size).is_none());
            app.pointer_button(ElementState::Released, MouseButton::Left);
            assert_eq!(app.selected, [own].into());
            for id in [enemy, building] {
                point_at(&mut app, id, size);
                assert_eq!(
                    app.hover_target(size),
                    Some(Target {
                        id,
                        color: ENEMY_COL
                    })
                );
                let commands = app.world_action(app.cursor).unwrap().commands;
                assert!(commands
                    .iter()
                    .all(|c| matches!(c, Command::Attack {target, ..} if *target==id)));
                if zoom == 1.0 {
                    proof(
                        &mut app,
                        size,
                        if id == enemy {
                            "attack-unit"
                        } else {
                            "attack-building"
                        },
                    );
                }
                app.pointer_button(ElementState::Pressed, MouseButton::Left);
                app.pointer_button(ElementState::Released, MouseButton::Left);
                assert_eq!(app.pending, commands);
                app.pending.clear();
            }
        }
    }
    let size = app.camera_size();
    point_at(&mut app, enemy, size);
    for guard in 0..12 {
        match guard {
            0 => app.pointer.focused = false,
            1 => app.pointer.inside = false,
            2 => app.menu.set(yr_app::menu::Page::Pause),
            3 => app.left_down = Some(app.cursor),
            4 => app.right_down = Some(app.cursor),
            5 => app.right_panning = true,
            6 => app.radar_drag = true,
            7 => app.placing = Some(0),
            8 => app.service_mode = ServiceMode::Repair,
            9 => app.shift = true,
            10 => app.selected.clear(),
            11 => app.session.as_mut().unwrap().outcome = Outcome::Won,
            _ => unreachable!(),
        }
        assert!(app.hover_target(size).is_none(), "guard {guard}");
        app.pointer.focused = true;
        app.pointer.inside = true;
        app.menu.set(yr_app::menu::Page::Closed);
        app.left_down = None;
        app.right_down = None;
        app.right_panning = false;
        app.radar_drag = false;
        app.placing = None;
        app.service_mode = ServiceMode::None;
        app.shift = false;
        app.selected = [own].into();
        app.session.as_mut().unwrap().outcome = Outcome::Ongoing;
    }
    // Control+Shift still routes to attack; Shift alone does not send orders.
    app.shift = true;
    app.control = true;
    assert_eq!(app.hover_target(size).unwrap().id, enemy);
    app.shift = false;
    app.control = false;
    app.selected = [u32::MAX].into();
    assert!(app.hover_target(size).is_none());
    app.selected = [own].into();
    app.pointer_moved((f64::from(size.0 - 10), 250.0));
    assert!(app.hover_target(size).is_none());
    point_at(&mut app, enemy, size);
    app.resize_window((640, 480), 1.0).unwrap();
    assert!(app.hover_target((640, 480)).is_none());
    assert!(app.pending.is_empty());
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    let unarmed = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_none())
        .unwrap()
        .id;
    point_at(&mut app, enemy, (640, 480));
    app.selected = [unarmed].into();
    assert!(app.hover_target((640, 480)).is_none());
    app.selected = [own].into();
    // The preview is recomputed, so removal cannot leave a stale id.
    point_at(&mut app, enemy, (640, 480));
    app.session
        .as_mut()
        .unwrap()
        .step(&[Command::Destroy { id: enemy }]);
    assert!(app.hover_target((640, 480)).is_none());
}

fn point_at(app: &mut App, id: u32, size: (u32, u32)) {
    let sim = &app.session.as_ref().unwrap().sim;
    let (x, y) = sim
        .units()
        .iter()
        .find(|u| u.id == id)
        .map(|u| (u.x, u.y))
        .or_else(|| {
            sim.buildings()
                .iter()
                .find(|b| b.id == id)
                .map(|b| b.centre())
        })
        .unwrap();
    let cell = (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0);
    app.center_world_at(cell, (size.0 - SIDEBAR_W, size.1));
    app.pointer_moved((f64::from(size.0 - SIDEBAR_W) / 2.0, f64::from(size.1) / 2.0));
}

fn proof(app: &mut App, size: (u32, u32), name: &str) {
    let Some(out) = std::env::var_os("OPENYR_HOVER_PROOF") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let now = Instant::now();
    let mut before = vec![0; (size.0 * size.1 * 4) as usize];
    app.pointer.inside = false;
    app.compose_at(&mut before, size.0, size.1, now);
    app.pointer.inside = true;
    let mut after = before.clone();
    app.draw_hover_target(&mut after, size);
    assert_ne!(before, after);
    let mut composed = vec![0; before.len()];
    app.compose_at(&mut composed, size.0, size.1, now);
    // The integrated compositor retains the preview outside the cursor canvas.
    assert!(
        before
            .as_chunks::<4>()
            .0
            .iter()
            .zip(after.as_chunks::<4>().0)
            .zip(composed.as_chunks::<4>().0)
            .filter(|((b, a), c)| b != a && a == c)
            .count()
            >= 22
    );
    for (phase, frame) in [("before", before), ("after", after), ("composed", composed)] {
        std::fs::write(
            out.join(format!("{}x{}-{name}-{phase}.png", size.0, size.1)),
            yr_render::png::encode_rgba(size.0, size.1, &frame),
        )
        .unwrap();
    }
}
