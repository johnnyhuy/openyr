use super::*;
use crate::menu::{LoadStatus, Page};

fn state() -> State {
    State {
        front: true,
        page: Page::Loading,
        loading: LoadStatus {
            name: "EB1.MMX".into(),
            map_name: "EB1.MMX".into(),
            battle: true,
            stage: LoadStage::Terrain,
            config: Default::default(),
        },
        ..Default::default()
    }
}

fn tiny_preview() -> Preview {
    Preview {
        width: 2,
        height: 1,
        rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
        authored: true,
        setup: Default::default(),
        starts: [None; 8],
        map_rect: (0, 0, 1, 1),
        local_rect: (0, 0, 1, 1),
    }
}

#[test]
fn preview_keeps_its_aspect_ratio_and_invalid_art_stays_recoverable() {
    let art = Art::default();
    let state = state();
    let mut preview = tiny_preview();
    let mut frame = vec![0; 800 * 600 * 4];
    draw(&mut frame, 800, 600, &art, &state, Some(&preview));
    let pixel =
        |frame: &[u8], x: usize, y: usize| frame[(y * 800 + x) * 4..(y * 800 + x) * 4 + 4].to_vec();
    assert_eq!(pixel(&frame, 500, 408), [0, 0, 0, 255]);
    assert_eq!(pixel(&frame, 500, 409), [248, 0, 0, 255]);
    assert_eq!(pixel(&frame, 607, 516), [248, 0, 0, 255]);
    assert_eq!(pixel(&frame, 608, 409), [0, 0, 248, 255]);
    assert_eq!(pixel(&frame, 715, 516), [0, 0, 248, 255]);
    assert_eq!(pixel(&frame, 715, 517), [0, 0, 0, 255]);
    for (width, height, rgba) in [
        (0, 1, vec![]),
        (u32::MAX, u32::MAX, vec![]),
        (2, 1, vec![0; 7]),
    ] {
        preview.width = width;
        preview.height = height;
        preview.rgba = rgba;
        draw(&mut frame, 800, 600, &art, &state, Some(&preview));
        assert_eq!(pixel(&frame, 608, 450), [0, 0, 0, 255]);
    }
    assert!(decode_background(&[], &[0; 768]).is_none());
    let mut hostile = vec![0, 0, 255, 255, 255, 255, 255, 255];
    hostile.resize(32, 0);
    assert!(decode_background(&hostile, &[0; 768]).is_none());
    let mut wrong_size = vec![0, 0, 1, 0, 1, 0, 1, 0];
    wrong_size.resize(32, 0);
    assert!(decode_background(&wrong_size, &[0; 768]).is_none());
}

#[test]
fn loading_cancel_hit_matches_the_visible_control_on_front_and_pause_surfaces() {
    let art = Art::default();
    let mut state = state();
    for front in [false, true] {
        state.front = front;
        for (width, height) in [(1, 1), (400, 300), (640, 480), (800, 600), (1280, 800)] {
            let mut frame = vec![17; (width * height * 4) as usize];
            draw(&mut frame, width, height, &art, &state, None);
            assert!(frame.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255));
            let layout = Layout::for_state(&state, width, height);
            let (x, y, w, h) = layout.button(0, Button::Back);
            assert_eq!((x, y, w, h), (16, 568, 160, 25));
            let position = (
                layout.origin.0 + f64::from(x + 12) * layout.scale,
                layout.origin.1 + f64::from(y + 12) * layout.scale,
            );
            assert_eq!(layout.hit(&state, position), Some(Button::Back));
            assert_eq!(layout.hit(&state, (-1.0, -1.0)), None);
        }
    }
    let mut invalid = [23; 3];
    draw(&mut invalid, 800, 600, &art, &state, None);
    assert_eq!(invalid, [23; 3]);
    draw(&mut [], 0, 0, &art, &state, None);
}

#[test]
fn retail_loading_preserves_the_stock_country_map_and_uses_the_selected_preview() {
    let directory = std::env::var_os("OPENYR_GAMEDIR").map_or_else(
        || {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
        },
        std::path::PathBuf::from,
    );
    if !directory.is_dir() {
        return;
    }
    let fs = AssetFs::open(&directory).unwrap();
    let art = Art::load(&fs);
    assert!(
        art.backgrounds.len() == Country::ALL.len(),
        "retail country artwork should load"
    );
    let mut state = state();
    let proof = std::env::var_os("OPENYR_LOADING_PROOF").map(std::path::PathBuf::from);
    let mut preview_hashes = Vec::new();
    for map in ["EB1.MMX", "XMP29U2.MAP"] {
        let preview = crate::frontend::load_preview(directory.to_str().unwrap(), map, 1).unwrap();
        state.loading.map_name = map.into();
        state.loading.name = map.into();
        let mut frame = vec![0; 800 * 600 * 4];
        for stage in [LoadStage::Session, LoadStage::Terrain, LoadStage::Artwork] {
            state.loading.stage = stage;
            draw(&mut frame, 800, 600, &art, &state, Some(&preview));
            // Independent receipt: raw 802-byte SHP rows, source MPLSU.PAL,
            // RGB565 output. This is a source-art check, not original runtime parity.
            let country = hash(&frame, (450, 32, 316, 313));
            assert_eq!(country, 0x97061e52e7661e85);
            if let Some(proof) = &proof {
                std::fs::create_dir_all(proof).unwrap();
                std::fs::write(
                    proof.join(format!("loading-{map}-{stage:?}.png")),
                    yr_render::png::encode_rgba(800, 600, &frame),
                )
                .unwrap();
            }
        }
        preview_hashes.push(hash(&frame, PREVIEW));
    }
    assert_ne!(
        preview_hashes[0], preview_hashes[1],
        "the chosen map changes the loading thumbnail"
    );
}

fn hash(frame: &[u8], rect: (i32, i32, i32, i32)) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for y in rect.1..rect.1 + rect.3 {
        for x in rect.0..rect.0 + rect.2 {
            for byte in &frame[((y * 800 + x) * 4) as usize..((y * 800 + x) * 4 + 4) as usize] {
                hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
            }
        }
    }
    hash
}

#[test]
fn retail_country_choices_preserve_each_source_map_and_selected_bar_color() {
    let directory = std::env::var_os("OPENYR_GAMEDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
        });
    if !directory.is_dir() {
        return;
    }
    let fs = AssetFs::open(&directory).unwrap();
    let art = Art::load(&fs);
    assert_eq!(art.backgrounds.len(), 9);
    let rules =
        yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    assert_eq!(
        art.captions,
        ["AlliedLoad", "SovietLoad"].map(|name| yr_render::named_color(&rules, name))
    );
    let mut state = state();
    state.loading.config.players[0].name = "Johnny".into();
    state.loading.config.players[0].color = 6;
    for country in Country::ALL {
        state.loading.config.players[0].country = country;
        assert!(
            art.strings
                .as_ref()
                .unwrap()
                .get(&format!("Name:{}", country.id()))
                .is_some(),
            "{country:?} name"
        );
        assert!(
            art.strings
                .as_ref()
                .unwrap()
                .get(&format!("LoadBriefShort:{}", country.brief()))
                .is_some(),
            "{country:?} brief"
        );
        let (shp, pal) = country.loading_art();
        let bytes = fs.read(shp).unwrap();
        let palette = fs.read(pal).unwrap();
        assert_eq!(bytes.len(), 481232);
        let mut frame = vec![0; 800 * 600 * 4];
        draw(&mut frame, 800, 600, &art, &state, None);
        // Independent compression-2 row traversal, without the SHP decoder.
        for y in 32..345usize {
            let row = &bytes[32 + y * 802..32 + (y + 1) * 802];
            assert_eq!(u16::from_le_bytes(row[..2].try_into().unwrap()), 802);
            for x in 450..766usize {
                let index = usize::from(row[2 + x]);
                let rgb: [u8; 3] = std::array::from_fn(|c| {
                    let v = if index == 0 {
                        0
                    } else {
                        palette[3 * index + c]
                    };
                    ((v << 2) | (v >> 4)) & [248, 252, 248][c]
                });
                let at = (y * 800 + x) * 4;
                assert_eq!(&frame[at..at + 3], &rgb, "{country:?} {x},{y}");
            }
        }
        let at = (510 * 800 + 160) * 4;
        let rgb = art.colors[6];
        assert_eq!(
            &frame[at..at + 4],
            &[rgb[0] & 248, rgb[1] & 252, rgb[2] & 248, 255]
        );
        if let Some(out) = std::env::var_os("OPENYR_LOADING_PROOF") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                std::path::PathBuf::from(out).join(format!("country-{country:?}.png")),
                yr_render::png::encode_rgba(800, 600, &frame),
            )
            .unwrap();
        }
    }
}

#[test]
fn unresolved_saved_identity_does_not_show_the_previous_country_or_names() {
    let mut art = Art::default();
    art.backgrounds.push((
        Country::America,
        Image {
            width: 800,
            height: 600,
            pixels: [255, 0, 0, 255].repeat(800 * 600),
        },
    ));
    let mut state = state();
    state.loading.map_name.clear();
    let mut frame = vec![0; 800 * 600 * 4];
    draw(&mut frame, 800, 600, &art, &state, None);
    assert_eq!(
        &frame[(90 * 800 + 24) * 4..(90 * 800 + 24) * 4 + 4],
        &[0, 8, 24, 255]
    );
    assert_eq!(
        &frame[(506 * 800 + 24) * 4..(506 * 800 + 24) * 4 + 4],
        &[0, 8, 24, 255]
    );
    assert_eq!(
        &frame[(32 * 800 + 450) * 4..(32 * 800 + 450) * 4 + 4],
        &[0, 8, 24, 255]
    );
}
