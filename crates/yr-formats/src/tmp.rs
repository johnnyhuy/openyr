//! TMP (TS/RA2 isometric terrain template) reader.
//!
//! Spec with evidence: `docs/formats/tmp.md`. A template is a grid of
//! diamond tiles (60x30 in RA2); each present cell carries colour and depth
//! diamonds plus an optional rectangular "extra" overlay (cliffs, trees on
//! tiles) and terrain metadata used by the simulation.

use crate::bytes::Cursor;

const TILE_HEADER_LEN: usize = 52;
const FLAG_EXTRA: u32 = 0x1;
const FLAG_DEPTH: u32 = 0x2;
const FLAG_DAMAGED: u32 = 0x4;

#[derive(Debug, PartialEq, Eq)]
pub enum TmpError {
    BadHeader,
    Truncated,
    BadTile(usize),
}

impl std::fmt::Display for TmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TmpError::BadHeader => write!(f, "not a TS/RA2 TMP file"),
            TmpError::Truncated => write!(f, "truncated TMP file"),
            TmpError::BadTile(i) => write!(f, "TMP cell {i} is malformed"),
        }
    }
}

impl std::error::Error for TmpError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmpExtra {
    /// Position relative to the template origin, in pixels.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// `width * height` palette indices, 0 = transparent.
    pub pixels: Vec<u8>,
    /// Matching depth values when the tile has a depth buffer.
    pub depth: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmpTile {
    /// Cell position in pixels relative to the template origin.
    pub x: i32,
    pub y: i32,
    /// Cell elevation in height levels.
    pub height: i8,
    /// Terrain class index (clear, water, rock, road, ... — see docs).
    pub terrain_type: u8,
    /// Slope/ramp shape for elevated terrain (0 = flat).
    pub ramp_type: u8,
    pub radar_left: [u8; 3],
    pub radar_right: [u8; 3],
    pub damaged: bool,
    /// Diamond colour pixels packed row by row (`w * h / 2` bytes); use
    /// [`TmpFile::diamond_rows`] to place them.
    pub pixels: Vec<u8>,
    pub depth: Option<Vec<u8>>,
    pub extra: Option<TmpExtra>,
}

#[derive(Debug, Clone)]
pub struct TmpFile {
    /// Template size in cells.
    pub cells_x: u32,
    pub cells_y: u32,
    /// Diamond size in pixels (60x30 in RA2, 48x24 in TS).
    pub tile_width: u32,
    pub tile_height: u32,
    /// Row-major `cells_x * cells_y`; `None` = empty cell.
    pub tiles: Vec<Option<TmpTile>>,
}

impl TmpFile {
    pub fn parse(data: &[u8]) -> Result<Self, TmpError> {
        let mut c = Cursor::new(data);
        let cells_x = c.u32().ok_or(TmpError::BadHeader)?;
        let cells_y = c.u32().ok_or(TmpError::BadHeader)?;
        let tile_width = c.u32().ok_or(TmpError::BadHeader)?;
        let tile_height = c.u32().ok_or(TmpError::BadHeader)?;
        let cells = (cells_x as usize).checked_mul(cells_y as usize);
        let valid_dims = (1..=64).contains(&cells_x)
            && (1..=64).contains(&cells_y)
            && tile_width > 0
            && u64::from(tile_width) == u64::from(tile_height) * 2
            && tile_width <= 256;
        let Some(cells) = cells.filter(|_| valid_dims) else {
            return Err(TmpError::BadHeader);
        };
        let diamond = (tile_width * tile_height / 2) as usize;
        let offsets = (0..cells)
            .map(|_| c.u32().ok_or(TmpError::Truncated))
            .collect::<Result<Vec<_>, _>>()?;
        let tiles = offsets
            .iter()
            .enumerate()
            .map(|(i, &off)| match off {
                0 => Ok(None),
                off => parse_tile(data, off as usize, diamond)
                    .map(Some)
                    .ok_or(TmpError::BadTile(i)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(TmpFile {
            cells_x,
            cells_y,
            tile_width,
            tile_height,
            tiles,
        })
    }

    /// Diamond scanlines as `(row, x_start, width)`: rows widen by 4 px from
    /// the top until the middle, then narrow. Packed pixel data follows this
    /// order. The last row of a 60x30 tile has width 0.
    pub fn diamond_rows(&self) -> Vec<(u32, u32, u32)> {
        let (w, h) = (self.tile_width, self.tile_height);
        let mut rows = Vec::with_capacity(h as usize);
        let mut width = 4u32;
        for y in 0..h {
            rows.push((y, (w - width) / 2, width));
            if y + 1 < h / 2 {
                width += 4;
            } else {
                width = width.saturating_sub(4);
            }
        }
        rows
    }
}

fn parse_tile(data: &[u8], base: usize, diamond: usize) -> Option<TmpTile> {
    let mut c = Cursor::at(data, base);
    let x = c.i32()?;
    let y = c.i32()?;
    let extra_ofs = c.u32()? as usize;
    let depth_ofs = c.u32()? as usize;
    let extra_depth_ofs = c.u32()? as usize;
    let extra_x = c.i32()?;
    let extra_y = c.i32()?;
    let extra_w = c.u32()?;
    let extra_h = c.u32()?;
    let flags = c.u32()?;
    let height = c.u8()? as i8;
    let terrain_type = c.u8()?;
    let ramp_type = c.u8()?;
    let radar_left = c.array::<3>()?;
    let radar_right = c.array::<3>()?;
    c.skip(3)?;
    debug_assert_eq!(c.pos() - base, TILE_HEADER_LEN);

    let at = |ofs: usize, len: usize| data.get(base + ofs..base + ofs + len).map(<[u8]>::to_vec);
    let pixels = c.take(diamond)?.to_vec();
    let has_depth = flags & FLAG_DEPTH != 0;
    let depth = if has_depth { Some(at(depth_ofs, diamond)?) } else { None };
    let extra = if flags & FLAG_EXTRA != 0 {
        let n = (extra_w as usize).checked_mul(extra_h as usize)?;
        Some(TmpExtra {
            x: extra_x,
            y: extra_y,
            width: extra_w,
            height: extra_h,
            pixels: at(extra_ofs, n)?,
            depth: if has_depth { Some(at(extra_depth_ofs, n)?) } else { None },
        })
    } else {
        None
    };
    Some(TmpTile {
        x,
        y,
        height,
        terrain_type,
        ramp_type,
        radar_left,
        radar_right,
        damaged: flags & FLAG_DAMAGED != 0,
        pixels,
        depth,
        extra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1x1 template of 8x4 diamonds (16 px each) with depth and a 2x1 extra.
    fn sample() -> Vec<u8> {
        let mut d = Vec::new();
        for v in [1u32, 1, 8, 4, 20] {
            d.extend(v.to_le_bytes());
        }
        let base = 20u32;
        let depth_ofs = 52 + 16;
        let extra_ofs = depth_ofs + 16;
        let extra_depth_ofs = extra_ofs + 2;
        for v in [0i32, 15] {
            d.extend(v.to_le_bytes());
        }
        for v in [extra_ofs, depth_ofs, extra_depth_ofs] {
            d.extend((v as u32).to_le_bytes());
        }
        for v in [3i32, -5, 2, 1] {
            d.extend(v.to_le_bytes());
        }
        d.extend((FLAG_EXTRA | FLAG_DEPTH).to_le_bytes());
        d.extend([2, 6, 0, 1, 2, 3, 4, 5, 6, 0, 0, 0]);
        assert_eq!(d.len() as u32, base + 52);
        d.extend((1..=16).collect::<Vec<u8>>());
        d.extend([9u8; 16]);
        d.extend([77, 0]);
        d.extend([5, 6]);
        d
    }

    #[test]
    fn parses_tile_with_extra_and_depth() {
        let tmp = TmpFile::parse(&sample()).unwrap();
        assert_eq!((tmp.cells_x, tmp.tile_width, tmp.tile_height), (1, 8, 4));
        let t = tmp.tiles[0].as_ref().unwrap();
        assert_eq!((t.x, t.y, t.height, t.terrain_type), (0, 15, 2, 6));
        assert_eq!(t.radar_right, [4, 5, 6]);
        assert_eq!(t.pixels.len(), 16);
        assert_eq!(t.depth.as_deref(), Some(&[9u8; 16][..]));
        let e = t.extra.as_ref().unwrap();
        assert_eq!((e.x, e.y, e.width, e.height), (3, -5, 2, 1));
        assert_eq!(e.pixels, vec![77, 0]);
        assert_eq!(e.depth.as_deref(), Some(&[5u8, 6][..]));
    }

    #[test]
    fn diamond_rows_cover_half_the_box() {
        let tmp = TmpFile {
            cells_x: 1,
            cells_y: 1,
            tile_width: 60,
            tile_height: 30,
            tiles: vec![None],
        };
        let rows = tmp.diamond_rows();
        assert_eq!(rows.iter().map(|r| r.2).sum::<u32>(), 900);
        assert_eq!(rows[0], (0, 28, 4));
        assert_eq!(rows[14], (14, 0, 60));
        assert_eq!(rows[29].2, 0);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(TmpFile::parse(b"Voxel Animation\0").unwrap_err(), TmpError::BadHeader);
    }
}
