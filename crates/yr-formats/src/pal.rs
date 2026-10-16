//! Westwood palette reader (`.pal`): 256 entries of 6-bit RGB.
//!
//! Retail check: `expandmd01.mix` carries 768-byte palettes whose components
//! are all <= 0x3F (6-bit), e.g. entry 0 = `3f 00 3f`.

/// 256-entry palette, 6-bit RGB as stored on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// `rgb[i] = (r, g, b)`, each 0..=63.
    pub entries: [(u8, u8, u8); 256],
}

impl Palette {
    /// Parse 768 bytes of 6-bit RGB triples.
    pub fn parse(data: &[u8]) -> Option<Palette> {
        if data.len() < 768 {
            return None;
        }
        let mut entries = [(0u8, 0u8, 0u8); 256];
        for (i, chunk) in data[..768].chunks_exact(3).enumerate() {
            entries[i] = (chunk[0], chunk[1], chunk[2]);
        }
        Some(Palette { entries })
    }

    /// Expand entry `i` to 8-bit RGBA (alpha 255). 6-bit values scale by *4
    /// plus the top-2-bit fill used by the original renderer (`v << 2 | v >> 4`).
    pub fn rgba(&self, i: u8) -> [u8; 4] {
        let (r, g, b) = self.entries[i as usize];
        let s = |v: u8| (v << 2) | (v >> 4);
        [s(r), s(g), s(b), 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_scales() {
        let mut raw = [0u8; 768];
        raw[0] = 0x3F;
        raw[1] = 0x00;
        raw[2] = 0x3F;
        let pal = Palette::parse(&raw).unwrap();
        assert_eq!(pal.entries[0], (0x3F, 0x00, 0x3F));
        assert_eq!(pal.entries[1], (0, 0, 0));
        assert_eq!(pal.rgba(0), [0xFF, 0x00, 0xFF, 255]);
    }

    #[test]
    fn rejects_short() {
        assert!(Palette::parse(&[0; 767]).is_none());
    }
}
