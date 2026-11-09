use super::*;
use yr_formats::map::IsoTile;
use yr_sim::{terrain::Ground, Sim, TerrainGrid};

fn map() -> MapFile {
    let mut map = MapFile::parse("[Map]\nTheater=TEMPERATE\nSize=0,0,8,8\nLocalSize=0,0,8,8\n[IsoMapPack5]\n1=CAAEABUAAAAAEQAA\n").unwrap();
    map.tiles.push(IsoTile {
        x: 2,
        y: 3,
        tile: Some(0),
        subtile: 0,
        level: 5,
    });
    map
}

fn assets(template: Option<Vec<u8>>) -> AssetFs {
    let mut files = vec![(
        "TEMPERATMD.INI".into(),
        b"[TileSet0000]\nFileName=Clear\nTilesInSet=1\n".to_vec(),
    )];
    if let Some(template) = template {
        files.push(("CLEAR01.TEM".into(), template));
    }
    AssetFs::from_files(files)
}

fn template() -> Vec<u8> {
    // Authored 1x1 template with one 8x4 color diamond, no extra/depth data.
    let mut bytes = Vec::new();
    for value in [1u32, 1, 8, 4, 20] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend([0; 52]);
    bytes[20 + 40] = 3;
    bytes[20 + 41] = 14;
    bytes[20 + 42] = 7;
    bytes.extend([1; 16]);
    bytes
}

#[test]
fn tmp_loading_reports_a_missing_referenced_template_without_panicking() {
    let mut fs = assets(None);
    let error = build_terrain(&mut fs, &map(), &[], Theater::Temperate)
        .err()
        .unwrap();
    assert_eq!(error, "terrain template CLEAR01.TEM is missing");
    println!("Missing TMP returned: {error}");
}

#[test]
fn tmp_loading_reports_malformed_referenced_templates_with_filename_and_reason() {
    for (bytes, reason) in [
        (b"bad".to_vec(), "not a TS/RA2 TMP file"),
        (template()[..72].to_vec(), "TMP cell 0 is malformed"),
    ] {
        let mut fs = assets(Some(bytes));
        let error = build_terrain(&mut fs, &map(), &[], Theater::Temperate)
            .err()
            .unwrap();
        assert_eq!(error, format!("terrain template CLEAR01.TEM: {reason}"));
        println!("Malformed TMP returned: {error}");
    }
}

#[test]
fn tmp_loading_preserves_valid_metadata_repeated_clear_references_and_saved_terrain() {
    let mut map = map();
    map.tiles.push(IsoTile {
        x: 4,
        y: 5,
        tile: None,
        subtile: 0,
        level: 2,
    });
    let mut fs = assets(Some(template()));
    let grid = build_terrain(&mut fs, &map, &[], Theater::Temperate).unwrap();
    assert_eq!((grid.width(), grid.height()), (5, 6));
    for (x, y, height) in [(2, 3, 5), (4, 5, 2)] {
        let cell = grid.cell(x, y).unwrap();
        assert_eq!(
            (cell.ground, cell.height, cell.ramp),
            (Ground::Rough, height, 7)
        );
    }
    assert!(grid.cell(3, 3).is_none());
    let mut actual = Sim::new(42);
    actual.set_terrain(grid);
    let mut expected = Sim::new(42);
    expected.set_terrain(TerrainGrid::from_tiles(
        0,
        0,
        5,
        6,
        [(2, 3, 5, 14, 7), (4, 5, 2, 14, 7)].into_iter(),
    ));
    let saved = actual.save_bytes().unwrap();
    assert_eq!(saved, expected.save_bytes().unwrap());
    assert_eq!(actual.state_hash(), expected.state_hash());
    let restored = Sim::from_save_bytes(&saved).unwrap();
    assert_eq!(restored.save_bytes().unwrap(), saved);
    assert_eq!(restored.state_hash(), actual.state_hash());
    println!("Valid TMP controls preserve Rough terrain, authored map heights, ramp 7, clear-sentinel reuse, full Sim save bytes ({}) and hash {:016x}.", saved.len(), actual.state_hash());
}

#[test]
fn tmp_loading_keeps_unresolved_templates_and_absent_subtiles_sparse() {
    let mut map = map();
    map.tiles.push(IsoTile {
        x: 3,
        y: 3,
        tile: Some(0),
        subtile: 1,
        level: 7,
    });
    map.tiles.push(IsoTile {
        x: 6,
        y: 6,
        tile: Some(42),
        subtile: 0,
        level: 1,
    });
    let mut fs = assets(Some(template()));
    let grid = build_terrain(&mut fs, &map, &[], Theater::Temperate).unwrap();
    assert!(grid.cell(2, 3).is_some());
    assert!(grid.cell(3, 3).is_none() && grid.cell(6, 6).is_none());
    let mut empty = template();
    empty[16..20].copy_from_slice(&0u32.to_le_bytes());
    let mut fs = assets(Some(empty));
    let grid = build_terrain(&mut fs, &map, &[], Theater::Temperate).unwrap();
    assert!(grid.cell(2, 3).is_none());
    println!("Unknown tile indexes, out-of-range subtiles and authored empty TMP cells remain absent ground.");
}
