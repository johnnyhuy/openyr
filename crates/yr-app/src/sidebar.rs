//! Shared window/headless production panel and its matching hit boxes.

use crate::session::{Session, TICKS_PER_SEC};
use font8x8::{UnicodeFonts, BASIC_FONTS, GREEK_FONTS, HIRAGANA_FONTS, LATIN_FONTS};

pub const WIDTH: u32 = 352;
const PAD: i32 = 8;
const TOP: i32 = 40;
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

fn origin(width: u32, slot: usize) -> (i32, i32) {
    (
        width.saturating_sub(WIDTH) as i32 + PAD + (slot % 2) as i32 * (SLOT_W + GAP),
        TOP + (slot / 2) as i32 * (SLOT_H + GAP),
    )
}

/// The whole card is clickable, including its name, cost and time.
pub fn slot_at(width: u32, slots: usize, pos: (f64, f64)) -> Option<usize> {
    (0..slots).find(|&i| {
        let (x, y) = origin(width, i);
        pos.0 >= f64::from(x)
            && pos.0 < f64::from(x + SLOT_W)
            && pos.1 >= f64::from(y)
            && pos.1 < f64::from(y + SLOT_H)
    })
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

pub fn draw(frame: &mut [u8], width: u32, height: u32, session: &Session, placing: Option<usize>) {
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
    for (slot, buildable) in session.buildables.iter().enumerate() {
        let (sx, sy) = origin(width, slot);
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
