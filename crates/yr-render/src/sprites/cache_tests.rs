use super::*;

fn sheet(frames: u16) -> Vec<u8> {
    let count = frames * 2;
    let mut data = Vec::new();
    for n in [0u16, 2, 2, count] {
        data.extend(n.to_le_bytes());
    }
    for i in 0..count {
        for n in [0u16, 0, 2, 2] {
            data.extend(n.to_le_bytes());
        }
        for n in [1u32, 0, 0, 8 + u32::from(count) * 24 + u32::from(i) * 4] {
            data.extend(n.to_le_bytes());
        }
    }
    for i in 0..count {
        data.extend(if i < frames {
            [0, 16, 32, 33 + (i % 30) as u8]
        } else {
            [1, 0, 0, 0]
        });
    }
    data
}

fn bank() -> SpriteBank {
    let rules_ini = IniFile::parse("[BuildingTypes]\n0=BASE\n[BASE]\nStrength=100\n");
    let art_ini = IniFile::parse(
        "[BASE]\nFoundation=1x1\nActiveAnim=WORK\nBibShape=BIB\n\
         [WORK]\nRate=225\nLoopEnd=600\nShadow=yes\n",
    );
    let palette: Vec<_> = (0..768).map(|n| (n % 64) as u8).collect();
    SpriteBank {
        vfs: AssetFs::from_files(vec![
            ("BASE.SHP".into(), sheet(2)),
            ("BIB.SHP".into(), sheet(2)),
            ("WORK.SHP".into(), sheet(600)),
        ]),
        rules: Rules::load(&rules_ini, &art_ini),
        rules_ini,
        art_ini,
        theater: Theater::Temperate,
        unit_pal: Palette::parse(&palette).unwrap(),
        infantry: Default::default(),
        cache: Default::default(),
        cache_order: Default::default(),
        voxels: Default::default(),
        buildings: Default::default(),
        map_structure_colors: Default::default(),
        map_unit_kinds: Default::default(),
        layers: Default::default(),
        effects: Default::default(),
        projectiles: Default::default(),
    }
}

fn signature(sprites: BuildingSprites<'_>) -> Vec<(u32, u32, i32, i32, Vec<u8>)> {
    sprites
        .bib
        .into_iter()
        .chain(sprites.layers)
        .map(|s| (s.w, s.h, s.ax, s.ay, s.rgba.clone()))
        .collect()
}

#[test]
fn building_layer_history_is_bounded_and_evicted_frames_rebuild() {
    let mut bank = bank();
    let animation = BuildingAnimation::default();
    let remap = [80, 160, 240];
    let initial = signature(bank.get_building("BASE", remap, animation).unwrap());
    assert_eq!(initial.len(), 3, "bib, body and working overlay");
    for color in [remap, [240, 80, 160]] {
        for tick in 0..600 {
            let drawn = signature(
                bank.get_building(
                    "BASE",
                    color,
                    BuildingAnimation {
                        elapsed: tick,
                        ..animation
                    },
                )
                .unwrap(),
            );
            assert_eq!(drawn.len(), 3, "current layers survive eviction");
            assert!(
                bank.layers.len() <= 512,
                "building history grows at tick {tick}"
            );
        }
    }
    assert_eq!(
        signature(bank.get_building("BASE", remap, animation).unwrap()),
        initial,
        "evicted body, shadow, remap and anchors reconstruct exactly"
    );
    println!("1,200 authored phase/remap requests stay within 512 entries; evicted layers reconstruct exactly.");
}

#[test]
fn building_request_protects_all_current_layers_under_entry_and_byte_pressure() {
    for cache in [
        LayerCache::with_limits(3, 1024),
        LayerCache::with_limits(512, 48),
    ] {
        let mut bank = bank();
        bank.layers = cache;
        let mut fresh = self::bank();
        for (tick, damaged, remap) in [
            (0, false, [80, 160, 240]),
            (1, false, [80, 160, 240]),
            (599, true, [240, 80, 160]),
            (1, false, [80, 160, 240]),
            (0, false, [80, 160, 240]),
        ] {
            let animation = BuildingAnimation {
                elapsed: tick,
                damaged,
                ..Default::default()
            };
            let expected = signature(fresh.get_building("BASE", remap, animation).unwrap());
            assert_eq!(
                signature(bank.get_building("BASE", remap, animation).unwrap()),
                expected
            );
            assert_eq!(bank.layers.len(), 3);
        }
    }
}

#[test]
fn oversized_building_working_set_keeps_base_and_retries_optional_art() {
    let mut bank = bank();
    let animation = BuildingAnimation::default();
    let remap = [80, 160, 240];
    let expected = signature(bank.get_building("BASE", remap, animation).unwrap());
    bank.layers = LayerCache::with_limits(512, 32);
    let result = signature(bank.get_building("BASE", remap, animation).unwrap());
    assert_eq!(
        result,
        expected[1..],
        "base and first active layer have priority over bib"
    );
    assert_eq!(bank.layers.len(), 2);
    bank.buildings
        .get_mut("BASE")
        .unwrap()
        .as_mut()
        .unwrap()
        .active
        .clear();
    let result = signature(bank.get_building("BASE", remap, animation).unwrap());
    assert_eq!(
        result,
        expected[..2],
        "bib retries when the working set shrinks"
    );
    assert_eq!(bank.layers.len(), 2);
}

#[test]
fn retail_building_cache_churn_preserves_complete_layer_pixels() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let mut bank = SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    let states = [
        BuildingAnimation {
            elapsed: 37,
            production: Some(12),
            powered: true,
            ..Default::default()
        },
        BuildingAnimation {
            elapsed: 51,
            damaged: true,
            powered: true,
            ..Default::default()
        },
        BuildingAnimation {
            elapsed: 65,
            unloading: Some(9),
            powered: true,
            ..Default::default()
        },
        BuildingAnimation {
            elapsed: u64::MAX,
            powered: false,
            ..Default::default()
        },
    ];
    let remap = [220, 30, 30];
    let requests: Vec<_> = ["GAWEAP", "GAREFN", "NACNST", "NAWEAP", "YACNST", "YAWEAP"]
        .into_iter()
        .flat_map(|kind| states.into_iter().map(move |state| (kind, state)))
        .collect();
    let expected: Vec<_> = requests
        .iter()
        .map(|&(kind, state)| signature(bank.get_building(kind, remap, state).unwrap()))
        .collect();
    let mut peak = (0, 0);
    for color in 0..600 {
        let color = [color as u8, (color / 256) as u8, 100];
        assert!(bank.get_building("GACNST", color, states[0]).is_some());
        let (entries, bytes, order) = bank.layers.stats();
        assert!(entries <= 512);
        assert!(bytes <= 128 * 1024 * 1024);
        assert_eq!(entries, order);
        peak = (peak.0.max(entries), peak.1.max(bytes));
    }
    let output = std::env::var_os("OPENYR_BUILDING_CACHE_PROOF").map(std::path::PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    for (index, &(kind, state)) in requests.iter().enumerate() {
        let rebuilt = signature(bank.get_building(kind, remap, state).unwrap());
        assert_eq!(rebuilt, expected[index], "{kind}, request {index}");
        if let Some(out) = &output {
            // One factory's complete body/bib/shell/active/production set.
            if index == 0 {
                for (phase, layers) in [("cold", &expected[index]), ("rebuilt", &rebuilt)] {
                    for (layer, (w, h, _, _, rgba)) in layers.iter().enumerate() {
                        std::fs::write(
                            out.join(format!("{phase}-{kind}-layer-{layer}.png")),
                            crate::png::encode_rgba(*w, *h, rgba),
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
    println!("600 retail color requests: peak {} entries, {} RGBA bytes; 24 complete factory/refinery/yard requests rebuild exactly (normal, damaged, production, unloading and unpowered).", peak.0, peak.1);
}
