use super::*;

fn spawn(app: &mut App, kind: u16, cell: (i32, i32), owner: u8) -> u32 {
    let sim = &mut app.session.as_mut().unwrap().sim;
    let id = sim.next_id_peek();
    sim.step(&[
        Command::Spawn {
            kind,
            x: Fix::from_int(cell.0),
            y: Fix::from_int(cell.1),
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::ZERO,
            hp: 100,
            armor: 0,
            weapon: None,
        },
        Command::AssignOwner { id, owner },
    ]);
    id
}
fn press_t(app: &mut App) {
    key(app, KeyCode::KeyT, ElementState::Pressed);
    key(app, KeyCode::KeyT, ElementState::Released);
}
fn bytes(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}

#[test]
fn type_selection_expands_visible_owned_types_without_mutating_the_match() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [1.0, 2.0, 4.0] {
            let (mut app, own, enemy) = scene(size);
            let buddy = spawn(&mut app, 0, (5, 4), yr_sim::OWNER_PLAYER);
            let other = spawn(&mut app, 1, (4, 5), yr_sim::OWNER_PLAYER);
            let far = spawn(&mut app, 0, (31, 31), yr_sim::OWNER_PLAYER);
            app.view.zoom = zoom;
            point_at(&mut app, own, size);
            app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                .into_iter()
                .cycle()
                .take(1600 * 1200 * 4)
                .collect();
            let before = bytes(&app);
            let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
            if zoom == 1.0 {
                capture(&app, &format!("types-{}-before", size.0));
            }
            press_t(&mut app);
            assert_eq!(app.selected, BTreeSet::from([own, buddy]));
            assert!(
                !app.selected.contains(&enemy)
                    && !app.selected.contains(&other)
                    && !app.selected.contains(&far)
            );
            assert_eq!(
                app.order_feedback.message(Instant::now()),
                Some("Type selection (2 units)")
            );
            if zoom == 1.0 {
                capture(&app, &format!("types-{}-after", size.0));
            }
            assert_eq!(bytes(&app), before);
            assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
            assert!(app.pending.is_empty());
        }
    }
    println!("Native T selects visible owned matching types at three layouts x three zooms, excluding enemy, unrelated and offscreen units; camera and complete Session bytes stay unchanged.");
}

#[test]
fn type_selection_uses_half_open_world_edges_at_low_and_high_zoom() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.25, 1.0, 4.0] {
            let (mut app, own, _) = scene(size);
            let buddy = spawn(&mut app, 0, (6, 4), yr_sim::OWNER_PLAYER);
            let map = app.map.as_mut().unwrap();
            // Projection-only padding keeps all tested camera positions in bounds.
            map.width = 12000;
            map.height = 8000;
            map.origin_x = -6000;
            map.origin_y = -4000;
            let (x, y) = yr_app::interaction::ground_point(
                map,
                app.session.as_ref().unwrap().sim.terrain(),
                (6.0, 4.0),
            );
            app.view.zoom = zoom;
            for bottom in [false, true] {
                app.view.cam_x = x - (size.0 - SIDEBAR_W) as f32 / zoom;
                app.view.cam_y = y - size.1 as f32 / (2.0 * zoom);
                if bottom {
                    app.view.cam_x += 10.0 / zoom;
                    app.view.cam_y = y - size.1 as f32 / zoom;
                }
                app.selected = BTreeSet::from([own]);
                let before = bytes(&app);
                press_t(&mut app);
                assert_eq!(
                    app.selected,
                    BTreeSet::from([own]),
                    "exact boundary hidden at {size:?} {zoom}"
                );
                if bottom {
                    app.view.cam_y += 1.0 / zoom;
                } else {
                    app.view.cam_x += 1.0 / zoom;
                }
                press_t(&mut app);
                assert_eq!(
                    app.selected,
                    BTreeSet::from([own, buddy]),
                    "one pixel inside at {size:?} {zoom}"
                );
                assert_eq!(bytes(&app), before);
            }
        }
    }
    println!("18 geometry cases keep anchors at the sidebar/bottom edge excluded and include them one framebuffer pixel inside, from zoom 0.25 to 4, without changing Session bytes.");
}

#[test]
fn mixed_type_selection_retains_owned_seeds_and_cancels_old_gestures_only() {
    let size = (800, 600);
    let (mut app, own, enemy) = scene(size);
    let buddy = spawn(&mut app, 0, (6, 4), yr_sim::OWNER_PLAYER);
    let other = spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
    let other_buddy = spawn(&mut app, 1, (5, 5), yr_sim::OWNER_PLAYER);
    let far = spawn(&mut app, 0, (31, 31), yr_sim::OWNER_PLAYER);
    point_at(&mut app, own, size);
    app.selected = BTreeSet::from([own, other, far, enemy, u32::MAX]);
    app.pending.push(Command::Stop { id: own });
    app.left_down = Some(app.cursor);
    app.right_down = Some(app.cursor);
    app.right_panning = true;
    app.radar_drag = true;
    app.attack_move = true;
    app.placing = Some(0);
    app.service_mode = ServiceMode::Repair;
    let before = bytes(&app);
    press_t(&mut app);
    assert_eq!(
        app.selected,
        BTreeSet::from([own, buddy, other, other_buddy, far])
    );
    assert!(
        app.left_down.is_none()
            && app.right_down.is_none()
            && !app.right_panning
            && !app.radar_drag
    );
    assert!(!app.attack_move && app.placing.is_none() && app.service_mode == ServiceMode::None);
    app.pointer_button(ElementState::Released, MouseButton::Left);
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert_eq!(app.pending, [Command::Stop { id: own }]);
    assert_eq!(bytes(&app), before);
    app.selected = BTreeSet::from([enemy, u32::MAX]);
    press_t(&mut app);
    assert!(app.selected.is_empty());
    assert_eq!(
        app.order_feedback.message(Instant::now()),
        Some("Select a unit to select its type")
    );
    assert_eq!(bytes(&app), before);
    println!("Mixed types expand together, offscreen owned seeds remain, enemy/stale IDs are pruned and old gestures cannot issue an order on release; queued Stop and Session bytes survive.");
}

#[test]
fn type_selection_respects_repeat_focus_modifiers_outcome_and_menu_text() {
    for guard in 0..9 {
        let (mut app, own, _) = scene((640, 480));
        spawn(&mut app, 0, (6, 4), yr_sim::OWNER_PLAYER);
        point_at(&mut app, own, (640, 480));
        let before = bytes(&app);
        match guard {
            0 => app.pointer.focused = false,
            1 => app.control = true,
            2 => app.shift = true,
            3 => app.super_key = true,
            4 => app.session.as_mut().unwrap().outcome = Outcome::Lost,
            5 => app.map = None,
            _ => {}
        }
        let guarded = bytes(&app);
        app.keyboard_input(
            PhysicalKey::Code(KeyCode::KeyT),
            ElementState::Pressed,
            Some("t"),
            guard == 6,
            guard == 7,
        );
        if guard == 8 {
            assert_eq!(app.selected.len(), 2);
        } else {
            assert_eq!(app.selected, BTreeSet::from([own]));
        }
        assert_eq!(bytes(&app), guarded);
        if guard != 4 {
            assert_eq!(bytes(&app), before);
        }
        assert!(app.pending.is_empty());
    }
    let (mut app, own, _) = scene((640, 480));
    let folder = PreferenceDirectory::new();
    app.save_path = folder.0.join("quicksave.oyrsave");
    app.open_menu(Page::Save);
    app.menu.name = "Name".into();
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
        Some("t"),
        false,
        false,
    );
    assert_eq!(app.menu.name, "Namet");
    assert_eq!(app.selected, BTreeSet::from([own]));
    let mut empty = new_app(String::new(), String::new(), 1);
    press_t(&mut empty);
    assert!(empty.pending.is_empty());
    println!("Repeat, synthetic/unfocused, tracked modifier, ended-match and missing-map/session guards are inert; focused T works and Save still accepts the letter t.");
}
