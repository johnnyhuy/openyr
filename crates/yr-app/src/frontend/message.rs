use super::Art;
use crate::{
    menu::{Page, State},
    sidebar::text::Typeface,
};

pub(super) const LEFT: i32 = 32;
pub(super) const TOP: i32 = 72;
pub(super) const WIDTH: i32 = 568;
const BOTTOM: i32 = 544;

pub(super) use crate::menu::message::line_step;

pub(super) fn visible_lines(face: Typeface<'_>) -> usize {
    ((BOTTOM - TOP) / line_step(face)) as usize
}

pub(super) fn lines<'a>(face: Typeface<'a>, text: &'a str) -> impl Iterator<Item = &'a str> {
    crate::menu::message::lines(face, text, WIDTH)
}

pub(super) fn viewport(face: Typeface<'_>, text: &str, requested: usize) -> (usize, usize, usize) {
    let count = lines(face, text).count();
    let rows = visible_lines(face);
    (requested.min(count.saturating_sub(rows)), rows, count)
}

/// Front recovery messages use the same layout for input and composition.
/// Return false so callers can retain their other menu scrolling behavior.
pub fn scroll_message(art: &Art, state: &mut State, delta: i32) -> bool {
    if !state.front {
        return false;
    }
    let Page::Message { text, .. } = &state.page else {
        return false;
    };
    let face = Typeface(art.font.as_ref());
    let (first, rows, count) = viewport(face, text, state.message_scroll);
    state.message_scroll = first
        .saturating_add_signed(delta as isize)
        .min(count.saturating_sub(rows));
    true
}

/// Front confirmations keep their own font and authored text region.
pub fn scroll_confirmation(art: &Art, state: &mut State, size: (u32, u32), delta: i32) -> bool {
    if !state.front {
        return false;
    }
    crate::menu::confirmation::scroll(art.font.as_ref(), state, size, delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{draw, Scene};

    #[test]
    fn fallback_messages_keep_words_paragraphs_and_every_long_token_character() {
        let face = Typeface(None);
        let sentence = format!("{}setup failed in OpenYR.", "Try again. ".repeat(6));
        assert_eq!(
            lines(face, &sentence).collect::<Vec<_>>(),
            [
                format!("{}setup", "Try again. ".repeat(6)),
                "failed in OpenYR.".into()
            ]
        );
        assert_eq!(
            lines(face, " First line.\r\n\nSecond line.\n").collect::<Vec<_>>(),
            ["First line.", "", "Second line.", ""]
        );
        for token in ["W".repeat(10000), "界é😀".repeat(1000)] {
            let wrapped: Vec<_> = lines(face, &token).collect();
            assert_eq!(wrapped.concat(), token);
            assert!(wrapped
                .iter()
                .all(|line| line.chars().count() * 8 <= WIDTH as usize));
        }
    }

    #[test]
    fn compositor_paints_a_complete_measured_line_beyond_old_character_caps() {
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
        let mut state = State {
            front: true,
            ..State::default()
        };
        let token = "A".repeat(280);
        assert_eq!(
            lines(Typeface(art.font.as_ref()), &token).collect::<Vec<_>>(),
            [token.as_str()]
        );
        let render = |state: &State| {
            let mut frame = vec![0; 800 * 600 * 4];
            draw(
                &mut frame,
                800,
                600,
                &art,
                Scene {
                    state,
                    movie: None,
                    preview: None,
                    phase: 0,
                    cursor: (-1.0, -1.0),
                },
            );
            frame
        };
        state.message(String::new(), Page::Load);
        let mut expected = render(&state);
        for index in 0..280 {
            let at = ((TOP * 800 + LEFT + index * 2) * 4) as usize;
            expected[at..at + 4].copy_from_slice(&[216, 232, 240, 255]);
        }
        state.message(token, Page::Load);
        assert_eq!(
            render(&state),
            expected,
            "all 280 source glyph pixels paint on one line"
        );
    }

    #[test]
    fn retail_recovery_messages_use_font_width_and_stay_inside_the_panel() {
        let directory = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !directory.is_dir() {
            return;
        }
        let fs = yr_assets::AssetFs::open(&directory).unwrap();
        let art = Art::load(&fs);
        let face = Typeface(art.font.as_ref());
        assert!(face.0.is_some());
        let narrow = "i".repeat(180);
        assert_eq!(lines(face, &narrow).collect::<Vec<_>>(), [narrow.as_str()]);
        let token = "W界é😀".repeat(500);
        let wrapped: Vec<_> = lines(face, &token).collect();
        assert_eq!(wrapped.concat(), token);
        assert!(wrapped.iter().all(|line| {
            let width: i32 = line.chars().map(|ch| face.advance(ch)).sum();
            width <= WIDTH + 1
        }));
        let render = |state: &State, width, height| {
            let mut pixels = vec![0; (width * height * 4) as usize];
            draw(
                &mut pixels,
                width,
                height,
                &art,
                Scene {
                    state,
                    movie: None,
                    preview: None,
                    phase: 0,
                    cursor: (-1.0, -1.0),
                },
            );
            pixels
        };
        let mut state = State {
            front: true,
            ..State::default()
        };
        state.message(String::new(), Page::Load);
        let baseline = render(&state, 800, 600);
        for text in [
            "Unable to load this match.".into(),
            "The saved match is incompatible.\n\nReturn to setup and try again in OpenYR.".into(),
            narrow,
            token,
        ] {
            state.message(text, Page::Load);
            let Page::Message { text, .. } = &state.page else {
                unreachable!()
            };
            let count = lines(face, text).count();
            let mut visited = 0;
            loop {
                let frame = render(&state, 800, 600);
                let mut changed = 0;
                for (at, (actual, original)) in frame
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(baseline.as_chunks::<4>().0)
                    .enumerate()
                {
                    if actual != original {
                        let (x, y) = ((at % 800) as i32, (at / 800) as i32);
                        assert!(
                            (LEFT..LEFT + WIDTH).contains(&x) && (TOP..BOTTOM).contains(&y)
                                || (8..618).contains(&x) && (578..600).contains(&y),
                            "message escaped its panel at {x},{y}"
                        );
                        changed += 1;
                    }
                }
                assert!(changed > 0);
                visited = visited.max((state.message_scroll + visible_lines(face)).min(count));
                let before = state.message_scroll;
                assert!(scroll_message(&art, &mut state, visible_lines(face) as i32));
                if state.message_scroll == before {
                    break;
                }
            }
            assert_eq!(visited, count, "every line can be reached");
            for (width, height) in [(1280, 800), (640, 480), (400, 300)] {
                let frame = render(&state, width, height);
                assert_eq!(frame.len(), (width * height * 4) as usize);
                let layout = crate::menu::Layout::for_state(&state, width, height);
                let (x, y, _, _) = layout.button(0, crate::menu::Button::Back);
                assert_eq!(
                    layout.hit(
                        &state,
                        (
                            layout.origin.0 + f64::from(x + 4) * layout.scale,
                            layout.origin.1 + f64::from(y + 4) * layout.scale,
                        )
                    ),
                    Some(crate::menu::Button::Back)
                );
            }
            state.message("Another failure".into(), Page::Load);
            assert_eq!(state.message_scroll, 0);
            state.escape();
            assert_eq!(state.page, Page::Load);
            assert!(!scroll_message(&art, &mut state, 1));
        }
    }
}
