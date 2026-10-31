//! Retail terrain support, including sparse map boundaries and clear sentinels.

use std::{collections::BTreeSet, path::PathBuf};
use yr_assets::{AssetFs, Theater};
use yr_formats::MapFile;
use yr_sim::terrain::Loco;

fn retail() -> Option<(PathBuf, AssetFs, MapFile)> {
    let dir =
        PathBuf::from(std::env::var_os("HOME")?).join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return None;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let map = MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).unwrap();
    Some((dir, fs, map))
}

#[test]
fn eb1_sparse_terrain_supports_every_spawn_and_rejects_missing_cells() {
    let Some((dir, _, map)) = retail() else {
        return;
    };
    let session = yr_app::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    let present: BTreeSet<_> = map.tiles.iter().map(|t| (t.x, t.y)).collect();
    let grid = session.sim.terrain().unwrap();
    let mut missing = 0;
    let mut traversable = 0;
    let mut placements = 0;
    let mut outside_crop = 0;
    for ly in 0..grid.height() {
        for lx in 0..grid.width() {
            let (x, y) = grid.cell_at_local(lx, ly);
            for loco in [Loco::Foot, Loco::Track, Loco::Ship, Loco::Fly] {
                if grid.passable(loco, x, y) {
                    assert!(
                        present.contains(&(x, y)),
                        "{loco:?} enters absent ({x},{y})"
                    );
                    traversable += 1;
                }
            }
            if !present.contains(&(x, y)) {
                missing += 1;
                assert!(grid.cell(x, y).is_none());
                for slot in 0..session.buildables.len() {
                    assert!(!session.can_place(slot, (x, y)));
                }
            } else {
                for slot in 0..session.buildables.len() {
                    placements += usize::from(session.can_place(slot, (x, y)));
                }
                let sx = i32::from(x) - i32::from(y) + map.size.2 - 1;
                let sy = (i32::from(x) + i32::from(y) - map.size.2 - 1).div_euclid(2);
                let (cx, cy, w, h) = map.local_size;
                if (sx < cx * 2 || sx >= (cx + w) * 2 || sy < cy || sy >= cy + h)
                    && grid.passable(Loco::Track, x, y)
                {
                    outside_crop += 1;
                }
            }
        }
    }
    assert!(missing > 0 && traversable > 0 && placements > 0);
    assert!(
        outside_crop > 0,
        "authored border tiles remain available pending oracle bounds calibration"
    );
    for unit in session.sim.units() {
        let cell = (unit.x.to_int_floor() as u16, unit.y.to_int_floor() as u16);
        assert!(
            present.contains(&cell),
            "spawned unit {} at {cell:?}",
            unit.id
        );
    }
    for building in session.sim.buildings() {
        for dy in 0..building.size.1 {
            for dx in 0..building.size.0 {
                assert!(present.contains(&(building.cell.0 + dx, building.cell.1 + dy)));
            }
        }
    }
    let bytes = session.sim.save_bytes().unwrap();
    let restored = yr_sim::Sim::from_save_bytes(&bytes).unwrap();
    assert_eq!(session.sim.state_hash(), restored.state_hash());
    println!("EB1 terrain {:?}, LocalSize {:?}, {} authored records, {missing} absent grid cells, {traversable} traversable locomotor/cell pairs, {placements} legal placement origins, {outside_crop} track cells outside the radar crop; restored hash {:016x}", map.size, map.local_size, present.len(), restored.state_hash());
}

#[test]
fn retail_clear_sentinels_keep_the_same_terrain_as_tile_zero() {
    let Some((_, mut fs, mut map)) = retail() else {
        return;
    };
    let mut clear = *map
        .tiles
        .iter()
        .find(|t| t.tile == Some(0) && t.subtile == 0)
        .unwrap();
    let at = (clear.x, clear.y);
    map.tiles = vec![clear];
    map.overlay.fill(yr_formats::map::NO_OVERLAY);
    let theater = Theater::from_map_name(&map.theater).unwrap();
    fs.set_theater(theater);
    let indexed = yr_render::build_terrain(&mut fs, &map, &[], theater).unwrap();
    clear.tile = None;
    map.tiles = vec![clear];
    let sentinel = yr_render::build_terrain(&mut fs, &map, &[], theater).unwrap();
    assert_eq!(sentinel.cell(at.0, at.1), indexed.cell(at.0, at.1));
    assert!(sentinel.passable(Loco::Track, at.0, at.1));
    assert_eq!(sentinel.hash(), indexed.hash());
}
