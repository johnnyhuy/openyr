use super::*;
use std::path::PathBuf;

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
        rate: 225,
        alpha: 255,
    };
    assert_eq!(clip.frame(0), Some(3));
    assert_eq!(clip.frame(3), Some(6));
    assert_eq!(clip.frame(4), None);
    assert_eq!(clip.frame(u64::MAX), None);
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
        "MGUN-NW", "TWLT070",
    ] {
        let shp = ShpFile::parse(fs.read(&format!("{name}.SHP")).unwrap()).unwrap();
        println!(
            "{name}: {}x{}, {} frames",
            shp.width,
            shp.height,
            shp.frames.len()
        );
        let alpha = if name.starts_with("MGUN-") { 255 } else { 127 };
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
