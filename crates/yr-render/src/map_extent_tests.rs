use super::*;

fn map() -> MapFile {
    let mut map = MapFile::parse("[Map]\nTheater=TEMPERATE\nSize=0,0,2,2\nLocalSize=0,0,2,2\n[IsoMapPack5]\n1=CAAEABUAAAAAEQAA\n").unwrap();
    map.tiles.push(yr_formats::map::IsoTile {
        x: 1,
        y: 1,
        tile: Some(0),
        subtile: 0,
        level: 0,
    });
    map
}

fn tmp(extra: Option<(i32, i32, i32, i32)>) -> Vec<u8> {
    let mut data = Vec::new();
    for n in [1u32, 1, 8, 4, 20] {
        data.extend(n.to_le_bytes());
    }
    let (tx, ty, ex, ey) = extra.unwrap_or((0, 0, 0, 0));
    data.extend(tx.to_le_bytes());
    data.extend(ty.to_le_bytes());
    for n in [if extra.is_some() { 68u32 } else { 0 }, 0, 0] {
        data.extend(n.to_le_bytes());
    }
    for n in [
        ex,
        ey,
        i32::from(extra.is_some()),
        i32::from(extra.is_some()),
    ] {
        data.extend(n.to_le_bytes());
    }
    data.extend(u32::from(extra.is_some()).to_le_bytes());
    data.extend([0, 13, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0]);
    assert_eq!(data.len(), 72);
    data.extend([1; 16]);
    if extra.is_some() {
        data.push(2);
    }
    assert!(TmpFile::parse(&data).is_ok());
    data
}

#[test]
fn map_extent_budget_blocks_programmatic_terrain_before_asset_access() {
    let mut map = map();
    map.tiles[0].x = u16::MAX;
    map.tiles[0].y = u16::MAX;
    let mut fs = AssetFs::from_files(Vec::new());
    assert_eq!(
        build_terrain(&mut fs, &map, &[], Theater::Temperate)
            .err()
            .unwrap(),
        "map terrain coordinate extent exceeds 262144 cells"
    );
}

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn install(template: Vec<u8>) -> Scratch {
    let stem = format!(
        "openyr-map-bounds-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let dir = scratch(&std::env::temp_dir(), &stem);
    let mut palette = vec![0; 768];
    palette[3..6].copy_from_slice(&[10, 20, 30]);
    palette[6..9].copy_from_slice(&[40, 50, 60]);
    for (name, bytes) in [
        (
            "TEMPERATMD.INI",
            b"[TileSet0000]\nFileName=Clear\nTilesInSet=1\n".to_vec(),
        ),
        ("ISOTEM.PAL", palette),
        ("RULESMD.INI", b"[General]\n".to_vec()),
        ("ARTMD.INI", b"[General]\n".to_vec()),
        ("CLEAR01.TEM", template),
    ] {
        fs::write(dir.0.join(name), bytes).unwrap();
    }
    // One authored cell (1,1), tile 0, with the normal zero trailer.
    fs::write(dir.0.join("SMALL.MAP"),"[Map]\nTheater=TEMPERATE\nSize=0,0,2,2\nLocalSize=0,0,2,2\n[IsoMapPack5]\n1=EwAPACABAAEAAAAAAAAAAAAAAAARAAA=\n").unwrap();
    dir
}

fn scratch(root: &std::path::Path, stem: &str) -> Scratch {
    for attempt in 0..128 {
        let path = root.join(format!("{stem}-{attempt}"));
        match fs::create_dir(&path) {
            // Cleanup belongs only to the caller that created this directory.
            Ok(()) => return Scratch(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("cannot create {}: {e}", path.display()),
        }
    }
    panic!("map-extent fixture exhausted 128 scratch candidates");
}

#[test]
fn map_extent_fixture_collision_preserves_existing_and_parallel_owners() {
    let root = install(tmp(None));
    let existing = root.0.join("collision-0");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("keep"), b"preexisting fixture").unwrap();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| scratch(&root.0, "collision"));
        let b = scope.spawn(|| scratch(&root.0, "collision"));
        let (a, b) = (a.join().unwrap(), b.join().unwrap());
        assert_ne!(a.0, b.0);
        fs::write(b.0.join("owned"), b"other owner").unwrap();
        let released = a.0.clone();
        drop(a);
        assert!(!released.exists());
        assert_eq!(fs::read(b.0.join("owned")).unwrap(), b"other owner");
    });
    assert_eq!(
        fs::read(existing.join("keep")).unwrap(),
        b"preexisting fixture"
    );
}

#[test]
fn map_extent_budget_preserves_small_raster_and_cancelling_extreme_extra_offsets() {
    let baseline = install(tmp(Some((0, 0, 0, 0))));
    let extreme = install(tmp(Some((i32::MIN, i32::MIN, i32::MIN, i32::MIN))));
    let a = render_map(baseline.0.to_str().unwrap(), "SMALL.MAP", 1).unwrap();
    let b = render_map(extreme.0.to_str().unwrap(), "SMALL.MAP", 1).unwrap();
    assert_eq!(
        (a.width, a.height, a.origin_x, a.origin_y),
        (240, 270, -120, -170)
    );
    assert_eq!(
        (a.width, a.height, a.origin_x, a.origin_y),
        (b.width, b.height, b.origin_x, b.origin_y)
    );
    assert_eq!(a.rgba, b.rgba);
    let extra_pixel = (200 * 240 + 90) * 4;
    assert_eq!(&a.rgba[extra_pixel..extra_pixel + 4], &[162, 203, 243, 255]);
    for scale in [2, 4] {
        let down = render_map(extreme.0.to_str().unwrap(), "SMALL.MAP", scale).unwrap();
        assert_eq!(
            (down.width, down.height),
            (240 / scale, 270u32.div_ceil(scale))
        );
    }
    let mut fs = AssetFs::open(&baseline.0).unwrap();
    let grid = build_terrain(&mut fs, &map(), &[], Theater::Temperate).unwrap();
    let mut sim = yr_sim::Sim::new(42);
    sim.set_terrain(grid);
    let before = sim.save_bytes().unwrap();
    let hash = sim.state_hash();
    let expected_grid = yr_sim::TerrainGrid::from_tiles(0, 0, 2, 2, [(1, 1, 0, 13, 0)].into_iter());
    let mut expected_sim = yr_sim::Sim::new(42);
    expected_sim.set_terrain(expected_grid);
    assert_eq!(hash, expected_sim.state_hash());
    assert_eq!(before, expected_sim.save_bytes().unwrap());
    if let Some(dir) = std::env::var_os("OPENYR_MAP_BOUNDS_PROOF") {
        let dir = std::path::PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("small-native.png"),
            crate::png::encode_rgba(a.width, a.height, &a.rgba),
        )
        .unwrap();
        fs::write(
            dir.join("small-extreme-cancel.png"),
            crate::png::encode_rgba(b.width, b.height, &b.rgba),
        )
        .unwrap();
        fs::write(dir.join("small-state.txt"),format!("size={}x{}\norigin={},{}\nrgba_equal=true\nextra_pixel=162,203,243,255\nsim_hash={hash:016x}\nsim_save_bytes={}\nsim_save_matches_expected_sparse_grid=true\nretail_art=false\nsession_present=false\n",a.width,a.height,a.origin_x,a.origin_y,before.len())).unwrap();
    }
    println!("Tiny 240x270 raster and cancelling i32::MIN TMP offsets match byte-for-byte; scale 2/4 preserve output, and actual terrain setup matches the independently specified sparse Sim bytes/hash.");
}

#[test]
fn map_extent_budget_rejects_huge_extra_bounds_before_downscaling() {
    let dir = install(tmp(Some((0, 0, i32::MAX, i32::MIN))));
    for scale in [1, 2, 4] {
        let error = render_map(dir.0.to_str().unwrap(), "SMALL.MAP", scale)
            .err()
            .unwrap();
        assert!(error.contains("raster exceeds"), "{error}");
    }
    println!("Extreme TMP extra bounds return raster-budget errors at scales 1/2/4 before allocating the native canvas.");
}
