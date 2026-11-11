use super::*;
use sidebar::Category;

fn scene(size: (u32, u32)) -> (App, u32, u32) {
    let (mut app, own, enemy) = super::scene(size);
    app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
        .into_iter()
        .cycle()
        .take(1600 * 1200 * 4)
        .collect();
    (app, own, enemy)
}

// Original test art: each frame is a flat color, independently packed into
// uncompressed SHP records and a classic MIX directory. No retail bytes.
fn art(side: yr_app::match_config::Side, width: u16, height: u16) -> sidebar::SidebarArt {
    let mut palette = vec![0; 768];
    for phase in 0..5 {
        palette[(phase + 1) * 3..(phase + 1) * 3 + 3].copy_from_slice(&[
            10 + phase as u8 * 9,
            25,
            15,
        ]);
    }
    let area = usize::from(width) * usize::from(height);
    let mut shp = Vec::new();
    for word in [0, width, height, 5] {
        shp.extend(word.to_le_bytes());
    }
    for phase in 0..5 {
        for word in [0u16, 0, width, height] {
            shp.extend(word.to_le_bytes());
        }
        shp.extend(1u32.to_le_bytes());
        shp.extend([0; 8]);
        shp.extend((128 + phase * area as u32).to_le_bytes());
    }
    for phase in 1..=5 {
        shp.extend(std::iter::repeat_n(phase, area));
    }
    let files = [
        ("SIDEBAR.PAL", palette),
        ("TAB00.SHP", shp.clone()),
        ("TAB01.SHP", shp.clone()),
        ("TAB02.SHP", shp.clone()),
        ("TAB03.SHP", shp),
    ];
    let mut archive = Vec::new();
    archive.extend((files.len() as u16).to_le_bytes());
    archive
        .extend((files.iter().map(|(_, bytes)| bytes.len()).sum::<usize>() as u32).to_le_bytes());
    let mut offset = 0u32;
    for (name, bytes) in &files {
        archive.extend(yr_formats::mix_id(name).to_le_bytes());
        archive.extend(offset.to_le_bytes());
        archive.extend((bytes.len() as u32).to_le_bytes());
        offset += bytes.len() as u32;
    }
    for (_, bytes) in files {
        archive.extend(bytes);
    }
    let fs = AssetFs::from_files(vec![(side.sidebar().into(), archive)]);
    sidebar::SidebarArt::load_for_side(&fs, side)
}

fn frame(app: &App) -> Vec<u8> {
    let size = app.camera_size();
    let mut pixels = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut pixels, size.0, size.1);
    pixels
}

fn assert_tab(app: &App, tab: usize, tab_size: (u16, u16), phase: u8, label: &str) {
    let size = app.camera_size();
    let pixels = frame(app);
    let x = size.0 - 168 + 26 + tab as u32 * u32::from(tab_size.0);
    // An interior crop also catches accidental selected-frame overlays.
    let expected = [
        [40, 101, 60, 255],
        [77, 101, 60, 255],
        [113, 101, 60, 255],
        [150, 101, 60, 255],
        [186, 101, 60, 255],
    ][usize::from(phase)];
    for y in 200..197 + u32::from(tab_size.1) - 4 {
        for x in x + 4..x + u32::from(tab_size.0) - 4 {
            let at = ((y * size.0 + x) * 4) as usize;
            assert_eq!(&pixels[at..at + 4], &expected, "{label} at {x},{y}");
        }
    }
    if size.0 == 640 && tab_size.0 == 28 && tab == 1 {
        if let Some(path) = std::env::var_os("OPENYR_SIDEBAR_PRESS_PROOF") {
            let path = PathBuf::from(path);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join(format!("{label}.png")),
                yr_render::png::encode_rgba(size.0, size.1, &pixels),
            )
            .unwrap();
        }
    }
}

#[test]
fn sidebar_tab_press_uses_authored_variants_and_tracks_origin_through_release() {
    for size in [(640, 480), (1280, 800)] {
        for (side, w, h) in [
            (yr_app::match_config::Side::Allied, 28, 27),
            (yr_app::match_config::Side::Soviet, 32, 28),
        ] {
            for (i, category) in Category::ALL.into_iter().enumerate() {
                let (mut app, own, _) = scene(size);
                app.session.as_mut().unwrap().sidebar_art = art(side, w, h);
                app.pending.push(Command::Stop { id: own });
                let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                let pending = app.pending.clone();
                let selected = app.selected.clone();
                let camera = (app.view.cam_x, app.view.cam_y, app.view.zoom);
                let at = (
                    f64::from(size.0 - 168 + 26 + i as u32 * u32::from(w) + 10),
                    207.,
                );
                app.pointer_moved(at);
                assert_tab(&app, i, (w, h), u8::from(i == 0), "00-before");
                app.pointer_button(ElementState::Pressed, MouseButton::Left);
                assert_eq!(
                    app.sidebar_view.category, category,
                    "selection remains immediate"
                );
                assert_tab(&app, i, (w, h), if i == 0 { 4 } else { 3 }, "01-held");
                app.pointer_moved((at.0, 240.));
                assert_tab(&app, i, (w, h), 1, "02-outside");
                app.pointer_moved(at);
                assert_tab(&app, i, (w, h), if i == 0 { 4 } else { 3 }, "03-reentered");
                app.pointer_button(ElementState::Released, MouseButton::Left);
                assert_tab(&app, i, (w, h), 1, "04-released");
                // No delayed action is armed, even after a second release in the world.
                app.pointer_moved((200., 200.));
                app.pointer_button(ElementState::Released, MouseButton::Left);
                assert_eq!(app.pending, pending);
                assert_eq!(app.selected, selected);
                assert_eq!((app.view.cam_x, app.view.cam_y, app.view.zoom), camera);
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    before
                );
            }
        }
    }
    println!("16 native Allied/Soviet tab sequences: both pressed variants, leave/reenter/release, source-color crops and unchanged Session bytes, selection, camera and queued orders.");
}

#[test]
fn sidebar_tab_press_cancels_on_pointer_focus_menu_right_press_and_shortcuts() {
    for route in 0..5 {
        let (mut app, _, _) = scene((640, 480));
        app.session.as_mut().unwrap().sidebar_art = art(yr_app::match_config::Side::Allied, 28, 27);
        let at = (536., 207.);
        app.pointer_moved(at);
        app.pointer_button(ElementState::Pressed, MouseButton::Left);
        assert_tab(&app, 1, (28, 27), 3, "01-held");
        match route {
            0 => {
                app.invalidate_pointer();
                app.pointer_moved(at);
            }
            1 => {
                app.focus_changed(false);
                app.focus_changed(true);
                app.pointer_moved(at);
            }
            2 => {
                key(&mut app, KeyCode::Escape, ElementState::Pressed);
                assert!(app.menu.visible());
                app.menu.set(Page::Closed);
            }
            3 => {
                app.pointer_button(ElementState::Pressed, MouseButton::Right);
            }
            _ => {
                key(&mut app, KeyCode::KeyW, ElementState::Pressed);
            }
        }
        assert_tab(&app, 1, (28, 27), 1, "05-cancelled");
        app.pointer_moved(at);
        assert_tab(&app, 1, (28, 27), 1, "06-no-rearm");
    }
    println!("Five native cancellation routes clear held art and prevent pointer reentry from rearming it.");
}

#[test]
fn sidebar_tab_press_fallback_recovers_exact_selected_pixels() {
    let (mut app, _, _) = scene((640, 480));
    app.sidebar_view.select(Category::Defences);
    app.pointer_moved((536., 207.));
    let before = frame(&app);
    let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    assert_ne!(
        frame(&app),
        before,
        "missing optional art still signals a held tab"
    );
    app.pointer_button(ElementState::Released, MouseButton::Left);
    assert_eq!(frame(&app), before);
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        saved
    );
    println!("Missing optional tab art shows a bounded fallback inset and restores exact selected pixels on release without changing Session bytes.");
}
