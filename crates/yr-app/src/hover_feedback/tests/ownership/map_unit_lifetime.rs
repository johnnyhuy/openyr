use super::*;
use std::fs;
use yr_formats::ShpFile;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn shp(body: usize) -> Vec<u8> {
    let frames = body * 2;
    let mut bytes = Vec::new();
    for v in [0u16, 9, 9, frames as u16] {
        bytes.extend(v.to_le_bytes());
    }
    for i in 0..frames {
        for v in [0u16, 0, 9, 9] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(1u32.to_le_bytes());
        bytes.extend([0; 8]);
        bytes.extend(((8 + 24 * frames + 81 * i) as u32).to_le_bytes());
    }
    let tips = [
        (4, 1),
        (1, 1),
        (1, 4),
        (1, 7),
        (4, 7),
        (7, 7),
        (7, 4),
        (7, 1),
    ];
    for i in 0..frames {
        let mut canvas = [0u8; 81];
        let d = i % 8;
        if i < body {
            let index = if body == 16 && i < 8 {
                48
            } else {
                16 + d as u8
            };
            canvas[4 * 9 + 4] = index;
            canvas[tips[d].1 * 9 + tips[d].0] = index;
        } else {
            canvas[8 * 9 + d] = 1;
        }
        bytes.extend(canvas);
    }
    assert!(ShpFile::parse(&bytes).is_ok());
    bytes
}
fn vxl() -> Vec<u8> {
    // Authored asymmetric 16x3x3 body with a differently colored nose.
    let columns = 48usize;
    let mut spans = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for i in 0..columns {
        starts.extend((spans.len() as i32).to_le_bytes());
        spans.extend([0, 3]);
        for _ in 0..3 {
            spans.extend([if i % 16 < 12 { 16 } else { 48 }, 0]);
        }
        spans.push(3);
        ends.extend(((spans.len() - 1) as i32).to_le_bytes());
    }
    let mut body = starts;
    body.extend(ends);
    body.extend(spans);
    let mut bytes = b"Voxel Animation\0".to_vec();
    for v in [1u32, 1, 1, body.len() as u32] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend([16, 31]);
    bytes.extend([0; 768]);
    let mut name = [0u8; 16];
    name[..4].copy_from_slice(b"body");
    bytes.extend(name);
    bytes.extend([0; 12]);
    bytes.extend(body);
    for v in [0u32, (columns * 4) as u32, (columns * 8) as u32] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend(1f32.to_le_bytes());
    for v in [
        1f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -8., -1.5, 0., 8., 1.5, 3.,
    ] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend([16, 3, 3, 4]);
    assert!(yr_formats::VxlFile::parse(&bytes).is_ok());
    bytes
}
fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "openyr-unit-life-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    let mut pal = [0u8; 768];
    pal[3..6].copy_from_slice(&[8, 10, 8]);
    for i in 40..=48 {
        pal[i * 3..i * 3 + 3].copy_from_slice(&[
            10 + (i - 40) as u8 * 5,
            40 - (i - 40) as u8 * 3,
            20,
        ]);
    }
    fs::write(path.join("ISOTEM.PAL"), pal).unwrap();
    fs::write(path.join("UNITTEM.PAL"), pal).unwrap();
    fs::write(path.join("RULESMD.INI"),"[Colors]\nFixtureTint=160,255,200\n[FixtureHouse]\nColor=FixtureTint\n[VehicleTypes]\n0=MTNK\n1=CMIN\n[InfantryTypes]\n0=E1\n[AircraftTypes]\n0=ORCA\n[MTNK]\nStrength=100\n[CMIN]\nImage=MISSING\nStrength=100\n[E1]\nStrength=100\n[ORCA]\nStrength=100\n").unwrap();
    fs::write(
        path.join("ARTMD.INI"),
        "[MTNK]\nVoxel=yes\n[E1]\nSequence=READY\n[READY]\nReady=8,1,1\n[ORCA]\n",
    )
    .unwrap();
    fs::write(path.join("E1.SHP"), shp(16)).unwrap();
    fs::write(path.join("ORCA.SHP"), shp(8)).unwrap();
    fs::write(path.join("MTNK.VXL"), vxl()).unwrap();
    fs::write(
        path.join("TEMPERATMD.INI"),
        "[TileSet0000]\nFileName=Clear\nTilesInSet=1\n",
    )
    .unwrap();
    let mut tmp = Vec::new();
    for v in [1u32, 1, 60, 30, 20] {
        tmp.extend(v.to_le_bytes());
    }
    tmp.extend([0; 52]);
    tmp[61] = 13;
    tmp.extend([1; 900]);
    fs::write(path.join("CLEAR01.TEM"), tmp).unwrap();
    Fixture(path)
}
fn map(objects: &str) -> String {
    format!("[Map]\nTheater=TEMPERATE\nSize=0,0,32,32\nLocalSize=0,0,32,32\n[IsoMapPack5]\n1=uAC0AMUUABQAAAAAAAAAABUAFAAAAAAAAAAAFgAUAAAAAAAAAAAXABQAAAAAAAAAABQAFQAAAAAAAAAAFQAVAAAAAAAAAAAWABUAAAAAAAAAABcAFQAAAAAAAAAAFAAWAAAAAAAAAAAVABYAAAAAAAAAABYAFgAAAAAAAAAAFwAWAAAAAAAAAAAUABcAAAAAAAAAABUAFwAAAAAAAAAAFgAXAAAAAAAAAAAXABcAAAAAAAAAAAAAAAARAAA=\n{objects}")
}

const OBJECTS: &str = "[Units]\n0=FixtureHouse,MTNK,256,20,20,0\n1=FixtureHouse,CMIN,256,23,20,0\n[Infantry]\n0=FixtureHouse,E1,256,21,20,0,Guard,64\n[Aircraft]\n0=FixtureHouse,ORCA,256,22,20,128\n";
const UNSUPPORTED: &str = "[Units]\n0=FixtureHouse,CMIN,256,23,20,0\n";

fn app_for(dir: &Fixture, objects: &str, size: (u32, u32)) -> App {
    fs::write(dir.0.join("LIFE.MAP"), map(objects)).unwrap();
    let root = dir.0.to_str().unwrap();
    let (mut app, _, _) = scene(size);
    app.session = Some(session_with_map_objects(objects));
    app.map = Some(yr_render::render_map_background(root, "LIFE.MAP", 1).unwrap());
    app.sprites = Some(std::cell::RefCell::new(
        yr_render::sprites::SpriteBank::open(root, "LIFE.MAP").unwrap(),
    ));
    point_at(&mut app, 1, size);
    app.pointer.inside = false;
    app.pending = vec![Command::Stop { id: 1 }];
    app.selected.clear();
    app
}
fn background(app: &App, size: (u32, u32), x: i32, y: i32) -> [u8; 4] {
    let map = app.map.as_ref().unwrap();
    assert!(x >= 0 && y >= 0 && x < size.0 as i32 && y < size.1 as i32);
    let mx = app.view.cam_x + x as f32 / app.view.zoom;
    let my = app.view.cam_y + y as f32 / app.view.zoom;
    if mx >= 0. && my >= 0. && (mx as u32) < map.width && (my as u32) < map.height {
        let at = ((my as u32 * map.width + mx as u32) * 4) as usize;
        map.rgba[at..at + 4].try_into().unwrap()
    } else {
        [16, 16, 20, 255]
    }
}
fn frame(app: &App, size: (u32, u32), phase: &str, ids: &[u32]) -> Vec<u8> {
    let session = app.session.as_ref().unwrap();
    let before = yr_app::snapshot::encode(session).unwrap();
    let pending = app.pending.clone();
    let selected = app.selected.clone();
    let mut rgba = vec![0; (size.0 * size.1 * 4) as usize];
    app.compose(&mut rgba, size.0, size.1);
    let mut body = 0;
    let mut shadow = 0;
    let mut coverage = 0;
    for id in ids {
        let previous = (body, shadow, coverage);
        let u = session.sim.units().iter().find(|u| u.id == *id).unwrap();
        let kind = &session.kinds[usize::from(u.kind)];
        let mut bank = app.sprites.as_ref().unwrap().borrow_mut();
        let house = session.map_unit_house(u).unwrap();
        let remap = if u.owner == yr_sim::OWNER_NEUTRAL {
            bank.house_color(house)
        } else {
            session.owner_colors[usize::from(u.owner)]
        };
        assert_ne!(bank.house_color(house), bank.house_color(""));
        let sprite = bank.get(kind, u.facing, remap).unwrap();
        let (mx, my) = yr_app::interaction::ground_point(
            app.map.as_ref().unwrap(),
            session.sim.terrain(),
            (u.x.raw() as f32 / 256., u.y.raw() as f32 / 256.),
        );
        let x0 = (mx - app.view.cam_x - sprite.ax as f32).round() as i32;
        let y0 = (my - app.view.cam_y - sprite.ay as f32).round() as i32;
        for (i, p) in sprite.rgba.as_chunks::<4>().0.iter().enumerate() {
            if p[3] == 0 {
                continue;
            }
            let x = x0 + (i % sprite.w as usize) as i32;
            let y = y0 + (i / sprite.w as usize) as i32;
            if x < 0 || y < 0 || x >= size.0 as i32 - SIDEBAR_W as i32 || y >= size.1 as i32 {
                continue;
            }
            let bg = background(app, size, x, y);
            let at = ((y as u32 * size.0 + x as u32) * 4) as usize;
            let mut expected = [0u8; 4];
            expected[3] = 255;
            if p[3] == yr_render::sprites::SHADOW_ALPHA {
                assert!(bg[0] > 0);
                for c in 0..3 {
                    expected[c] = bg[c] / 2;
                }
                shadow += 1;
            } else {
                for c in 0..3 {
                    expected[c] = ((u32::from(p[c]) * u32::from(p[3])
                        + u32::from(bg[c]) * u32::from(255 - p[3])
                        + 127)
                        / 255) as u8;
                }
                body += 1;
                coverage += usize::from(p[3] < 255);
            }
            assert_eq!(
                &rgba[at..at + 4],
                &expected,
                "{phase} {kind} id={id} source pixel {i}"
            );
        }
        assert!(body > previous.0, "visible body for {phase} {kind}");
        if kind == "MTNK" {
            assert!(coverage > previous.2);
        } else {
            assert!(shadow > previous.1);
        }
    }
    if !ids.is_empty() {
        assert!(body > 0);
    }
    assert_eq!(yr_app::snapshot::encode(session).unwrap(), before);
    assert_eq!(app.pending, pending);
    assert_eq!(app.selected, selected);
    if let Some(path) = std::env::var_os("OPENYR_UNIT_LIFETIME_PROOF") {
        let path = PathBuf::from(path);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join(format!("{}-{phase}.png", size.0)),
            yr_render::png::encode_rgba(size.0, size.1, &rgba),
        )
        .unwrap();
        fs::write(path.join(format!("{}-{phase}.json",size.0)),serde_json::to_vec_pretty(&serde_json::json!({
            "phase":phase,"body_pixels":body,"shadow_pixels":shadow,"fractional_pixels":coverage,
            "tick":session.sim.tick(),"hash":format!("{:016x}",session.sim.state_hash()),
            "snapshot_bytes":before.len(),"retail":false,"selected":app.selected,
            "units":session.sim.units().iter().map(|u|serde_json::json!({"id":u.id,"kind":session.kinds[usize::from(u.kind)],"hp":u.hp,"max_hp":u.max_hp,"owner":u.owner,"house":session.map_unit_house(u),"position":[u.x.raw(),u.y.raw()],"facing":u.facing})).collect::<Vec<_>>()
        })).unwrap()).unwrap();
    }
    rgba
}

#[test]
fn map_unit_lifetime_background_retains_preview_and_unsupported_fallback() {
    let dir = fixture();
    let root = dir.0.to_str().unwrap();
    fs::write(dir.0.join("LIFE.MAP"), map(OBJECTS)).unwrap();
    fs::write(dir.0.join("FALLBACK.MAP"), map(UNSUPPORTED)).unwrap();
    let bg = yr_render::render_map_background(root, "LIFE.MAP", 1).unwrap();
    let fallback = yr_render::render_map(root, "FALLBACK.MAP", 1).unwrap();
    let preview = yr_render::render_map(root, "LIFE.MAP", 1).unwrap();
    assert_eq!(
        bg.rgba, fallback.rgba,
        "known unit bodies/shadows must not be baked"
    );
    assert_ne!(
        preview.rgba, bg.rgba,
        "ordinary static preview keeps unit art"
    );
    let bank = yr_render::sprites::SpriteBank::open(root, "LIFE.MAP").unwrap();
    for kind in ["MTNK", "E1", "ORCA"] {
        assert!(bank.supports_map_unit(kind));
    }
    assert!(!bank.supports_map_unit("CMIN"));
    assert!(!bank.supports_map_unit("UNKNOWN"));
    println!("Match background equals static unsupported-only fallback; full previews retain known SHP/VXL units. Bank and background share eligibility.");
}

#[test]
fn map_unit_lifetime_native_healthy_damage_movement_load_owner_and_destruction() {
    for size in [(640, 480), (1280, 800)] {
        let dir = fixture();
        let mut app = app_for(&dir, OBJECTS, size);
        frame(&app, size, "healthy", &[1, 3, 4]);
        app.session.as_mut().unwrap().sim.step(&[
            Command::SetMapUnitHealth { id: 1, health: 128 },
            Command::SetMapUnitHealth { id: 2, health: 128 },
            Command::SetMapUnitHealth { id: 3, health: 128 },
            Command::SetMapUnitHealth { id: 4, health: 128 },
        ]);
        frame(&app, size, "damaged", &[1, 3, 4]);
        app.session.as_mut().unwrap().sim.step(&[Command::Move {
            id: 1,
            x: Fix::from_raw(20 * 256 + 128),
            y: Fix::from_raw(23 * 256 + 128),
        }]);
        for _ in 0..8 {
            app.session.as_mut().unwrap().sim.step(&[]);
        }
        let unit = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .find(|u| u.id == 1)
            .unwrap();
        assert_ne!(unit.y, Fix::from_raw(20 * 256 + 128));
        assert_eq!(
            app.session.as_ref().unwrap().map_unit_house(unit),
            Some("FixtureHouse")
        );
        let moved = frame(&app, size, "moved", &[1, 3, 4]);
        let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
        let mut restored = session_with_map_objects(OBJECTS);
        yr_app::snapshot::restore(&mut restored, &saved).unwrap();
        app.session = Some(restored);
        assert_eq!(frame(&app, size, "restored", &[1, 3, 4]), moved);
        app.session
            .as_mut()
            .unwrap()
            .sim
            .step(&[Command::AssignOwner {
                id: 1,
                owner: yr_sim::OWNER_PLAYER,
            }]);
        frame(&app, size, "owned", &[1, 3, 4]);
        app.session.as_mut().unwrap().sim.step(&[
            Command::Destroy { id: 1 },
            Command::Destroy { id: 3 },
            Command::Destroy { id: 4 },
        ]);
        let dead = frame(&app, size, "dead", &[]);
        // Both original and moved supported-unit neighborhoods return to the
        // background. Unsupported art remains its intentionally static fallback.
        for (x, y) in [(20.5, 20.5), (21.5, 20.5), (22.5, 20.5), (20.5, 23.5)] {
            let (mx, my) = yr_app::interaction::ground_point(
                app.map.as_ref().unwrap(),
                app.session.as_ref().unwrap().sim.terrain(),
                (x, y),
            );
            let (cx, cy) = (
                (mx - app.view.cam_x).round() as i32,
                (my - app.view.cam_y).round() as i32,
            );
            for dy in -7..=7 {
                for dx in -10..=10 {
                    let (px, py) = (cx + dx, cy + dy);
                    if px < 0
                        || py < 0
                        || px >= size.0 as i32 - SIDEBAR_W as i32
                        || py >= size.1 as i32
                    {
                        continue;
                    }
                    let at = ((py as u32 * size.0 + px as u32) * 4) as usize;
                    assert_eq!(
                        &dead[at..at + 4],
                        &background(&app, size, px, py),
                        "stale body/shadow at {x},{y}"
                    );
                }
            }
        }
    }
    println!("Two native layouts: supported SHP/VXL units have one body/coverage/shadow, original house remap survives movement/load, new ownership uses owner colors, and destruction leaves no original/moved baked body; every draw preserves complete Session bytes and pending input.");
}

#[test]
fn map_unit_lifetime_overlapping_shadows_darken_once() {
    let objects="[Infantry]\n0=FixtureHouse,E1,256,20,20,0,Guard,64\n1=FixtureHouse,E1,256,20,20,0,Guard,64\n";
    let dir = fixture();
    let app = app_for(&dir, objects, (640, 480));
    frame(&app, (640, 480), "overlap", &[1, 2]);
    println!("Two overlapping neutral SHP shadows darken their ground pixels once, with exact original-house body pixels.");
}

#[test]
fn map_unit_lifetime_missing_bank_uses_a_marker_without_mutating_state() {
    let dir = fixture();
    let mut app = app_for(&dir, OBJECTS, (640, 480));
    app.sprites = None;
    let session = app.session.as_ref().unwrap();
    let unit = session.sim.units().iter().find(|u| u.id == 1).unwrap();
    let (mx, my) = yr_app::interaction::ground_point(
        app.map.as_ref().unwrap(),
        session.sim.terrain(),
        (unit.x.raw() as f32 / 256., unit.y.raw() as f32 / 256.),
    );
    let (x, y) = ((mx - app.view.cam_x) as u32, (my - app.view.cam_y) as u32);
    let before = yr_app::snapshot::encode(session).unwrap();
    let mut rgba = vec![0; 640 * 480 * 4];
    app.compose(&mut rgba, 640, 480);
    let at = ((y * 640 + x) * 4) as usize;
    assert_eq!(
        &rgba[at..at + 4],
        &selected_owner_col(Some(session), yr_sim::OWNER_NEUTRAL)
    );
    assert_eq!(yr_app::snapshot::encode(session).unwrap(), before);
    println!("An unavailable live sprite bank degrades tracked map units to markers rather than hiding healthy bodies; Session bytes remain unchanged.");
}
