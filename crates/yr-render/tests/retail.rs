//! Retail terrain-grid test: build a sim TerrainGrid from a real
//! temperate map and check passability on known cells.
//! Skips when `~/Games/RA2YR/prefix/drive_c/Westwood/RA2` is absent.

use std::collections::HashMap;
use std::path::PathBuf;
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, MapFile, MixArchive, TileSetIndex, TmpFile};
use yr_sim::terrain::{Ground, Loco, TerrainGrid};

fn install_dir() -> Option<PathBuf> {
    let dir =
        PathBuf::from(std::env::var("HOME").ok()?).join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    dir.is_dir().then_some(dir)
}

#[test]
fn temperate_map_builds_a_sane_terrain_grid() {
    let Some(dir) = install_dir() else {
        eprintln!("skip: retail install not present");
        return;
    };
    let mix_data = std::fs::read(dir.join("mapsmd03.mix")).expect("mapsmd03 readable");
    let mix = MixArchive::parse(&mix_data).expect("mapsmd03 parses");
    let mut found = None;
    for e in mix.entries() {
        let text = String::from_utf8_lossy(mix.get_by_id(e.id).unwrap());
        if !text.contains("[IsoMapPack5]") {
            continue;
        }
        let map = MapFile::parse(&text).expect("map parses");
        if map.theater == "TEMPERATE" && map.tiles.len() > 5_000 {
            found = Some(map);
            break;
        }
    }
    let map = found.expect("a temperate map exists");
    let mut fs = AssetFs::open(&dir).expect("install opens");
    fs.set_theater(Theater::Temperate);
    let ini_name = Theater::Temperate
        .tileset_inis()
        .iter()
        .find(|n| fs.read(n).is_some())
        .unwrap();
    let ini = IniFile::parse(&String::from_utf8_lossy(fs.read(ini_name).unwrap()));
    let index = TileSetIndex::parse(&ini, Theater::Temperate.tile_extension());

    let mut tmps: HashMap<String, TmpFile> = HashMap::new();
    let mut rows: Vec<(u16, u16, u8, u8, u8)> = Vec::new();
    let mut water_cell = None;
    let mut cliff_cell = None;
    for t in &map.tiles {
        let Some(id) = t.tile else { continue };
        let Some(name) = index.file_for(id) else {
            continue;
        };
        let tmp = tmps.entry(name.to_string()).or_insert_with(|| {
            TmpFile::parse(fs.read(name).expect("TMP readable")).expect("TMP parses")
        });
        let Some(cell) = tmp.tiles.get(t.subtile as usize).and_then(Option::as_ref) else {
            continue;
        };
        rows.push((t.x, t.y, t.level, cell.terrain_type, cell.ramp_type));
        if cell.terrain_type == 9 && water_cell.is_none() {
            water_cell = Some((t.x, t.y));
        }
        if cell.terrain_type == 15 && cliff_cell.is_none() {
            cliff_cell = Some((t.x, t.y));
        }
    }
    // Iso tile coordinates run past the Size= rectangle (diamond map),
    // so the grid spans the tile extents.
    let (w, h) = rows.iter().fold((0usize, 0usize), |(w, h), r| {
        (w.max(r.0 as usize + 1), h.max(r.1 as usize + 1))
    });
    let grid = TerrainGrid::from_tiles(0, 0, w, h, rows.into_iter());
    let counts = grid.ground_counts();
    let count = |g: Ground| {
        counts
            .iter()
            .find(|(k, _)| *k == g)
            .map(|e| e.1)
            .unwrap_or(0)
    };
    assert!(
        count(Ground::Water) > 100,
        "temperate map has water: {counts:?}"
    );
    assert!(
        count(Ground::Cliff) > 100,
        "temperate map has cliffs: {counts:?}"
    );
    assert!(
        count(Ground::Clear) > 1_000,
        "temperate map is mostly clear: {counts:?}"
    );

    let (wx, wy) = water_cell.expect("water cell found");
    assert!(
        !grid.passable(Loco::Track, wx, wy),
        "vehicles cannot enter water"
    );
    assert!(grid.passable(Loco::Ship, wx, wy), "ships can enter water");
    let (cx, cy) = cliff_cell.expect("cliff cell found");
    assert!(!grid.passable(Loco::Foot, cx, cy), "cliffs block infantry");

    // The grid plugs into the sim and changes its fingerprint.
    let mut sim = yr_sim::Sim::new(1);
    let before = sim.state_hash();
    sim.set_terrain(grid);
    assert_ne!(
        sim.state_hash(),
        before,
        "terrain is part of the state hash"
    );
}

/// Live-object sprites resolve for the sandbox's units and base on EB1:
/// infantry SHP, voxel tank, NewTheater building art. With
/// `OPENYR_SPRITE_DUMP=<dir>` the 8 tank octants are written as PNGs
/// (eyeball check of facing).
#[test]
fn sprite_bank_resolves_sandbox_art() {
    let Some(dir) = install_dir() else {
        eprintln!("skip: retail install not present");
        return;
    };
    let mut bank =
        yr_render::sprites::SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").expect("bank opens");
    let remap = bank.house_color("Americans");
    for id in [
        "E1", "MTNK", "GACNST", "GAWEAP", "GAPILE", "GAREFN", "GAPOWR",
    ] {
        let s = bank.get(id, 0, remap).unwrap_or_else(|| panic!("{id} art"));
        assert!(s.w > 0 && s.h > 0, "{id} empty");
        assert!(
            s.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 255),
            "{id} blank"
        );
    }
    // Infantry facings pick distinct standing frames.
    let a = bank.get("E1", 0, remap).unwrap().rgba.clone();
    let b = bank.get("E1", 128, remap).unwrap().rgba.clone();
    assert_ne!(a, b, "facing changes the infantry frame");
    if let Ok(out) = std::env::var("OPENYR_SPRITE_DUMP") {
        std::fs::create_dir_all(&out).unwrap();
        for o in 0..8u8 {
            let s = bank.get("MTNK", o * 32, remap).unwrap();
            let png = yr_render::png::encode_rgba(s.w, s.h, &s.rgba);
            std::fs::write(format!("{out}/mtnk-sim{:03}.png", o * 32), png).unwrap();
        }
    }
}
