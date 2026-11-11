use super::*;
use yr_app::match_config::Side;

// Authored flat-color SHPs and classic MIX records, without game bytes.
fn shp(w: u16, h: u16, frames: u16, index: u8) -> Vec<u8> {
    let area = u32::from(w) * u32::from(h);
    let start = 8 + u32::from(frames) * 24;
    let mut bytes = Vec::new();
    for v in [0, w, h, frames] {
        bytes.extend(v.to_le_bytes());
    }
    for frame in 0..u32::from(frames) {
        for v in [0, 0, w, h] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(1u32.to_le_bytes());
        bytes.extend([0; 8]);
        bytes.extend((start + frame * area).to_le_bytes());
    }
    bytes.extend(std::iter::repeat_n(
        index,
        area as usize * usize::from(frames),
    ));
    bytes
}
fn mix(files: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend((files.len() as u16).to_le_bytes());
    bytes.extend((files.iter().map(|(_, b)| b.len()).sum::<usize>() as u32).to_le_bytes());
    let mut offset = 0u32;
    for (name, b) in &files {
        bytes.extend(yr_formats::mix_id(name).to_le_bytes());
        bytes.extend(offset.to_le_bytes());
        bytes.extend((b.len() as u32).to_le_bytes());
        offset += b.len() as u32;
    }
    for (_, b) in files {
        bytes.extend(b);
    }
    bytes
}
fn palette() -> Vec<u8> {
    let mut p = vec![0; 768];
    for (i, c) in [[40, 0, 0], [0, 50, 0], [0, 0, 60], [63, 0, 63]]
        .into_iter()
        .enumerate()
    {
        p[(i + 1) * 3..(i + 1) * 3 + 3].copy_from_slice(&c);
    }
    p
}
fn dimensions(side: Side) -> (u16, u16) {
    if side == Side::Allied {
        (28, 27)
    } else {
        (32, 28)
    }
}
fn files() -> Vec<(String, Vec<u8>)> {
    let archive = |side| {
        let (w, h) = dimensions(side);
        mix(vec![
            ("SIDEBAR.PAL", palette()),
            (
                "TAB00.SHP",
                shp(w, h, 5, if side == Side::Allied { 1 } else { 3 }),
            ),
            ("TAB01.SHP", shp(w, h, 5, 2)),
            ("CREDITS.SHP", shp(168, 16, 1, 2)),
        ])
    };
    vec![
        ("SIDEC01.MIX".into(), archive(Side::Allied)),
        ("SIDEC02.MIX".into(), archive(Side::Soviet)),
        (
            "RA2MD.MIX".into(),
            mix(vec![
                ("SIDEBAR.PAL", vec![63; 768]),
                ("TAB00.SHP", shp(28, 27, 5, 4)),
            ]),
        ),
    ]
}
fn canvas(size: (u32, u32)) -> App {
    let (mut app, _, _) = scene(size);
    let map = app.map.as_mut().unwrap();
    map.rgba = [30, 40, 30, 255]
        .into_iter()
        .cycle()
        .take((map.width * map.height * 4) as usize)
        .collect();
    app.pointer_moved((200., 200.));
    app
}
fn frame(app: &App) -> Vec<u8> {
    let size = app.camera_size();
    let mut p = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut p, size.0, size.1);
    p
}
fn assert_hits(app: &App, side: Side) {
    let session = app.session.as_ref().unwrap();
    let (w, h) = dimensions(side);
    for (i, category) in sidebar::Category::ALL.into_iter().enumerate() {
        let x = app.camera_size().0 - 168 + 26 + i as u32 * u32::from(w);
        for (dx, dy) in [(0, 0), (w - 1, h - 1)] {
            assert_eq!(
                sidebar::session_tab_at(
                    app.camera_size().0,
                    session,
                    (f64::from(x) + f64::from(dx), 197. + f64::from(dy))
                ),
                Some(category)
            );
        }
        assert_eq!(
            sidebar::session_tab_at(
                app.camera_size().0,
                session,
                (f64::from(x), 197. + f64::from(h))
            ),
            None
        );
    }
}

fn assert_tab(app: &App, side: Side, tab: u32, color: [u8; 4]) {
    assert_hits(app, side);
    let size = app.camera_size();
    let (w, h) = dimensions(side);
    let p = frame(app);
    let left = size.0 - 168 + 26 + tab * u32::from(w);
    for y in 200..197 + u32::from(h) - 4 {
        for x in left + 4..left + u32::from(w) - 4 {
            let at = ((y * size.0 + x) * 4) as usize;
            assert_eq!(&p[at..at + 4], &color, "{side:?} tab {tab} at {x},{y}");
        }
    }
}
fn assert_fallback(app: &App, side: Side) {
    assert_hits(app, side);
    let size = app.camera_size();
    let p = frame(app);
    let at = ((201 * size.0 + size.0 - 168 + 30) * 4) as usize;
    assert_eq!(&p[at..at + 4], &[255, 220, 96, 255]);
}

#[test]
fn sidebar_loose_overrides_preserve_selected_archive_isolation_and_session_bytes() {
    for size in [(640, 480), (1280, 800)] {
        for side in [Side::Allied, Side::Soviet] {
            let mut app = canvas(size);
            let mut f = files();
            let fs = AssetFs::from_files(f.clone());
            assert!(
                fs.read("TAB00.SHP").is_some(),
                "shared namespace contains a misleading sheet"
            );
            assert!(fs.read_loose("TAB00.SHP").is_none());
            let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            app.session.as_mut().unwrap().sidebar_art =
                sidebar::SidebarArt::load_for_side(&fs, side);
            assert_tab(
                &app,
                side,
                0,
                if side == Side::Allied {
                    [162, 0, 0, 255]
                } else {
                    [0, 0, 243, 255]
                },
            );
            assert_tab(&app, side, 1, [0, 203, 0, 255]);
            capture(&app, side, "archive");
            let (w, h) = dimensions(side);
            f.push(("tAb00.sHp".into(), shp(w, h, 5, 4)));
            let fs = AssetFs::from_files(f.clone());
            app.session.as_mut().unwrap().sidebar_art =
                sidebar::SidebarArt::load_for_side(&fs, side);
            assert_tab(&app, side, 0, [255, 0, 255, 255]);
            assert_tab(&app, side, 1, [0, 203, 0, 255]);
            capture(&app, side, "sheet");
            let mut pal = palette();
            pal[12..15].copy_from_slice(&[0, 63, 63]);
            f.push(("sidebar.pal".into(), pal));
            let fs = AssetFs::from_files(f);
            app.session.as_mut().unwrap().sidebar_art =
                sidebar::SidebarArt::load_for_side(&fs, side);
            assert_tab(&app, side, 0, [0, 255, 255, 255]);
            assert_tab(&app, side, 1, [0, 203, 0, 255]);
            capture(&app, side, "palette");
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                before
            );
        }
    }
    println!("Four side/layout cases resolve selected-archive art before shared mounts, then case-insensitive loose sheet/palette overrides; exact compositor crops and complete Session bytes agree.");
}
#[test]
fn sidebar_loose_overrides_work_without_a_valid_side_archive() {
    for broken in [false, true] {
        let mut app = canvas((640, 480));
        let mut f = vec![
            ("SIDEBAR.PAL".into(), palette()),
            ("TAB00.SHP".into(), shp(28, 27, 5, 4)),
        ];
        if broken {
            f.push(("SIDEC01.MIX".into(), b"bad archive".to_vec()));
        }
        let fs = AssetFs::from_files(f);
        app.session.as_mut().unwrap().sidebar_art = sidebar::SidebarArt::load(&fs);
        assert_tab(&app, Side::Allied, 0, [255, 0, 255, 255]);
    }
    println!("Loose palette/sheet pairs load independently of missing or malformed selected-side archives.");
}
#[test]
fn sidebar_loose_overrides_bound_bad_art_without_falling_through_to_other_archives() {
    for side in [Side::Allied, Side::Soviet] {
        for bad in 0..4 {
            let mut app = canvas((640, 480));
            let (w, h) = dimensions(side);
            let mut f = files();
            let mut bytes = match bad {
                0 => b"bad shp".to_vec(),
                1 => shp(w + 1, h, 5, 4),
                2 => shp(w, h, 1, 4),
                _ => shp(w, h, 5, 4),
            };
            if bad == 3 {
                bytes[2..4].copy_from_slice(&u16::MAX.to_le_bytes());
                bytes[4..6].copy_from_slice(&u16::MAX.to_le_bytes());
            }
            f.push(("TAB00.SHP".into(), bytes));
            let fs = AssetFs::from_files(f);
            app.session.as_mut().unwrap().sidebar_art =
                sidebar::SidebarArt::load_for_side(&fs, side);
            assert_fallback(&app, side);
            assert_tab(&app, side, 1, [0, 203, 0, 255]);
            capture(&app, side, "invalid");
        }
    }
    for side in [Side::Allied, Side::Soviet] {
        let mut app = canvas((640, 480));
        for mut f in [files(), Vec::new()] {
            f.push(("SIDEBAR.PAL".into(), b"bad palette".to_vec()));
            app.session.as_mut().unwrap().sidebar_art =
                sidebar::SidebarArt::load_for_side(&AssetFs::from_files(f), side);
            assert_fallback(&app, side);
        }
    }
    println!("Eight malformed/dimension/count/hostile-canvas overrides suppress only the affected sheet; invalid palette retains safe fallback without cross-side/shared-archive recovery.");
}
fn capture(app: &App, side: Side, phase: &str) {
    if app.camera_size().0 != 640 {
        return;
    }
    let Some(dir) = std::env::var_os("OPENYR_SIDEBAR_OVERRIDES_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let size = app.camera_size();
    std::fs::write(
        dir.join(format!("{side:?}-{phase}.png")),
        yr_render::png::encode_rgba(size.0, size.1, &frame(app)),
    )
    .unwrap();
}
