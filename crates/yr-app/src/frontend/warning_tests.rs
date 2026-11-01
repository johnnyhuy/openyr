use super::*;
use crate::menu::{Page, State};

#[test]
fn retail_warning_frames_match_original_capture_well_and_wrap() {
    let directory = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !directory.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&directory).unwrap();
    let art = Art::load(&fs);
    assert_eq!(art.warning.len(), 91);
    let state = State {
        front: true,
        page: Page::Main,
        ..State::default()
    };
    // FNV-1a of observed RGBA at (660,40,112,68), including the display rim.
    // These are original renderer captures, not hashes of our decoded sheets.
    let observations = [
        (5, 0x4dc254e8633fd125u64),
        (11, 0x8d781dd4ee1e2b8d),
        (21, 0xeae8c94c93fd9a15),
        (22, 0x6875aade37dd4521),
        (27, 0xef49ef5f2a2738cd),
        (33, 0xf13d843e271ef2a9),
        (37, 0xd0096beedfa84471),
        (42, 0x6ac1f9df3b985f11),
        (44, 0x77d8b29558bb19b5),
        (48, 0x6a82df8c84817935),
        (89, 0xf96c78ff6ffd8045),
    ];
    let mut mismatches = Vec::new();
    let proof = std::env::var_os("OPENYR_WARNING_PROOF").map(std::path::PathBuf::from);
    if let Some(path) = &proof {
        std::fs::create_dir_all(path).unwrap();
    }
    for (index, expected) in observations {
        for lap in 0..if index == 89 { 3 } else { 1 } {
            let phase = index + lap * 91;
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
                    phase,
                    cursor: (-1.0, -1.0),
                },
            );
            let hash = (40..108)
                .flat_map(|row| {
                    let at = (row * 800 + 660) * 4;
                    pixels[at..at + 112 * 4].iter().copied()
                })
                .fold(0xcbf29ce484222325u64, |h, byte| {
                    (h ^ u64::from(byte)).wrapping_mul(0x100000001b3)
                });
            if hash != expected {
                mismatches.push((phase, hash, expected));
            }
            if lap == 0 {
                if let Some(path) = &proof {
                    std::fs::write(
                        path.join(format!("warning-frame-{index:02}.png")),
                        yr_render::png::encode_rgba(800, 600, &pixels),
                    )
                    .unwrap();
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "original warning mismatches: {mismatches:?}"
    );
}
