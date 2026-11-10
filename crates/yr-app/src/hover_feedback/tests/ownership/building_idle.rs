use super::*;

struct Install(PathBuf);
impl Drop for Install {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn shp(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [0u16, 5, 5, frames.len() as u16] {
        bytes.extend(value.to_le_bytes());
    }
    for (i, _) in frames.iter().enumerate() {
        for value in [0u16, 0, 5, 5] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(1u32.to_le_bytes());
        bytes.extend([0; 8]);
        bytes.extend((8 + 24 * frames.len() + 25 * i).to_le_bytes()[..4].iter());
    }
    for frame in frames {
        bytes.extend(frame);
    }
    assert!(yr_formats::ShpFile::parse(&bytes).is_ok());
    bytes
}
fn install() -> Install {
    let path = std::env::temp_dir().join(format!(
        "openyr-idle-building-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    let mut palette = vec![0; 768];
    for (index, rgb) in [
        (1, [12, 16, 12]),
        (2, [0, 63, 0]),
        (3, [0, 0, 63]),
        (4, [63, 63, 0]),
    ] {
        palette[index * 3..index * 3 + 3].copy_from_slice(&rgb);
    }
    std::fs::write(path.join("ISOTEM.PAL"), &palette).unwrap();
    std::fs::write(path.join("UNITTEM.PAL"), &palette).unwrap();
    std::fs::write(path.join("RULESMD.INI"),"[Colors]\nFixtureBlue=160,255,200\nLightGrey=0,0,170\n[FixtureHouse]\nColor=FixtureBlue\n[BuildingTypes]\n0=GACNST\n1=BADBODY\n[GACNST]\nStrength=100\n[BADBODY]\nStrength=100\n").unwrap();
    std::fs::write(path.join("ARTMD.INI"),"[BADBODY]\nImage=NOTTHERE\nFoundation=1x1\n[GACNST]\nFoundation=1x1\nActiveAnim=BLINK\nActiveAnimTwo=MISSING\nActiveAnimThree=BROKEN\n[BLINK]\nLoopStart=0\nLoopEnd=2\nRate=225\n[BROKEN]\nImage=BLINK\nRate=0\n").unwrap();
    std::fs::write(path.join("IDLE.MAP"),"[Map]\nTheater=TEMPERATE\nSize=0,0,32,32\nLocalSize=0,0,32,32\n[IsoMapPack5]\n1=EwAPACAUABQAAAAAAAAAAAAAAAARAAA=\n[Structures]\n0=FixtureHouse,GACNST,256,20,20,0\n1=FixtureHouse,BADBODY,256,21,20,0\n").unwrap();
    std::fs::write(
        path.join("TEMPERATMD.INI"),
        "[TileSet0000]\nFileName=Clear\nTilesInSet=1\n",
    )
    .unwrap();
    let mut tmp = Vec::new();
    for value in [1u32, 1, 8, 4, 20] {
        tmp.extend(value.to_le_bytes());
    }
    tmp.extend([0; 52]);
    tmp[61] = 13;
    tmp.extend([1; 16]);
    std::fs::write(path.join("CLEAR01.TEM"), tmp).unwrap();
    std::fs::write(
        path.join("GACNST.SHP"),
        shp(&[vec![22; 25], vec![4; 25], vec![0; 25], vec![0; 25]]),
    )
    .unwrap();
    let mut a = vec![0; 25];
    a[12] = 2;
    let mut b = vec![0; 25];
    b[12] = 3;
    std::fs::write(
        path.join("BLINK.SHP"),
        shp(&[a, b, vec![0; 25], vec![0; 25]]),
    )
    .unwrap();
    Install(path)
}

#[test]
fn building_idle_background_excludes_known_structures_but_preview_keeps_them() {
    let dir = install();
    let path = dir.0.to_str().unwrap();
    let background = yr_render::render_map_background(path, "IDLE.MAP", 1).unwrap();
    let preview = yr_render::render_map(path, "IDLE.MAP", 1).unwrap();
    assert_eq!(
        (
            background.width,
            background.height,
            background.origin_x,
            background.origin_y
        ),
        (
            preview.width,
            preview.height,
            preview.origin_x,
            preview.origin_y
        )
    );
    assert_ne!(
        background.rgba, preview.rgba,
        "the preview retains its static body; match background must not bake it"
    );
    let bank = yr_render::sprites::SpriteBank::open(path, "IDLE.MAP").unwrap();
    assert_eq!(
        bank.map_structure_color("gacnst", (20, 20)),
        Some(bank.house_color("FixtureHouse"))
    );
    assert_ne!(
        bank.map_structure_color("GACNST", (20, 20)),
        Some(bank.house_color(""))
    );
    assert_eq!(bank.map_structure_color("GACNST", (21, 20)), None);
    assert_eq!(bank.map_structure_color("BADBODY", (21, 20)), None);
    assert!(
        background
            .rgba
            .as_chunks::<4>()
            .0
            .contains(&[255, 0, 200, 255]),
        "unsupported bodies retain the original static fallback"
    );
    println!("Authored match/preview maps retain identical layout but omit the match's known building body; original map-house remap is recovered by type/cell rather than neutral fallback.");
}

#[test]
fn building_idle_neutral_layers_animate_without_production_or_saved_state_changes() {
    let dir = install();
    let path = dir.0.to_str().unwrap();
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, own, _) = scene(size);
        app.map = Some(yr_render::render_map_background(path, "IDLE.MAP", 1).unwrap());
        let session = app.session.as_mut().unwrap();
        let kind = session.kinds.iter().position(|k| k == "GACNST").unwrap() as u16;
        let id = session.sim.next_id_peek();
        session.sim.step(&[Command::PlaceBuilding {
            kind,
            x: 20,
            y: 20,
            w: 1,
            h: 1,
            hp: 100,
            armor: 0,
            power: 0,
            owner: yr_sim::OWNER_NEUTRAL,
            role: yr_sim::Role::Combat,
        }]);
        app.sprites = Some(std::cell::RefCell::new(
            yr_render::sprites::SpriteBank::open(path, "IDLE.MAP").unwrap(),
        ));
        app.pending = vec![Command::Stop { id: own }];
        point_at(&mut app, id, size);
        app.pointer_moved((50.0, 50.0));
        let map_color = app
            .sprites
            .as_ref()
            .unwrap()
            .borrow()
            .house_color("FixtureHouse");
        let body = app
            .sprites
            .as_ref()
            .unwrap()
            .borrow_mut()
            .get_building(
                "GACNST",
                map_color,
                yr_render::sprites::BuildingAnimation {
                    powered: true,
                    ..Default::default()
                },
            )
            .unwrap()
            .layers[0]
            .rgba[..4]
            .to_vec();
        let mut captured = Vec::new();
        for phase in 0..2 {
            let session = app.session.as_ref().unwrap();
            assert!(session.sim.production_of(yr_sim::OWNER_NEUTRAL).is_empty());
            let before = yr_app::snapshot::encode(session).unwrap();
            let hash = session.sim.state_hash();
            let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
            app.compose(&mut frame, size.0, size.1);
            let center = ((size.1 / 2 * size.0 + (size.0 - SIDEBAR_W) / 2) * 4) as usize;
            assert_eq!(
                &frame[center..center + 4],
                if phase == 0 {
                    &[0, 255, 0, 255]
                } else {
                    &[0, 0, 255, 255]
                }
            );
            assert!(frame
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p.as_slice() == body));
            assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                before
            );
            assert_eq!(app.pending, [Command::Stop { id: own }]);
            assert_eq!(app.selected, [own].into());
            capture(&app, size, phase, &frame, id, map_color);
            captured.push(frame);
            app.session.as_mut().unwrap().sim.step(&[]);
        }
        assert_ne!(captured[0], captured[1]);
        app.session
            .as_mut()
            .unwrap()
            .sim
            .step(&[Command::Destroy { id }]);
        let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
        app.compose(&mut frame, size.0, size.1);
        let center = ((size.1 / 2 * size.0 + (size.0 - SIDEBAR_W) / 2) * 4) as usize;
        let (mx, my) =
            app.window_to_map((f64::from(size.0 - SIDEBAR_W) * 0.5, f64::from(size.1) * 0.5));
        let map = app.map.as_ref().unwrap();
        let source = ((my as u32 * map.width + mx as u32) * 4) as usize;
        assert_eq!(
            &frame[center..center + 4],
            &map.rgba[source..source + 4],
            "destroyed building leaves no baked body"
        );
    }
    println!("Three authored live neutral-building layouts advance idle layers with no production, retain source map-house body colors and skip missing/malformed optional overlays. Draws preserve full Session bytes/hash, selection and pending orders; destruction leaves no baked body.");
}
fn capture(app: &App, size: (u32, u32), phase: usize, frame: &[u8], id: u32, color: [u8; 3]) {
    let Some(dir) = std::env::var_os("OPENYR_BUILDING_IDLE_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let name = format!("{}-{phase}", size.0);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(size.0, size.1, frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    std::fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
        "tick":session.sim.tick(),"building":id,"owner":"neutral","map_house_remap":color,
        "sim_hash":format!("{:016x}",session.sim.state_hash()),"snapshot_bytes":yr_app::snapshot::encode(session).unwrap().len(),
        "selected":app.selected,"pending":format!("{:?}",app.pending),"production_idle":true,"retail_art":false,
    })).unwrap()).unwrap();
}
