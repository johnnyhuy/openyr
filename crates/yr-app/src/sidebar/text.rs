//! Shared measured text layout for the live and headless sidebar.

use yr_formats::{FontGlyph, UnicodeFont};

pub(crate) const MAX_WIDTH: i32 = 224;
pub(crate) const MAX_CHARS: usize = 224;
const MAX_LINES: usize = 3;

#[derive(Clone, Copy)]
pub(crate) struct Typeface<'a>(pub Option<&'a UnicodeFont>);

impl<'a> Typeface<'a> {
    pub fn glyph(self, ch: char) -> Option<FontGlyph<'a>> {
        let font = self.0?;
        font.glyph(ch).or_else(|| font.glyph('?'))
    }

    pub fn advance(self, ch: char) -> i32 {
        self.glyph(ch).map_or(8, |g| i32::from(g.width) + 1)
    }

    pub fn width(self, text: &str) -> i32 {
        let mut width = 0;
        let mut trailing = 0;
        for ch in text.chars().take(MAX_CHARS) {
            width += self.advance(ch);
            trailing = i32::from(self.glyph(ch).is_some());
        }
        width - trailing
    }

    pub fn line_height(self) -> i32 {
        self.0.map_or(12, |f| f.line_height as i32)
    }

    /// Keep word boundaries, split long words, and bound work for arbitrary
    /// localized names. Width comes from the same advances used to paint.
    pub fn name_lines(self, name: &str) -> Vec<String> {
        let mut rest = name.trim();
        let mut lines = Vec::new();
        while !rest.is_empty() && lines.len() < MAX_LINES {
            let mut width = 0;
            let end = rest.char_indices().take(MAX_CHARS + 1).find_map(|(i, ch)| {
                width += self.advance(ch);
                (width - i32::from(self.glyph(ch).is_some()) > MAX_WIDTH).then_some(i)
            });
            let end = end.or_else(|| rest.char_indices().nth(MAX_CHARS).map(|(i, _)| i));
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
                while !last.is_empty() && self.width(last) + self.width("..") + 1 > MAX_WIDTH {
                    last.pop();
                }
                last.push_str("..");
            }
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_names_and_long_words_stay_inside_tooltips() {
        let face = Typeface(None);
        assert_eq!(
            face.name_lines("Airforce Command Headquarters"),
            ["Airforce Command", "Headquarters"]
        );
        let lines = face.name_lines(&"界".repeat(10000));
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| face.width(line) <= MAX_WIDTH));
        assert!(lines.last().unwrap().ends_with(".."));
        assert_eq!(face.width(""), 0);
    }
}

#[cfg(test)]
mod retail_tests {
    use super::*;
    use crate::sidebar::Canvas;
    use std::path::PathBuf;

    #[test]
    fn retail_text_pixels_metrics_and_wide_tooltips_match_game_font() {
        let dir = PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            eprintln!("skip: retail install not present");
            return;
        }
        let fs = yr_assets::AssetFs::open(&dir).unwrap();
        let raw = fs.read("GAME.FNT").unwrap();
        let font = UnicodeFont::parse(raw).unwrap();
        let face = Typeface(Some(&font));
        assert!(face.width("11111") < face.width("WWWWW"));
        assert_eq!(face.width("10000"), 28);
        assert_eq!(face.line_height(), 17);
        let lines = face.name_lines(&"界".repeat(10000));
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| face.width(line) <= MAX_WIDTH));
        assert!(lines.last().unwrap().ends_with(".."));
        let lines = face.name_lines("Air Force Command Headquarters");
        assert_eq!(lines, ["Air Force Command Headquarters"]);
        let mut actual = vec![0u8; 800 * 140 * 4];
        let mut expected = actual.clone();
        let color = [255, 220, 96, 255];
        for (line, sample) in [
            "10000 $700 READY Power Plant",
            "Air Force Command Headquarters",
            "éèäöß 界日本",
            "Unknown: \u{1f600}",
        ]
        .into_iter()
        .enumerate()
        {
            let y0 = 2 + line as u32 * 25;
            Canvas {
                frame: &mut actual,
                width: 800,
                height: 140,
                face,
            }
            .text((2, y0 as i32), sample, color, MAX_CHARS);
            let mut dx = 2;
            for ch in sample.chars() {
                let map = |c: char| -> u16 {
                    let at = 28 + c as usize * 2;
                    if c as u32 > 65535 {
                        return 0;
                    }
                    u16::from_le_bytes(raw[at..at + 2].try_into().unwrap())
                };
                let id = match map(ch) {
                    0 => map('?'),
                    id => id,
                };
                let offset = 131100 + (usize::from(id) - 1) * 49;
                let width = raw[offset];
                for y in 0..16 {
                    for x in 0..u32::from(width) {
                        let byte = raw[offset + 1 + y as usize * 3 + x as usize / 8];
                        if byte & (128 >> (x % 8)) != 0 {
                            let at = (((y0 + y) * 800 + dx + x) * 4) as usize;
                            expected[at..at + 4].copy_from_slice(&color);
                        }
                    }
                }
                dx += u32::from(width) + 1;
            }
        }
        assert_eq!(
            actual, expected,
            "every painted glyph pixel matches raw retail records"
        );
        if let Some(out) = std::env::var_os("OPENYR_FONT_DUMP") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                PathBuf::from(out).join("retail-glyph-samples.png"),
                yr_render::png::encode_rgba(800, 140, &actual),
            )
            .unwrap();
        }
    }
}

#[cfg(test)]
mod hostile_layout_tests {
    use super::*;

    #[test]
    fn zero_width_glyphs_cannot_expand_tooltip_allocation() {
        let mut bytes = b"fonT".to_vec();
        for value in [8u32, 1, 1, 1, 1, 2] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.resize(28 + 65536 * 2, 0);
        let at = 28 + 'A' as usize * 2;
        bytes[at..at + 2].copy_from_slice(&1u16.to_le_bytes());
        bytes.extend([0, 0]);
        let font = UnicodeFont::parse(&bytes).unwrap();
        let face = Typeface(Some(&font));
        let lines = face.name_lines(&"A".repeat(100000));
        assert_eq!(lines.len(), 3);
        assert!(lines
            .iter()
            .all(|line| line.chars().count() <= MAX_CHARS && face.width(line) <= MAX_WIDTH));
        assert!(lines.last().unwrap().ends_with(".."));
    }
}
