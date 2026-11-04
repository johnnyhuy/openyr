use super::*;

#[test]
fn bounded_cursor_decode_rejects_hostile_sheets() {
    let palette = [0; 768];
    for (w, h, count) in [(65535u16, 65535u16, 517u16), (55, 43, 65535), (1, 1, 517)] {
        let mut bytes = vec![0; 8 + 24 * 517];
        bytes[2..4].copy_from_slice(&w.to_le_bytes());
        bytes[4..6].copy_from_slice(&h.to_le_bytes());
        bytes[6..8].copy_from_slice(&count.to_le_bytes());
        assert!(Art::decode(&bytes, &palette).is_none());
    }
    assert!(Art::decode(&[0; 7], &palette).is_none());
    assert!(Art::decode(&[], &palette[..767]).is_none());
}

#[test]
fn lifecycle_edges_clipping_and_tick_clock_are_bounded() {
    let art = Art {
        frames: vec![vec![255; WIDTH * HEIGHT * 4]; 517],
    };
    let mut presence = Presence::default();
    assert!(!presence.custom(&art, (80, 60), (20.0, 30.0)));
    presence.inside = true;
    presence.focused = true;
    assert!(presence.custom(&art, (80, 60), (20.0, 30.0)));
    for pos in [(-0.1, 0.0), (80.0, 0.0), (0.0, 60.0), (f64::NAN, 0.0)] {
        assert!(!presence.custom(&art, (80, 60), pos));
        let mut frame = vec![0; 80 * 60 * 4];
        art.draw(&mut frame, (80, 60), pos, Kind::Pointer, 0);
        assert!(frame.iter().all(|&v| v == 0));
    }
    for kind in Kind::ALL {
        let (first, count) = kind.frames();
        for tick in [0, 1, 2, 99, u64::MAX] {
            assert!((first..first + count).contains(&kind.frame(tick)));
        }
        let mut frame = vec![0; 80 * 60 * 4];
        art.draw(&mut frame, (80, 60), (0.0, 0.0), kind, 0);
        assert!(frame.iter().any(|&v| v != 0));
        art.draw(&mut [0; 4], (80, 60), (20.0, 20.0), kind, 0);
    }
    presence.focused = false;
    assert!(!presence.custom(&art, (80, 60), (20.0, 30.0)));
    presence.focused = true;
    presence.inside = false;
    assert!(!presence.custom(&art, (80, 60), (20.0, 30.0)));
    assert!(!Presence {
        inside: true,
        focused: true
    }
    .custom(&Art::default(), (80, 60), (20.0, 30.0)));
}

#[test]
fn retail_yr_and_ra2_frames_palette_and_final_hotspot_pixels_match_source() {
    use std::path::PathBuf;
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let palette = Palette::parse(fs.read("MOUSEPAL.PAL").unwrap()).unwrap();
    let yr = fs.read("MOUSE.SHA").unwrap();
    let ra2 = fs.read("MOUSE.SHP").unwrap();
    let art = Art::load(&fs);
    assert_eq!(art.frames.len(), 517);
    let source = ShpFile::parse(yr).unwrap();
    let old = ShpFile::parse(ra2).unwrap();
    assert_eq!(
        (source.width, source.height, source.frames.len()),
        (55, 43, 517)
    );
    assert_eq!(old.frames.len(), 450);
    for i in 0..517 {
        let raw = source.decode_frame_canvas(i).unwrap();
        for (p, &index) in raw.iter().enumerate() {
            assert_eq!(
                &art.frames[i][p * 4..p * 4 + 4],
                &if index == 0 {
                    [0; 4]
                } else {
                    palette.rgba(index)
                },
                "frame {i}, pixel {p}"
            );
        }
        if i < 450 {
            assert_eq!(
                raw,
                old.decode_frame_canvas(i).unwrap(),
                "YR preserves RA2 frame {i}"
            );
        }
    }
    for kind in Kind::ALL {
        let (start, count) = kind.frames();
        for offset in 0..count {
            let tick = offset as u64 * 2;
            let mut frame = vec![0; 140 * 110 * 4];
            art.draw(&mut frame, (140, 110), (65.75, 55.5), kind, tick);
            let raw = source.decode_frame_canvas(start + offset).unwrap();
            let (hx, hy) = kind.hotspot();
            for (p, &index) in raw.iter().enumerate() {
                let (x, y) = (65 - hx + (p % 55) as i32, 55 - hy + (p / 55) as i32);
                let at = (y * 140 + x) as usize * 4;
                assert_eq!(
                    &frame[at..at + 4],
                    &if index == 0 {
                        [0; 4]
                    } else {
                        palette.rgba(index)
                    },
                    "draw {kind:?}, phase {offset}, pixel {p}"
                );
            }
        }
    }
    assert!(Art::decode(
        &yr[..source.frames.last().unwrap().offset as usize],
        fs.read("MOUSEPAL.PAL").unwrap()
    )
    .is_none());
    assert!(Art::decode(yr, &[0; 767]).is_none());
    if let Some(path) = std::env::var_os("OPENYR_CURSOR_PROOF") {
        let path = PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        for kind in Kind::ALL {
            let (_, count) = kind.frames();
            for phase in 0..count {
                let mut frame = vec![0; 140 * 110 * 4];
                art.draw(&mut frame, (140, 110), (65.0, 55.0), kind, phase as u64 * 2);
                std::fs::write(
                    path.join(format!("{kind:?}-{phase:02}.png")),
                    yr_render::png::encode_rgba(140, 110, &frame),
                )
                .unwrap();
            }
        }
    }
}

#[test]
fn scroll_canvases_remain_inside_each_viewport_edge() {
    let art = Art {
        frames: vec![vec![255; WIDTH * HEIGHT * 4]; 517],
    };
    for direction in Direction::ALL {
        let (x, y) = direction.axes();
        assert_eq!(Direction::from_axes(x, y), Some(direction));
        let axis = |a: i8, size: u32| match a {
            -1 => 0.0,
            1 => f64::from(size - 1),
            _ => f64::from(size / 2),
        };
        for blocked in [false, true] {
            let kind = Kind::Scroll(direction, blocked);
            assert_eq!(
                kind.frames(),
                (2 + direction as usize + usize::from(blocked) * 8, 1)
            );
            let mut frame = vec![0; 140 * 110 * 4];
            art.draw(
                &mut frame,
                (140, 110),
                (axis(x, 140), axis(y, 110)),
                kind,
                u64::MAX,
            );
            assert_eq!(
                frame
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[3] != 0)
                    .count(),
                WIDTH * HEIGHT
            );
        }
    }
    assert_eq!(Direction::from_axes(0, 0), None);
    assert_eq!(Direction::from_axes(2, 1), None);
}
