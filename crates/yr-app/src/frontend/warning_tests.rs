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
        (9, 0x7c738541f7e74821u64),
        (11, 0x8d781dd4ee1e2b8du64),
        (13, 0xab493259c695a691u64),
        (21, 0xeae8c94c93fd9a15u64),
        (22, 0x6875aade37dd4521u64),
        (26, 0xc3490219909c2bd9u64),
        (27, 0xef49ef5f2a2738cdu64),
        (28, 0x0d212b768b7aa781u64),
        (31, 0xbd1a72fc98b593ddu64),
        (33, 0xf13d843e271ef2a9u64),
        (37, 0xd0096beedfa84471u64),
        (40, 0x702455d076dbc3c9u64),
        (42, 0x6ac1f9df3b985f11u64),
        (44, 0x77d8b29558bb19b5u64),
        (46, 0x1aa71ad9ef6c82b9u64),
        (48, 0x6a82df8c84817935u64),
        (53, 0x7d4879f53df7c9cdu64),
        (63, 0xa735e8a15e216ff5u64),
        (67, 0x6815fec4423b7479u64),
        (75, 0xe29eef20daddaca5u64),
        (79, 0x9a967d9afc820ff5u64),
        (82, 0x6723e0045e6e609du64),
        (83, 0x691aac2e863f86c9u64),
        (89, 0xf96c78ff6ffd8045u64),
        (90, 0x432770c65ad3108du64),
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
