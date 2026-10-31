//! Retail `GAME.FNT` Unicode bitmaps. No font data is bundled with the engine.
//! Layout and empirical checks are recorded in `docs/formats/font.md`.

use crate::bytes::Cursor;

const HEADER: usize = 28;
const MAP_ENTRIES: usize = 65536;
const MAP_END: usize = HEADER + MAP_ENTRIES * 2;
const MAX_DIMENSION: u32 = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontError {
    BadHeader,
    Truncated,
    BadMapping(usize),
    BadGlyph(usize),
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadHeader => write!(f, "invalid Unicode bitmap font header"),
            Self::Truncated => write!(f, "truncated Unicode bitmap font"),
            Self::BadMapping(code) => write!(f, "font mapping U+{code:04X} is out of bounds"),
            Self::BadGlyph(index) => write!(f, "font glyph {index} exceeds its bitmap stride"),
        }
    }
}

impl std::error::Error for FontError {}

#[derive(Debug, Clone)]
pub struct UnicodeFont {
    /// Metadata for the ideographic space. Language-specific spacing is not
    /// inferred from this value by the parser.
    pub ideographic_space_width: u32,
    pub stride: u32,
    pub lines: u32,
    pub line_height: u32,
    pub glyph_count: u32,
    record_size: usize,
    mapping: Vec<u16>,
    records: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct FontGlyph<'a> {
    pub width: u8,
    pub height: u32,
    stride: usize,
    bitmap: &'a [u8],
}

impl FontGlyph<'_> {
    /// Rows are byte-aligned; the most significant bit is the left pixel.
    pub fn pixel(&self, x: u32, y: u32) -> bool {
        if x >= u32::from(self.width) || y >= self.height {
            return false;
        }
        self.bitmap[y as usize * self.stride + x as usize / 8] & (0x80 >> (x % 8)) != 0
    }
}

impl UnicodeFont {
    pub fn parse(data: &[u8]) -> Result<Self, FontError> {
        let mut c = Cursor::new(data);
        if c.take(4) != Some(b"fonT".as_slice()) {
            return Err(FontError::BadHeader);
        }
        let ideographic_space_width = c.u32().ok_or(FontError::Truncated)?;
        let stride = c.u32().ok_or(FontError::Truncated)?;
        let lines = c.u32().ok_or(FontError::Truncated)?;
        let line_height = c.u32().ok_or(FontError::Truncated)?;
        let glyph_count = c.u32().ok_or(FontError::Truncated)?;
        let record_size = c.u32().ok_or(FontError::Truncated)? as usize;
        let width = stride.checked_mul(8).ok_or(FontError::BadHeader)?;
        if stride == 0
            || width > MAX_DIMENSION
            || lines == 0
            || lines > MAX_DIMENSION
            || line_height < lines
            || line_height > MAX_DIMENSION
            || glyph_count == 0
            || glyph_count > u32::from(u16::MAX)
        {
            return Err(FontError::BadHeader);
        }
        let expected_size = (stride as usize)
            .checked_mul(lines as usize)
            .and_then(|n| n.checked_add(1))
            .ok_or(FontError::BadHeader)?;
        if record_size != expected_size {
            return Err(FontError::BadHeader);
        }
        let end = (glyph_count as usize)
            .checked_mul(record_size)
            .and_then(|n| n.checked_add(MAP_END))
            .ok_or(FontError::BadHeader)?;
        // Validate the complete tables and glyph ranges before allocating.
        let map = data.get(HEADER..MAP_END).ok_or(FontError::Truncated)?;
        let records = data.get(MAP_END..end).ok_or(FontError::Truncated)?;
        for (code, bytes) in map.as_chunks::<2>().0.iter().enumerate() {
            if u32::from(u16::from_le_bytes(*bytes)) > glyph_count {
                return Err(FontError::BadMapping(code));
            }
        }
        for (i, record) in records.chunks_exact(record_size).enumerate() {
            if u32::from(record[0]) > width {
                return Err(FontError::BadGlyph(i));
            }
        }
        Ok(Self {
            ideographic_space_width,
            stride,
            lines,
            line_height,
            glyph_count,
            record_size,
            mapping: map
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes(*b))
                .collect(),
            records: records.to_vec(),
        })
    }

    /// Zero means unmapped; nonzero map entries use one-based glyph ids.
    /// Supplementary-plane characters have no entry in this BMP-only format.
    pub fn glyph(&self, ch: char) -> Option<FontGlyph<'_>> {
        let index = usize::from(*self.mapping.get(ch as usize)?).checked_sub(1)?;
        let start = index * self.record_size;
        let record = &self.records[start..start + self.record_size];
        Some(FontGlyph {
            width: record[0],
            height: self.lines,
            stride: self.stride as usize,
            bitmap: &record[1..],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut data = b"fonT".to_vec();
        for value in [8u32, 1, 2, 3, 2, 3] {
            data.extend(value.to_le_bytes());
        }
        data.resize(MAP_END, 0);
        for (ch, id) in [('A', 1u16), ('B', 1), (' ', 2)] {
            let at = HEADER + ch as usize * 2;
            data[at..at + 2].copy_from_slice(&id.to_le_bytes());
        }
        data.extend([3, 0b1010_0000, 0b0100_0000, 2, 0, 0]);
        data
    }

    #[test]
    fn one_based_aliases_spaces_and_msb_first_rows() {
        let font = UnicodeFont::parse(&sample()).unwrap();
        assert_eq!((font.lines, font.line_height, font.glyph_count), (2, 3, 2));
        for ch in ['A', 'B'] {
            let glyph = font.glyph(ch).unwrap();
            assert_eq!(glyph.width, 3);
            assert!(glyph.pixel(0, 0));
            assert!(!glyph.pixel(1, 0));
            assert!(glyph.pixel(2, 0));
            assert!(glyph.pixel(1, 1));
            assert!(!glyph.pixel(3, 0));
            assert!(!glyph.pixel(0, 2));
            assert!(!glyph.pixel(u32::MAX, u32::MAX));
        }
        let space = font.glyph(' ').unwrap();
        assert_eq!(space.width, 2);
        assert!(!space.pixel(0, 0));
        assert!(font.glyph('C').is_none());
        assert!(font.glyph('\u{1f600}').is_none());
    }

    #[test]
    fn hostile_headers_tables_and_glyphs_are_rejected_before_allocation() {
        let data = sample();
        for end in [0, 3, 27, HEADER, MAP_END - 1, data.len() - 1] {
            assert!(UnicodeFont::parse(&data[..end]).is_err());
        }
        for (at, value) in [
            (8, 0),
            (8, u32::MAX),
            (12, u32::MAX),
            (16, u32::MAX),
            (20, u32::MAX),
            (20, 0),
            (24, u32::MAX),
            (24, 0),
            (24, 2),
        ] {
            let mut bad = data.clone();
            bad[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(UnicodeFont::parse(&bad).is_err(), "field {at}={value}");
        }
        let mut bad = data.clone();
        bad[HEADER..HEADER + 2].copy_from_slice(&3u16.to_le_bytes());
        assert!(matches!(
            UnicodeFont::parse(&bad),
            Err(FontError::BadMapping(0))
        ));
        let mut bad = data;
        bad[MAP_END] = 9;
        assert!(matches!(
            UnicodeFont::parse(&bad),
            Err(FontError::BadGlyph(0))
        ));
    }
}
