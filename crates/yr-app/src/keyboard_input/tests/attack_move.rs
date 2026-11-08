use super::*;

fn press(app: &mut App, repeat: bool, synthetic: bool) {
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        None,
        repeat,
        synthetic,
    );
}

#[test]
fn attack_move_key_repeat_cannot_rearm_after_an_order() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.selected.extend([7, 9]);
    press(&mut app, false, false);
    assert!(app.attack_move);
    app.radar_click((12.5, 20.5));
    assert!(!app.attack_move);
    assert_eq!(app.pending.len(), 2);
    assert!(app
        .pending
        .iter()
        .all(|c| matches!(c, Command::AttackMove { .. })));
    press(&mut app, true, false);
    assert!(
        !app.attack_move,
        "OS repeat must not arm another one-shot order"
    );
    app.radar_click((18.5, 7.5));
    assert!(app.pending[2..]
        .iter()
        .all(|c| matches!(c, Command::Move { .. })));
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Released,
        None,
        false,
        false,
    );
    press(&mut app, false, false);
    app.radar_click((5.5, 6.5));
    assert_eq!(app.pending.len(), 6);
    assert!(app.pending[4..]
        .iter()
        .all(|c| matches!(c, Command::AttackMove { .. })));
}

#[test]
fn attack_move_key_cancels_conflicting_modes_and_stale_pointer_gestures() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for mode in 0..4 {
            for gesture in 0..4 {
                let mut app = new_app(String::new(), String::new(), 1);
                app.frame_size = size;
                app.selected.insert(17);
                app.pending.push(Command::Stop { id: 17 });
                app.pointer_moved((300.0, 300.0));
                match mode {
                    1 => app.placing = Some(3),
                    2 => app.service_mode = ServiceMode::Repair,
                    3 => app.service_mode = ServiceMode::Sell,
                    _ => {}
                }
                match gesture {
                    0 => app.left_down = Some((100.0, 120.0)),
                    1 | 2 => {
                        app.right_down = Some((100.0, 120.0));
                        app.right_panning = gesture == 2;
                    }
                    _ => app.radar_drag = true,
                }
                press(&mut app, false, false);
                assert!(app.attack_move);
                assert!(app.placing.is_none());
                assert_eq!(app.service_mode, ServiceMode::None);
                assert!(app.left_down.is_none() && app.right_down.is_none());
                assert!(!app.right_panning && !app.radar_drag);
                app.pointer_button(ElementState::Released, MouseButton::Left);
                app.pointer_button(ElementState::Released, MouseButton::Right);
                assert!(app.attack_move);
                assert_eq!(app.selected, [17].into());
                assert_eq!(app.pending, [Command::Stop { id: 17 }]);
            }
        }
    }
}

#[test]
fn attack_move_key_repeat_respects_cancellation_focus_and_menu_guards() {
    for cancel in 0..2 {
        let mut app = new_app(String::new(), String::new(), 1);
        app.pointer_moved((100.0, 120.0));
        press(&mut app, false, false);
        if cancel == 0 {
            app.keyboard_input(
                PhysicalKey::Code(KeyCode::Escape),
                ElementState::Pressed,
                None,
                false,
                false,
            );
        } else {
            app.pointer_button(ElementState::Pressed, MouseButton::Right);
            app.pointer_button(ElementState::Released, MouseButton::Right);
        }
        assert!(!app.attack_move);
        press(&mut app, true, false);
        assert!(!app.attack_move);
        assert!(app.pending.is_empty());
    }
    let mut app = new_app(String::new(), String::new(), 1);
    app.placing = Some(3);
    app.left_down = Some((100.0, 120.0));
    press(&mut app, false, true);
    press(&mut app, true, false);
    assert!(!app.attack_move);
    assert_eq!(app.placing, Some(3));
    assert!(app.left_down.is_some());
    app.focus_changed(false);
    press(&mut app, false, false);
    assert!(!app.attack_move);
    app.focus_changed(true);
    app.menu.set(Page::Save);
    app.menu.name = "checkpoint".into();
    app.control = true;
    press(&mut app, false, false);
    assert!(app.menu.name_selected);
    assert_eq!(app.menu.name, "checkpoint");
    assert!(!app.attack_move);
}

#[test]
fn attack_move_key_removes_the_old_rubber_band_before_stale_release() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.frame_size = (640, 480);
    app.selected.extend([7, 9]);
    app.pending.push(Command::Stop { id: 7 });
    app.map = Some(yr_render::RenderedMap {
        width: 640,
        height: 480,
        rgba: [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(640 * 480 * 4)
            .collect(),
        summary: "Synthetic gesture canvas".into(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    });
    app.pointer_moved((100.0, 120.0));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_moved((300.0, 300.0));
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    assert!(frame.as_chunks::<4>().0.contains(&SELECT_COL));
    capture(&app, "00-old-box-drag", &frame);
    press(&mut app, false, false);
    app.compose(&mut frame, 640, 480);
    assert!(!frame.as_chunks::<4>().0.contains(&SELECT_COL));
    capture(&app, "01-attack-move-armed", &frame);
    let armed = frame.clone();
    app.pointer_button(ElementState::Released, MouseButton::Left);
    app.compose(&mut frame, 640, 480);
    assert_eq!(frame, armed);
    capture(&app, "02-stale-release", &frame);
    assert!(app.attack_move);
    assert_eq!(app.selected, [7, 9].into());
    assert_eq!(app.pending, [Command::Stop { id: 7 }]);
    app.radar_click((12.5, 20.5));
    assert!(!app.attack_move);
    assert!(app.pending[1..]
        .iter()
        .all(|c| matches!(c, Command::AttackMove { .. })));
    press(&mut app, true, false);
    assert!(!app.attack_move);
    app.radar_click((18.5, 7.5));
    assert!(app.pending[3..]
        .iter()
        .all(|c| matches!(c, Command::Move { .. })));
    app.compose(&mut frame, 640, 480);
    capture(&app, "03-repeat-stays-disarmed", &frame);
}

fn capture(app: &App, name: &str, frame: &[u8]) {
    let Some(dir) = std::env::var_os("OPENYR_ATTACK_MOVE_KEY_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(640, 480, frame),
    )
    .unwrap();
    std::fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
        "attack_move":app.attack_move,"placing":app.placing,"selected":app.selected,
        "left_down":app.left_down,"right_down":app.right_down,
        "pending":format!("{:?}",app.pending),"session_present":app.session.is_some(),"retail_art":false,
    })).unwrap()).unwrap();
}
