use super::*;
use std::path::PathBuf;

#[test]
fn impact_list_bands_use_delivered_damage_and_reject_unsupported_or_hostile_values() {
    let ini = IniFile::parse("[AP]\nAnimList=SMALL,MEDIUM,LARGE\n");
    let section = ini.section("AP").unwrap();
    for (damage, expected) in [
        (i64::MIN, None),
        (-1, None),
        (0, None),
        (1, Some("SMALL")),
        (24, Some("SMALL")),
        (25, Some("MEDIUM")),
        (49, Some("MEDIUM")),
        (50, Some("LARGE")),
        (i64::MAX, Some("LARGE")),
    ] {
        assert_eq!(impact_name(section, damage).as_deref(), expected);
    }
    for value in [
        "AnimList=",
        "AnimList=../../unsafe",
        "AnimList=A,B\nEMEffect=yes",
        &format!("AnimList={}", vec!["A"; 129].join(",")),
    ] {
        let ini = IniFile::parse(&format!("[AP]\n{value}"));
        assert!(impact_name(ini.section("AP").unwrap(), 65).is_none());
    }
}

#[test]
fn hostile_optional_effects_are_rejected_before_canvas_decoding() {
    let dir = std::env::temp_dir().join(format!("openyr-effect-bounds-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let fixture = |width: u16, height: u16, count: u16| {
        let mut bytes = Vec::new();
        for n in [0, width, height, count, 0, 0, 2, 2] {
            bytes.extend(n.to_le_bytes());
        }
        for n in [1u32, 0, 0, 32] {
            bytes.extend(n.to_le_bytes());
        }
        bytes.extend([1, 2, 3, 4]);
        bytes
    };
    for (w, h, count, ini, valid) in [
        (2, 2, 1, "[SAFE]", true),
        (u16::MAX, u16::MAX, 1, "[SAFE]", false),
        (2, 2, u16::MAX, "[SAFE]", false),
        (2, 2, 1, "[SAFE]\nRate=0", false),
        (2, 2, 1, "[SAFE]\nEnd=2", false),
        (2, 2, 1, "[SAFE]\nLoopEnd=2", false),
        (2, 2, 1, "[SAFE]\nLoopStart=1", false),
        (2, 2, 1, "[SAFE]\nLoopEnd=0", false),
        (2, 2, 1, "[SAFE]\nLoopCount=0", false),
        (2, 2, 1, "[SAFE]\nLoopCount=1024", true),
        (2, 2, 1, "[SAFE]\nLoopCount=-1", false),
        (2, 2, 1, "[SAFE]\nLoopCount=invalid", false),
        (2, 2, 1, "[SAFE]\nLoopCount=1025", false),
        (2, 2, 1, "[SAFE]\nLoopCount=18446744073709551615", false),
        (2, 2, 1, "[SAFE]\nLoopStart=18446744073709551615", false),
        (2, 2, 1, "[SAFE]\nLoopEnd=18446744073709551615", false),
        (2, 2, 1, "[SAFE]\nStart=18446744073709551615", false),
        (2, 2, 1, "[SAFE]\nTranslucency=101", false),
        (2, 2, 1, "[SAFE]\nImage=../../secret", false),
    ] {
        std::fs::write(dir.join("SAFE.SHP"), fixture(w, h, count)).unwrap();
        let fs = AssetFs::open(&dir).unwrap();
        assert_eq!(
            Clip::load(&fs, &IniFile::parse(ini), "SAFE").is_some(),
            valid
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
    let clip = Clip {
        file: String::new(),
        start: 3,
        end: 7,
        loop_start: 3,
        loop_end: 7,
        loop_count: 1,
        rate: 225,
        alpha: 255,
    };
    assert_eq!(clip.frame(0), Some(3));
    assert_eq!(clip.frame(3), Some(6));
    assert_eq!(clip.frame(4), None);
    assert_eq!(clip.frame(u64::MAX), None);
}

#[test]
fn finite_effect_loops_preserve_prefix_tail_and_total_traversals() {
    let mut clip = Clip {
        file: String::new(),
        start: 2,
        end: 10,
        loop_start: 4,
        loop_end: 7,
        loop_count: 3,
        rate: 225,
        alpha: 255,
    };
    let expected = [2, 3, 4, 5, 6, 4, 5, 6, 4, 5, 6, 7, 8, 9];
    for (tick, frame) in expected.into_iter().enumerate() {
        assert_eq!(clip.frame(tick as u64), Some(frame));
    }
    assert_eq!(clip.frame(expected.len() as u64), None);
    assert_eq!(clip.frame(u64::MAX), None);
    clip.loop_count = 1;
    for tick in 0..8 {
        assert_eq!(clip.frame(tick), Some(2 + tick as usize));
    }
    assert_eq!(clip.frame(8), None);
    clip.loop_count = 1024;
    clip.rate = u64::MAX;
    assert_eq!(clip.frame(u64::MAX), None);
}

#[test]
fn retail_electro_loop_frames_match_source_and_expire_after_the_tail() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("ELECTRO.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (60, 56, 15));
    let mut effects = Effects::default();
    let mut phases = Vec::new();
    for tick in 0..=31 {
        let phase = tick * 200 / 225;
        let sprite = effects.get(&fs, &art, "ELECTRO", tick);
        if phase >= 27 {
            assert!(sprite.is_none());
            continue;
        }
        // LoopEnd=3, LoopCount=5: five native traversals of [0, 3),
        // followed by the twelve-frame tail [3, 15).
        let frame = if phase < 15 { phase % 3 } else { phase - 12 };
        phases.push(frame);
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                }
            })
            .collect();
        let sprite = sprite.unwrap();
        assert_eq!((sprite.w, sprite.h, sprite.ax, sprite.ay), (60, 56, 30, 28));
        assert!(
            sprite.rgba == expected,
            "ELECTRO tick {tick}, frame {frame}"
        );
    }
    assert_eq!(
        phases
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        15
    );
    assert_eq!(effects.frames.len(), 15, "loops share cached source frames");
    assert!(!effects.alive(&fs, &art, "ELECTRO", u64::MAX));
}

#[test]
fn retail_virus_death_frames_match_source_and_expire() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("VIRUSD.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (186, 190, 35));
    let mut effects = Effects::default();
    let mut frames = std::collections::BTreeSet::new();
    for tick in 0..=40 {
        let frame = tick * 200 / 225;
        let sprite = effects.get(&fs, &art, "VIRUSD", tick);
        if frame >= 35 {
            assert!(sprite.is_none());
            continue;
        }
        frames.insert(frame);
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                }
            })
            .collect();
        let sprite = sprite.unwrap();
        assert_eq!(
            (sprite.w, sprite.h, sprite.ax, sprite.ay),
            (186, 190, 93, 95)
        );
        assert!(sprite.rgba == expected, "VIRUSD tick {tick}, frame {frame}");
    }
    assert_eq!(frames.len(), 35);
    assert_eq!(effects.frames.len(), 35);
    assert!(!effects.alive(&fs, &art, "VIRUSD", u64::MAX));
}

#[test]
fn retail_mutate_death_frames_match_source_and_expire() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("GENDEATH.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (260, 304, 60));
    let mut effects = Effects::default();
    let mut frames = std::collections::BTreeSet::new();
    for tick in 0..=68 {
        let frame = tick * 200 / 225;
        let sprite = effects.get(&fs, &art, "GENDEATH", tick);
        if frame >= 60 {
            assert!(sprite.is_none());
            continue;
        }
        frames.insert(frame);
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                }
            })
            .collect();
        let sprite = sprite.unwrap();
        assert_eq!(
            (sprite.w, sprite.h, sprite.ax, sprite.ay),
            (260, 304, 130, 152)
        );
        assert!(
            sprite.rgba == expected,
            "GENDEATH tick {tick}, frame {frame}"
        );
    }
    assert_eq!(frames.len(), 60);
    assert_eq!(effects.frames.len(), 60);
    assert!(!effects.alive(&fs, &art, "GENDEATH", u64::MAX));
}

#[test]
fn retail_head_pop_frames_match_source_and_expire() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("YURIDIE.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (230, 150, 30));
    let mut effects = Effects::default();
    let mut frames = std::collections::BTreeSet::new();
    for tick in 0..=34 {
        let frame = tick * 200 / 225;
        let sprite = effects.get(&fs, &art, "YURIDIE", tick);
        if frame >= 30 {
            assert!(sprite.is_none());
            continue;
        }
        frames.insert(frame);
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                }
            })
            .collect();
        let sprite = sprite.unwrap();
        assert_eq!(
            (sprite.w, sprite.h, sprite.ax, sprite.ay),
            (230, 150, 115, 75)
        );
        assert!(
            sprite.rgba == expected,
            "YURIDIE tick {tick}, frame {frame}"
        );
    }
    assert_eq!(frames.len(), 30, "all source frames use body art");
    assert_eq!(effects.frames.len(), 30);
    assert!(!effects.alive(&fs, &art, "YURIDIE", u64::MAX));
}

#[test]
fn retail_nuke_melt_frames_match_source_and_expire() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("NUKEDIE.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (60, 60, 60));
    let mut effects = Effects::default();
    let mut frames = std::collections::BTreeSet::new();
    for tick in 0..=68 {
        let frame = tick * 200 / 225;
        let sprite = effects.get(&fs, &art, "NUKEDIE", tick);
        if frame >= 60 {
            assert!(sprite.is_none());
            continue;
        }
        frames.insert(frame);
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                }
            })
            .collect();
        let sprite = sprite.unwrap();
        assert_eq!((sprite.w, sprite.h, sprite.ax, sprite.ay), (60, 60, 30, 30));
        assert!(
            sprite.rgba == expected,
            "NUKEDIE tick {tick}, frame {frame}"
        );
    }
    assert_eq!(
        frames.len(),
        60,
        "all body frames, no shadow-half assumption"
    );
    assert_eq!(
        effects.frames.len(),
        60,
        "repeated phases share source frames"
    );
    assert!(!effects.alive(&fs, &art, "NUKEDIE", u64::MAX));
}

#[test]
fn flaming_collapse_layout_rejects_invalid_or_overflowing_ranges() {
    let dir = std::env::temp_dir().join(format!("openyr-flaming-bounds-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for count in [19u16, 20] {
        // Complete empty-frame tables keep this independent of pixel decoding.
        let mut bytes = Vec::new();
        for n in [0u16, 2, 2, count] {
            bytes.extend(n.to_le_bytes());
        }
        bytes.resize(8 + usize::from(count) * 24, 0);
        std::fs::write(dir.join("SAFE.SHP"), bytes).unwrap();
        let fs = AssetFs::open(&dir).unwrap();
        for (options, valid) in [
            ("RunningFrames=1", true),
            ("RunningFrames=1\nStart=9\nEnd=10", true),
            ("", false),
            ("RunningFrames=0", false),
            ("RunningFrames=2", false),
            ("RunningFrames=-1", false),
            ("RunningFrames=invalid", false),
            ("RunningFrames=18446744073709551615", false),
            ("RunningFrames=1\nStart=7", false),
            ("RunningFrames=1\nEnd=11", false),
            ("RunningFrames=1\nLoopStart=7", false),
            ("RunningFrames=1\nLoopEnd=11", false),
        ] {
            let art = IniFile::parse(&format!("[SAFE]\nIsFlamingGuy=true\n{options}"));
            assert_eq!(
                Clip::load(&fs, &art, "SAFE").is_some(),
                valid && count == 20,
                "count {count}, {options}"
            );
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn retail_flaming_collapse_matches_every_source_frame_and_excludes_running_groups() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let mut art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("FLAMEGUY.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (52, 102, 216));
    assert_eq!(art.value("FLAMEGUY", "RunningFrames"), Some("6"));
    assert!(shp.frames[108..].iter().all(|frame| frame.is_empty()));
    let clip = Clip::load(&fs, &art, "FLAMEGUY").unwrap();
    let mut effects = Effects::default();
    for age in 0..=27 {
        let frame = 48 + age * 500 / 225;
        assert_eq!(clip.frame(age), (frame < 108).then_some(frame as usize));
        assert_eq!(effects.alive(&fs, &art, "FLAMEGUY", age), age < 27);
    }
    assert!(!effects.alive(&fs, &art, "FLAMEGUY", u64::MAX));
    // Slow only the test clock to visit every source canvas, including the
    // frames skipped by Rate=500 at the native 15 Hz simulation cadence.
    art.merge_from(&IniFile::parse("[FLAMEGUY]\nRate=225"));
    let mut effects = Effects::default();
    for frame in 48..108 {
        let sprite = effects
            .get(&fs, &art, "FLAMEGUY", (frame - 48) as u64)
            .unwrap();
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                }
            })
            .collect();
        assert_eq!(
            (sprite.w, sprite.h, sprite.ax, sprite.ay),
            (52, 102, 26, 51)
        );
        assert!(sprite.rgba == expected, "FLAMEGUY source frame {frame}");
    }
    assert!(effects.get(&fs, &art, "FLAMEGUY", 60).is_none());
}

#[test]
fn retail_infantry_explosion_frames_match_source_alpha_and_expire() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("S_BANG34.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (148, 158, 13));
    let mut effects = Effects::default();
    let mut frames = std::collections::BTreeSet::new();
    for tick in 0..=15 {
        let frame = tick * 200 / 225;
        let sprite = effects.get(&fs, &art, "S_BANG34", tick);
        if frame >= 13 {
            assert!(sprite.is_none());
            continue;
        }
        frames.insert(frame);
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    let [r, g, b, _] = palette.rgba(index);
                    [r, g, b, 127]
                }
            })
            .collect();
        let sprite = sprite.unwrap();
        assert_eq!(
            (sprite.w, sprite.h, sprite.ax, sprite.ay),
            (148, 158, 74, 79)
        );
        assert!(
            sprite.rgba == expected,
            "S_BANG34 tick {tick}, frame {frame}"
        );
    }
    assert_eq!(
        frames.len(),
        13,
        "all body frames, no shadow-half assumption"
    );
    assert_eq!(
        effects.frames.len(),
        13,
        "repeated phases share source frames"
    );
    assert!(!effects.alive(&fs, &art, "S_BANG34", u64::MAX));
}

#[test]
fn retail_brute_smash_frames_match_source_and_expire() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let shp = ShpFile::parse(fs.read("BRUTDIE.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height, shp.frames.len()), (102, 118, 60));
    let mut effects = Effects::default();
    let mut frames = std::collections::BTreeSet::new();
    for tick in 0..=68 {
        let frame = tick * 200 / 225;
        let sprite = effects.get(&fs, &art, "BRUTDIE", tick);
        if frame >= 60 {
            assert!(sprite.is_none());
            continue;
        }
        frames.insert(frame);
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                }
            })
            .collect();
        let sprite = sprite.unwrap();
        assert_eq!(
            (sprite.w, sprite.h, sprite.ax, sprite.ay),
            (102, 118, 51, 59)
        );
        assert!(
            sprite.rgba == expected,
            "BRUTDIE tick {tick}, frame {frame}"
        );
    }
    assert_eq!(
        frames.len(),
        60,
        "all body frames, no shadow-half assumption"
    );
    assert_eq!(
        effects.frames.len(),
        60,
        "repeated phases share source frames"
    );
    assert!(!effects.alive(&fs, &art, "BRUTDIE", u64::MAX));
}

#[test]
fn retail_effect_frames_match_anim_palette_and_cache_stays_bounded() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let art = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let palette = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    let mut bank = Effects::default();
    for name in [
        "GUNFIRE", "MGUN-N", "MGUN-NE", "MGUN-E", "MGUN-SE", "MGUN-S", "MGUN-SW", "MGUN-W",
        "MGUN-NW", "TWLT070", "S_CLSN16", "S_CLSN22", "PIFFPIFF",
    ] {
        let shp = ShpFile::parse(fs.read(&format!("{name}.SHP")).unwrap()).unwrap();
        println!(
            "{name}: {}x{}, {} frames",
            shp.width,
            shp.height,
            shp.frames.len()
        );
        let alpha = if name.starts_with("MGUN-") || name == "PIFFPIFF" {
            255
        } else {
            127
        };
        let mut seen = std::collections::BTreeSet::new();
        for tick in 0..1000 {
            let phase = tick * 200 / 225;
            let sprite = bank.get(&fs, &art, name, tick);
            if phase >= shp.frames.len() as u64 {
                assert!(sprite.is_none());
                break;
            }
            let sprite = sprite.unwrap();
            let expected: Vec<_> = shp
                .decode_frame_canvas(phase as usize)
                .unwrap()
                .into_iter()
                .flat_map(|p| {
                    if p == 0 {
                        [0; 4]
                    } else {
                        let [r, g, b, _] = palette.rgba(p);
                        [r, g, b, alpha]
                    }
                })
                .collect();
            assert!(
                sprite.rgba == expected,
                "{name} frame {phase} source pixels"
            );
            assert_eq!(
                (sprite.ax, sprite.ay),
                (i32::from(shp.width) / 2, i32::from(shp.height) / 2)
            );
            seen.insert(phase);
            assert!(bank.frames.len() <= MAX_CACHED_FRAMES);
        }
        assert_eq!(
            seen.len(),
            shp.frames.len(),
            "all effect frames, no shadow-half assumption"
        );
        assert!(!bank.alive(&fs, &art, name, u64::MAX));
    }
    assert_eq!(bank.frames.len(), MAX_CACHED_FRAMES);
}

#[test]
fn retail_smoke_trailer_all_canvases_match_with_translucency() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let pal = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    // Unit-rate overrides let the existing decoder expose every canvas,
    // including odd DURASMOKE frames skipped at its retail Rate=450.
    let art = IniFile::parse("[DURASMOKE]\nTranslucent=yes\nRate=225\n[SMOKEY2]\nTranslucent=yes\nRate=225\n[BBBLELRG]\nRate=225\n");
    let mut cache = Effects::default();
    for name in ["DURASMOKE", "SMOKEY2", "BBBLELRG"] {
        let shp = ShpFile::parse(fs.read(&format!("{name}.SHP")).unwrap()).unwrap();
        for frame in 0..shp.frames.len() {
            let expected: Vec<_> = shp
                .decode_frame_canvas(frame)
                .unwrap()
                .into_iter()
                .flat_map(|i| {
                    if i == 0 {
                        [0; 4]
                    } else {
                        let [r, g, b, _] = pal.rgba(i);
                        [r, g, b, if name == "BBBLELRG" { 255 } else { 127 }]
                    }
                })
                .collect();
            assert_eq!(
                cache.get(&fs, &art, name, frame as u64).unwrap().rgba,
                expected
            );
        }
        assert!(!cache.alive(&fs, &art, name, shp.frames.len() as u64));
    }
    assert!(cache.frames.len() <= MAX_CACHED_FRAMES);
}
