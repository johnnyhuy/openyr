//! VXL voxel model reader, vehicle bodies, turrets, barrels.
//!
//! Spec with evidence: `docs/formats/vxl.md`. A file holds one or more limbs
//! (sections); each limb is a sparse grid of (colour, normal) voxels with its
//! own scale, bounds and base transform. HVA files animate the limbs.

use crate::bytes::{fixed_name, Cursor};

const SIGNATURE: &[u8; 16] = b"Voxel Animation\0";
const HEADER_LEN: usize = 802;
const LIMB_HEADER_LEN: usize = 28;
const LIMB_TAILER_LEN: usize = 92;

#[derive(Debug, PartialEq, Eq)]
pub enum VxlError {
    BadSignature,
    Truncated,
    BadLimb(usize),
}

impl std::fmt::Display for VxlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VxlError::BadSignature => write!(f, "not a VXL file"),
            VxlError::Truncated => write!(f, "truncated VXL file"),
            VxlError::BadLimb(i) => write!(f, "VXL limb {i} has invalid span data"),
        }
    }
}

impl std::error::Error for VxlError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voxel {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    /// Palette index (remappable range is `VxlFile::remap`).
    pub color: u8,
    /// Index into the normal table selected by `VxlLimb::normals_mode`.
    pub normal: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VxlLimb {
    pub name: String,
    pub scale: f32,
    /// Row-major 3x4 base transform (rotation | translation).
    pub transform: [[f32; 4]; 3],
    pub min_bounds: [f32; 3],
    pub max_bounds: [f32; 3],
    /// Grid dimensions (x, y, z).
    pub size: [u8; 3],
    /// 2 = Tiberian Sun normal table (36 entries), 4 = RA2 table (244 entries).
    pub normals_mode: u8,
    pub voxels: Vec<Voxel>,
}

#[derive(Debug, Clone)]
pub struct VxlFile {
    /// Embedded 6-bit palette. Retail models carry one but the game renders
    /// with the theater/unit palette instead.
    pub palette: [u8; 768],
    /// Inclusive player-colour remap range in the palette.
    pub remap: (u8, u8),
    pub limbs: Vec<VxlLimb>,
}

impl VxlFile {
    pub fn parse(data: &[u8]) -> Result<Self, VxlError> {
        let mut c = Cursor::new(data);
        if c.take(16) != Some(SIGNATURE.as_slice()) {
            return Err(VxlError::BadSignature);
        }
        let _palette_count = c.u32().ok_or(VxlError::Truncated)?;
        let limb_count = c.u32().ok_or(VxlError::Truncated)? as usize;
        let tailer_count = c.u32().ok_or(VxlError::Truncated)? as usize;
        let body_size = c.u32().ok_or(VxlError::Truncated)? as usize;
        let remap = (
            c.u8().ok_or(VxlError::Truncated)?,
            c.u8().ok_or(VxlError::Truncated)?,
        );
        let palette = c.array::<768>().ok_or(VxlError::Truncated)?;
        debug_assert_eq!(c.pos(), HEADER_LEN);
        if limb_count != tailer_count || limb_count == 0 {
            return Err(VxlError::BadSignature);
        }
        // Check the counts against the file before allocating per limb.
        let fixed = limb_count
            .checked_mul(LIMB_HEADER_LEN + LIMB_TAILER_LEN)
            .and_then(|n| n.checked_add(HEADER_LEN + body_size));
        if fixed.is_none_or(|n| n > data.len()) {
            return Err(VxlError::Truncated);
        }

        let mut names = Vec::with_capacity(limb_count);
        for _ in 0..limb_count {
            let name = fixed_name(c.take(16).ok_or(VxlError::Truncated)?);
            c.skip(12).ok_or(VxlError::Truncated)?; // limb number, two reserved words
            names.push(name);
        }

        let body_start = HEADER_LEN + LIMB_HEADER_LEN * limb_count;
        let body = data
            .get(body_start..body_start + body_size)
            .ok_or(VxlError::Truncated)?;
        let mut t = Cursor::at(data, body_start + body_size);
        if data.len() < t.pos() + LIMB_TAILER_LEN * limb_count {
            return Err(VxlError::Truncated);
        }

        let mut limbs = Vec::with_capacity(limb_count);
        for (i, name) in names.into_iter().enumerate() {
            let span_start = t.u32().ok_or(VxlError::Truncated)? as usize;
            let span_end = t.u32().ok_or(VxlError::Truncated)? as usize;
            let span_data = t.u32().ok_or(VxlError::Truncated)? as usize;
            let scale = t.f32().ok_or(VxlError::Truncated)?;
            let m = t.f32s::<12>().ok_or(VxlError::Truncated)?;
            let min_bounds = t.f32s::<3>().ok_or(VxlError::Truncated)?;
            let max_bounds = t.f32s::<3>().ok_or(VxlError::Truncated)?;
            let size = t.array::<3>().ok_or(VxlError::Truncated)?;
            let normals_mode = t.u8().ok_or(VxlError::Truncated)?;
            let transform = [
                [m[0], m[1], m[2], m[3]],
                [m[4], m[5], m[6], m[7]],
                [m[8], m[9], m[10], m[11]],
            ];
            let voxels = read_spans(body, span_start, span_end, span_data, size)
                .ok_or(VxlError::BadLimb(i))?;
            limbs.push(VxlLimb {
                name,
                scale,
                transform,
                min_bounds,
                max_bounds,
                size,
                normals_mode,
                voxels,
            });
        }
        Ok(VxlFile {
            palette,
            remap,
            limbs,
        })
    }
}

/// Decode one limb's column spans. Each (x, y) column has a start and end
/// offset into the span data (`-1` = empty column). A column is a run of
/// `skip, count, count * (colour, normal), count` segments until z reaches
/// the limb height.
fn read_spans(
    body: &[u8],
    span_start: usize,
    span_end: usize,
    span_data: usize,
    size: [u8; 3],
) -> Option<Vec<Voxel>> {
    let (sx, sy, sz) = (size[0] as usize, size[1] as usize, size[2] as usize);
    let columns = sx * sy;
    let table_bytes = columns.checked_mul(4)?;
    let mut starts = Cursor::new(body.get(span_start..span_start.checked_add(table_bytes)?)?);
    let mut ends = Cursor::new(body.get(span_end..span_end.checked_add(table_bytes)?)?);
    let data = body.get(span_data..)?;
    let mut voxels = Vec::new();
    for col in 0..columns {
        let start = starts.i32()?;
        let end = ends.i32()?;
        if start == -1 {
            continue;
        }
        let (x, y) = ((col % sx) as u8, (col / sx) as u8);
        let first = usize::try_from(start).ok()?;
        let last = usize::try_from(end).ok()?;
        // The inclusive end must bound every read, including the repeated
        // count. Do not walk into neighboring columns before rejecting it.
        let column = data.get(first..last.checked_add(1)?)?;
        let mut c = Cursor::new(column);
        let mut z = 0usize;
        while z < sz {
            z += c.u8()? as usize;
            let count = c.u8()? as usize;
            if z > sz || count > sz - z {
                return None;
            }
            for _ in 0..count {
                let color = c.u8()?;
                let normal = c.u8()?;
                voxels.push(Voxel {
                    x,
                    y,
                    z: z as u8,
                    color,
                    normal,
                });
                z += 1;
            }
            if c.u8()? as usize != count {
                return None;
            }
        }
        // The end offset names the column's final byte (the trailing count).
        if c.pos() != column.len() {
            return None;
        }
    }
    Some(voxels)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One 2x1x4 limb: column 0 has voxels at z=1,2; column 1 empty.
    fn sample() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend(SIGNATURE);
        for v in [1u32, 1, 1] {
            d.extend(v.to_le_bytes());
        }
        // body: starts[2], ends[2], data
        let span_data: Vec<u8> = vec![1, 2, 11, 101, 12, 102, 2, 1, 0, 0];
        let mut body = Vec::new();
        body.extend(0i32.to_le_bytes());
        body.extend((-1i32).to_le_bytes());
        body.extend(((span_data.len() - 1) as i32).to_le_bytes());
        body.extend((-1i32).to_le_bytes());
        body.extend(&span_data);
        d.extend((body.len() as u32).to_le_bytes());
        d.extend([16, 31]); // remap range
        d.extend([0u8; 768]);
        let mut name = [0u8; 16];
        name[..4].copy_from_slice(b"body");
        d.extend(name);
        d.extend([0u8; 12]);
        d.extend(&body);
        // tailer
        for v in [0u32, 8, 16] {
            d.extend(v.to_le_bytes());
        }
        d.extend(0.5f32.to_le_bytes());
        let ident = [1f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.];
        for v in ident {
            d.extend(v.to_le_bytes());
        }
        for v in [-1f32, -0.5, 0., 1., 0.5, 4.] {
            d.extend(v.to_le_bytes());
        }
        d.extend([2, 1, 4, 4]);
        d
    }

    #[test]
    fn parses_limb_and_voxels() {
        let vxl = VxlFile::parse(&sample()).unwrap();
        assert_eq!(vxl.remap, (16, 31));
        assert_eq!(vxl.limbs.len(), 1);
        let l = &vxl.limbs[0];
        assert_eq!(l.name, "body");
        assert_eq!(l.scale, 0.5);
        assert_eq!(l.size, [2, 1, 4]);
        assert_eq!(l.normals_mode, 4);
        assert_eq!(l.transform[1], [0., 1., 0., 0.]);
        assert_eq!(
            l.voxels,
            vec![
                Voxel {
                    x: 0,
                    y: 0,
                    z: 1,
                    color: 11,
                    normal: 101
                },
                Voxel {
                    x: 0,
                    y: 0,
                    z: 2,
                    color: 12,
                    normal: 102
                },
            ]
        );
    }

    #[test]
    fn rejects_bad_signature() {
        assert_eq!(
            VxlFile::parse(b"Voxel Animatiox\0").unwrap_err(),
            VxlError::BadSignature
        );
    }

    #[test]
    fn hostile_limb_count_rejected_before_allocating() {
        let mut d = sample();
        d[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        d[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(VxlFile::parse(&d).unwrap_err(), VxlError::Truncated);
    }

    #[test]
    fn rejects_a_column_that_skips_beyond_its_declared_height() {
        let mut d = sample();
        let spans = HEADER_LEN + LIMB_HEADER_LEN + 16;
        // The last empty run follows two valid voxels at z=1 and z=2.
        // Skipping two more cells would end at z=5 in a four-cell column.
        d[spans + 7] = 2;
        assert_eq!(VxlFile::parse(&d).unwrap_err(), VxlError::BadLimb(0));
    }

    #[test]
    fn column_ranges_and_complete_tables_are_required() {
        let original = sample();
        let body = HEADER_LEN + LIMB_HEADER_LEN;
        let tailer = original.len() - LIMB_TAILER_LEN;
        for (offset, value) in [
            (body, -2),
            (body, i32::MAX),
            (body, 10),
            (body + 8, -1),
            (body + 8, i32::MAX),
            (body + 8, 8), // excludes the final repeated count
            (body, 9),     // an incomplete run at the end of the column
            (tailer, i32::MAX),
            (tailer + 4, i32::MAX),
            (tailer + 8, i32::MAX),
            (tailer, 23),     // only three bytes remain for two starts
            (tailer + 4, 23), // only three bytes remain for two ends
        ] {
            let mut d = original.clone();
            d[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert_eq!(
                VxlFile::parse(&d).unwrap_err(),
                VxlError::BadLimb(0),
                "offset {offset}, value {value}"
            );
        }
        // A reversed, in-bounds range must fail too.
        let mut d = original.clone();
        d[body..body + 4].copy_from_slice(&8i32.to_le_bytes());
        d[body + 8..body + 12].copy_from_slice(&2i32.to_le_bytes());
        assert_eq!(VxlFile::parse(&d).unwrap_err(), VxlError::BadLimb(0));
        for len in 0..original.len() {
            assert!(VxlFile::parse(&original[..len]).is_err(), "prefix {len}");
        }
    }

    fn one_column(bytes: &[u8], end: i32) -> Option<Vec<Voxel>> {
        let mut body = 0i32.to_le_bytes().to_vec();
        body.extend(end.to_le_bytes());
        body.extend(bytes);
        read_spans(&body, 0, 4, 8, [1, 1, 4])
    }

    #[test]
    fn runs_require_complete_bytes_matching_counts_and_exact_height() {
        for bytes in [
            &[][..],
            &[4],
            &[4, 0],
            &[4, 0, 1], // inconsistent repeated count
            &[0, 1, 11],
            &[0, 1, 11, 101],
            &[0, 1, 11, 101, 2],
            &[0, 0, 0],                   // consumes bytes without advancing z
            &[3, 2, 11, 101, 12, 102, 2], // run exceeds height
            &[4, 1, 11, 101, 1],
            &[4, 0, 0, 0], // unconsumed byte inside the column
        ] {
            assert!(one_column(bytes, bytes.len() as i32 - 1).is_none());
        }
        // Empty runs may reach exactly the top, but never pass it.
        for skip in 4..=u8::MAX {
            let result = one_column(&[skip, 0, 0], 2);
            if skip == 4 {
                assert_eq!(result, Some(vec![]));
            } else {
                assert!(result.is_none(), "skip {skip}");
            }
        }
        // A valid next column cannot supply this column's missing count.
        assert!(one_column(&[4, 0, 0, 4, 0, 0], 1).is_none());
        assert_eq!(one_column(&[4, 0, 0, 4, 0, 0], 2), Some(vec![]));
    }

    #[test]
    fn sparse_full_and_empty_columns_preserve_voxels_and_coordinates() {
        let mut body = Vec::new();
        for offset in [0i32, -1, 10, 21, 9, -1, 20, 23] {
            body.extend(offset.to_le_bytes());
        }
        body.extend([1, 2, 11, 101, 12, 102, 2, 1, 0, 0]);
        body.extend([0, 4, 21, 111, 22, 112, 23, 113, 24, 114, 4]);
        body.extend([4, 0, 0]);
        let voxels = read_spans(&body, 0, 16, 32, [2, 2, 4]).unwrap();
        let positions: Vec<_> = voxels.iter().map(|v| (v.x, v.y, v.z)).collect();
        assert_eq!(
            positions,
            [
                (0, 0, 1),
                (0, 0, 2),
                (0, 1, 0),
                (0, 1, 1),
                (0, 1, 2),
                (0, 1, 3)
            ]
        );
        let pixels: Vec<_> = voxels.iter().map(|v| (v.color, v.normal)).collect();
        assert_eq!(
            pixels,
            [
                (11, 101),
                (12, 102),
                (21, 111),
                (22, 112),
                (23, 113),
                (24, 114)
            ]
        );
    }

    #[test]
    fn rejects_truncated() {
        let d = sample();
        assert_eq!(
            VxlFile::parse(&d[..d.len() - 10]).unwrap_err(),
            VxlError::Truncated
        );
    }
}
