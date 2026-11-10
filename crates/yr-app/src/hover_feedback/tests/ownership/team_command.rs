use super::*;

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];
fn tap(app: &mut App, code: KeyCode) {
    key(app, code, ElementState::Pressed);
    key(app, code, ElementState::Released);
}
fn saved(app: &App) -> Vec<u8> {
    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap()
}
fn assign(app: &mut App, code: KeyCode) {
    app.control = true;
    tap(app, code);
    app.control = false;
}

#[test]
fn team_command_chords_preserve_selection_modes_gestures_and_membership() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for code in DIGITS {
            for chord in 0..4 {
                let (mut app, own, _) = scene(size);
                let buddy = type_selection::spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
                assign(&mut app, code);
                app.selected = BTreeSet::from([buddy]);
                app.pending.push(Command::Stop { id: own });
                app.attack_move = true;
                app.placing = Some(0);
                app.service_mode = ServiceMode::Repair;
                app.left_down = Some((100.0, 100.0));
                app.right_down = Some((101.0, 101.0));
                app.right_panning = true;
                app.radar_drag = true;
                app.camera.edge_armed = true;
                let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
                let before = saved(&app);
                app.super_key = true;
                app.control = chord & 1 != 0;
                app.shift = chord & 2 != 0;
                tap(&mut app, code);
                assert_eq!(
                    app.selected,
                    BTreeSet::from([buddy]),
                    "{size:?} {code:?} chord {chord}"
                );
                assert!(
                    app.attack_move
                        && app.placing == Some(0)
                        && app.service_mode == ServiceMode::Repair
                );
                assert_eq!(app.left_down, Some((100.0, 100.0)));
                assert_eq!(app.right_down, Some((101.0, 101.0)));
                assert!(app.right_panning && app.radar_drag && app.camera.edge_armed);
                assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
                assert_eq!(app.pending, [Command::Stop { id: own }]);
                assert_eq!(saved(&app), before);
                app.super_key = false;
                app.control = false;
                app.shift = false;
                app.selected.clear();
                tap(&mut app, code);
                assert_eq!(
                    app.selected,
                    BTreeSet::from([own]),
                    "rejected chord must not replace assigned team"
                );
                assert_eq!(saved(&app), before);
            }
        }
    }
    println!("108 actual Command(+Control/Shift)+number cases preserve selection, assigned teams, modes, gestures, camera, pending Stop and encoded Session bytes.");
}

#[test]
fn team_command_control_cannot_clear_a_team_with_empty_selection() {
    for code in DIGITS {
        let (mut app, own, _) = scene((640, 480));
        assign(&mut app, code);
        app.selected.clear();
        let before = saved(&app);
        app.super_key = true;
        app.control = true;
        tap(&mut app, code);
        app.super_key = false;
        app.control = false;
        tap(&mut app, code);
        assert_eq!(app.selected, BTreeSet::from([own]));
        assert_eq!(saved(&app), before);
    }
    println!(
        "Nine Command+Control+number cases cannot clear stored teams from an empty selection."
    );
}

#[test]
fn team_command_chords_cannot_center_or_bridge_a_plain_recall_pair() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, own, _) = scene(size);
        point_at(&mut app, own, size);
        app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(1600 * 1200 * 4)
            .collect();
        assign(&mut app, KeyCode::Digit1);
        let now = Instant::now();
        assert!(app.group_key(KeyCode::Digit1, ElementState::Pressed, false, now));
        app.view.cam_x += 80.0;
        app.view.cam_y += 30.0;
        let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
        let before = saved(&app);
        proof(&app, &format!("{}-before", size.0));
        app.super_key = true;
        assert!(app.group_key(
            KeyCode::Digit1,
            ElementState::Pressed,
            false,
            now + Duration::from_millis(50)
        ));
        proof(&app, &format!("{}-after", size.0));
        assert_eq!(
            (app.view.cam_x, app.view.cam_y, app.view.zoom),
            camera,
            "Command+1 cannot act as the second recall"
        );
        assert!(app.groups.message(Instant::now()).is_none());
        app.super_key = false;
        assert!(app.group_key(
            KeyCode::Digit1,
            ElementState::Pressed,
            false,
            now + Duration::from_millis(100)
        ));
        assert_eq!(
            (app.view.cam_x, app.view.cam_y, app.view.zoom),
            camera,
            "rejected fresh chord interrupts old recall timing"
        );
        assert_eq!(app.selected, BTreeSet::from([own]));
        assert_eq!(saved(&app), before);
    }
    println!("Three native compositor cases reproduce the old unintended centering; fresh rejected chords also break an old plain-recall pair without changing Session bytes.");
}

#[test]
fn team_command_guards_keep_classic_shortcuts_and_menu_input_working() {
    for code in DIGITS {
        let (mut app, own, _) = scene((640, 480));
        let buddy = type_selection::spawn(&mut app, 1, (5, 4), yr_sim::OWNER_PLAYER);
        assign(&mut app, code);
        app.selected = BTreeSet::from([buddy]);
        app.shift = true;
        tap(&mut app, code);
        assert_eq!(app.selected, BTreeSet::from([own, buddy]));
        app.shift = false;
        tap(&mut app, code);
        assert_eq!(app.selected, BTreeSet::from([own]));
        app.super_key = true;
        app.keyboard_input(
            PhysicalKey::Code(code),
            ElementState::Pressed,
            None,
            true,
            false,
        );
        assert!(
            app.groups.message(Instant::now()).is_some(),
            "ignored repeat retains prior notice"
        );
        app.keyboard_input(
            PhysicalKey::Code(code),
            ElementState::Pressed,
            None,
            false,
            true,
        );
        assert!(
            app.groups.message(Instant::now()).is_some(),
            "synthetic key is not a new interruption"
        );
        app.super_key = false;
        let at = Instant::now() + Duration::from_secs(1);
        assert!(app.group_key(code, ElementState::Pressed, false, at));
        let camera = (app.view.cam_x, app.view.cam_y);
        assert!(app.group_key(
            code,
            ElementState::Pressed,
            false,
            at + Duration::from_millis(50)
        ));
        assert_ne!(
            (app.view.cam_x, app.view.cam_y),
            camera,
            "plain double recall still centers"
        );
        app.menu.set(Page::Save);
        app.menu.name.clear();
        app.keyboard_input(
            PhysicalKey::Code(code),
            ElementState::Pressed,
            Some("1"),
            false,
            false,
        );
        assert_eq!(app.menu.name, "1");
    }
    println!("All nine slots retain Ctrl assignment, Shift additive recall, plain/double recall, ignored-repeat/synthetic guards and menu number text.");
}

fn proof(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_TEAM_COMMAND_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let size = app.camera_size();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose_at(&mut frame, size.0, size.1, Instant::now());
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
        "camera":[app.view.cam_x,app.view.cam_y,app.view.zoom],"selected":app.selected,
        "pending":format!("{:?}",app.pending),"tick":session.sim.tick(),
        "sim_hash":format!("{:016x}",session.sim.state_hash()),"snapshot_bytes":saved(app).len(),
        "group_notice":app.groups.message(Instant::now()),"retail_art":false
    })).unwrap()).unwrap();
}
