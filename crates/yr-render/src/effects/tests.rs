use super::*;
use std::path::PathBuf;

fn smoke_fixture(count: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    for n in [0u16, 2, 2, count] {
        bytes.extend(n.to_le_bytes());
    }
    for frame in 0..count {
        for n in [0u16, 0, 2, 2] {
            bytes.extend(n.to_le_bytes());
        }
        for n in [1u32, 0, 0, 8 + u32::from(count) * 24 + u32::from(frame) * 4] {
            bytes.extend(n.to_le_bytes());
        }
    }
    for frame in 0..count {
        bytes.extend([0, 1, 2, 3 + frame as u8]);
    }
    bytes
}

fn smoke_rules(extra: &str) -> IniFile {
    IniFile::parse(&format!(
        "[AudioVisual]\nConditionYellow=50%\n[UNIT]\nDamageParticleSystems=SPARK,SYS\n\
         [SPARK]\nBehavesLike=Spark\n[SYS]\nBehavesLike=Smoke\nSpawns=yes\n\
         HoldsWhat=PUFF\nSpawnFrames=10\nParticleCap=7\n[PUFF]\n\
         Image=SMOKE\nBehavesLike=Smoke\nDeleteOnStateLimit=yes\n\
         EndStateAI=20\nStateAIAdvance=4\nTranslucency=50\n{extra}"
    ))
}

#[test]
fn damage_smoke_requires_explicit_bounded_definitions_and_exact_living_health() {
    let ini = smoke_rules("");
    let spec = damage_smoke_spec(&ini, "UNIT", 50, 100).unwrap();
    assert_eq!(spec.particle, "PUFF");
    assert_eq!((spec.interval, spec.cap), (3, 7));
    for (hp, maximum, expected) in [
        (51, 100, false),
        (50, 100, true),
        (49, 100, true),
        (1, 100, true),
        (0, 100, false),
        (-1, 100, false),
        (1, 0, false),
        (1, -1, false),
        (i64::MAX, i64::MAX, false),
        (i64::MAX / 2, i64::MAX, true),
    ] {
        assert_eq!(
            damage_smoke_spec(&ini, "UNIT", hp, maximum).is_some(),
            expected
        );
    }
    for extra in [
        "[AudioVisual]\nConditionYellow=invalid",
        "[AudioVisual]\nConditionYellow=0",
        "[AudioVisual]\nConditionYellow=101%",
        "[AudioVisual]\nConditionYellow=9223372036854.999999",
        "[UNIT]\nDamageParticleSystems=SPARK",
        "[UNIT]\nDamageParticleSystems=",
        "[SYS]\nSpawns=no",
        "[SYS]\nSpawnFrames=0",
        "[SYS]\nSpawnFrames=241",
        "[SYS]\nParticleCap=0",
        "[SYS]\nParticleCap=65",
        "[SYS]\nParticleCap=18446744073709551615",
        "[SYS]\nHoldsWhat=../unsafe",
    ] {
        assert!(
            damage_smoke_spec(&smoke_rules(extra), "UNIT", 1, 100).is_none(),
            "{extra}"
        );
    }
    let exact = smoke_rules("[AudioVisual]\nConditionYellow=49.9999%");
    assert!(damage_smoke_spec(&exact, "UNIT", 500_000, 1_000_000).is_none());
    assert!(damage_smoke_spec(&exact, "UNIT", 499_999, 1_000_000).is_some());
    let overlong = smoke_rules(&format!(
        "[UNIT]\nDamageParticleSystems={}",
        vec!["SYS"; 33].join(",")
    ));
    assert!(damage_smoke_spec(&overlong, "UNIT", 1, 100).is_none());
}

#[test]
fn smoke_particle_pixels_clocks_and_animation_cache_names_stay_independent() {
    let palette: Vec<_> = (0..768).map(|n| (n % 64) as u8).collect();
    let fs = AssetFs::from_files(vec![
        ("SMOKE.SHP".into(), smoke_fixture(20)),
        ("ANIM.PAL".into(), palette.clone()),
    ]);
    let rules = smoke_rules("");
    let pal = Palette::parse(&palette).unwrap();
    let mut bank = Effects::default();
    for frame in 0..20 {
        let sprite = bank.smoke(&fs, &rules, "PUFF", frame).unwrap();
        assert_eq!((sprite.w, sprite.h, sprite.ax, sprite.ay), (2, 2, 1, 1));
        let expected: Vec<_> = [0, 1, 2, 3 + frame as u8]
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    let [r, g, b, _] = pal.rgba(index);
                    [r, g, b, 127]
                }
            })
            .collect();
        assert_eq!(sprite.rgba, expected);
    }
    assert!(bank.smoke(&fs, &rules, "PUFF", 20).is_none());
    assert!(bank.smoke(&fs, &rules, "PUFF", u64::MAX).is_none());
    let art = IniFile::parse("[PUFF]\nImage=SMOKE\nRate=225\nTranslucency=0");
    assert_eq!(bank.get(&fs, &art, "PUFF", 0).unwrap().rgba[7], 255);
    assert_eq!(bank.smoke(&fs, &rules, "PUFF", 0).unwrap().rgba[7], 127);
    let slow = smoke_rules("[PUFF]\nStartFrame=2\nEndStateAI=5\nStateAIAdvance=8");
    let clip = Clip::load_smoke(&fs, &slow, "PUFF").unwrap();
    for (age, expected) in [
        (0, Some(2)),
        (1, Some(2)),
        (2, Some(3)),
        (4, Some(4)),
        (6, None),
    ] {
        assert_eq!(clip.frame(age), expected);
    }
    for extra in [
        "[PUFF]\nImage=../unsafe",
        "[PUFF]\nBehavesLike=Fire",
        "[PUFF]\nDeleteOnStateLimit=no",
        "[PUFF]\nStartFrame=20",
        "[PUFF]\nEndStateAI=21",
        "[PUFF]\nEndStateAI=18446744073709551615",
        "[PUFF]\nStateAIAdvance=0",
        "[PUFF]\nStateAIAdvance=241",
        "[PUFF]\nTranslucency=101",
        "[PUFF]\nEndStateAI=-1",
    ] {
        assert!(
            Clip::load_smoke(&fs, &smoke_rules(extra), "PUFF").is_none(),
            "{extra}"
        );
    }
    let mut hostile = smoke_fixture(20);
    hostile[2..6].copy_from_slice(&[255; 4]);
    let bad = AssetFs::from_files(vec![("SMOKE.SHP".into(), hostile)]);
    assert!(Clip::load_smoke(&bad, &rules, "PUFF").is_none());
}

#[test]
fn retail_damage_smoke_matches_all_twenty_particle_source_canvases() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let rules = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    let particle = rules.section("SmallGreySmoke").unwrap();
    assert_eq!(particle.get("Image"), Some("SGRYSMK1"));
    assert_eq!(particle.get("EndStateAI"), Some("20"));
    assert_eq!(particle.get("StateAIAdvance"), Some("4"));
    let shp = ShpFile::parse(fs.read("SGRYSMK1.SHP").unwrap()).unwrap();
    let pal = Palette::parse(fs.read("ANIM.PAL").unwrap()).unwrap();
    assert!(damage_smoke_spec(&rules, "MTNK", 150, 300).is_some());
    assert!(
        Clip::load_smoke(&fs, &rules, "SmallGreySmoke").is_some(),
        "{}x{}, {} frames",
        shp.width,
        shp.height,
        shp.frames.len()
    );
    let mut bank = crate::sprites::SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    let spec = bank.damage_smoke("MTNK", 150, 300).unwrap();
    assert_eq!(spec.particle, "SMALLGREYSMOKE");
    assert_eq!((spec.interval, spec.cap), (3, 7));
    for kind in ["E1", "GACNST", "ORCA", "STNK", "missing"] {
        assert!(bank.damage_smoke(kind, 1, 100).is_none(), "{kind}");
    }
    assert!(bank.damage_smoke("MTNK", 151, 300).is_none());
    for frame in 0..20 {
        let sprite = bank.get_damage_smoke(&spec.particle, frame).unwrap();
        let expected: Vec<_> = shp
            .decode_frame_canvas(frame as usize)
            .unwrap()
            .into_iter()
            .flat_map(|index| {
                if index == 0 {
                    [0; 4]
                } else {
                    let [r, g, b, _] = pal.rgba(index);
                    [r, g, b, 127]
                }
            })
            .collect();
        assert_eq!(sprite.rgba, expected, "particle frame {frame}");
    }
    assert!(!bank.damage_smoke_alive(&spec.particle, 20));
}

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
        divisor: 225,
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
        divisor: 225,
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
