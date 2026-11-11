use super::*;
use yr_app::menu::Page;
use yr_assets::AssetFs;
use yr_formats::{IniFile, MapFile};

// All bytes below are authored fixtures: one-pixel uncompressed SHPs and one
// clear TMP diamond. Public setup builds the normal catalogs, terrain, services,
// initial bases and snapshot-capable Session without any retail files.
fn session() -> Session {
    session_with_map_objects("")
}

fn session_with_map_structures(structures: &str) -> Session {
    session_with_map_objects(&format!("[Structures]\n{structures}"))
}

fn session_with_map_objects(objects: &str) -> Session {
    let buildings = ["GACNST", "GAPOWR", "GAREFN", "GAWEAP", "GAPILE"];
    let vehicles = ["AMCV", "CMIN", "MTNK"];
    let mut text = "[MultiplayerDialogSettings]\nUnitCount=0\n[General]\nRepairRate=0.01\nRepairPercent=0.2\nRepairStep=5\nRefundPercent=0.5\nBuildSpeed=0.001\n[BuildingTypes]\n".to_string();
    for (i, id) in buildings.iter().enumerate() {
        text.push_str(&format!("{i}={id}\n"));
    }
    text.push_str("[VehicleTypes]\n");
    for (i, id) in vehicles.iter().enumerate() {
        text.push_str(&format!("{i}={id}\n"));
    }
    text.push_str("[InfantryTypes]\n0=E1\n");
    text.push_str("[AircraftTypes]\n0=ORCA\n");
    let mut art = String::new();
    let mut files = vec![(
        "TEMPERATMD.INI".into(),
        b"[TileSet0000]\nFileName=Clear\nTilesInSet=1\n".to_vec(),
    )];
    for id in buildings
        .iter()
        .chain(vehicles.iter())
        .chain(["E1", "ORCA"].iter())
    {
        text.push_str(&format!(
            "[{id}]\nOwner=Americans\nStrength=100\nCost=100\nSpeed=4\nTechLevel=1\n"
        ));
        if *id == "GACNST" {
            text.push_str("Factory=BuildingType\nConstructionYard=yes\n");
        }
        if *id == "CMIN" {
            text.push_str("Harvester=yes\n");
        }
        if buildings.contains(id) {
            text.push_str("Prerequisite=GACNST\n");
        }
        art.push_str(&format!("[{id}]\nFoundation=1x1\n"));
        // One 1x1 frame, uncompressed, with its pixel payload at byte 32.
        let mut shp = Vec::new();
        for v in [0u16, 1, 1, 1, 0, 0, 1, 1] {
            shp.extend(v.to_le_bytes());
        }
        shp.extend(1u32.to_le_bytes());
        shp.extend([0; 8]);
        shp.extend(32u32.to_le_bytes());
        shp.push(1);
        assert!(yr_formats::ShpFile::parse(&shp).is_ok());
        files.push((format!("{id}.SHP"), shp));
    }
    files.push(("RULESMD.INI".into(), text.as_bytes().to_vec()));
    // One 8x4 diamond, type 13 (clear), with sixteen authored pixels.
    let mut tmp = Vec::new();
    for v in [1u32, 1, 8, 4, 20] {
        tmp.extend(v.to_le_bytes());
    }
    tmp.extend([0; 52]);
    tmp[61] = 13;
    tmp.extend([1; 16]);
    files.push(("CLEAR01.TEM".into(), tmp));
    let mut fs = AssetFs::from_files(files);
    let rules = yr_rules::Rules::load(&IniFile::parse(&text), &IniFile::parse(&art));
    let mut map = MapFile::parse(&format!("[Map]\nTheater=TEMPERATE\nSize=0,0,32,32\nLocalSize=0,0,32,32\n[IsoMapPack5]\n1=CAAEABUAAAAAEQAA\n{objects}")).unwrap();
    map.tiles = (0..32)
        .flat_map(|y| {
            (0..32).map(move |x| yr_formats::map::IsoTile {
                x,
                y,
                tile: Some(0),
                subtile: 0,
                level: 0,
            })
        })
        .collect();
    yr_app::session::setup(&mut fs, &map, &rules, &[], 42).unwrap()
}

fn scene(size: (u32, u32)) -> (App, u32, u32) {
    let mut app = new_app(String::new(), String::new(), 1);
    app.frame_size = size;
    app.session = Some(session());
    app.map = Some(yr_render::RenderedMap {
        width: 1600,
        height: 1200,
        rgba: Vec::new(),
        summary: "Synthetic ownership canvas".into(),
        origin_x: -600,
        origin_y: -200,
        scale: 1,
    });
    let sim = &mut app.session.as_mut().unwrap().sim;
    let own = sim.next_id_peek();
    sim.step(&[
        Command::Spawn {
            kind: 0,
            x: Fix::from_int(4),
            y: Fix::from_int(4),
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::ONE,
            hp: 100,
            armor: 0,
            weapon: Some(yr_sim::WeaponSpec {
                source: None,
                damage: 10,
                rof_ticks: 10,
                range: Fix::ONE,
                projectile_speed: Fix::ZERO,
                verses: [yr_rules::Decimal::ONE; 11],
            }),
        },
        Command::AssignOwner {
            id: own,
            owner: yr_sim::OWNER_PLAYER,
        },
    ]);
    let enemy = sim.next_id_peek();
    sim.step(&[
        Command::Spawn {
            kind: 0,
            x: Fix::from_int(8),
            y: Fix::from_int(4),
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::ONE,
            hp: 100,
            armor: 0,
            weapon: None,
        },
        Command::AssignOwner {
            id: enemy,
            owner: yr_sim::OWNER_ENEMY,
        },
    ]);
    app.selected.insert(own);
    (app, own, enemy)
}

#[test]
fn world_feedback_ownership_matches_ignored_keyboard_pan_clicks() {
    let size = (640, 480);
    let (mut app, own, enemy) = scene(size);
    app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
        .into_iter()
        .cycle()
        .take(1600 * 1200 * 4)
        .collect();
    point_at(&mut app, enemy, size);
    assert_eq!(app.cursor_kind(size), Kind::Attack);
    assert!(app.hover_target(size).is_some());
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    capture(&app, "00-actionable-enemy");
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::ArrowRight),
        ElementState::Pressed,
        None,
        false,
        false,
    );
    capture(&app, "01-held-arrow");
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_button(ElementState::Released, MouseButton::Left);
    capture(&app, "02-ignored-click");
    assert_eq!(app.selected, [own].into());
    assert!(app.pending.is_empty(), "held arrow rejects tactical clicks");
    assert_eq!(
        app.cursor_kind(size),
        Kind::Pointer,
        "ignored click cannot advertise Attack"
    );
    assert!(app.hover_target(size).is_none());
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    key(&mut app, KeyCode::ArrowRight, ElementState::Released);
    assert_eq!(app.cursor_kind(size), Kind::Attack);
    assert!(app.hover_target(size).is_some());
    capture(&app, "03-key-up");
    click(&mut app);
    assert_eq!(
        app.pending,
        [Command::Attack {
            id: own,
            target: enemy
        }]
    );
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    capture(&app, "04-fresh-attack-intent");
}
fn key(app: &mut App, code: KeyCode, state: ElementState) {
    app.keyboard_input(PhysicalKey::Code(code), state, None, false, false);
}

fn click(app: &mut App) {
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_button(ElementState::Released, MouseButton::Left);
}

fn point_cell(app: &mut App, cell: (f32, f32), size: (u32, u32)) {
    app.center_world_at(cell, (size.0 - SIDEBAR_W, size.1));
    app.pointer_moved((f64::from(size.0 - SIDEBAR_W) / 2.0, f64::from(size.1) / 2.0));
}

#[test]
fn world_feedback_ownership_covers_real_actions_modes_and_saved_session() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for arrow in [
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ] {
            for mode in 0..9 {
                let (mut app, own, enemy) = scene(size);
                let building = app
                    .session
                    .as_ref()
                    .unwrap()
                    .sim
                    .buildings()
                    .iter()
                    .find(|b| b.owner == yr_sim::OWNER_PLAYER)
                    .unwrap()
                    .id;
                let expected = match mode {
                    0 => {
                        point_at(&mut app, enemy, size);
                        Kind::Attack
                    }
                    1 => {
                        app.selected.clear();
                        point_at(&mut app, own, size);
                        Kind::Select
                    }
                    2 => {
                        point_cell(&mut app, (12.0, 12.0), size);
                        Kind::Move
                    }
                    3 => {
                        app.attack_move = true;
                        point_cell(&mut app, (12.0, 12.0), size);
                        Kind::AttackMove
                    }
                    4 => {
                        app.service_mode = ServiceMode::Sell;
                        point_at(&mut app, building, size);
                        Kind::Sell
                    }
                    5 => {
                        app.service_mode = ServiceMode::Repair;
                        point_at(&mut app, building, size);
                        Kind::NoRepair
                    }
                    6 => {
                        let s = app.session.as_mut().unwrap();
                        let slot = s
                            .buildables
                            .iter()
                            .position(|b| b.footprint().is_some())
                            .unwrap();
                        let produce = s.produce(slot).unwrap();
                        s.sim.step(&[produce]);
                        for _ in 0..10 {
                            if s.ready_structure().is_some() {
                                break;
                            }
                            s.sim.step(&[]);
                        }
                        assert_eq!(s.ready_structure(), Some(slot));
                        app.placing = Some(slot);
                        point_at(&mut app, building, size);
                        Kind::Blocked // Occupied structure cell is not placeable.
                    }
                    7 => {
                        let sim = &mut app.session.as_mut().unwrap().sim;
                        let mut terrain = sim.terrain().unwrap().clone();
                        terrain.set_resource(12, 12, yr_sim::terrain::Overlay::Ore, 20);
                        sim.set_terrain(terrain);
                        sim.step(&[Command::AssignRole {
                            id: own,
                            role: yr_sim::Role::Harvester,
                        }]);
                        point_cell(&mut app, (12.0, 12.0), size);
                        Kind::Harvest
                    }
                    8 => {
                        app.shift = true;
                        point_at(&mut app, own, size);
                        Kind::Select
                    }
                    _ => unreachable!(),
                };
                assert_eq!(app.cursor_kind(size), expected, "baseline mode {mode}");
                let hover = app.hover_target(size);
                let selection = app.selected.clone();
                let modes = (app.service_mode, app.placing, app.attack_move);
                app.pending.push(Command::Stop { id: own });
                let pending = app.pending.clone();
                let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                let hash = app.session.as_ref().unwrap().sim.state_hash();
                key(&mut app, arrow, ElementState::Pressed);
                assert_eq!(
                    app.cursor_kind(size),
                    Kind::Pointer,
                    "held {arrow:?} mode {mode}"
                );
                assert!(app.hover_target(size).is_none());
                click(&mut app);
                app.pointer_button(ElementState::Pressed, MouseButton::Right);
                app.pointer_button(ElementState::Released, MouseButton::Right);
                assert_eq!(app.selected, selection);
                assert_eq!((app.service_mode, app.placing, app.attack_move), modes);
                assert_eq!(app.pending, pending);
                assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    saved
                );
                key(&mut app, arrow, ElementState::Released);
                assert_eq!(app.cursor_kind(size), expected);
                assert_eq!(app.hover_target(size), hover);
                let action = app.world_action(app.cursor).unwrap();
                click(&mut app);
                if mode == 1 {
                    assert_eq!(app.selected, [own].into());
                }
                if mode == 8 {
                    assert!(
                        app.selected.is_empty(),
                        "fresh Shift-click toggles selection"
                    );
                }
                if matches!(mode, 0 | 2 | 3 | 4 | 7) {
                    assert!(!action.commands.is_empty());
                    assert_eq!(&app.pending[pending.len()..], action.commands.as_slice());
                }
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    saved,
                    "fresh clicks queue intents without advancing the session"
                );
            }
        }
    }
}

#[test]
fn world_feedback_ownership_handles_net_zero_right_press_and_existing_routes() {
    let size = (640, 480);
    for (a, b) in [
        (KeyCode::ArrowLeft, KeyCode::ArrowRight),
        (KeyCode::ArrowUp, KeyCode::ArrowDown),
    ] {
        let (mut app, own, enemy) = scene(size);
        point_at(&mut app, enemy, size);
        key(&mut app, a, ElementState::Pressed);
        key(&mut app, b, ElementState::Pressed);
        assert_eq!(app.cursor_kind(size), Kind::Attack);
        assert!(app.hover_target(size).is_some());
        click(&mut app);
        assert!(
            matches!(app.pending.as_slice(), [Command::Attack {id, target}] if *id == own && *target == enemy)
        );
        key(&mut app, b, ElementState::Released);
        assert_eq!(app.cursor_kind(size), Kind::Pointer);
        assert!(app.hover_target(size).is_none());
        key(&mut app, a, ElementState::Released);
        let pending = app.pending.clone();
        app.pointer_button(ElementState::Pressed, MouseButton::Right);
        assert!(app.right_down.is_some() && !app.right_panning);
        assert_eq!(app.cursor_kind(size), Kind::Pointer);
        assert!(app.hover_target(size).is_none());
        click(&mut app);
        assert_eq!(app.selected, [own].into());
        assert_eq!(app.pending, pending);
        app.pointer_button(ElementState::Released, MouseButton::Right);
        assert!(
            app.selected.is_empty(),
            "right release retains its cancellation policy"
        );
        app.selected.insert(own);
        assert_eq!(app.cursor_kind(size), Kind::Attack);
        assert!(app.hover_target(size).is_some());
        key(&mut app, a, ElementState::Pressed);
        app.service_mode = ServiceMode::Repair;
        app.pointer_moved((639.0, 200.0));
        assert_eq!(app.cursor_kind(size), Kind::Pointer);
        app.pointer_button(ElementState::Pressed, MouseButton::Right);
        assert_eq!(app.service_mode, ServiceMode::None);
        app.open_menu(Page::Pause);
        key(&mut app, KeyCode::ArrowDown, ElementState::Pressed);
        assert!(app.keys.is_empty());
        assert_eq!(app.menu.focus, 1);
    }
}

// Actual shared compositor and native handler state; no cursor sheet is supplied.
// The JSON reports Kind rather than claiming synthetic pixels are retail art.
fn capture(app: &App, name: &str) {
    let Some(directory) = std::env::var_os("OPENYR_WORLD_FEEDBACK_PROOF") else {
        return;
    };
    let directory = PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let size = app.camera_size();
    let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut frame, size.0, size.1);
    std::fs::write(
        directory.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "cursor_kind": format!("{:?}", app.cursor_kind(size)),
            "hover_target": app.hover_target(size).map(|target| target.id),
            "selected": app.selected, "pending": format!("{:?}", app.pending),
            "keys": format!("{:?}", app.keys), "keyboard_pan": app.keyboard_pan(),
            "session_present": true, "tick": session.sim.tick(),
            "sim_hash": format!("{:016x}", session.sim.state_hash()),
            "snapshot_bytes": yr_app::snapshot::encode(session).unwrap().len(),
            "retail_art": false, "redraw_handler_called": false,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn world_feedback_ownership_keeps_committed_targets_and_clears_on_focus_loss() {
    let size = (640, 480);
    let (mut app, own, enemy) = scene(size);
    app.session.as_mut().unwrap().sim.step(&[Command::Attack {
        id: own,
        target: enemy,
    }]);
    point_at(&mut app, enemy, size);
    let targets = app.current_attack_targets();
    assert_eq!(targets, [enemy].into());
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    let cursor = app.cursor;
    app.pointer_moved((cursor.0 + 4.0, cursor.1));
    assert!(app.right_panning);
    assert_eq!(app.cursor_kind(size), Kind::Pointer);
    assert!(app.hover_target(size).is_none());
    assert_eq!(app.current_attack_targets(), targets);
    app.pointer_button(ElementState::Released, MouseButton::Right);
    point_at(&mut app, enemy, size);
    key(&mut app, KeyCode::ArrowUp, ElementState::Pressed);
    assert!(app.hover_target(size).is_none());
    assert_eq!(app.current_attack_targets(), targets);
    app.focus_changed(false);
    assert!(app.keys.is_empty());
    assert!(app.hover_target(size).is_none());
    app.focus_changed(true);
    point_at(&mut app, enemy, size);
    assert_eq!(app.cursor_kind(size), Kind::Attack);
    assert!(app.hover_target(size).is_some());
    assert_eq!(app.current_attack_targets(), targets);
    assert!(app.pending.is_empty());
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
}

struct PreferenceDirectory(PathBuf);
impl PreferenceDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "openyr-preferences-modal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for PreferenceDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn press_key(app: &mut App, code: KeyCode) {
    key(app, code, ElementState::Pressed);
    key(app, code, ElementState::Released);
}

#[test]
fn preferences_error_modal_cancels_world_modes_before_recovery() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for mode in [KeyCode::KeyA, KeyCode::KeyK, KeyCode::KeyL] {
            let (mut app, own, enemy) = scene(size);
            let directory = PreferenceDirectory::new();
            let destination = directory.0.join("interface.json");
            // A directory cannot be replaced by the writer's regular file.
            std::fs::create_dir(&destination).unwrap();
            std::fs::write(destination.join("keep"), b"protected destination").unwrap();
            app.preferences_path = Some(destination.clone());
            app.control = true;
            press_key(&mut app, KeyCode::Digit1);
            app.control = false;
            press_key(&mut app, mode);
            point_at(&mut app, enemy, size);
            if mode == KeyCode::KeyA {
                app.pointer_button(ElementState::Pressed, MouseButton::Left);
                assert!(app.left_down.is_some() && app.attack_move);
            } else {
                assert_ne!(app.service_mode, ServiceMode::None);
            }
            app.pending.push(Command::Stop { id: own });
            let pending = app.pending.clone();
            let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            let proof = size == (640, 480) && mode == KeyCode::KeyA;
            app.shift = true;
            app.control = true;
            app.super_key = true;
            if proof {
                preferences_modal_capture(&mut app, "00-armed-click");
            }
            key(&mut app, KeyCode::KeyM, ElementState::Pressed);
            assert_eq!(app.menu.page, Page::Closed, "Command+M is not mute");
            assert_eq!(app.pending, pending);
            if mode == KeyCode::KeyA {
                assert!(app.left_down.is_some() && app.attack_move);
            } else {
                assert_ne!(app.service_mode, ServiceMode::None);
            }
            app.super_key = false;
            app.keyboard_input(
                PhysicalKey::Code(KeyCode::KeyM),
                ElementState::Pressed,
                None,
                true,
                false,
            );
            assert_eq!(app.menu.page, Page::Closed, "old repeat remains rejected");
            key(&mut app, KeyCode::KeyM, ElementState::Released);
            // Fresh M still reaches the real writer and modal recovery path.
            key(&mut app, KeyCode::KeyM, ElementState::Pressed);
            assert!(
                matches!(app.menu.page, Page::Message { ref back, .. } if **back == Page::Controls)
            );
            assert!(app.keys.is_empty());
            assert!(!app.shift && !app.control && !app.super_key);
            assert!(app.left_down.is_none() && app.right_down.is_none());
            assert!(!app.attack_move && app.placing.is_none());
            assert_eq!(app.service_mode, ServiceMode::None);
            assert!(app.order_feedback.message(Instant::now()).is_none());
            assert_eq!(app.pending, pending);
            assert_eq!(
                std::fs::read(destination.join("keep")).unwrap(),
                b"protected destination"
            );
            assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
            if proof {
                preferences_modal_capture(&mut app, "01-error-modal");
            }
            key(&mut app, KeyCode::KeyM, ElementState::Released);
            assert!(!app.advance_clock(Instant::now() + Duration::from_secs(3600)));
            for page in [Page::Controls, Page::Pause, Page::Closed] {
                press_key(&mut app, KeyCode::Escape);
                assert_eq!(app.menu.page, page);
            }
            app.pointer_button(ElementState::Released, MouseButton::Left);
            assert_eq!(app.pending, pending, "stale release adds no command");
            if proof {
                preferences_modal_capture(&mut app, "02-resumed");
            }
            point_cell(&mut app, (8.0, 8.0), size);
            click(&mut app);
            assert_eq!(
                app.pending,
                [
                    Command::Stop { id: own },
                    Command::Move {
                        id: own,
                        x: Fix::from_raw(2176),
                        y: Fix::from_raw(2176)
                    }
                ],
                "fresh click cannot reuse pre-dialog modes"
            );
            assert_eq!(app.selected, [own].into());
            if proof {
                preferences_modal_capture(&mut app, "03-fresh-move");
            }
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                saved
            );
            // Modal entry interrupts recall decoration, not assigned team membership.
            app.selected.clear();
            press_key(&mut app, KeyCode::Digit1);
            assert_eq!(app.selected, [own].into());
        }
    }
    println!("Nine native mode/layout failures preserve encoded Sessions, queued orders, selection and team membership; modal recovery cancels modes and stale releases, and fresh clicks queue Move.");
}

#[test]
fn preferences_error_modal_keeps_successful_live_mute_nonmodal() {
    let (mut app, own, enemy) = scene((640, 480));
    let directory = PreferenceDirectory::new();
    let destination = directory.0.join("interface.json");
    app.preferences_path = Some(destination.clone());
    press_key(&mut app, KeyCode::KeyA);
    point_at(&mut app, enemy, (640, 480));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    press_key(&mut app, KeyCode::KeyM);
    assert_eq!(app.menu.page, Page::Closed);
    assert!(app.menu.muted && app.attack_move && app.left_down.is_some());
    assert!(std::fs::read_to_string(destination)
        .unwrap()
        .contains("\"muted\":true"));
    assert_eq!(app.selected, [own].into());
    assert!(app.pending.is_empty());
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    println!("Successful live M remains nonmodal and preserves armed input and Session bytes.");
}

fn preferences_modal_capture(app: &mut App, name: &str) {
    let Some(path) = std::env::var_os("OPENYR_PREFERENCES_MODAL_PROOF") else {
        return;
    };
    let path = PathBuf::from(path);
    std::fs::create_dir_all(&path).unwrap();
    if app.map.as_ref().is_some_and(|m| m.rgba.is_empty()) {
        app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(1600 * 1200 * 4)
            .collect();
    }
    let (w, h) = app.frame_size;
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let mut frame = vec![0; (w * h * 4) as usize];
    app.compose(&mut frame, w, h);
    std::fs::write(
        path.join(format!("{name}.png")),
        yr_render::png::encode_rgba(w, h, &frame),
    )
    .unwrap();
    std::fs::write(
        path.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "page":format!("{:?}",app.menu.page), "attack_move":app.attack_move,
            "service_mode":format!("{:?}",app.service_mode), "left_down":app.left_down,
            "held_keys":format!("{:?}",app.keys), "pending":format!("{:?}",app.pending),
            "selected":app.selected, "tick":app.session.as_ref().unwrap().sim.tick(),
            "sim_hash":format!("{:016x}",app.session.as_ref().unwrap().sim.state_hash()),
            "snapshot_bytes":saved.len(), "retail_art":false
        }))
        .unwrap(),
    )
    .unwrap();
}

mod placement;

mod new_match_saves;
mod save_catalog;

mod unit_status;

mod modal_repeat;

mod type_selection;

mod base_navigation;

mod type_interruption;

mod sidebar_keys;

mod all_selection;

mod team_command;

mod team_centering;

mod command_world;

mod next_unit;

mod hover_health;

mod unit_follow;

mod building_idle;
mod map_health;
mod map_unit_facing;
mod map_unit_health;
mod map_unit_lifetime;
mod menu_click;

mod sidebar_press;

mod sidebar_ownership;

mod sidebar_overrides;
