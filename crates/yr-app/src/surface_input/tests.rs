use super::*;

fn chooser() -> App {
    let mut app = new_app(String::new(), String::new(), 1);
    app.menu.front = true;
    app.menu.set(Page::ChooseMap);
    app.menu.maps.entries = (0..40)
        .map(|i| yr_app::menu::maps::MapChoice {
            asset: format!("M{i}.MAP"),
            title: format!("Map {i}"),
        })
        .collect();
    app.menu.maps.top = 4;
    app
}

#[test]
fn invalid_surface_mapping_cancels_every_pointer_gesture() {
    for mapped in [
        None,
        Some((-1.0, 30.0)),
        Some((1280.0, 30.0)),
        Some((30.0, -1.0)),
        Some((30.0, 800.0)),
        Some((f64::NAN, 30.0)),
        Some((30.0, f64::INFINITY)),
    ] {
        let mut app = chooser();
        app.pointer_moved((500.0, 300.0));
        app.left_down = Some(app.cursor);
        app.right_down = Some(app.cursor);
        app.right_panning = true;
        app.radar_drag = true;
        app.menu.pressed = Some(yr_app::menu::Button::UseMap);
        app.menu.maps.drag = Some((200, 4));
        app.camera.edge_armed = true;
        app.pointer_frame_moved(mapped);
        assert!(!app.pointer.inside && !app.camera.edge_armed);
        assert!(app.left_down.is_none() && app.right_down.is_none());
        assert!(!app.right_panning && !app.radar_drag);
        assert!(app.menu.pressed.is_none() && app.menu.maps.drag.is_none());
        app.pointer_button(ElementState::Released, MouseButton::Left);
        app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
        assert_eq!(app.menu.maps.top, 4);
        assert_eq!(app.menu.page, Page::ChooseMap);
    }
}

#[test]
fn wheel_requires_fresh_in_frame_motion_and_ignores_empty_or_invalid_deltas() {
    let mut app = chooser();
    app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
    assert_eq!(app.menu.maps.top, 4);
    app.pointer_moved((500.0, 300.0));
    app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
    assert_eq!(app.menu.maps.top, 5);
    for y in [0.0, f32::NAN, f32::INFINITY] {
        app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, y));
    }
    assert_eq!(app.menu.maps.top, 5);
    app.focus_changed(false);
    app.pointer_moved((500.0, 300.0));
    assert!(!app.pointer.inside);
    app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
    app.focus_changed(true);
    app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
    assert_eq!(app.menu.maps.top, 5);
    app.pointer_moved((500.0, 300.0));
    app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
    assert_eq!(app.menu.maps.top, 6);
    for physical in [(f64::NAN, 0.0), (f64::MAX, 0.0), (0.0, f64::NEG_INFINITY)] {
        app.pointer_moved(physical);
        assert!(!app.pointer.inside);
    }
}

#[test]
fn surface_change_between_press_and_release_cannot_activate_a_menu_control() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.menu.front = true;
    app.menu.set(Page::Main);
    let index = app
        .menu
        .buttons()
        .iter()
        .position(|b| b.0 == yr_app::menu::Button::Quit)
        .unwrap();
    let layout = Layout::for_state(&app.menu, WIN_W, WIN_H);
    let (x, y, _, _) = layout.button(index, yr_app::menu::Button::Quit);
    let position = (
        layout.origin.0 + f64::from(x + 10) * layout.scale,
        layout.origin.1 + f64::from(y + 10) * layout.scale,
    );
    app.pointer_moved(position);
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    assert_eq!(app.menu.pressed, Some(yr_app::menu::Button::Quit));
    app.invalidate_pointer();
    app.pointer_button(ElementState::Released, MouseButton::Left);
    app.pointer_moved(position);
    app.pointer_button(ElementState::Released, MouseButton::Left);
    assert_eq!(app.menu.page, Page::Main);
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_button(ElementState::Released, MouseButton::Left);
    assert_eq!(app.menu.page, Page::Exit);
}

#[test]
fn menu_motion_keeps_the_first_match_pan_delta_local() {
    let mut app = new_app(String::new(), String::new(), 1);
    app.map = Some(yr_render::RenderedMap {
        width: 4000,
        height: 4000,
        rgba: Vec::new(),
        summary: String::new(),
        origin_x: 0,
        origin_y: 0,
        scale: 1,
    });
    app.view.cam_x = 100.0;
    app.view.cam_y = 100.0;
    app.view.zoom = 1.0;
    app.menu.front = true;
    app.menu.set(Page::Main);
    app.pointer_moved((250.0, 250.0));
    app.menu.front = false;
    app.menu.set(Page::Closed);
    app.selected.insert(17);
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    app.pointer_moved((260.0, 255.0));
    assert_eq!((app.view.cam_x, app.view.cam_y), (90.0, 95.0));
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert!(app.selected.contains(&17));
    app.pointer_button(ElementState::Pressed, MouseButton::Right);
    app.invalidate_pointer();
    app.pointer_moved((700.0, 400.0));
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert!(app.selected.contains(&17));
    assert_eq!((app.view.cam_x, app.view.cam_y), (90.0, 95.0));
}

#[test]
fn retail_pointer_guards_preserve_match_saves_and_valid_world_actions() {
    let directory = std::env::var_os("OPENYR_GAMEDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
        });
    if !directory.is_dir() {
        return;
    }
    let mut app = new_app(directory.to_string_lossy().into(), "EB1.MMX".into(), 1);
    app.session = Some(app.load_session().unwrap());
    app.map = Some(yr_render::render_map(&app.gamedir, &app.map_name, 1).unwrap());
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    let unit = app
        .session
        .as_ref()
        .unwrap()
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER)
        .unwrap()
        .clone();
    app.view.center_on_ground(
        app.map.as_ref().unwrap(),
        app.session.as_ref().unwrap().sim.terrain(),
        (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0),
        (WIN_W - SIDEBAR_W, WIN_H),
    );
    let position = (f64::from((WIN_W - SIDEBAR_W) / 2), f64::from(WIN_H / 2));
    app.pointer_moved(position);
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_button(ElementState::Released, MouseButton::Left);
    assert!(app.selected.contains(&unit.id));
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.invalidate_pointer();
    app.pointer_button(ElementState::Released, MouseButton::Left);
    app.pointer_wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
    assert!(app.pending.is_empty());
    assert!(app.selected.contains(&unit.id));
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
}
