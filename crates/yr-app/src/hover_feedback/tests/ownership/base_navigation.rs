use super::*;

fn h(app: &mut App) {
    key(app, KeyCode::KeyH, ElementState::Pressed);
    key(app, KeyCode::KeyH, ElementState::Released);
}
fn bytes(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}
fn yard(app: &App) -> (u32, (f32, f32)) {
    let b = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .buildings()
        .iter()
        .filter(|b| {
            b.owner == yr_sim::OWNER_PLAYER && b.hp > 0 && b.role == yr_sim::Role::ConstructionYard
        })
        .min_by_key(|b| b.id)
        .unwrap();
    let (x, y) = b.centre();
    (b.id, (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0))
}

#[test]
fn base_shortcut_centers_the_drawn_yard_without_changing_match_state() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            let (mut app, own, _) = scene(size);
            app.view.zoom = zoom;
            let map = app.map.as_mut().unwrap();
            map.width = 12000;
            map.height = 8000;
            map.origin_x = -6000;
            map.origin_y = -4000;
            let (_, cell) = yard(&app);
            let sim = &mut app.session.as_mut().unwrap().sim;
            let mut grid = sim.terrain().unwrap().clone();
            grid.cell_mut(cell.0.floor() as u16, cell.1.floor() as u16)
                .unwrap()
                .height = 4;
            sim.set_terrain(grid);
            let (x, y) = yr_app::interaction::ground_point(
                app.map.as_ref().unwrap(),
                app.session.as_ref().unwrap().sim.terrain(),
                cell,
            );
            let before = bytes(&app);
            app.pending.push(Command::Stop { id: own });
            h(&mut app);
            assert!(
                (((x - app.view.cam_x) * zoom) - (size.0 - SIDEBAR_W) as f32 * 0.5).abs() < 0.01
            );
            assert!((((y - app.view.cam_y) * zoom) - size.1 as f32 * 0.5).abs() < 0.01);
            assert_eq!(app.view.zoom, zoom);
            assert_eq!(app.selected, BTreeSet::from([own]));
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(bytes(&app), before);
            assert_eq!(
                app.order_feedback.message(Instant::now()),
                Some("Base centered")
            );
        }
    }
    println!("Nine native H cases center the elevated owned yard's drawn ground anchor inside the world pane across three layouts/zooms, preserving selection, zoom, pending Stop and exact Session bytes.");
}

#[test]
fn base_shortcut_clamps_prefers_oldest_owned_yard_and_reports_no_base() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, own, _) = scene(size);
        let (old, cell) = yard(&app);
        let sim = &mut app.session.as_mut().unwrap().sim;
        let new = sim.next_id_peek();
        sim.step(&[Command::PlaceBuilding {
            kind: 0,
            x: 30,
            y: 30,
            w: 1,
            h: 1,
            hp: 100,
            armor: 0,
            power: 0,
            owner: yr_sim::OWNER_PLAYER,
            role: yr_sim::Role::ConstructionYard,
        }]);
        assert!(sim.buildings().iter().any(|b| b.id == new));
        for zoom in [0.25, 1.0, 4.0] {
            app.view.zoom = zoom;
            let map = app.map.as_ref().unwrap();
            let (x, y) = yr_app::interaction::ground_point(
                map,
                app.session.as_ref().unwrap().sim.terrain(),
                cell,
            );
            let clamp = |target: f32, extent: u32, visible: u32| {
                let spare = extent as f32 - visible as f32 / zoom;
                if spare <= 0.0 {
                    spare * 0.5
                } else {
                    (target - visible as f32 / (2.0 * zoom)).clamp(0.0, spare)
                }
            };
            let expected = (
                clamp(x, map.width, size.0 - SIDEBAR_W),
                clamp(y, map.height, size.1),
            );
            let before = bytes(&app);
            h(&mut app);
            assert_eq!((app.view.cam_x, app.view.cam_y), expected);
            assert_eq!(bytes(&app), before);
        }
        app.session
            .as_mut()
            .unwrap()
            .sim
            .step(&[Command::Destroy { id: old }]);
        assert_eq!(yard(&app).0, new);
        h(&mut app);
        app.session
            .as_mut()
            .unwrap()
            .sim
            .step(&[Command::Destroy { id: new }]);
        let camera = (app.view.cam_x, app.view.cam_y);
        let before = bytes(&app);
        h(&mut app);
        assert_eq!((app.view.cam_x, app.view.cam_y), camera);
        assert_eq!(
            app.order_feedback.message(Instant::now()),
            Some("No construction yard available")
        );
        assert_eq!(app.selected, BTreeSet::from([own]));
        assert!(app.pending.is_empty());
        assert_eq!(bytes(&app), before);
    }
    println!("H obeys raster bounds/oversized-view centering, prefers the oldest owned yard, skips destroyed yards and leaves the camera unchanged when only other buildings or enemy yards remain.");
}

#[test]
fn base_navigation_preserves_modes_and_cancels_old_pointer_releases() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for mode in 0..5 {
            let (mut app, own, _) = scene(size);
            point_at(&mut app, own, size);
            app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                .into_iter()
                .cycle()
                .take(1600 * 1200 * 4)
                .collect();
            match mode {
                1 => app.attack_move = true,
                2 => app.placing = Some(0),
                3 => app.service_mode = ServiceMode::Repair,
                4 => app.service_mode = ServiceMode::Sell,
                _ => {}
            }
            let modes = (app.attack_move, app.placing, app.service_mode);
            let before = bytes(&app);
            app.pending.push(Command::Stop { id: own });
            if mode == 0 {
                capture(&app, &format!("base-{}-before", size.0));
                app.pointer_button(ElementState::Pressed, MouseButton::Right);
                app.pointer_moved((app.cursor.0 + 10.0, app.cursor.1 + 5.0));
                assert!(app.right_panning);
            }
            h(&mut app);
            assert!(
                app.left_down.is_none()
                    && app.right_down.is_none()
                    && !app.right_panning
                    && !app.radar_drag
            );
            assert_eq!((app.attack_move, app.placing, app.service_mode), modes);
            app.pointer_button(ElementState::Released, MouseButton::Left);
            app.pointer_button(ElementState::Released, MouseButton::Right);
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(app.selected, BTreeSet::from([own]));
            assert_eq!(bytes(&app), before);
            if mode == 0 {
                capture(&app, &format!("base-{}-after", size.0));
            }
        }
    }
    println!("15 native layout/mode cases preserve placement, repair/sell and attack-move state; a real prior right-pan release cannot clear selection or queue an order after H. Compositor states use authored fixtures.");
}

#[test]
fn base_binding_guards_menu_text_and_ctrl_h_harvest_dispatch() {
    for guard in 0..9 {
        let (mut app, _, _) = scene((640, 480));
        match guard {
            0 => app.pointer.focused = false,
            1 => app.shift = true,
            2 => app.super_key = true,
            3 => app.session.as_mut().unwrap().outcome = Outcome::Lost,
            4 => app.map = None,
            5 => {
                app.control = true;
                app.shift = true;
            }
            _ => {}
        }
        let before = bytes(&app);
        let camera = (app.view.cam_x, app.view.cam_y);
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyH),
            ElementState::Pressed,
            Some("h"),
            guard == 6,
            guard == 7,
        );
        if guard != 8 {
            assert_eq!((app.view.cam_x, app.view.cam_y), camera);
            assert!(app.order_feedback.message(Instant::now()).is_none());
        }
        assert!(app.pending.is_empty());
        assert_eq!(bytes(&app), before);
    }
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, own, enemy) = scene(size);
        app.session
            .as_mut()
            .unwrap()
            .sim
            .step(&[Command::AssignRole {
                id: own,
                role: yr_sim::Role::Harvester,
            }]);
        app.selected = BTreeSet::from([own, enemy]);
        let before = bytes(&app);
        h(&mut app);
        assert!(app.pending.is_empty(), "bare H never harvests");
        app.control = true;
        h(&mut app);
        app.control = false;
        assert_eq!(app.pending, [Command::Harvest { id: own }]);
        assert_eq!(bytes(&app), before);
        assert_eq!(
            app.order_feedback.message(Instant::now()),
            Some("Harvest order sent (1 unit)")
        );
        let folder = PreferenceDirectory::new();
        app.save_path = folder.0.join("quicksave.oyrsave");
        app.open_menu(Page::Save);
        app.menu.name = "Name".into();
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyH),
            ElementState::Pressed,
            Some("h"),
            false,
            false,
        );
        assert_eq!(app.menu.name, "Nameh");
        assert_eq!(bytes(&app), before);
    }
    let mut empty = new_app(String::new(), String::new(), 1);
    h(&mut empty);
    assert!(empty.pending.is_empty());
    println!("Bare H is navigation, Ctrl+H queues only eligible owned Harvest with the original feedback, guards remain inert and Save still accepts h. No simulation step is caused by either shortcut.");
}

#[test]
fn match_entry_centers_owned_ground_and_preserves_commands_selection_and_save() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            let (mut app, own, _) = scene(size);
            app.view.zoom = zoom;
            let map = app.map.as_mut().unwrap();
            map.width = 12000;
            map.height = 8000;
            map.origin_x = -6000;
            map.origin_y = -4000;
            let (_, cell) = yard(&app);
            let sim = &mut app.session.as_mut().unwrap().sim;
            let mut grid = sim.terrain().unwrap().clone();
            grid.cell_mut(cell.0.floor() as u16, cell.1.floor() as u16)
                .unwrap()
                .height = 4;
            sim.set_terrain(grid);
            let point = yr_app::interaction::ground_point(
                app.map.as_ref().unwrap(),
                app.session.as_ref().unwrap().sim.terrain(),
                cell,
            );
            app.pending.push(Command::Stop { id: own });
            let before = bytes(&app);
            app.center_match_view();
            assert!(
                ((point.0 - app.view.cam_x) * zoom - (size.0 - SIDEBAR_W) as f32 / 2.0).abs()
                    < 0.01
            );
            assert!(((point.1 - app.view.cam_y) * zoom - size.1 as f32 / 2.0).abs() < 0.01);
            assert_eq!(app.view.zoom, zoom);
            assert_eq!(app.selected, BTreeSet::from([own]));
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(bytes(&app), before);
        }
    }
}

#[test]
fn match_entry_falls_back_to_owned_force_and_then_bounded_map_center() {
    let (mut app, own, _) = scene((800, 600));
    let sim = &mut app.session.as_mut().unwrap().sim;
    let remove: Vec<_> = sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_PLAYER)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    let remove = remove
        .into_iter()
        .chain(
            sim.units()
                .iter()
                .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.id != own)
                .map(|u| Command::Remove { id: u.id }),
        )
        .collect::<Vec<_>>();
    sim.step(&remove);
    let unit = sim.units().iter().find(|u| u.id == own).unwrap();
    let cell = (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0);
    let before = bytes(&app);
    app.center_match_view();
    let camera = (app.view.cam_x, app.view.cam_y);
    app.center_world_at(cell, (800 - SIDEBAR_W, 600));
    app.bound_camera((800, 600));
    assert_eq!((app.view.cam_x, app.view.cam_y), camera);
    assert_eq!(bytes(&app), before);
    let sim = &mut app.session.as_mut().unwrap().sim;
    let remove: Vec<_> = sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
        .map(|u| Command::Remove { id: u.id })
        .collect();
    sim.step(&remove);
    let before = bytes(&app);
    app.view.cam_x = -10000.0;
    app.view.cam_y = 10000.0;
    app.center_match_view();
    let camera = (app.view.cam_x, app.view.cam_y);
    app.view
        .center_map(app.map.as_ref().unwrap(), (800 - SIDEBAR_W, 600));
    app.bound_camera((800, 600));
    assert_eq!((app.view.cam_x, app.view.cam_y), camera);
    assert_eq!(bytes(&app), before);
}
