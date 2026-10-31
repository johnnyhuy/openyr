//! Shared window/headless production panel and its matching hit boxes.

mod art;
pub mod radar;
mod text;
pub use art::SidebarArt;
use text::Typeface;

pub use crate::session::BuildCategory as Category;
use crate::session::{Session, TICKS_PER_SEC};
use font8x8::{UnicodeFonts, BASIC_FONTS, GREEK_FONTS, HIRAGANA_FONTS, LATIN_FONTS};

pub const WIDTH: u32 = 168;
const PAD: i32 = 23;
const TAB_TOP: i32 = 197;
const TAB_H: i32 = 27;
const TOP: i32 = 228;
const SLOT_W: i32 = 60;
const SLOT_H: i32 = 48;
const GAP: i32 = 2;
const NAME_COLS: usize = text::MAX_CHARS;
const PANEL: [u8; 4] = [24, 26, 32, 255];
const TEXT: [u8; 4] = [232, 236, 240, 255];
const GOLD: [u8; 4] = [255, 220, 96, 255];

impl Category {
    pub const ALL: [Self; 4] = [
        Self::Structures,
        Self::Defences,
        Self::Infantry,
        Self::Vehicles,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Structures => "Structures",
            Self::Defences => "Defences",
            Self::Infantry => "Infantry",
            Self::Vehicles => "Vehicles",
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Structures => "structures",
            Self::Defences => "defences",
            Self::Infantry => "infantry",
            Self::Vehicles => "vehicles",
        }
    }
}

/// Original session slots, in display order. Never reindex production ids.
pub fn slots(session: &Session, category: Category) -> impl Iterator<Item = usize> + '_ {
    session
        .buildables
        .iter()
        .enumerate()
        .filter_map(move |(slot, b)| (b.category == category).then_some(slot))
}

/// Native-pixel geometry, also used by window input and proof tests.
pub fn slot_origin(width: u32, index: usize) -> (i32, i32) {
    (
        width.saturating_sub(WIDTH) as i32 + PAD + (index % 2) as i32 * (SLOT_W + GAP),
        TOP + (index / 2) as i32 * (SLOT_H + GAP),
    )
}

pub fn tab_origin(width: u32, category: Category) -> (i32, i32) {
    let index = Category::ALL.iter().position(|&c| c == category).unwrap();
    (
        width.saturating_sub(WIDTH) as i32 + 26 + index as i32 * 28,
        TAB_TOP,
    )
}

fn contains((x, y): (i32, i32), (w, h): (i32, i32), pos: (f64, f64)) -> bool {
    pos.0 >= f64::from(x)
        && pos.0 < f64::from(x + w)
        && pos.1 >= f64::from(y)
        && pos.1 < f64::from(y + h)
}

pub fn tab_at(width: u32, pos: (f64, f64)) -> Option<Category> {
    Category::ALL
        .into_iter()
        .find(|&category| contains(tab_origin(width, category), (28, TAB_H), pos))
}

pub fn slot_at(
    width: u32,
    height: u32,
    session: &Session,
    category: Category,
    pos: (f64, f64),
) -> Option<usize> {
    slots(session, category).enumerate().find_map(|(i, slot)| {
        let origin = slot_origin(width, i);
        (origin.1 + SLOT_H <= height as i32 - 26 && contains(origin, (SLOT_W, SLOT_H), pos))
            .then_some(slot)
    })
}

pub fn ready_category(session: &Session) -> Option<Category> {
    session
        .buildables
        .get(session.ready_structure()?)
        .map(|b| b.category)
}

/// Full-power duration, rounded up to the next tenth of a second.
pub fn duration(ticks: u32) -> String {
    let tenths = (u64::from(ticks) * 10).div_ceil(TICKS_PER_SEC);
    if tenths.is_multiple_of(10) {
        format!("{}s", tenths / 10)
    } else {
        format!("{}.{:01}s", tenths / 10, tenths % 10)
    }
}

/// Queue progress selects authored angular coverage, never a redraw clock.
fn clock_phase(done: i64, total: i64, frames: usize) -> usize {
    if total <= 0 || frames == 0 {
        return 0;
    }
    if done >= total || frames < 2 {
        return 0;
    }
    // Frame 0 is clear. Frames 1..54 progressively uncover the cameo.
    1 + (i128::from(done.max(0)) * (frames - 1) as i128 / i128::from(total)) as usize
}

pub fn draw(
    frame: &mut [u8],
    width: u32,
    height: u32,
    session: &Session,
    category: Category,
    placing: Option<usize>,
) {
    let mut canvas = Canvas {
        frame,
        width,
        height,
        face: Typeface(session.sidebar_art.font.as_ref()),
    };
    let x0 = width.saturating_sub(WIDTH) as i32;
    let art = &session.sidebar_art;
    canvas.rect((x0, 0), (WIDTH as i32, height as i32), PANEL);
    for (image, y) in [(&art.credits, 0), (&art.top, 16), (&art.header, 158)] {
        if let Some(image) = image {
            canvas.image((x0, y), image);
        }
    }
    session.radar.draw(&mut canvas, session);
    let credits = session.sim.credits().to_string();
    let cx = x0 + (WIDTH as i32 - canvas.face.width(&credits)) / 2;
    let credit_y = if canvas.face.0.is_some() { 0 } else { 4 };
    canvas.text((cx, credit_y), &credits, TEXT, 18);

    // SIDE2 includes the native two-pixel gutters and the left power channel.
    for y in (TOP - 1..height as i32).step_by((SLOT_H + GAP) as usize) {
        if let Some(image) = &art.row {
            canvas.image((x0, y), image);
        }
    }
    if let Some(image) = &art.bottom {
        canvas.image((x0, height as i32 - 26), image);
    }
    let ready = session.ready_structure();
    let ready_tab = ready_category(session);
    for (i, tab) in Category::ALL.into_iter().enumerate() {
        let (x, y) = tab_origin(width, tab);
        // Retail sheet order: up, selected, disabled, pressed-up, pressed-selected.
        let selected = tab == category;
        if let Some(image) = art.tabs[i].get(usize::from(selected)) {
            canvas.image((x, y), image);
        } else {
            canvas.rect((x, y), (28, TAB_H), if selected { GOLD } else { PANEL });
            canvas.text((x + 10, y + 10), &i.to_string(), TEXT, 1);
        }
        if ready_tab == Some(tab) {
            canvas.rect((x + 3, y + TAB_H - 3), (22, 2), [96, 255, 96, 255]);
        }
    }
    for (i, slot) in slots(session, category).enumerate() {
        let buildable = &session.buildables[slot];
        let (sx, sy) = slot_origin(width, i);
        if sy + SLOT_H > height as i32 - 26 {
            break;
        }
        if let Some((w, h, pixels)) = &buildable.cameo {
            for y in 0..(*h).min(SLOT_H as u32) {
                for x in 0..(*w).min(SLOT_W as u32) {
                    let at = ((y * w + x) * 4) as usize;
                    if pixels[at + 3] != 0 {
                        canvas.pixel(
                            sx + x as i32,
                            sy + y as i32,
                            [pixels[at], pixels[at + 1], pixels[at + 2], 255],
                        );
                    }
                }
            }
        }
        let (count, head) = session.slot_status(slot);
        // Paid work remains bright even when the remaining balance is below
        // the full price. The authored clock alone covers unfinished work.
        if !session.slot_available(slot) && head.is_none() {
            canvas.shade((sx, sy), (SLOT_W, SLOT_H));
        }
        if let Some((done, total)) = head.filter(|(_, total)| *total > 0) {
            let phase = clock_phase(done, total, art.clock.len());
            if let Some(mask) = art.clock.get(phase) {
                for (p, &coverage) in mask.iter().enumerate() {
                    if coverage != 0 {
                        canvas.shade((sx + p as i32 % SLOT_W, sy + p as i32 / SLOT_W), (1, 1));
                    }
                }
            }
        }
        if count > 0 && buildable.footprint().is_none() {
            let y = if canvas.face.0.is_some() {
                sy - 2
            } else {
                sy + 2
            };
            canvas.text((sx + 2, y), &count.to_string(), TEXT, 6);
        }
        if ready == Some(slot) {
            canvas.rect((sx + 6, sy + 17), (48, 13), [0, 0, 0, 255]);
            let x = sx + (SLOT_W - canvas.face.width("READY")) / 2;
            let y = if canvas.face.0.is_some() {
                sy + 16
            } else {
                sy + 20
            };
            canvas.text((x, y), "READY", GOLD, 5);
        }
        if placing == Some(slot) {
            canvas.rect((sx, sy), (SLOT_W, 1), TEXT);
            canvas.rect((sx, sy + SLOT_H - 1), (SLOT_W, 1), TEXT);
            canvas.rect((sx, sy), (1, SLOT_H), TEXT);
            canvas.rect((sx + SLOT_W - 1, sy), (1, SLOT_H), TEXT);
        }
    }
}

/// Localized details remain available without widening the retail cameo grid.
pub fn draw_tooltip(
    frame: &mut [u8],
    width: u32,
    height: u32,
    session: &Session,
    category: Category,
    cursor: (f64, f64),
) {
    let Some(slot) = slot_at(width, height, session, category, cursor) else {
        return;
    };
    let buildable = &session.buildables[slot];
    let face = Typeface(session.sidebar_art.font.as_ref());
    let mut lines = face.name_lines(&buildable.name);
    lines.push(format!("${} {}", buildable.cost, duration(buildable.ticks)));
    let w = lines.iter().map(|line| face.width(line)).max().unwrap_or(0) + 12;
    let h = lines.len() as i32 * face.line_height() + 8;
    let x = (width.saturating_sub(WIDTH) as i32 - w - 4).max(0);
    let y = (cursor.1 as i32).clamp(0, (height as i32 - h).max(0));
    let mut canvas = Canvas {
        frame,
        width,
        height,
        face: Typeface(session.sidebar_art.font.as_ref()),
    };
    canvas.rect((x, y), (w, h), GOLD);
    canvas.rect((x + 1, y + 1), (w - 2, h - 2), [0, 0, 0, 255]);
    for (i, line) in lines.iter().enumerate() {
        let top = if face.0.is_some() { 1 } else { 5 };
        canvas.text(
            (x + 6, y + top + i as i32 * face.line_height()),
            line,
            GOLD,
            NAME_COLS,
        );
    }
}

struct Canvas<'a> {
    frame: &'a mut [u8],
    width: u32,
    height: u32,
    face: Typeface<'a>,
}

impl Canvas<'_> {
    fn pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            let at = ((y as u32 * self.width + x as u32) * 4) as usize;
            self.frame[at..at + 4].copy_from_slice(&color);
        }
    }

    fn image(&mut self, (x, y): (i32, i32), image: &art::Image) {
        for (p, color) in image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .take((image.width * image.height) as usize)
            .enumerate()
        {
            if color[3] != 0 {
                self.pixel(
                    x + (p as u32 % image.width) as i32,
                    y + (p as u32 / image.width) as i32,
                    *color,
                );
            }
        }
    }

    fn rect(&mut self, (x, y): (i32, i32), (w, h): (i32, i32), color: [u8; 4]) {
        for dy in 0..h {
            for dx in 0..w {
                self.pixel(x + dx, y + dy, color);
            }
        }
    }

    fn shade(&mut self, (x, y): (i32, i32), (w, h): (i32, i32)) {
        for py in y.max(0)..(y + h).min(self.height as i32) {
            for px in x.max(0)..(x + w).min(self.width as i32) {
                let at = ((py as u32 * self.width + px as u32) * 4) as usize;
                for c in &mut self.frame[at..at + 3] {
                    *c /= 3;
                }
            }
        }
    }

    fn text(&mut self, (x, y): (i32, i32), text: &str, color: [u8; 4], columns: usize) {
        let mut dx = x;
        for ch in text.chars().take(columns) {
            if let Some(glyph) = self.face.glyph(ch) {
                for row in 0..glyph.height {
                    for col in 0..u32::from(glyph.width) {
                        if glyph.pixel(col, row) {
                            self.pixel(dx + col as i32, y + row as i32, color);
                        }
                    }
                }
            } else {
                let glyph = BASIC_FONTS
                    .get(ch)
                    .or_else(|| LATIN_FONTS.get(ch))
                    .or_else(|| GREEK_FONTS.get(ch))
                    .or_else(|| HIRAGANA_FONTS.get(ch))
                    .unwrap_or_else(|| BASIC_FONTS.get('?').unwrap());
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            self.pixel(dx + col, y + row as i32, color);
                        }
                    }
                }
            }
            dx += self.face.advance(ch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_tab_and_cameo_geometry_excludes_gutters() {
        for width in [WIDTH, 1280] {
            let x = f64::from(width - WIDTH);
            for (i, category) in Category::ALL.into_iter().enumerate() {
                for pos in [
                    (26.0 + i as f64 * 28.0, 197.0),
                    (53.0 + i as f64 * 28.0, 223.0),
                ] {
                    assert_eq!(tab_at(width, (x + pos.0, pos.1)), Some(category));
                }
            }
            for pos in [(25.0, 197.0), (138.0, 197.0), (26.0, 196.0), (26.0, 224.0)] {
                assert_eq!(tab_at(width, (x + pos.0, pos.1)), None);
            }
            assert_eq!(slot_origin(width, 0), (x as i32 + 23, 228));
            assert_eq!(slot_origin(width, 1), (x as i32 + 85, 228));
            assert_eq!(slot_origin(width, 2), (x as i32 + 23, 278));
        }
    }

    #[test]
    fn clock_is_bounded_by_actual_progress_including_extreme_values() {
        assert_eq!(clock_phase(0, 100, 55), 1);
        assert_eq!(clock_phase(25, 100, 55), 14);
        assert_eq!(clock_phase(50, 100, 55), 28);
        assert_eq!(clock_phase(100, 100, 55), 0);
        assert_eq!(clock_phase(i64::MIN, i64::MAX, 55), 1);
        assert_eq!(clock_phase(i64::MAX, i64::MAX, 55), 0);
        assert_eq!(clock_phase(10, 0, 55), 0);
        assert_eq!(clock_phase(0, 100, 0), 0);
    }

    #[test]
    fn full_power_duration_rounding_is_bounded() {
        assert_eq!(duration(441), "29.4s");
        assert_eq!(duration(1), "0.1s");
        assert_eq!(duration(u32::MAX), "286331153s");
    }
}
