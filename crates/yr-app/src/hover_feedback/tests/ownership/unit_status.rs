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
            assert!(frame.as_chunks::<4>().0.contains(&SELECT_COL));
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
    println!("Three layouts x three zooms: native selection brackets and health, Attack/Select cursor kinds, right-click deselection and reselection; complete Session bytes and pending commands unchanged. No retail cursor pixels supplied.");
}
