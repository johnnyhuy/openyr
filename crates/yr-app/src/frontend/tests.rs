use super::*;
use crate::menu::{Button, Effect, Layout, Page, State};

#[test]
fn retail_front_headings_match_original_renderer_captures() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let art = Art::load(&fs);
    let preview = load_preview(dir.to_str().unwrap(), "EB1.MMX", 1).unwrap();
    assert!(preview.authored);
    let proof = std::env::var_os("OPENYR_TITLE_PROOF").map(std::path::PathBuf::from);
    if let Some(path) = &proof {
        std::fs::create_dir_all(path).unwrap();
    }
    let mut mismatches = Vec::new();
    // Original RGBA at (632,0,168,height), including each heading frame.
    for (name, page, height, expected) in [
        ("main", Page::Main, 40, 0xe789aaaf10a7f8d1u64),
        ("single-player", Page::SinglePlayer, 40, 0x29e7b0529ab9bf19),
        ("skirmish", Page::Skirmish, 40, 0x1091ed8ace62e049),
    ] {
        let mut state = State {
            front: true,
            ..State::default()
        };
        state.set(page);
        let mut pixels = vec![0; 800 * 600 * 4];
        draw(
            &mut pixels,
            800,
            600,
            &art,
            Scene {
                state: &state,
                movie: None,
                preview: matches!(&state.page, Page::Skirmish).then_some(&preview),
                phase: 0,
                cursor: (-1.0, -1.0),
            },
        );
        let hash = (0..height)
            .flat_map(|row| {
                let at = (row * 800 + 632) * 4;
                pixels[at..at + 168 * 4].iter().copied()
            })
            .fold(0xcbf29ce484222325u64, |h, byte| {
                (h ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            });
        if hash != expected {
            mismatches.push((name, hash, expected));
        }
        if let Some(path) = &proof {
            std::fs::write(
                path.join(format!("{name}.png")),
                yr_render::png::encode_rgba(800, 600, &pixels),
            )
            .unwrap();
        }
    }
    assert!(mismatches.is_empty(), "original headings: {mismatches:?}");
}

#[test]
fn retail_skirmish_preview_matches_original_unoccluded_pixels() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let art = Art::load(&fs);
    let proof = std::env::var_os("OPENYR_PREVIEW_PROOF").map(std::path::PathBuf::from);
    let mut mismatches = Vec::new();
    for (name, asset, size, top, markers, expected, expected_samples) in [
        (
            "alamo",
            "XMP29U2.MAP",
            (110, 50),
            0,
            [(765, 65, 18, 18), (647, 107, 18, 18)],
            0x4ab7b3efc5ec5dc9u64,
            25_392,
        ),
        (
            "lake",
            "XMP08T2.MAP",
            (70, 67),
            37,
            [(668, 49, 18, 18), (698, 126, 18, 18)],
            0x22dd8120185cff9d,
            19_176,
        ),
    ] {
        let map = load_preview(dir.to_str().unwrap(), asset, 1).unwrap();
        assert!(map.authored);
        assert_eq!((map.width, map.height), size);
        let mut state = State {
            front: true,
            ..State::default()
        };
        state.set(Page::Skirmish);
        let mut pixels = vec![0; 800 * 600 * 4];
        draw(
            &mut pixels,
            800,
            600,
            &art,
            Scene {
                state: &state,
                movie: None,
                preview: Some(&map),
                phase: 0,
                cursor: (-1.0, -1.0),
            },
        );
        if let Some(path) = &proof {
            std::fs::create_dir_all(path).unwrap();
            std::fs::write(
                path.join(format!("{name}.png")),
                yr_render::png::encode_rgba(800, 600, &pixels),
            )
            .unwrap();
        }
        let mut hash = 0xcbf29ce484222325u64;
        let mut samples = 0;
        // Start markers are separate work in #152. Modal heading color is #153,
        // so the Lake observation checks the preview and surrounding rim only.
        for y in top..155 {
            for x in 632..800 {
                if markers
                    .iter()
                    .any(|&(mx, my, w, h)| (mx..mx + w).contains(&x) && (my..my + h).contains(&y))
                {
                    continue;
                }
                let at = (y * 800 + x) * 4;
                for &byte in &pixels[at..at + 4] {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
                }
                samples += 1;
            }
        }
        assert_eq!(samples, expected_samples, "{name}");
        if hash != expected {
            mismatches.push((name, hash, expected));
        }
    }
    assert!(mismatches.is_empty(), "original previews: {mismatches:?}");
}

#[test]
fn preview_client_fits_both_axes_and_keeps_invalid_images_black() {
    let render = |preview: Option<&Preview>| {
        let mut state = State {
            front: true,
            ..State::default()
        };
        state.set(Page::Skirmish);
        let mut frame = vec![0; 800 * 600 * 4];
        draw(
            &mut frame,
            800,
            600,
            &Art::default(),
            Scene {
                state: &state,
                movie: None,
                preview,
                phase: 0,
                cursor: (-1.0, -1.0),
            },
        );
        frame
    };
    let pixel = |frame: &[u8], x: usize, y: usize| {
        let at = (y * 800 + x) * 4;
        <[u8; 4]>::try_from(&frame[at..at + 4]).unwrap()
    };
    let red = [248, 0, 0, 255];
    let green = [0, 252, 0, 255];
    let black = [0, 0, 0, 255];
    let mut map = Preview {
        width: 2,
        height: 1,
        rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
        authored: true,
    };
    let wide = render(Some(&map));
    assert_eq!(pixel(&wide, 644, 57), red);
    assert_eq!(pixel(&wide, 715, 128), red);
    assert_eq!(pixel(&wide, 716, 57), green);
    assert_eq!(pixel(&wide, 787, 128), green);
    for (x, y) in [(644, 37), (644, 56), (788, 57), (787, 129), (788, 149)] {
        assert_eq!(pixel(&wide, x, y), black);
    }
    map.width = 1;
    map.height = 2;
    let tall = render(Some(&map));
    assert_eq!(pixel(&tall, 688, 37), red);
    assert_eq!(pixel(&tall, 743, 92), red);
    assert_eq!(pixel(&tall, 688, 93), green);
    assert_eq!(pixel(&tall, 743, 148), green);
    for (x, y) in [(687, 37), (744, 37), (688, 149)] {
        assert_eq!(pixel(&tall, x, y), black);
    }
    for (width, height, rgba) in [
        (0, 1, Vec::new()),
        (1, 0, Vec::new()),
        (2, 1, vec![0; 7]),
        (u32::MAX, u32::MAX, Vec::new()),
    ] {
        map.width = width;
        map.height = height;
        map.rgba = rgba;
        let frame = render(Some(&map));
        assert_eq!(pixel(&frame, 644, 37), black);
        assert_eq!(pixel(&frame, 788, 149), black);
    }
    let unavailable = render(None);
    assert_eq!(pixel(&unavailable, 700, 50), [8, 12, 16, 255]);
}

#[test]
fn retail_front_static_regions_match_original_renderer_observations() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let art = Art::load(&fs);
    // FNV-1a of RGBA crops from the local retail renderer, 2026-11-01.
    // Coordinates, raw capture SHA256 and scope are in formats/front-menu.md.
    for (page, regions) in [
        (
            Page::Main,
            vec![
                ((632, 0, 168, 8), 0xbb8be164b63a8b65u64),
                ((632, 140, 168, 59), 0x3800aa0f95589be5),
                ((0, 570, 632, 8), 0x5916703b917a4c81),
                ((632, 409, 168, 126), 0xf479b292bacc4861),
                ((632, 577, 168, 4), 0x81c8ee303ad1f191),
                ((644, 199, 156, 12), 0x8b0d40ddf7ddee4d),
                ((644, 230, 156, 11), 0xaa9d92d10df12785),
                ((644, 199, 156, 42), 0x5cc895a15249c25d),
                ((644, 367, 156, 42), 0x7f8187862b1871fd),
                ((644, 535, 156, 42), 0xef59c7ba54f7c219),
            ],
        ),
        (
            Page::SinglePlayer,
            vec![
                ((632, 325, 168, 210), 0xb72a48ec29d9b121),
                ((644, 283, 156, 42), 0xfd4fc484ed4c627d),
                ((644, 535, 156, 42), 0x84ab987638a33ca5),
            ],
        ),
    ] {
        let mut state = State {
            front: true,
            ..State::default()
        };
        state.set(page);
        let mut pixels = vec![0; 800 * 600 * 4];
        draw(
            &mut pixels,
            800,
            600,
            &art,
            Scene {
                state: &state,
                movie: None,
                preview: None,
                phase: 0,
                cursor: (-1.0, -1.0),
            },
        );
        for ((x, y, width, height), expected) in regions {
            let hash = (y..y + height)
                .flat_map(|row| {
                    let at = (row * 800 + x) * 4;
                    pixels[at..at + width * 4].iter().copied()
                })
                .fold(0xcbf29ce484222325u64, |h, byte| {
                    (h ^ u64::from(byte)).wrapping_mul(0x100000001b3)
                });
            assert_eq!(
                hash, expected,
                "{:?} region ({x},{y},{width},{height})",
                state.page
            );
        }
    }
}
#[test]
fn front_navigation_geometry_and_disabled_modes_are_consistent() {
    let mut s = State {
        front: true,
        ..State::default()
    };
    s.set(Page::Main);
    assert!(!s.keyboard_focus);
    s.navigate(1);
    assert!(s.keyboard_focus);
    assert_eq!(s.buttons()[s.focus].0, Button::Controls);
    s.set(Page::Main);
    assert!(!s.keyboard_focus);
    for b in [Button::Internet, Button::Network, Button::Movies] {
        assert_eq!(s.activate(b), Effect::None);
        assert_eq!(s.page, Page::Main);
    }
    s.activate(Button::SinglePlayer);
    assert_eq!(s.page, Page::SinglePlayer);
    assert_eq!(s.activate(Button::Campaign), Effect::None);
    s.activate(Button::Load);
    s.escape();
    assert_eq!(s.page, Page::SinglePlayer);
    s.activate(Button::MainMenu);
    s.activate(Button::Controls);
    s.escape();
    assert_eq!(s.page, Page::Main);
    s.escape();
    assert_eq!(s.page, Page::Exit);
    s.escape();
    assert_eq!(s.page, Page::Main);
    for (w, h) in [(1280, 800), (800, 600), (640, 480), (400, 300)] {
        let l = Layout::for_state(&s, w, h);
        let (x, y, _, _) = l.button(0, Button::SinglePlayer);
        assert_eq!(
            l.hit(
                &s,
                (
                    l.origin.0 + f64::from(x + 4) * l.scale,
                    l.origin.1 + f64::from(y + 4) * l.scale
                )
            ),
            Some(Button::SinglePlayer)
        );
        assert_eq!(l.hit(&s, (-1.0, 0.0)), None);
    }
}
#[test]
fn retail_shell_uses_its_own_button_palette_and_preserves_source_pixels() {
    use yr_formats::{Palette, ShpFile};
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let art = Art::load(&fs);
    if let Some(out) = std::env::var_os("OPENYR_FRONT_CALIBRATION_PROOF") {
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        let write = |name: &str, width, height, pixels: &[u8]| {
            std::fs::write(
                out.join(format!("{name}.png")),
                yr_render::png::encode_rgba(width, height, pixels),
            )
            .unwrap();
        };
        for (name, page) in [("main", Page::Main), ("single-player", Page::SinglePlayer)] {
            let mut state = State {
                front: true,
                ..State::default()
            };
            state.set(page);
            let mut pixels = vec![0; 800 * 600 * 4];
            draw(
                &mut pixels,
                800,
                600,
                &art,
                Scene {
                    state: &state,
                    movie: None,
                    preview: None,
                    phase: 0,
                    cursor: (-1.0, -1.0),
                },
            );
            write(name, 800, 600, &pixels);
        }
    }
    for (name, palette, images) in [
        ("SDTP.SHP", "SHELL.PAL", &art.top),
        ("SDWRNANM.SHP", "SHELL2.PAL", &art.warning),
        ("SDBTNANM.SHP", "SDBTNANM.PAL", &art.buttons),
    ] {
        let s = ShpFile::parse(fs.read(name).unwrap()).unwrap();
        let p = Palette::parse(fs.read(palette).unwrap()).unwrap();
        assert_eq!(images.len(), s.frames.len());
        for (i, image) in images.iter().enumerate() {
            for (index, value) in s.decode_frame_canvas(i).unwrap().into_iter().enumerate() {
                assert_eq!(
                    &image.pixels[index * 4..index * 4 + 4],
                    &if value == 0 { [0; 4] } else { p.rgba(value) },
                    "{name} frame {i}"
                );
            }
        }
    }
    let s = State {
        front: true,
        page: Page::Main,
        ..State::default()
    };
    let mut frame = vec![0; 800 * 600 * 4];
    draw(
        &mut frame,
        800,
        600,
        &art,
        Scene {
            state: &s,
            movie: None,
            preview: None,
            phase: 0,
            cursor: (-1.0, -1.0),
        },
    );
    for y in [0, 1, 2, 40, 41] {
        for x in 0..156 {
            let source = (y * 156 + x) * 4;
            let pixel = &art.buttons[2].pixels[source..source + 4];
            if pixel[3] != 0 {
                let dest = ((199 + y) * 800 + 644 + x) * 4;
                assert_eq!(
                    &frame[dest..dest + 4],
                    &[pixel[0] & 0xf8, pixel[1] & 0xfc, pixel[2] & 0xf8, pixel[3]],
                );
            }
        }
    }
}
