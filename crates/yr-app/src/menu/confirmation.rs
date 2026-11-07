//! Target filenames for native saved-match confirmations.

use super::{message, Art, Intent, Layout, Page, State};
use crate::sidebar::{text::Typeface, Canvas};
use yr_formats::UnicodeFont;

fn name(intent: &Intent) -> String {
    let (Intent::Save(path) | Intent::Load(path) | Intent::Delete(path)) = intent;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    format!("\"{}\"", name.escape_debug())
}

pub(crate) fn region(layout: Layout) -> message::Region {
    if layout.front {
        message::Region {
            left: 32,
            top: 112,
            width: 568,
            height: 432,
        }
    } else {
        message::Region::pause(layout)
    }
}

// Filenames are opaque: retain spaces and split only at character boundaries.
fn lines<'a>(face: Typeface<'a>, text: &'a str, available: i32) -> impl Iterator<Item = &'a str> {
    let mut rest = Some(text);
    std::iter::from_fn(move || {
        let text = rest.take()?;
        let mut width = 0;
        for (at, ch) in text.char_indices() {
            width += face.advance(ch);
            if width - i32::from(face.glyph(ch).is_some()) > available {
                let end = if at == 0 { ch.len_utf8() } else { at };
                if end < text.len() {
                    rest = Some(&text[end..]);
                }
                return Some(&text[..end]);
            }
        }
        Some(text)
    })
}

fn viewport(
    face: Typeface<'_>,
    text: &str,
    layout: Layout,
    requested: usize,
) -> (usize, usize, usize) {
    let region = region(layout);
    let count = lines(face, text, region.width).count();
    let rows = (region.height / message::line_step(face)).max(1) as usize;
    (requested.min(count.saturating_sub(rows)), rows, count)
}

pub(crate) fn draw(c: &mut Canvas<'_>, layout: Layout, intent: &Intent, scroll: usize) {
    let text = name(intent);
    let region = region(layout);
    let (first, rows, _) = viewport(c.face, &text, layout, scroll);
    for (row, line) in lines(c.face, &text, region.width)
        .skip(first)
        .take(rows)
        .enumerate()
    {
        c.text(
            (
                region.left,
                region.top + row as i32 * message::line_step(c.face),
            ),
            line,
            [220, 232, 240, 255],
            usize::MAX,
        );
    }
}

pub(crate) fn status(face: Typeface<'_>, layout: Layout, intent: &Intent, scroll: usize) -> String {
    let (first, rows, count) = viewport(face, &name(intent), layout, scroll);
    if count > rows {
        format!(
            "Lines {}-{} of {count}. Page Up/Down or wheel scroll; Tab selects controls.",
            first + 1,
            (first + rows).min(count)
        )
    } else {
        "Escape returns. Tab and arrows select controls.".into()
    }
}

/// Confirmation scrolling uses the same font and authored region as drawing.
pub(crate) fn scroll(
    font: Option<&UnicodeFont>,
    state: &mut State,
    size: (u32, u32),
    delta: i32,
) -> bool {
    let Page::Confirm(intent) = &state.page else {
        return false;
    };
    let layout = Layout::for_state(state, size.0, size.1);
    let (first, rows, count) =
        viewport(Typeface(font), &name(intent), layout, state.message_scroll);
    state.message_scroll = first
        .saturating_add_signed(delta as isize)
        .min(count.saturating_sub(rows));
    true
}

/// Pause confirmation scrolling retains the pause skin's font ownership.
pub fn scroll_confirmation(art: &Art, state: &mut State, size: (u32, u32), delta: i32) -> bool {
    if state.front {
        return false;
    }
    scroll(art.font.as_ref(), state, size, delta)
}

#[cfg(test)]
mod tests;
