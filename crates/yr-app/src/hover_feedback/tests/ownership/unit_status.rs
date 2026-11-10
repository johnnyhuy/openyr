use super::*;

#[test]
fn classic_status_tracks_selection_cursor_cancel_and_preserves_session() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for zoom in [0.5, 1.0, 2.0] {
            let (mut app, own, enemy) = scene(size);
            app.view.zoom = zoom;
            app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                .into_iter()
                .cycle()
                .take(1600 * 1200 * 4)
                .collect();
            point_at(&mut app, own, size);
            // Move off the unit so hover color does not replace white selection.
            let point = app.cursor;
            app.pointer_moved((point.0 + 70.0, point.1 + 70.0));
            let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            let mut frame = vec![0; size.0 as usize * size.1 as usize * 4];
            app.compose(&mut frame, size.0, size.1);
            let session = app.session.as_ref().unwrap();
            let unit = session.sim.units().iter().find(|u| u.id == own).unwrap();
            let (mx, my) = yr_app::interaction::ground_point(
                app.map.as_ref().unwrap(),
                session.sim.terrain(),
                (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0),
            );
            let center = (
                ((mx - app.view.cam_x) * zoom).round() as u32,
                ((my - app.view.cam_y) * zoom).round() as u32,
            );
            let half = (14.0 * zoom).max(8.0) + 6.0;
            let x = center.0 - half.round() as u32;
            let y = center.1 - (half * 0.5).round() as u32;
            let at = ((y * size.0 + x) * 4) as usize;
            assert_ne!(
                &frame[at..at + 4],
                &SELECT_COL,
                "selected units must not draw the white corner overlay at {zoom}"
            );
            assert!(
                !frame.as_chunks::<4>().0.contains(&SELECT_COL),
                "no white selected-object overlay anywhere at {zoom}"
            );
            assert!(frame.as_chunks::<4>().0.contains(&[0, 252, 0, 255]));
            if zoom == 1.0 {
                capture(&app, &format!("status-{}-selected", size.0));
            }
            point_at(&mut app, enemy, size);
            assert_eq!(app.cursor_kind(size), Kind::Attack);
            if zoom == 1.0 {
                capture(&app, &format!("status-{}-attack", size.0));
            }
            app.pointer_button(ElementState::Pressed, MouseButton::Right);
            app.pointer_button(ElementState::Released, MouseButton::Right);
            assert!(app.selected.is_empty());
            assert_ne!(app.cursor_kind(size), Kind::Attack);
            frame.fill(0);
            app.compose(&mut frame, size.0, size.1);
            assert!(!frame.as_chunks::<4>().0.contains(&SELECT_COL));
            point_at(&mut app, own, size);
            assert_eq!(app.cursor_kind(size), Kind::Select);
            click(&mut app);
            assert_eq!(app.selected, BTreeSet::from([own]));
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                before
            );
            assert!(app.pending.is_empty());
        }
    }
    println!("Three layouts x three zooms: no white selected-object overlay, health retained, Attack/Select cursor kinds, right-click deselection and reselection; complete Session bytes and pending commands unchanged. No retail cursor pixels supplied.");
}
