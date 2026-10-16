//! PCX reader (ZSoft v5, RLE): loading screens, menu art, briefing images.
//!
//! Spec with evidence: `docs/formats/pcx.md`. Retail ships two flavours:
//! 8-bit paletted (palette trailer `0x0C` + 768 bytes of 8-bit RGB) and
//! 24-bit as three 8-bit planes (R, G, B) per scanline.

use crate::bytes::Cursor;

const HEADER_LEN: usize = 128;
const PALETTE_TRAILER: usize = 769;

#[derive(Debug, PartialEq, Eq)]
pub enum PcxError {
    BadHeader,
    Unsupported { bits: u8, planes: u8 },
    Truncated,
    MissingPalette,
}

impl std::fmt::Display for PcxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PcxError::BadHeader => write!(f, "not a PCX file"),
            PcxError::Unsupported { bits, planes } => {
                write!(f, "unsupported PCX layout: {bits} bpp x {planes} planes")
            }
            PcxError::Truncated => write!(f, "truncated PCX data"),
            PcxError::MissingPalette => write!(f, "8-bit PCX without palette trailer"),
        }
    }
}

impl std::error::Error for PcxError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcxPixels {
    /// Palette indices (`width * height`) with an 8-bit RGB palette.
    Indexed {
        indices: Vec<u8>,
        palette: Box<[[u8; 3]; 256]>,
    },
    /// Packed RGB (`width * height * 3`).
    Rgb(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcxImage {
    pub width: u32,
    pub height: u32,
    pub pixels: PcxPixels,
}

impl PcxImage {
    pub fn parse(data: &[u8]) -> Result<Self, PcxError> {
        let mut c = Cursor::new(data);
        let header = c.take(HEADER_LEN).ok_or(PcxError::BadHeader)?;
        let (manufacturer, encoding, bits) = (header[0], header[2], header[3]);
        if manufacturer != 0x0A || encoding != 1 {
            return Err(PcxError::BadHeader);
        }
        let mut h = Cursor::at(header, 4);
        let (x0, y0, x1, y1) = (h.u16(), h.u16(), h.u16(), h.u16());
        let (x0, y0, x1, y1) = (
            x0.unwrap_or(0),
            y0.unwrap_or(0),
            x1.unwrap_or(0),
            y1.unwrap_or(0),
        );
        if x1 < x0 || y1 < y0 {
            return Err(PcxError::BadHeader);
        }
        let planes = header[65];
        let bytes_per_line = u16::from_le_bytes([header[66], header[67]]) as usize;
        let (width, height) = (usize::from(x1 - x0) + 1, usize::from(y1 - y0) + 1);
        if bits != 8 || !(planes == 1 || planes == 3) {
            return Err(PcxError::Unsupported { bits, planes });
        }
        if bytes_per_line < width {
            return Err(PcxError::BadHeader);
        }

        // RLE runs may cross scanline/plane boundaries, so decode the whole
        // stream into one buffer of `bytes_per_line * planes * height`.
        let stride = bytes_per_line * planes as usize;
        let mut raw = vec![0u8; stride * height];
        let body_end = if planes == 1 {
            data.len().checked_sub(PALETTE_TRAILER).ok_or(PcxError::MissingPalette)?
        } else {
            data.len()
        };
        let body = data.get(HEADER_LEN..body_end).ok_or(PcxError::Truncated)?;
        unrle(body, &mut raw)?;

        let pixels = if planes == 1 {
            let trailer = &data[body_end..];
            if trailer[0] != 0x0C {
                return Err(PcxError::MissingPalette);
            }
            let mut palette = Box::new([[0u8; 3]; 256]);
            for (dst, src) in palette.iter_mut().zip(trailer[1..].chunks_exact(3)) {
                *dst = [src[0], src[1], src[2]];
            }
            let mut indices = Vec::with_capacity(width * height);
            for row in raw.chunks_exact(stride) {
                indices.extend_from_slice(&row[..width]);
            }
            PcxPixels::Indexed { indices, palette }
        } else {
            let mut rgb = Vec::with_capacity(width * height * 3);
            for row in raw.chunks_exact(stride) {
                let (r, rest) = row.split_at(bytes_per_line);
                let (g, b) = rest.split_at(bytes_per_line);
                for x in 0..width {
                    rgb.extend_from_slice(&[r[x], g[x], b[x]]);
                }
            }
            PcxPixels::Rgb(rgb)
        };
        Ok(PcxImage {
            width: width as u32,
            height: height as u32,
            pixels,
        })
    }

    /// Expand to RGBA8 (fully opaque).
    pub fn to_rgba(&self) -> Vec<u8> {
        match &self.pixels {
            PcxPixels::Indexed { indices, palette } => indices
                .iter()
                .flat_map(|&i| {
                    let [r, g, b] = palette[i as usize];
                    [r, g, b, 255]
                })
                .collect(),
            PcxPixels::Rgb(rgb) => rgb
                .chunks_exact(3)
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
        }
    }
}

/// PCX RLE: a byte with the top two bits set is a run of `b & 0x3F` copies of
/// the next byte; anything else is a literal.
fn unrle(src: &[u8], out: &mut [u8]) -> Result<(), PcxError> {
    let mut i = 0;
    let mut o = 0;
    while o < out.len() {
        let b = *src.get(i).ok_or(PcxError::Truncated)?;
        i += 1;
        if b >= 0xC0 {
            let n = usize::from(b & 0x3F);
            let v = *src.get(i).ok_or(PcxError::Truncated)?;
            i += 1;
            let end = (o + n).min(out.len());
            out[o..end].fill(v);
            o = end;
        } else {
            out[o] = b;
            o += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(w: u16, h: u16, planes: u8, bpl: u16) -> Vec<u8> {
        let mut d = vec![0u8; HEADER_LEN];
        d[..4].copy_from_slice(&[0x0A, 5, 1, 8]);
        d[8..10].copy_from_slice(&(w - 1).to_le_bytes());
        d[10..12].copy_from_slice(&(h - 1).to_le_bytes());
        d[65] = planes;
        d[66..68].copy_from_slice(&bpl.to_le_bytes());
        d
    }

    #[test]
    fn paletted_with_runs_and_padding() {
        // 3x2 image, bytes_per_line 4 (one pad byte per row).
        let mut d = header(3, 2, 1, 4);
        d.extend([0xC3, 7, 0]); // row 0: 7 7 7 | pad 0
        d.extend([1, 0xC2, 9, 0]); // row 1: 1 9 9 | pad 0
        d.push(0x0C);
        let mut pal = vec![0u8; 768];
        pal[7 * 3..7 * 3 + 3].copy_from_slice(&[10, 20, 30]);
        d.extend(pal);
        let img = PcxImage::parse(&d).unwrap();
        assert_eq!((img.width, img.height), (3, 2));
        let PcxPixels::Indexed { indices, palette } = &img.pixels else {
            panic!("expected indexed");
        };
        assert_eq!(indices, &vec![7, 7, 7, 1, 9, 9]);
        assert_eq!(palette[7], [10, 20, 30]);
        assert_eq!(&img.to_rgba()[..4], &[10, 20, 30, 255]);
    }

    #[test]
    fn rgb_planes() {
        let mut d = header(2, 1, 3, 2);
        d.extend([1, 2, 3, 4, 5, 6]); // R plane, G plane, B plane
        let img = PcxImage::parse(&d).unwrap();
        assert_eq!(img.pixels, PcxPixels::Rgb(vec![1, 3, 5, 2, 4, 6]));
    }

    #[test]
    fn rejects_truncated_and_foreign() {
        let mut d = header(4, 4, 3, 4);
        d.extend([0xC5, 1]);
        assert_eq!(PcxImage::parse(&d).unwrap_err(), PcxError::Truncated);
        assert_eq!(PcxImage::parse(&[0u8; 200]).unwrap_err(), PcxError::BadHeader);
    }
}
