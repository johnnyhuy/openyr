//! Side-specific sidebar assets, kept outside the world's shared filename namespace.

use yr_assets::AssetFs;
use yr_formats::{mix_id, MixArchive, Palette, ShpFile, UnicodeFont};

pub(crate) struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Default)]
pub struct SidebarArt {
    pub(super) tab_size: Option<(u16, u16)>,
    pub(crate) font: Option<UnicodeFont>,
    pub(super) credits: Option<Image>,
    pub(super) top: Option<Image>,
    pub(super) radar: Vec<Image>,
    pub(super) header: Option<Image>,
    pub(super) row: Option<Image>,
    pub(super) bottom: Option<Image>,
    pub(super) repair: Vec<Image>,
    pub(super) sell: Vec<Image>,
    pub(super) power: Vec<Image>,
    pub(super) tabs: [Vec<Image>; 4],
    /// Coverage only. The clock's palette colour is not painted on the cameo.
    pub(super) clock: Vec<Vec<u8>>,
}

impl SidebarArt {
    pub fn load(fs: &AssetFs) -> Self {
        Self::load_for_side(fs, crate::match_config::Side::Allied)
    }
    pub fn load_for_side(fs: &AssetFs, side: crate::match_config::Side) -> Self {
        let (control_w, control_h, pip_w, tab_w, tab_h) = match side {
            crate::match_config::Side::Allied => (64, 31, 12, 28, 27),
            crate::match_config::Side::Soviet => (52, 32, 16, 32, 28),
        };
        let font = fs
            .read("GAME.FNT")
            .and_then(|data| match UnicodeFont::parse(data) {
                Ok(font) if font.stride <= 8 && font.lines <= 32 && font.line_height <= 40 => {
                    Some(font)
                }
                Ok(_) => {
                    eprintln!("OpenYR: UI font exceeds supported text dimensions");
                    None
                }
                Err(e) => {
                    eprintln!("OpenYR: UI font unavailable ({e})");
                    None
                }
            });
        // SIDEC archives repeat
        // filenames, so mounting all sides would silently mix their artwork.
        let archive = fs.read(side.sidebar()).and_then(|bytes| {
            MixArchive::parse(bytes)
                .inspect_err(|_| eprintln!("OpenYR: cannot decode selected sidebar archive"))
                .ok()
        });
        let read = |name: &str| {
            fs.read_loose(name)
                .or_else(|| archive.as_ref()?.get_by_id(mix_id(name)))
        };
        let Some(palette) = read("SIDEBAR.PAL").and_then(Palette::parse) else {
            return Self {
                font,
                tab_size: Some((tab_w, tab_h)),
                ..Self::default()
            };
        };
        let frames = |name: &str, w: u16, h: u16, count: usize| -> Vec<Image> {
            let Some(shp) = read(name).and_then(|data| ShpFile::parse(data).ok()) else {
                return Vec::new();
            };
            // Bound decoded UI memory by the observed retail canvas/count.
            // A malformed optional sheet must not hide the remaining panel.
            if (shp.width, shp.height, shp.frames.len()) != (w, h, count) {
                eprintln!("OpenYR: unexpected sidebar sheet {name}");
                return Vec::new();
            }
            (0..count)
                .map(|frame| {
                    let indices = shp.decode_frame_canvas(frame).ok()?;
                    let pixels = indices
                        .into_iter()
                        .flat_map(|i| if i == 0 { [0; 4] } else { palette.rgba(i) })
                        .collect();
                    Some(Image {
                        width: u32::from(w),
                        height: u32::from(h),
                        pixels,
                    })
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default()
        };
        let one = |name, w, h| frames(name, w, h, 1).pop();
        Self {
            font,
            tab_size: Some((tab_w, tab_h)),
            credits: one("CREDITS.SHP", 168, 16),
            top: one("TOP.SHP", 168, 32),
            radar: frames("RADAR.SHP", 168, 110, 33),
            header: one("SIDE1.SHP", 168, 69),
            row: one("SIDE2.SHP", 168, 50),
            bottom: one("SIDE3.SHP", 168, 26),
            repair: frames("REPAIR.SHP", control_w, control_h, 2),
            sell: frames("SELL.SHP", control_w, control_h, 2),
            power: frames("POWERP.SHP", pip_w, 2, 5),
            tabs: std::array::from_fn(|i| frames(&format!("TAB{i:02}.SHP"), tab_w, tab_h, 5)),
            clock: frames("GCLOCK2.SHP", 60, 48, 55)
                .into_iter()
                .map(|image| {
                    image
                        .pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|p| p[3])
                        .collect()
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidebar::{self, Category};
    use std::path::PathBuf;

    #[test]
    fn retail_soviet_controls_use_source_dimensions_and_bounded_hits() {
        let dir = std::env::var_os("OPENYR_GAMEDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
            });
        if !dir.is_dir() {
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let mut config = crate::match_config::MatchConfig::default();
        config.players[0].country = crate::match_config::Country::Russia;
        let session = crate::session::load_with_config(
            dir.to_str().unwrap(),
            "EB1.MMX",
            42,
            Default::default(),
            config,
        )
        .unwrap();
        let art = &session.sidebar_art;
        let archive = MixArchive::parse(fs.read("SIDEC02.MIX").unwrap()).unwrap();
        let palette = Palette::parse(archive.get_by_id(mix_id("SIDEBAR.PAL")).unwrap()).unwrap();
        for (name, images, dimensions) in [
            ("REPAIR.SHP", &art.repair, (52, 32, 2)),
            ("SELL.SHP", &art.sell, (52, 32, 2)),
            ("POWERP.SHP", &art.power, (16, 2, 5)),
            ("TAB00.SHP", &art.tabs[0], (32, 28, 5)),
        ] {
            assert_eq!(images.len(), dimensions.2);
            let source = ShpFile::parse(archive.get_by_id(mix_id(name)).unwrap()).unwrap();
            for (index, image) in images.iter().enumerate() {
                assert_eq!((image.width, image.height), (dimensions.0, dimensions.1));
                let expected: Vec<_> = source
                    .decode_frame_canvas(index)
                    .unwrap()
                    .into_iter()
                    .flat_map(|i| if i == 0 { [0; 4] } else { palette.rgba(i) })
                    .collect();
                assert_eq!(image.pixels, expected, "{name} frame {index}");
            }
        }
        for y in 160..201 {
            for x in 0..168 {
                let _ = sidebar::control_at(168, &session, (f64::from(x), f64::from(y)));
            }
        }
        for (i, category) in Category::ALL.into_iter().enumerate() {
            assert_eq!(
                sidebar::session_tab_at(168, &session, ((26 + i * 32 + 31) as f64, 224.0)),
                Some(category)
            );
        }
    }

    #[test]
    fn retail_sidebar_matches_source_pixels_and_all_clock_phases() {
        let dir = PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            eprintln!("skip: retail install not present");
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let archive = MixArchive::parse(fs.read("SIDEC01.MIX").unwrap()).unwrap();
        let source = |name: &str| ShpFile::parse(archive.get_by_id(mix_id(name)).unwrap()).unwrap();
        let palette = Palette::parse(archive.get_by_id(mix_id("SIDEBAR.PAL")).unwrap()).unwrap();
        let mut session = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 11).unwrap();
        session.sim.set_ai(None);
        let art = &session.sidebar_art;
        assert_eq!(art.clock.len(), 55);
        let clock = source("GCLOCK2.SHP");
        let mut previous = 0;
        for (i, mask) in art.clock[..55].iter().enumerate() {
            let raw = clock.decode_frame_canvas(i).unwrap();
            assert_eq!(mask.len(), 60 * 48);
            for (&pixel, &coverage) in raw.iter().zip(mask) {
                assert_eq!(coverage != 0, pixel != 0, "clock {i}");
            }
            let covered = raw.iter().filter(|&&p| p != 0).count();
            if i > 1 {
                assert!(covered < previous, "clock coverage shrinks after frame 1");
            }
            previous = covered;
        }

        assert!(art.clock[0].iter().all(|&p| p == 0));
        assert_eq!(previous, 60, "last unfinished mask is one column");

        // Check the actual drawn canvas at the observed screen coordinates,
        // using separately decoded SHPs rather than the cached RGBA images.
        let width = 1280;
        let height = 800;
        let x0 = width - sidebar::WIDTH;
        let mut frame = vec![0; width as usize * height as usize * 4];
        sidebar::draw(
            &mut frame,
            width,
            height,
            &session,
            Category::Vehicles,
            None,
        );
        for (name, frame_index, x, y) in [
            ("CREDITS.SHP", 0, x0, 0),
            ("TOP.SHP", 0, x0, 16),
            ("RADAR.SHP", 0, x0, 48),
            ("TAB00.SHP", 0, x0 + 26, 197),
            ("TAB01.SHP", 0, x0 + 54, 197),
            ("TAB02.SHP", 0, x0 + 82, 197),
            ("TAB03.SHP", 1, x0 + 110, 197),
            ("SIDE2.SHP", 0, x0, 427),
            ("SIDE3.SHP", 0, x0, 774),
        ] {
            let shp = source(name);
            let indices = shp.decode_frame_canvas(frame_index).unwrap();
            let mut checked = 0;
            for (p, &index) in indices.iter().enumerate() {
                let (dx, dy) = (
                    p as u32 % u32::from(shp.width),
                    p as u32 / u32::from(shp.width),
                );
                // Credit glyphs overlay the source strip.
                if index == 0 || (name == "CREDITS.SHP" && (4..13).contains(&dy)) {
                    continue;
                }
                let at = (((y + dy) * width + x + dx) * 4) as usize;
                assert_eq!(&frame[at..at + 4], &palette.rgba(index), "{name} {dx},{dy}");
                checked += 1;
            }
            assert!(checked > 100, "{name} checked visible source pixels");
        }
        assert!(
            fs.read("SIDE1.SHP").is_none(),
            "side namespace stays separate"
        );

        // Every real queue phase is compared to the raw retail mask over
        // the untouched cameo. A credit stall must freeze its coverage.
        let slot = sidebar::slots(&session, Category::Structures)
            .next()
            .unwrap();
        sidebar::draw(
            &mut frame,
            width,
            height,
            &session,
            Category::Structures,
            None,
        );
        let baseline = frame.clone();
        session.step(&[session.produce(slot).unwrap()]);
        let mut phases = std::collections::BTreeSet::new();
        while session.ready_structure().is_none() {
            let (_, Some((done, total))) = session.slot_status(slot) else {
                panic!("queue head")
            };
            let phase = 1 + ((i128::from(done) * 54) / i128::from(total)) as usize;
            let mask = clock.decode_frame_canvas(phase).unwrap();
            sidebar::draw(
                &mut frame,
                width,
                height,
                &session,
                Category::Structures,
                None,
            );
            for (p, &coverage) in mask.iter().enumerate() {
                let at = (((228 + p as u32 / 60) * width + x0 + 23 + p as u32 % 60) * 4) as usize;
                for c in 0..3 {
                    let expected = if coverage != 0 {
                        baseline[at + c] / 3
                    } else {
                        baseline[at + c]
                    };
                    assert_eq!(
                        frame[at + c],
                        expected,
                        "phase {phase} pixel {p} channel {c}"
                    );
                }
            }
            if phases.insert(phase) {
                if let Some(out) = std::env::var_os("OPENYR_RETAIL_UI_DUMP") {
                    std::fs::create_dir_all(&out).unwrap();
                    std::fs::write(
                        PathBuf::from(out).join(format!("clock-{:02}.png", phase - 1)),
                        yr_render::png::encode_rgba(width, height, &frame),
                    )
                    .unwrap();
                }
            }
            if phase == 27 {
                let funded = frame.clone();
                session.sim.set_credits(0);
                sidebar::draw(
                    &mut frame,
                    width,
                    height,
                    &session,
                    Category::Structures,
                    None,
                );
                for y in 228..276 {
                    let start = ((y * width + x0 + 23) * 4) as usize;
                    assert_eq!(&frame[start..start + 60 * 4], &funded[start..start + 60 * 4],
                        "an active paid head keeps its brightness when credits fall below full price");
                }
                let status = session.slot_status(slot);
                for _ in 0..8 {
                    session.step(&[]);
                }
                assert_eq!(
                    session.slot_status(slot),
                    status,
                    "credit starvation freezes the clock"
                );
                session.sim.set_credits(10_000);
            }
            session.step(&[]);
        }
        assert_eq!(phases.len(), 54, "all unfinished phases covered");
        assert_eq!(session.ready_structure(), Some(slot));
        let hash = session.sim.state_hash();
        sidebar::draw(
            &mut frame,
            width,
            height,
            &session,
            Category::Structures,
            None,
        );
        sidebar::draw_tooltip(
            &mut frame,
            width,
            height,
            &session,
            Category::Structures,
            (f64::from(x0 + 24), 229.0),
        );
        assert_eq!(session.sim.state_hash(), hash);
        assert_eq!(
            sidebar::slot_at(
                width,
                260,
                &session,
                Category::Structures,
                (f64::from(x0 + 24), 229.0)
            ),
            None,
            "a clipped cameo cannot be clicked"
        );
        if let Some(out) = std::env::var_os("OPENYR_RETAIL_UI_DUMP") {
            std::fs::write(
                PathBuf::from(out).join("ready-tooltip.png"),
                yr_render::png::encode_rgba(width, height, &frame),
            )
            .unwrap();
        }
    }
}
