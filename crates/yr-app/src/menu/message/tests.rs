use super::*;

#[test]
fn pause_compositor_paints_a_complete_line_beyond_the_old_character_cap() {
    let mut bytes = b"fonT".to_vec();
    for value in [8u32, 1, 1, 1, 1, 2] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.resize(28 + 65536 * 2, 0);
    let at = 28 + 'A' as usize * 2;
    bytes[at..at + 2].copy_from_slice(&1u16.to_le_bytes());
    bytes.extend([1, 128]);
    let art = Art {
        font: Some(yr_formats::UnicodeFont::parse(&bytes).unwrap()),
        ..Art::default()
    };
    let mut state = State::default();
    let render = |state: &State| {
        let mut frame = vec![0; 800 * 600 * 4];
        super::super::draw(&mut frame, 800, 600, &art, state, (-1.0, -1.0));
        frame
    };
    state.message(String::new(), Page::Load);
    let mut expected = render(&state);
    for index in 0..200 {
        let at = ((72 * 800 + 40 + index * 2) * 4) as usize;
        expected[at..at + 4].copy_from_slice(&[220, 232, 240, 255]);
    }
    state.message("A".repeat(200), Page::Load);
    assert!(
        render(&state) == expected,
        "the compositor must paint all 200 glyphs on the measured line"
    );
}

#[test]
fn pause_wrap_scroll_and_resize_preserve_the_complete_diagnostic() {
    let mut art = Art::default();
    check(&art);
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if dir.is_dir() {
        art = Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
        assert!(art.font.is_some());
        check(&art);
    }

    fn check(art: &Art) {
        let face = Typeface(art.font.as_ref());
        let text = format!(
            "Could not load match.\n\n{}\n{}\nEND: select another save or return to the match.",
            "Wide words WWW iii. ".repeat(250),
            "界é😀".repeat(500)
        );
        for size in [(640, 480), (800, 600), (1280, 800), (360, 240)] {
            let region = Region::pause(Layout::new(size.0, size.1));
            let wrapped: Vec<_> = lines(face, &text, region.width).collect();
            assert_eq!(wrapped[0], "Could not load match.");
            assert_eq!(wrapped[1], "");
            assert_eq!(
                wrapped
                    .concat()
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>(),
                text.chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>()
            );
            for line in &wrapped {
                let width: i32 = line.chars().map(|ch| face.advance(ch)).sum();
                let trailing = line
                    .chars()
                    .last()
                    .map_or(0, |ch| i32::from(face.glyph(ch).is_some()));
                assert!(width - trailing <= region.width);
            }
            let mut state = State::default();
            state.message(text.clone(), Page::Load);
            let (_, rows, count) = region.viewport(face, &text, 0);
            let mut visited = rows;
            while state.message_scroll < count - rows {
                assert!(scroll_message(art, &mut state, size, rows as i32));
                visited = visited.max(state.message_scroll + rows);
            }
            assert_eq!(visited, count);
            assert!(wrapped.last().unwrap().ends_with("return to the match."));
            assert!(region.top + rows as i32 * line_step(face) <= region.top + region.height);
            assert!(scroll_message(art, &mut state, size, i32::MAX));
            assert_eq!(state.message_scroll, count - rows);
            assert!(scroll_message(art, &mut state, size, i32::MIN));
            assert_eq!(state.message_scroll, 0);
            state.page = Page::Load;
            assert!(!scroll_message(art, &mut state, size, 1));
        }
    }
}

#[test]
fn oversized_unicode_glyphs_always_advance() {
    let text = "界é😀";
    assert_eq!(lines(Typeface(None), text, 1).collect::<String>(), text);
}
