//! SHP (TS/RA2 variant) sprite reader, cameos, buildings, infantry, anims.
//!
//! Spec with evidence: `docs/formats/shp.md`. Frames are palette-indexed;
//! index 0 is transparent.

use crate::bytes::Cursor;

const FILE_HEADER_LEN: usize = 8;
const FRAME_HEADER_LEN: usize = 24;
/// Zero-run RLE can expand 2 bytes into 255 pixels; no frame may claim more
/// pixels than its remaining data could produce.
const MAX_EXPANSION: usize = 128;
/// Canvas cap for `decode_frame_canvas` (retail maximum is 632x568).
const MAX_CANVAS_PIXELS: usize = 4096 * 4096;

#[derive(Debug, PartialEq, Eq)]
pub enum ShpError {
    BadHeader,
    Truncated,
    BadFrame(usize),
    UnknownCompression { frame: usize, kind: u8 },
}

impl std::fmt::Display for ShpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShpError::BadHeader => write!(f, "not a TS/RA2 SHP file"),
            ShpError::Truncated => write!(f, "truncated SHP file"),
            ShpError::BadFrame(i) => write!(f, "SHP frame {i} is out of bounds"),
            ShpError::UnknownCompression { frame, kind } => {
                write!(f, "SHP frame {frame} has unknown compression {kind}")
            }
        }
    }
}

impl std::error::Error for ShpError {}

/// Per-frame header. Pixel data lives at `offset`; `offset == 0` marks an
/// empty frame (common in shadow halves and unused facings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShpFrame {
    /// Placement of the frame's bounding box inside the full canvas.
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// Low byte of the flags word: 1 = raw, 2 = raw rows with u16 length
    /// prefix, 3 = rows with u16 length prefix + zero-run RLE.
    pub compression: u8,
    /// Average colour used for the radar/minimap, RGB.
    pub radar_color: [u8; 3],
    pub offset: u32,
}

impl ShpFrame {
    pub fn is_empty(&self) -> bool {
        self.offset == 0 || self.width == 0 || self.height == 0
    }
}

#[derive(Debug, Clone)]
pub struct ShpFile<'a> {
    /// Full canvas size shared by every frame.
    pub width: u16,
    pub height: u16,
    pub frames: Vec<ShpFrame>,
    data: &'a [u8],
}

impl<'a> ShpFile<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ShpError> {
        let mut c = Cursor::new(data);
        let zero = c.u16().ok_or(ShpError::BadHeader)?;
        let width = c.u16().ok_or(ShpError::BadHeader)?;
        let height = c.u16().ok_or(ShpError::BadHeader)?;
        let count = c.u16().ok_or(ShpError::BadHeader)? as usize;
        // A 0x0 canvas is legal when every frame is empty (retail NULL.SHP).
        if zero != 0 || count == 0 {
            return Err(ShpError::BadHeader);
        }
        if FILE_HEADER_LEN + count * FRAME_HEADER_LEN > data.len() {
            return Err(ShpError::Truncated);
        }
        let mut frames = Vec::with_capacity(count);
        for i in 0..count {
            let x = c.u16().ok_or(ShpError::Truncated)?;
            let y = c.u16().ok_or(ShpError::Truncated)?;
            let w = c.u16().ok_or(ShpError::Truncated)?;
            let h = c.u16().ok_or(ShpError::Truncated)?;
            let flags = c.u32().ok_or(ShpError::Truncated)?;
            let color = c.array::<4>().ok_or(ShpError::Truncated)?;
            c.skip(4).ok_or(ShpError::Truncated)?; // reserved
            let offset = c.u32().ok_or(ShpError::Truncated)?;
            let frame = ShpFrame {
                x,
                y,
                width: w,
                height: h,
                compression: flags as u8,
                radar_color: [color[0], color[1], color[2]],
                offset,
            };
            let fits = u32::from(x) + u32::from(w) <= u32::from(width)
                && u32::from(y) + u32::from(h) <= u32::from(height);
            if !frame.is_empty() && (!fits || offset as usize >= data.len()) {
                return Err(ShpError::BadFrame(i));
            }
            frames.push(frame);
        }
        Ok(ShpFile {
            width,
            height,
            frames,
            data,
        })
    }

    /// Decode frame `i` into `width * height` palette indices covering only
    /// the frame's own bounding box. Empty frames return an empty vec.
    pub fn decode_frame(&self, i: usize) -> Result<Vec<u8>, ShpError> {
        let f = *self.frames.get(i).ok_or(ShpError::BadFrame(i))?;
        if f.is_empty() {
            return Ok(Vec::new());
        }
        let (w, h) = (f.width as usize, f.height as usize);
        let remaining = self.data.len() - f.offset as usize;
        if w * h > remaining.saturating_mul(MAX_EXPANSION) {
            return Err(ShpError::Truncated);
        }
        let mut out = vec![0u8; w * h];
        let mut c = Cursor::at(self.data, f.offset as usize);
        match f.compression {
            0 | 1 => {
                let px = c.take(w * h).ok_or(ShpError::Truncated)?;
                out.copy_from_slice(px);
            }
            2 => {
                // Retail: every row prefix equals width + 2 (checked corpus-wide).
                for row in out.chunks_exact_mut(w) {
                    if row_len(&mut c)? != w {
                        return Err(ShpError::BadFrame(i));
                    }
                    row.copy_from_slice(c.take(w).ok_or(ShpError::Truncated)?);
                }
            }
            3 => {
                for row in out.chunks_exact_mut(w) {
                    let len = row_len(&mut c)?;
                    let src = c.take(len).ok_or(ShpError::Truncated)?;
                    unrle_zero(src, row).ok_or(ShpError::BadFrame(i))?;
                }
            }
            kind => return Err(ShpError::UnknownCompression { frame: i, kind }),
        }
        Ok(out)
    }

    /// Decode frame `i` onto a transparent canvas of the file's full size.
    pub fn decode_frame_canvas(&self, i: usize) -> Result<Vec<u8>, ShpError> {
        let f = *self.frames.get(i).ok_or(ShpError::BadFrame(i))?;
        let (cw, ch) = (self.width as usize, self.height as usize);
        if cw * ch > MAX_CANVAS_PIXELS {
            return Err(ShpError::BadHeader);
        }
        let mut canvas = vec![0u8; cw * ch];
        let px = self.decode_frame(i)?;
        let (w, x, y) = (f.width as usize, f.x as usize, f.y as usize);
        for (r, row) in px.chunks_exact(w.max(1)).enumerate() {
            let dst = (y + r) * cw + x;
            canvas[dst..dst + w].copy_from_slice(row);
        }
        Ok(canvas)
    }
}

/// Row length prefix counts itself (2 bytes).
fn row_len(c: &mut Cursor) -> Result<usize, ShpError> {
    let len = c.u16().ok_or(ShpError::Truncated)? as usize;
    len.checked_sub(2).ok_or(ShpError::Truncated)
}

/// Zero-run RLE: `0x00 n` = n transparent pixels, any other byte = literal.
/// Retail encoders may include one extra transparent pixel at the row end.
/// Validate every pair, including bytes after the visible width is filled.
fn unrle_zero(src: &[u8], row: &mut [u8]) -> Option<()> {
    let mut x = 0usize;
    let mut i = 0;
    while i < src.len() {
        let b = src[i];
        i += 1;
        if b == 0 {
            let n = usize::from(*src.get(i)?);
            i += 1;
            x = x.checked_add(n)?;
            if x > row.len().checked_add(1)? {
                return None;
            }
        } else {
            *row.get_mut(x)? = b;
            x += 1;
        }
    }
    (x >= row.len()).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(w: u16, h: u16, n: u16) -> Vec<u8> {
        [0u16, w, h, n]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    fn frame(x: u16, y: u16, w: u16, h: u16, kind: u32, off: u32) -> Vec<u8> {
        let mut v: Vec<u8> = [x, y, w, h].iter().flat_map(|v| v.to_le_bytes()).collect();
        v.extend(kind.to_le_bytes());
        v.extend([0x10, 0x20, 0x30, 0]); // radar colour
        v.extend([0; 4]);
        v.extend(off.to_le_bytes());
        v
    }

    #[test]
    fn raw_frame_and_canvas() {
        let mut d = header(4, 3, 1);
        d.extend(frame(1, 1, 2, 2, 1, 32));
        d.extend([1, 2, 3, 4]);
        let shp = ShpFile::parse(&d).unwrap();
        assert_eq!(shp.frames[0].radar_color, [0x10, 0x20, 0x30]);
        assert_eq!(shp.decode_frame(0).unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(
            shp.decode_frame_canvas(0).unwrap(),
            vec![0, 0, 0, 0, 0, 1, 2, 0, 0, 3, 4, 0]
        );
    }

    #[test]
    fn rle_zero_rows() {
        let mut d = header(5, 2, 1);
        d.extend(frame(0, 0, 5, 2, 3, 32));
        // row 0: 7, skip 3, 9  -> len = 2 + 4
        d.extend([6, 0, 7, 0, 3, 9]);
        // row 1: skip 4, 5     -> len = 2 + 3
        d.extend([5, 0, 0, 4, 5]);
        let shp = ShpFile::parse(&d).unwrap();
        assert_eq!(
            shp.decode_frame(0).unwrap(),
            vec![7, 0, 0, 0, 9, 0, 0, 0, 0, 5]
        );
    }

    #[test]
    fn rejects_malformed_rle_rows_in_frame_and_canvas() {
        for row in [
            vec![],                 // Empty encoded row.
            vec![7, 0],             // Missing zero-run count.
            vec![7, 0, 6],          // Run exceeds the retail one-pixel padding.
            vec![7, 0, 2],          // Decoded row is too short.
            vec![1, 2, 3, 4, 5, 6], // Literal after the complete row.
            vec![0, 5, 0],          // Incomplete run after the complete row.
            vec![0, 5, 0, 2],       // Extra run exceeds the padding allowance.
        ] {
            let mut data = header(5, 1, 1);
            data.extend(frame(0, 0, 5, 1, 3, 32));
            data.extend(((row.len() + 2) as u16).to_le_bytes());
            data.extend(&row);
            let shp = ShpFile::parse(&data).unwrap();
            assert_eq!(shp.decode_frame(0), Err(ShpError::BadFrame(0)), "{row:?}");
            assert_eq!(
                shp.decode_frame_canvas(0),
                Err(ShpError::BadFrame(0)),
                "{row:?}"
            );
        }
    }

    #[test]
    fn rle_rows_keep_exact_width_runs_and_canvas_placement() {
        let mut data = header(260, 4, 1);
        data.extend(frame(1, 1, 258, 2, 3, 32));
        // The first row ends exactly; the second has retail transparent padding.
        for row in [
            &[0, 0, 7, 0, 255, 0, 1, 9, 0, 0][..],
            &[7, 0, 255, 0, 1, 9, 0, 1][..],
        ] {
            data.extend(((row.len() + 2) as u16).to_le_bytes());
            data.extend(row);
        }
        let shp = ShpFile::parse(&data).unwrap();
        let mut expected = vec![0; 258 * 2];
        for offset in [0, 258] {
            expected[offset] = 7;
            expected[offset + 257] = 9;
        }
        assert_eq!(shp.decode_frame(0).unwrap(), expected);
        let canvas = shp.decode_frame_canvas(0).unwrap();
        let mut placed = vec![0; 260 * 4];
        placed[261..519].copy_from_slice(&expected[..258]);
        placed[521..779].copy_from_slice(&expected[258..]);
        assert_eq!(canvas, placed);
    }

    #[test]
    fn rle_runs_cannot_borrow_width_or_count_bytes_from_the_next_row() {
        for first in [vec![7, 0], vec![7, 0, 2], vec![0, 7]] {
            let mut data = header(5, 2, 1);
            data.extend(frame(0, 0, 5, 2, 3, 32));
            data.extend(((first.len() + 2) as u16).to_le_bytes());
            data.extend(first);
            data.extend([4, 0, 0, 5]); // Complete second row of transparent pixels.
            let shp = ShpFile::parse(&data).unwrap();
            assert_eq!(shp.decode_frame(0), Err(ShpError::BadFrame(0)));
        }
    }

    #[test]
    fn empty_frame_allowed() {
        let mut d = header(4, 4, 2);
        d.extend(frame(0, 0, 0, 0, 0, 0));
        d.extend(frame(0, 0, 1, 1, 1, 56));
        d.push(42);
        let shp = ShpFile::parse(&d).unwrap();
        assert!(shp.frames[0].is_empty());
        assert!(shp.decode_frame(0).unwrap().is_empty());
        assert_eq!(shp.decode_frame(1).unwrap(), vec![42]);
    }

    #[test]
    fn null_canvas_with_empty_frames() {
        // Retail CONQUER.MIX/NULL.SHP: 0x0 canvas, two empty frames whose
        // reserved bytes hold MSVC's 0xCC uninitialised-memory fill.
        let mut d = header(0, 0, 2);
        for _ in 0..2 {
            let mut f = frame(0, 0, 0, 0, 0, 0);
            f[12..16].copy_from_slice(&[0xcc; 4]);
            d.extend(f);
        }
        let shp = ShpFile::parse(&d).unwrap();
        assert!(shp.frames.iter().all(ShpFrame::is_empty));
    }

    #[test]
    fn hostile_frame_size_rejected_before_allocating() {
        // 65535x65535 raw frame backed by 4 bytes.
        let mut d = header(u16::MAX, u16::MAX, 1);
        d.extend(frame(0, 0, u16::MAX, u16::MAX, 1, 32));
        d.extend([0; 4]);
        let shp = ShpFile::parse(&d).unwrap();
        assert_eq!(shp.decode_frame(0).unwrap_err(), ShpError::Truncated);
        assert_eq!(shp.decode_frame_canvas(0).unwrap_err(), ShpError::BadHeader);
    }

    #[test]
    fn rejects_out_of_canvas_frame() {
        let mut d = header(2, 2, 1);
        d.extend(frame(1, 1, 2, 2, 1, 32));
        d.extend([0; 4]);
        assert_eq!(ShpFile::parse(&d).unwrap_err(), ShpError::BadFrame(0));
    }

    #[test]
    fn rejects_non_shp() {
        assert_eq!(
            ShpFile::parse(b"[General]\r\n").unwrap_err(),
            ShpError::BadHeader
        );
    }
}
