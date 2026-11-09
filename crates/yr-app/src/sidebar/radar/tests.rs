use super::*;
use crate::sidebar::{self, Category};
use std::path::PathBuf;
use yr_formats::{mix_id, MixArchive, Palette, ShpFile};
use yr_sim::{Command, Role, OWNER_ENEMY};

#[test]
fn clock_uses_sim_ticks_reverses_without_jumps_and_freezes_on_redraw() {
    let mut clock = Clock::default();
    clock.reset(1, false);
    clock.observe(2, true);
    assert_eq!(clock.frame, 0);
    for t in 3..=32 {
        clock.observe(t, true);
    }
    assert_eq!(clock.frame, 15);
    for _ in 0..10 {
        clock.observe(32, true);
    }
    assert_eq!(clock.frame, 15);
    clock.observe(33, false);
    assert_eq!(clock.frame, 15);
    clock.observe(43, false);
    assert_eq!(clock.frame, 10);
    clock.observe(44, true);
    clock.observe(88, true);
    assert!(clock.ready());
    clock.observe(89, false);
    assert!(!clock.ready(), "power loss immediately blocks input");
    clock.observe(u64::MAX, false);
    assert_eq!(clock.frame, 0);
    clock.observe(1, true);
    assert!(clock.ready(), "rewinding uses the reset policy");
}

#[test]
fn letterboxing_targets_only_painted_cells() {
    let radar = Radar {
        terrain: Terrain {
            width: 2,
            height: 1,
            offset: (0, 0),
            map_width: 2,
            colors: vec![[40, 50, 60]; 2],
            cells: vec![Some((4, 5)), None],
        },
        clock: Clock {
            frame: 32,
            enabled: true,
            tick: Some(1),
            remainder: 0,
        },
        ..Radar::default()
    };
    let rect = radar.terrain.fit(1280).unwrap();
    assert_eq!((rect.w, rect.h), (140, 70));
    assert_eq!(
        radar.target(1280, (f64::from(rect.x), f64::from(rect.y))),
        Some((4.5, 5.5))
    );
    assert_eq!(
        radar.target(1280, (f64::from(rect.x + 70), f64::from(rect.y))),
        None
    );
    for pos in [
        (rect.x - 1, rect.y),
        (rect.x, rect.y - 1),
        (rect.x + rect.w, rect.y),
        (rect.x, rect.y + rect.h),
    ] {
        assert_eq!(
            radar.target(1280, (f64::from(pos.0), f64::from(pos.1))),
            None
        );
    }
}

fn corpus() -> Option<PathBuf> {
    let dir =
        PathBuf::from(std::env::var("HOME").ok()?).join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    dir.is_dir().then_some(dir)
}

fn capture(session: &Session, name: &str) {
    if let Some(dir) = std::env::var_os("OPENYR_RADAR_DUMP") {
        std::fs::create_dir_all(&dir).unwrap();
        let mut frame = vec![0; 1280 * 800 * 4];
        sidebar::draw(&mut frame, 1280, 800, session, Category::Structures, None);
        session
            .radar
            .draw_viewport(&mut frame, 1280, 800, [(79.0, 75.0), (94.0, 80.0)]);
        std::fs::write(
            PathBuf::from(&dir).join(format!("{name}.png")),
            yr_render::png::encode_rgba(1280, 800, &frame),
        )
        .unwrap();
    }
}

fn save(session: &Session, name: &str) {
    if let Some(dir) = std::env::var_os("OPENYR_RADAR_DUMP") {
        std::fs::create_dir_all(&dir).unwrap();
        let path = PathBuf::from(dir).join(format!("{name}.oyrsave"));
        crate::snapshot::save(session, &path).unwrap();
        println!(
            "{name}: tick {}, hash {:016x}, power {:?}",
            session.sim.tick(),
            session.sim.state_hash(),
            session.sim.power()
        );
    }
}

#[test]
fn retail_radar_frames_rules_power_live_map_and_save_roundtrip() {
    let Some(dir) = corpus() else {
        eprintln!("skip: retail install not present");
        return;
    };
    let fs = AssetFs::open(&dir).unwrap();
    let rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    for id in [
        "AMRADR", "GAAIRC", "NARADR", "NAPSIS", "GASPYSAT", "CACHIG05", "CANEWY05",
    ] {
        assert_eq!(rules.techno(id).unwrap().radar, Some(true), "{id}");
    }
    assert_ne!(rules.techno("GAPOWR").unwrap().radar, Some(true));
    let mut session = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    session.sim.set_ai(None);
    let raw_map = MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).unwrap();
    let mut bad_map = raw_map.clone();
    bad_map.local_size.2 = i32::MAX;
    assert!(Terrain::load(&fs, &bad_map).is_err());
    assert_eq!(
        (session.radar.terrain.width, session.radar.terrain.height),
        (112, 52)
    );
    assert!(session.radar.terrain.cells.iter().flatten().count() > 5000);
    assert!(
        session
            .radar
            .terrain
            .colors
            .iter()
            .filter(|&&c| c != [0; 3])
            .count()
            > 5000
    );
    capture(&session, "off");
    save(&session, "off");
    let slot = session
        .buildables
        .iter()
        .position(|b| b.id == "AMRADR")
        .unwrap();
    // Enemy ownership and enemy power cannot supply or disable player radar.
    let grid = session.sim.terrain().unwrap();
    let enemy_spot = (0..grid.height())
        .find_map(|ly| {
            (0..grid.width())
                .map(|lx| grid.cell_at_local(lx, ly))
                .find(|&cell| session.can_place(slot, cell))
        })
        .unwrap();
    session.step(&[Command::PlaceBuilding {
        kind: slot as u16,
        x: enemy_spot.0,
        y: enemy_spot.1,
        w: 3,
        h: 2,
        hp: 600,
        armor: 0,
        power: -5000,
        owner: OWNER_ENEMY,
        role: Role::Combat,
    }]);
    assert!(!session.radar.available(&session.sim, &session.kinds));
    session.step(&[session.produce(slot).unwrap()]);
    while session.ready_structure().is_none() {
        session.step(&[]);
        assert!(session.sim.tick() < 2000);
    }
    let yard = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == OWNER_PLAYER && b.role == Role::ConstructionYard)
        .unwrap()
        .cell;
    let spot = (0..16)
        .find_map(|r| {
            (-r..=r).find_map(|dy| {
                (-r..=r).find_map(|dx| {
                    let x = u16::try_from(i32::from(yard.0) + dx).ok()?;
                    let y = u16::try_from(i32::from(yard.1) + dy).ok()?;
                    session.can_place(slot, (x, y)).then_some((x, y))
                })
            })
        })
        .unwrap();
    session.step(&[session.place(slot, spot).unwrap()]);
    assert!(session.radar.available(&session.sim, &session.kinds));
    assert!(!session.radar.ready());
    let provider = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == OWNER_PLAYER && usize::from(b.kind) == slot)
        .unwrap()
        .id;
    for phase in 0..=32 {
        assert_eq!(session.radar.clock.frame, phase);
        capture(&session, &format!("opening-{phase:02}"));
        if phase < 32 {
            session.step(&[]);
            session.step(&[]);
        }
    }
    assert!(session.radar.ready());
    save(&session, "active");
    let before = crate::snapshot::encode(&session).unwrap();
    let hash = session.sim.state_hash();
    // All authored frame pixels, including transparent source pixels, are
    // checked separately from the generated minimap in the open state.
    let mix = MixArchive::parse(fs.read("SIDEC01.MIX").unwrap()).unwrap();
    let shp = ShpFile::parse(mix.get_by_id(mix_id("RADAR.SHP")).unwrap()).unwrap();
    let pal = Palette::parse(mix.get_by_id(mix_id("SIDEBAR.PAL")).unwrap()).unwrap();
    assert_eq!(shp.frames.len(), 33);
    for i in 0..33 {
        session.radar.clock.frame = i as u8;
        session.radar.clock.enabled = false;
        let mut frame = vec![0; 168 * 800 * 4];
        sidebar::draw(&mut frame, 168, 800, &session, Category::Structures, None);
        for (p, index) in shp.decode_frame_canvas(i).unwrap().into_iter().enumerate() {
            let at = ((48 + p / 168) * 168 + p % 168) * 4;
            let expected = if index == 0 {
                super::super::PANEL
            } else {
                pal.rgba(index)
            };
            assert_eq!(&frame[at..at + 4], &expected, "radar frame {i} pixel {p}");
        }
    }
    session.radar.reset(&session.sim, &session.kinds);
    assert_eq!(
        crate::snapshot::encode(&session).unwrap(),
        before,
        "presentation stays outside saves"
    );
    assert_eq!(session.sim.state_hash(), hash);
    let rect = session.radar.terrain.fit(1280).unwrap();
    let mut sampled = 0;
    for y in 0..rect.h {
        for x in 0..rect.w {
            let pos = (f64::from(rect.x + x) + 0.5, f64::from(rect.y + y) + 0.5);
            if let Some((cx, cy)) = session.radar.target(1280, pos) {
                assert!(
                    session
                        .sim
                        .terrain()
                        .unwrap()
                        .cell(cx.floor() as u16, cy.floor() as u16)
                        .is_some(),
                    "painted minimap targets must have simulation terrain"
                );
                let sx = x as usize * 112 / rect.w as usize;
                let sy = y as usize * 52 / rect.h as usize;
                assert_eq!(
                    session.radar.terrain.cells[sy * 112 + sx],
                    Some((cx.floor() as u16, cy.floor() as u16))
                );
                sampled += 1;
            }
        }
    }
    assert!(sampled > 8000);
    for &(x, y) in session.radar.terrain.cells.iter().flatten() {
        let point = session
            .radar
            .point(rect, (f32::from(x) + 0.5, f32::from(y) + 0.5));
        assert_eq!(
            session
                .radar
                .target(1280, (f64::from(point.0), f64::from(point.1))),
            Some((f32::from(x) + 0.5, f32::from(y) + 0.5)),
            "marker/click source cell {x},{y}"
        );
    }
    capture(&session, "active");
    let mut restored = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    crate::snapshot::restore(&mut restored, &before).unwrap();
    assert!(restored.radar.ready());
    assert_eq!(restored.sim.state_hash(), hash);
    let mut original_frame = vec![0; 1280 * 800 * 4];
    let mut restored_frame = original_frame.clone();
    sidebar::draw(
        &mut original_frame,
        1280,
        800,
        &session,
        Category::Structures,
        None,
    );
    sidebar::draw(
        &mut restored_frame,
        1280,
        800,
        &restored,
        Category::Structures,
        None,
    );
    assert_eq!(
        original_frame, restored_frame,
        "active radar is reconstructed on load"
    );
    // The viewport changes only pixels inside the active fitted image.
    let baseline = original_frame.clone();
    session
        .radar
        .draw_viewport(&mut original_frame, 1280, 800, [(79.0, 75.0), (94.0, 80.0)]);
    let changed: Vec<_> = original_frame
        .as_chunks::<4>()
        .0
        .iter()
        .zip(baseline.as_chunks::<4>().0)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| (i % 1280, i / 1280))
        .collect();
    assert!(changed.len() > 10);
    assert!(changed.iter().all(|&(x, y)| x >= rect.x as usize
        && x < (rect.x + rect.w) as usize
        && y >= rect.y as usize
        && y < (rect.y + rect.h) as usize));
    // A removed unit must disappear rather than remain baked into the map.
    let unit = restored
        .sim
        .units()
        .iter()
        .find(|u| u.owner == OWNER_PLAYER)
        .unwrap()
        .clone();
    let marker = restored.radar.point(
        rect,
        (unit.x.raw() as f32 / 256.0, unit.y.raw() as f32 / 256.0),
    );
    let at = (marker.1 as usize * 1280 + marker.0 as usize) * 4;
    assert_eq!(
        &restored_frame[at..at + 4],
        &restored.radar.object_color(OWNER_PLAYER)
    );
    restored.step(&[Command::Remove { id: unit.id }]);
    sidebar::draw(
        &mut restored_frame,
        1280,
        800,
        &restored,
        Category::Structures,
        None,
    );
    assert_ne!(
        &restored_frame[at..at + 4],
        &restored.radar.object_color(OWNER_PLAYER)
    );
    let plant = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == OWNER_PLAYER && b.power > 0)
        .unwrap()
        .clone();
    let destroy: Vec<_> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == OWNER_PLAYER && b.power > 0)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    session.step(&destroy);
    assert!(!session.radar.ready());
    assert!(session.radar.target(1280, (1200.0, 100.0)).is_none());
    for phase in (0..=32).rev() {
        assert_eq!(session.radar.clock.frame, phase);
        capture(&session, &format!("closing-{:02}", 32 - phase));
        if phase > 0 {
            session.step(&[]);
            session.step(&[]);
        }
    }
    save(&session, "low-power");
    session.step(&[Command::PlaceBuilding {
        kind: plant.kind,
        x: plant.cell.0,
        y: plant.cell.1,
        w: plant.size.0,
        h: plant.size.1,
        hp: plant.max_hp,
        armor: plant.armor,
        power: plant.power,
        owner: OWNER_PLAYER,
        role: plant.role,
    }]);
    for _ in 0..64 {
        session.step(&[]);
    }
    assert!(session.radar.ready());
    capture(&session, "recovered");
    save(&session, "recovered");
    session.step(&[Command::Destroy { id: provider }]);
    assert!(!session.radar.ready());
    for _ in 0..64 {
        session.step(&[]);
    }
    assert_eq!(session.radar.clock.frame, 0);
    capture(&session, "provider-lost");
    save(&session, "provider-lost");
}

#[test]
fn tileset_budget_error_reaches_radar_terrain_loading() {
    let fs = AssetFs::from_files(vec![(
        "TEMPERATMD.INI".into(),
        b"[TileSet0000]\nFileName=Clear\nTilesInSet=4294967295\n".to_vec(),
    )]);
    let map = MapFile::parse("[Map]\nTheater=TEMPERATE\nSize=0,0,2,2\nLocalSize=0,0,2,2\n[IsoMapPack5]\n1=CAAEABUAAAAAEQAA\n").unwrap();
    let error = Terrain::load(&fs, &map).err().unwrap();
    assert_eq!(
        error,
        "radar tileset: theater tile index exceeds 65536 filenames"
    );
    println!("Radar terrain loader returned: {error}");
}

#[test]
fn map_extent_budget_revalidates_programmatic_radar_maps_before_loading() {
    let mut map=MapFile::parse("[Map]\nTheater=TEMPERATE\nSize=0,0,2,2\nLocalSize=0,0,2,2\n[IsoMapPack5]\n1=CAAEABUAAAAAEQAA\n").unwrap();
    map.tiles.push(yr_formats::map::IsoTile {
        x: u16::MAX,
        y: u16::MAX,
        tile: Some(0),
        subtile: 0,
        level: 0,
    });
    let fs = AssetFs::from_files(Vec::new());
    assert_eq!(
        Terrain::load(&fs, &map).err().unwrap(),
        "map terrain coordinate extent exceeds 262144 cells"
    );
    println!("Programmatic radar input rejects excessive declared terrain extents before loading assets or coordinate-sized buffers.");
}
