//! Shared window/headless production panel and its matching hit boxes.

pub use crate::session::BuildCategory as Category;
use crate::session::{Session, TICKS_PER_SEC};
use font8x8::{UnicodeFonts, BASIC_FONTS, GREEK_FONTS, HIRAGANA_FONTS, LATIN_FONTS};

pub const WIDTH: u32 = 352;
const PAD: i32 = 8;
const TAB_TOP: i32 = 40;
const TAB_H: i32 = 24;
const TOP: i32 = 104;
const SLOT_W: i32 = 164;
const SLOT_H: i32 = 48;
const GAP: i32 = 8;
const CAMEO_W: i32 = 60;
const NAME_COLS: usize = 12;
const NAME_LINES: usize = 3;
const PANEL: [u8; 4] = [24, 26, 32, 255];
const TEXT: [u8; 4] = [232, 236, 240, 255];
const DIM: [u8; 4] = [120, 124, 136, 255];
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

fn origin(width: u32, index: usize, top: i32, row_height: i32) -> (i32, i32) {
    (
        width.saturating_sub(WIDTH) as i32 + PAD + (index % 2) as i32 * (SLOT_W + GAP),
        top + (index / 2) as i32 * (row_height + GAP),
    )
}

fn contains((x, y): (i32, i32), height: i32, pos: (f64, f64)) -> bool {
    pos.0 >= f64::from(x)
        && pos.0 < f64::from(x + SLOT_W)
        && pos.1 >= f64::from(y)
        && pos.1 < f64::from(y + height)
}

pub fn tab_at(width: u32, pos: (f64, f64)) -> Option<Category> {
    Category::ALL
        .into_iter()
        .enumerate()
        .find_map(|(i, category)| {
            contains(origin(width, i, TAB_TOP, TAB_H), TAB_H, pos).then_some(category)
        })
}

/// The whole card is clickable, including its name, cost and time.
pub fn slot_at(
    width: u32,
    session: &Session,
    category: Category,
    pos: (f64, f64),
) -> Option<usize> {
    slots(session, category)
        .enumerate()
        .find_map(|(i, slot)| contains(origin(width, i, TOP, SLOT_H), SLOT_H, pos).then_some(slot))
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

/// Keep word boundaries when possible, split long words, and bound both
/// allocation and drawing for arbitrary localized names. Two dots mark a cut.
fn name_lines(name: &str) -> Vec<String> {
    let mut rest = name.trim();
    let mut lines = Vec::new();
    while !rest.is_empty() && lines.len() < NAME_LINES {
        let end = rest.char_indices().nth(NAME_COLS).map(|(i, _)| i);
        let Some(end) = end else {
            lines.push(rest.to_string());
            return lines;
        };
        let split = rest[..end]
            .rfind(char::is_whitespace)
            .filter(|&i| i > 0)
            .unwrap_or(end);
        lines.push(rest[..split].to_string());
        rest = rest[split..].trim_start();
    }
    if !rest.is_empty() {
        if let Some(last) = lines.last_mut() {
            *last = last.chars().take(NAME_COLS - 2).collect();
            last.push_str("..");
        }
    }
    lines
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
    };
    let x0 = width.saturating_sub(WIDTH) as i32;
    canvas.rect((x0, 0), (WIDTH as i32, height as i32), PANEL);
    canvas.text(
        (x0 + PAD, 7),
        &format!("${}", session.sim.credits()),
        GOLD,
        40,
    );
    canvas.text((x0 + PAD, 23), "Build times at full power", DIM, 40);

    let ready = session.ready_structure();
    let ready_tab = ready_category(session);
    for (i, tab) in Category::ALL.into_iter().enumerate() {
        let (x, y) = origin(width, i, TAB_TOP, TAB_H);
        canvas.rect(
            (x, y),
            (SLOT_W, TAB_H),
            if tab == category {
                [66, 72, 90, 255]
            } else {
                [32, 35, 42, 255]
            },
        );
        let count: usize = slots(session, tab).map(|s| session.slot_status(s).0).sum();
        let label = if ready_tab == Some(tab) {
            format!("{} READY", tab.label())
        } else if count > 0 {
            format!("{} x{count}", tab.label())
        } else {
            tab.label().into()
        };
        let color = if ready_tab == Some(tab) {
            [96, 255, 96, 255]
        } else if tab == category {
            GOLD
        } else {
            TEXT
        };
        canvas.text((x + 6, y + 8), &label, color, 19);
        if tab == category {
            canvas.rect((x, y + TAB_H - 2), (SLOT_W, 2), GOLD);
        }
    }
    for (i, slot) in slots(session, category).enumerate() {
        let buildable = &session.buildables[slot];
        let (sx, sy) = origin(width, i, TOP, SLOT_H);
        canvas.rect((sx, sy), (SLOT_W, SLOT_H), [32, 35, 42, 255]);
        canvas.rect((sx, sy), (CAMEO_W, SLOT_H), [70, 70, 80, 255]);
        if let Some((w, h, pixels)) = &buildable.cameo {
            for y in 0..(*h).min(SLOT_H as u32) {
                for x in 0..(*w).min(CAMEO_W as u32) {
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
        let available = session.slot_available(slot);
        let (count, head) = session.slot_status(slot);
        // Affordability is a visual hint; existing gradual-payment orders
        // remain queueable. Keep the progress wipe independent of that hint.
        if !available {
            canvas.shade((sx, sy), (CAMEO_W, SLOT_H));
        }
        if let Some((done, total)) = head.filter(|(_, total)| *total > 0) {
            let rows = (i64::from(SLOT_H) * (total - done) / total).clamp(0, i64::from(SLOT_H));
            canvas.shade((sx, sy), (CAMEO_W, rows as i32));
        }
        let name_color = if available { TEXT } else { DIM };
        for (line, name) in name_lines(&buildable.name).iter().enumerate() {
            canvas.text(
                (sx + 64, sy + 2 + line as i32 * 10),
                name,
                name_color,
                NAME_COLS,
            );
        }
        canvas.text(
            (sx + 64, sy + 39),
            &format!("${} {}", buildable.cost, duration(buildable.ticks)),
            if available { GOLD } else { DIM },
            NAME_COLS,
        );
        if count > 0 {
            let count = count.to_string();
            canvas.rect(
                (sx + 1, sy + 1),
                ((count.len() * 8 + 4) as i32, 12),
                [0, 0, 0, 255],
            );
            canvas.text((sx + 3, sy + 3), &count, TEXT, 6);
        }
        let outline = if placing == Some(slot) {
            Some([255, 255, 255, 255])
        } else if ready == Some(slot) {
            Some([96, 255, 96, 255])
        } else {
            None
        };
        if let Some(color) = outline {
            for inset in 0..2 {
                let w = SLOT_W - 2 * inset;
                let h = SLOT_H - 2 * inset;
                let (x, y) = (sx + inset, sy + inset);
                canvas.rect((x, y), (w, 1), color);
                canvas.rect((x, y + h - 1), (w, 1), color);
                canvas.rect((x, y), (1, h), color);
                canvas.rect((x + w - 1, y), (1, h), color);
            }
        }
    }
}

struct Canvas<'a> {
    frame: &'a mut [u8],
    width: u32,
    height: u32,
}

impl Canvas<'_> {
    fn pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            let at = ((y as u32 * self.width + x as u32) * 4) as usize;
            self.frame[at..at + 4].copy_from_slice(&color);
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
        for (i, ch) in text.chars().take(columns).enumerate() {
            let glyph = BASIC_FONTS
                .get(ch)
                .or_else(|| LATIN_FONTS.get(ch))
                .or_else(|| GREEK_FONTS.get(ch))
                .or_else(|| HIRAGANA_FONTS.get(ch))
                .unwrap_or_else(|| BASIC_FONTS.get('?').unwrap());
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        self.pixel(x + i as i32 * 8 + col, y + row as i32, color);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_hit_boxes_match_both_rows_and_exclude_gaps() {
        for width in [WIDTH, 1280] {
            let x = f64::from(width - WIDTH);
            for (pos, category) in [
                ((8.0, 40.0), Category::Structures),
                ((343.0, 63.0), Category::Defences),
                ((8.0, 72.0), Category::Infantry),
                ((343.0, 95.0), Category::Vehicles),
            ] {
                assert_eq!(tab_at(width, (x + pos.0, pos.1)), Some(category));
            }
            for pos in [
                (7.0, 40.0),
                (172.0, 40.0),
                (8.0, 64.0),
                (8.0, 96.0),
                (8.0, 104.0),
            ] {
                assert_eq!(tab_at(width, (x + pos.0, pos.1)), None);
            }
        }
    }

    #[test]
    fn long_localized_names_stay_inside_cards() {
        assert_eq!(
            name_lines("Airforce Command Headquarters"),
            ["Airforce", "Command", "Headquarters"]
        );
        assert_eq!(name_lines("Infanterie élite"), ["Infanterie", "élite"]);
        let long = name_lines(&"界".repeat(100));
        assert_eq!(long.len(), NAME_LINES);
        assert!(long.iter().all(|s| s.chars().count() <= NAME_COLS));
        assert!(long.last().unwrap().ends_with(".."));
        assert_eq!(duration(441), "29.4s");
        assert_eq!(duration(1), "0.1s");
        assert_eq!(duration(3150), "210s");
        assert_eq!(duration(u32::MAX), "286331153s");
    }
}
