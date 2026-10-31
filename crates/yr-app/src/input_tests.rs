use super::*;

#[cfg(test)]
mod focus_input_tests {
    use super::*;

    #[test]
    fn focus_loss_clears_held_input_modes_and_pointer_ownership_but_keeps_selection() {
        let mut app = new_app(String::new(), String::new(), 2);
        app.keys.insert(KeyCode::ArrowLeft);
        app.shift = true;
        app.control = true;
        app.left_down = Some((1.0, 2.0));
        app.right_down = Some((3.0, 4.0));
        app.right_panning = true;
        app.radar_drag = true;
        app.attack_move = true;
        app.placing = Some(3);
        app.service_mode = ServiceMode::Repair;
        app.selected.insert(123);
        app.pointer.inside = true;
        app.focus_changed(false);
        assert!(!app.pointer.focused && app.keys.is_empty() && !app.shift && !app.control);
        assert!(
            app.left_down.is_none()
                && app.right_down.is_none()
                && !app.right_panning
                && !app.radar_drag
        );
        assert!(app.placing.is_none() && !app.attack_move && app.service_mode == ServiceMode::None);
        assert!(app.selected.contains(&123));
        app.focus_changed(true);
        assert!(app.pointer.focused && app.keys.is_empty());
    }

    #[test]
    fn right_click_cancels_modes_before_selection_and_cancels_gestures_without_orders() {
        let mut app = new_app(String::new(), String::new(), 2);
        app.selected.insert(1);
        app.service_mode = ServiceMode::Sell;
        app.cancel_mode_or_selection();
        assert_eq!(app.selected.len(), 1);
        assert_eq!(app.service_mode, ServiceMode::None);
        app.cancel_mode_or_selection();
        assert!(app.selected.is_empty());
        app.left_down = Some((1.0, 2.0));
        app.right_down = Some((3.0, 4.0));
        app.right_panning = true;
        app.cancel_pointer_action();
        assert!(
            app.left_down.is_none()
                && app.right_down.is_none()
                && !app.right_panning
                && app.pending.is_empty()
        );
    }
}

#[cfg(test)]
mod retail_pointer_tests {
    use super::*;
    use yr_app::cursor::Kind;

    fn point(app: &App, cell: (i32, i32)) -> (f64, f64) {
        let map = app.map.as_ref().unwrap();
        let session = app.session.as_ref().unwrap();
        let (x, mut y) = map.cell_to_px(cell.0 as f32 + 0.5, cell.1 as f32 + 0.5);
        let level = session
            .sim
            .terrain()
            .and_then(|g| g.cell(cell.0 as u16, cell.1 as u16))
            .map_or(0, |c| c.height);
        y -= f32::from(level) * 15.0 / map.scale as f32;
        (
            f64::from((x - app.view.cam_x) * app.view.zoom),
            f64::from((y - app.view.cam_y) * app.view.zoom),
        )
    }

    fn centre(app: &mut App, cell: (i32, i32)) {
        let (x, y) = app
            .map
            .as_ref()
            .unwrap()
            .cell_to_px(cell.0 as f32 + 0.5, cell.1 as f32 + 0.5);
        app.view.cam_x = x - (WIN_W - SIDEBAR_W) as f32 / (2.0 * app.view.zoom);
        app.view.cam_y = y - WIN_H as f32 / (2.0 * app.view.zoom);
        app.cursor = point(app, cell);
    }

    fn proof(app: &App, name: &str) {
        if let Some(out) = std::env::var_os("OPENYR_CURSOR_PROOF") {
            let out = PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
            app.compose(&mut frame, WIN_W, WIN_H);
            std::fs::write(
                out.join(format!("native-{name}.png")),
                yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
            )
            .unwrap();
        }
    }

    #[test]
    fn retail_left_orders_right_cancels_drag_does_not_order_and_service_cursors_share_validation() {
        let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            eprintln!("skip: retail install not present");
            return;
        }
        let mut app = new_app(dir.to_string_lossy().into(), "EB1.MMX".into(), 2);
        app.session = Some(app.load_session().unwrap());
        app.session.as_mut().unwrap().sim.set_ai(None);
        app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, app.scale).unwrap());
        app.sprites = load_sprites(&app.gamedir, &app.map_name);
        app.pointer.inside = true;
        let unit = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some())
            .unwrap()
            .clone();
        let cell = (unit.x.to_int_floor(), unit.y.to_int_floor());
        centre(&mut app, cell);
        // Exercise the actual picking and box handler on a lifted object.
        let original_grid = app.session.as_ref().unwrap().sim.terrain().unwrap().clone();
        let mut lifted = original_grid.clone();
        lifted
            .cell_mut(cell.0 as u16, cell.1 as u16)
            .unwrap()
            .height = 4;
        app.session.as_mut().unwrap().sim.set_terrain(lifted);
        app.cursor = point(&app, cell);
        assert_eq!(app.world_action(app.cursor).unwrap().select, Some(unit.id));
        let p = app.cursor;
        app.select_box((p.0 - 3.0, p.1 - 3.0), (p.0 + 3.0, p.1 + 3.0), false);
        assert!(
            app.selected.contains(&unit.id),
            "box selection uses the drawn height lift"
        );
        app.selected.clear();
        app.session.as_mut().unwrap().sim.set_terrain(original_grid);
        centre(&mut app, cell);
        assert_eq!(app.cursor_kind(WIN_W), Kind::Select);
        proof(&app, "select-tick-1");
        app.left_down = Some(app.cursor);
        app.left_release();
        assert_eq!(app.selected, [unit.id].into());
        assert!(app.pending.is_empty());
        app.shift = true;
        app.left_down = Some(app.cursor);
        app.left_release();
        assert!(app.selected.is_empty(), "Shift toggles off");
        app.shift = false;
        app.left_down = Some(app.cursor);
        app.left_release();
        let target = (-4..=4)
            .flat_map(|dy| (-4..=4).map(move |dx| (cell.0 + dx, cell.1 + dy)))
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
        app.cursor = point(&app, target);
        assert_eq!(app.cursor_kind(WIN_W), Kind::Move);
        proof(&app, "move-tick-1");
        app.left_down = Some(app.cursor);
        app.left_release();
        assert!(matches!(app.pending.as_slice(),[Command::Move {id,..}] if *id==unit.id));
        app.control = true;
        app.shift = true;
        assert_eq!(app.cursor_kind(WIN_W), Kind::AttackMove);
        proof(&app, "attack-move-tick-1");
        app.left_down = Some(app.cursor);
        app.left_release();
        assert!(matches!(app.pending.last(),Some(Command::AttackMove {id,..}) if *id==unit.id));
        app.control = false;
        app.shift = false;
        let queued = app.pending.len();
        app.right_down = Some(app.cursor);
        app.right_panning = true;
        app.right_release();
        assert_eq!(app.pending.len(), queued);
        assert_eq!(app.selected.len(), 1);
        app.right_down = Some(app.cursor);
        app.right_release();
        assert!(app.selected.is_empty());
        assert_eq!(app.pending.len(), queued);
        app.pending.clear();
        app.selected.insert(unit.id);
        let enemy = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_ENEMY)
            .unwrap()
            .clone();
        centre(&mut app, (i32::from(enemy.cell.0), i32::from(enemy.cell.1)));
        assert_eq!(app.cursor_kind(WIN_W), Kind::Attack);
        proof(&app, "attack-tick-1");
        app.left_down = Some(app.cursor);
        app.left_release();
        assert!(
            matches!(app.pending.as_slice(),[Command::Attack {target,..}] if *target==enemy.id)
        );
        app.pending.clear();
        let plant = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .buildings()
            .iter()
            .find(|b| {
                b.owner == yr_sim::OWNER_PLAYER
                    && app.session.as_ref().unwrap().kinds[b.kind as usize] == "GAPOWR"
            })
            .unwrap()
            .clone();
        centre(&mut app, (i32::from(plant.cell.0), i32::from(plant.cell.1)));
        app.service_mode = ServiceMode::Repair;
        assert_eq!(app.cursor_kind(WIN_W), Kind::NoRepair);
        proof(&app, "no-repair-tick-1");
        let session = app.session.as_mut().unwrap();
        let bytes = session.sim.save_bytes().unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
        for building in json["buildings"].as_array_mut().unwrap() {
            if building["id"] == plant.id {
                building["hp"] = (plant.max_hp / 2).into();
            }
        }
        let mut bytes = bytes[..12].to_vec();
        bytes.extend(serde_json::to_vec(&json).unwrap());
        session.sim = yr_sim::Sim::from_save_bytes(&bytes).unwrap();
        assert_eq!(app.cursor_kind(WIN_W), Kind::Repair);
        proof(&app, "repair-tick-1");
        assert!(
            matches!(app.world_action(app.cursor).unwrap().commands.as_slice(),[Command::Repair {id,..}] if *id==plant.id)
        );
        app.service_mode = ServiceMode::Sell;
        assert_eq!(app.cursor_kind(WIN_W), Kind::Sell);
        proof(&app, "sell-tick-1");
        app.service_click();
        assert!(matches!(app.pending.as_slice(),[Command::Sell {id,..}] if *id==plant.id));
        centre(&mut app, (i32::from(enemy.cell.0), i32::from(enemy.cell.1)));
        assert_eq!(app.cursor_kind(WIN_W), Kind::NoSell);
        proof(&app, "no-sell-tick-1");
        // Leave/focus suppression paints no software pointer and retains the hash.
        let hash = app.session.as_ref().unwrap().sim.state_hash();
        app.focus_changed(false);
        proof(&app, "unfocused-tick-1");
        assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
        println!("retail input: left select/move/attack, Shift toggle, Ctrl+Shift attack-move, right cancel and drag suppression passed; full-health/repair/sale ownership share cursor validation");
    }
}
