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

/// Check live animation against independently selected retail body and
/// shadow canvases. Optional dumps contain EA art and stay outside git.
#[test]
fn infantry_animation_matches_retail_sequence_canvases() {
    use yr_render::sprites::{screen_octant, InfantryAnimation, SpriteBank, SHADOW_ALPHA};

    let Some(dir) = install_dir() else {
        eprintln!("skip: retail install not present");
        return;
    };
    let mut fs = AssetFs::open(&dir).expect("install opens");
    fs.set_theater(Theater::Temperate);
    let palette = yr_formats::Palette::parse(fs.read("UNITTEM.PAL").unwrap()).unwrap();
    let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    let remap = bank.house_color("Americans");
    let out = std::env::var("OPENYR_ANIMATION_DUMP").ok();
    if let Some(out) = &out {
        std::fs::create_dir_all(out).unwrap();
    }
    // artmd.ini: GISequence and GuardianGISequence both walk at 8,
    // idle at 56/71, but FireUp starts at 164 and 204 respectively.
    for (id, image, fire_start) in [("E1", "GI.SHP", 164), ("GGI", "GGI.SHP", 204)] {
        let shp = yr_formats::ShpFile::parse(fs.read(image).unwrap()).unwrap();
        for facing in (0..=224u8).step_by(32) {
            let direction = usize::from((8 - screen_octant(facing)) % 8);
            for phase in 0..15u64 {
                for (action, animation, expected) in [
                    (
                        "walk",
                        InfantryAnimation {
                            moving: true,
                            elapsed: phase * 2,
                            since_shot: None,
                        },
                        8 + direction * 6 + (phase % 6) as usize,
                    ),
                    (
                        "fire",
                        InfantryAnimation {
                            moving: false,
                            elapsed: 0,
                            since_shot: Some(phase * 2),
                        },
                        if phase < 6 {
                            fire_start + direction * 6 + phase as usize
                        } else {
                            direction
                        },
                    ),
                    (
                        "idle",
                        InfantryAnimation {
                            moving: false,
                            elapsed: 180 + phase * 2,
                            since_shot: None,
                        },
                        56 + phase as usize,
                    ),
                ] {
                    let sprite = bank.get_animated(id, facing, remap, animation).unwrap();
                    let body = shp.decode_frame_canvas(expected).unwrap();
                    let shadow = shp
                        .decode_frame_canvas(expected + shp.frames.len() / 2)
                        .unwrap();
                    for (p, &index) in body.iter().enumerate() {
                        let pixel = &sprite.rgba[p * 4..p * 4 + 4];
                        let alpha = if index != 0 {
                            255
                        } else if shadow[p] != 0 {
                            SHADOW_ALPHA
                        } else {
                            0
                        };
                        assert_eq!(
                            pixel[3], alpha,
                            "{id} {action} facing {facing} phase {phase}, pixel {p}"
                        );
                        if index != 0 && !(16..=31).contains(&index) {
                            assert_eq!(pixel, palette.rgba(index), "retail body pixel");
                        }
                    }
                    if let Some(out) = &out {
                        std::fs::write(
                            format!("{out}/{id}-{action}-{facing:03}-{phase:02}.png"),
                            yr_render::png::encode_rgba(sprite.w, sprite.h, &sprite.rgba),
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
}

/// Check each resolved layer against independent retail SHP canvases,
/// including damaged ranges, shadow flags, anchors and production gating.
#[test]
fn building_layers_match_retail_canvases() {
    use yr_formats::{Palette, ShpFile};
    use yr_render::sprites::{BuildingAnimation, Sprite, SpriteBank, SHADOW_ALPHA};

    let Some(dir) = install_dir() else { return };
    let mut fs = AssetFs::open(&dir).unwrap();
    fs.set_theater(Theater::Temperate);
    let palette = Palette::parse(fs.read("UNITTEM.PAL").unwrap()).unwrap();
    let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    let remap = bank.house_color("Americans");
    let out = std::env::var("OPENYR_BUILDING_DUMP").ok();
    if let Some(out) = &out {
        std::fs::create_dir_all(out).unwrap();
    }
    let check = |sprite: &Sprite, file: &str, frame, shadow, foundation: (i32, i32)| {
        let shp = ShpFile::parse(fs.read(file).unwrap()).unwrap();
        assert_eq!(
            (sprite.w, sprite.h),
            (u32::from(shp.width), u32::from(shp.height)),
            "{file}"
        );
        let (fx, fy) = foundation;
        assert_eq!(sprite.ax, i32::from(shp.width) / 2 - (fx - fy) * 15);
        assert_eq!(
            sprite.ay,
            i32::from(shp.height) / 2 - (fx + fy - 2) * 15 / 2
        );
        let body = shp.decode_frame_canvas(frame).unwrap();
        let mask = shp
            .decode_frame_canvas(frame + shp.frames.len() / 2)
            .unwrap();
        for (p, &index) in body.iter().enumerate() {
            let pixel = &sprite.rgba[p * 4..p * 4 + 4];
            if index != 0 {
                assert_eq!(pixel[3], 255, "{file}, frame {frame}, pixel {p}");
                if !(16..=31).contains(&index) {
                    assert_eq!(pixel, palette.rgba(index));
                }
            } else {
                assert_eq!(
                    pixel[3],
                    if shadow && mask[p] != 0 {
                        SHADOW_ALPHA
                    } else {
                        0
                    },
                    "{file}, frame {frame}, pixel {p}"
                );
            }
        }
    };
    for damaged in [false, true] {
        let d = usize::from(damaged);
        for tick in 0..24 {
            let phase = tick * 200 / 225;
            for (id, image, foundation, overlays, bib) in [
                (
                    "GACNST",
                    "GGCNST",
                    (4, 4),
                    vec![("GGCNST_A.SHP", d * 3 + phase as usize % 3, false)],
                    None,
                ),
                (
                    "GAPOWR",
                    "GGPOWR",
                    (2, 2),
                    vec![(
                        "GGPOWR_A.SHP",
                        d * 8 + (tick * 220 / 225) as usize % 8,
                        false,
                    )],
                    None,
                ),
                (
                    "GAPILE",
                    "GGPILE",
                    (3, 2),
                    vec![(
                        "GGPILE_A.SHP",
                        d * 16 + (tick * 300 / 225) as usize % 15,
                        true,
                    )],
                    None,
                ),
                (
                    "GAWEAP",
                    "GGWEAP",
                    (5, 3),
                    vec![
                        (
                            "GGWEAP_A.SHP",
                            d * 16 + phase as usize % if damaged { 13 } else { 15 },
                            false,
                        ),
                        ("GGWEAP_B.SHP", (tick * 400 / 225) as usize % 5, false),
                    ],
                    Some("GGWEAPBB.SHP"),
                ),
                (
                    "GAREFN",
                    "GGREFN",
                    (4, 3),
                    ["GGREFNL1.SHP", "GGREFNL2.SHP", "GGREFNL3.SHP"]
                        .map(|file| (file, phase as usize % 3, false))
                        .to_vec(),
                    Some("GGREFNBB.SHP"),
                ),
            ] {
                for production in [None, Some(tick)] {
                    let state = BuildingAnimation {
                        elapsed: tick,
                        production,
                        unloading: None,
                        damaged,
                        powered: true,
                    };
                    let sprites = bank.get_building(id, remap, state).unwrap();
                    let mut expected = vec![(format!("{image}.SHP"), d, true)];
                    if id == "GAWEAP" {
                        expected.extend([
                            ("GGWEAP_1.SHP".into(), d, false),
                            ("GGWEAP_4.SHP".into(), d, false),
                            ("GGWEAP_3.SHP".into(), d, false),
                            ("GGWEAP_2.SHP".into(), d, false),
                        ]);
                    }
                    expected.extend(
                        overlays
                            .iter()
                            .map(|&(file, frame, shadow)| (file.to_string(), frame, shadow)),
                    );
                    if id == "GACNST" && production.is_some() {
                        expected.push((
                            "GGCNST_B.SHP".into(),
                            d * 20 + phase as usize % 20,
                            !damaged,
                        ));
                    }
                    assert_eq!(sprites.layers.len(), expected.len(), "{id}");
                    for (sprite, (file, frame, shadow)) in sprites.layers.iter().zip(expected) {
                        check(sprite, &file, frame, shadow, foundation);
                    }
                    assert_eq!(sprites.bib.is_some(), bib.is_some());
                    if let Some((sprite, file)) = sprites.bib.zip(bib) {
                        check(sprite, file, d, true, foundation);
                    }
                    if let Some(out) = &out {
                        if production.is_some() {
                            // All probed layers share their parent's canvas and anchor.
                            let base = sprites.layers[0];
                            for before in [false, true] {
                                if before && id != "GAWEAP" {
                                    continue;
                                }
                                let layers: Vec<_> = sprites
                                    .bib
                                    .into_iter()
                                    .chain(sprites.layers.iter().copied().enumerate().filter_map(
                                        |(i, sprite)| {
                                            // Omit only the newly added shells to reproduce
                                            // the previous renderer at the same health/tick.
                                            (!before || !(1..=4).contains(&i)).then_some(sprite)
                                        },
                                    ))
                                    .collect();
                                let mut rgba = vec![0u8; base.rgba.len()];
                                for alpha in [SHADOW_ALPHA, 255] {
                                    for sprite in &layers {
                                        assert_eq!(
                                            (sprite.w, sprite.h, sprite.ax, sprite.ay),
                                            (base.w, base.h, base.ax, base.ay)
                                        );
                                        for (dst, src) in rgba
                                            .as_chunks_mut::<4>()
                                            .0
                                            .iter_mut()
                                            .zip(sprite.rgba.as_chunks::<4>().0)
                                        {
                                            if src[3] == alpha {
                                                dst.copy_from_slice(src);
                                            }
                                        }
                                    }
                                }
                                std::fs::write(
                                    format!(
                                        "{out}/{id}-{}-{tick:02}{}.png",
                                        if damaged { "damaged" } else { "normal" },
                                        if before { "-before" } else { "" }
                                    ),
                                    yr_render::png::encode_rgba(base.w, base.h, &rgba),
                                )
                                .unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
    // Static shell selection is independent of elapsed time, power and
    // queue progress. Soviet damaged sections redirect Image explicitly;
    // Allied and Yuri shells select the damaged half without a redirect.
    for (id, prefix, order) in [
        ("GAWEAP", "GGWEAP", [1, 4, 3, 2]),
        ("NAWEAP", "NGWEAP", [1, 3, 4, 2]),
        ("YAWEAP", "YGWEAP", [1, 4, 3, 2]),
    ] {
        for damaged in [false, true] {
            for elapsed in [0, 17, u64::MAX] {
                let sprites = bank
                    .get_building(
                        id,
                        remap,
                        BuildingAnimation {
                            elapsed,
                            production: Some(elapsed),
                            damaged,
                            powered: false,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                for (&part, sprite) in order.iter().zip(&sprites.layers[1..5]) {
                    let file = format!("{prefix}_{part}.SHP");
                    let shp = ShpFile::parse(fs.read(&file).unwrap()).unwrap();
                    assert_eq!(shp.frames.len(), 4, "{file} has two health states");
                    check(sprite, &file, usize::from(damaged), false, (5, 3));
                }
            }
        }
    }
    // SpecialAnim is a one-shot tied to unloading, with normal art as
    // the damaged fallback. Inspect every selected body pixel directly.
    for damaged in [false, true] {
        for elapsed in 0..=30 {
            let phase = (elapsed * 200 / 225) as usize;
            let sprites = bank
                .get_building(
                    "GAREFN",
                    remap,
                    BuildingAnimation {
                        unloading: Some(elapsed),
                        damaged,
                        powered: true,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(sprites.layers.len(), 4 + usize::from(phase < 19));
            if phase < 19 {
                check(sprites.layers[4], "GGREFNOR.SHP", phase, false, (4, 3));
            }
        }
    }
    assert!(bank
        .get_building("E1", remap, BuildingAnimation::default())
        .is_none());
    assert!(bank
        .get_building("MISSING", remap, BuildingAnimation::default())
        .is_none());
}
