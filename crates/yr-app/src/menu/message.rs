//! Measured recovery text shared by front and pause menus.

use super::{Art, Layout, Page, State};
use crate::sidebar::text::Typeface;

pub(crate) fn line_step(face: Typeface<'_>) -> i32 {
    let ink_height = face.0.map_or(8, |f| f.lines as i32);
    face.line_height().max(ink_height).max(20) + 4
}

/// Borrow complete lines, splitting long tokens only at character boundaries.
pub(crate) fn lines<'a>(
    face: Typeface<'a>,
    text: &'a str,
    available: i32,
) -> impl Iterator<Item = &'a str> {
    text.split('\n').flat_map(move |paragraph| {
        let mut rest = Some(paragraph.trim());
        std::iter::from_fn(move || {
            let text = rest.take()?;
            let mut width = 0;
            let mut word_break = None;
            for (at, ch) in text.char_indices() {
                width += face.advance(ch);
                let painted_width = width - i32::from(face.glyph(ch).is_some());
                if painted_width > available {
                    // A single oversized glyph must still consume input.
                    let end = if at == 0 {
                        ch.len_utf8()
                    } else if ch.is_whitespace() {
                        at
                    } else {
                        word_break.unwrap_or(at)
                    };
                    rest = Some(text[end..].trim_start());
                    return Some(text[..end].trim_end());
                }
                if ch.is_whitespace() && at > 0 {
                    word_break = Some(at);
                }
            }
            Some(text)
        })
    })
}

pub(crate) struct Region {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Region {
    pub fn pause(layout: Layout) -> Self {
        Self {
            left: 40,
            top: 72,
            width: layout.width as i32 - 168 - 80,
            height: layout.height as i32 - 48 - 72,
        }
    }

    pub fn viewport(
        &self,
        face: Typeface<'_>,
        text: &str,
        requested: usize,
    ) -> (usize, usize, usize) {
        let count = lines(face, text, self.width).count();
        let rows = (self.height / line_step(face)).max(1) as usize;
        (requested.min(count.saturating_sub(rows)), rows, count)
    }
}

/// Pause recovery messages use identical geometry for scrolling and painting.
pub fn scroll_message(art: &Art, state: &mut State, size: (u32, u32), delta: i32) -> bool {
    if state.front {
        return false;
    }
    let Page::Message { text, .. } = &state.page else {
        return false;
    };
    let region = Region::pause(Layout::new(size.0, size.1));
    let (first, rows, count) =
        region.viewport(Typeface(art.font.as_ref()), text, state.message_scroll);
    state.message_scroll = first
        .saturating_add_signed(delta as isize)
        .min(count.saturating_sub(rows));
    true
}

#[cfg(test)]
mod tests;
