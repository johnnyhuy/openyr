//! VXL voxel model reader — vehicle bodies, turrets, barrels.
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
    let mut starts = Cursor::at(body, span_start);
    let mut ends = Cursor::at(body, span_end);
    let data = body.get(span_data..)?;
    let mut voxels = Vec::new();
    for col in 0..columns {
        let start = starts.i32()?;
        let end = ends.i32()?;
        if start == -1 {
            continue;
        }
        let (x, y) = ((col % sx) as u8, (col / sx) as u8);
        let mut c = Cursor::at(data, usize::try_from(start).ok()?);
        let last = usize::try_from(end).ok()?;
        let mut z = 0usize;
        while z < sz {
            z += c.u8()? as usize;
            let count = c.u8()? as usize;
            for _ in 0..count {
                let color = c.u8()?;
                let normal = c.u8()?;
                if z >= sz {
                    return None;
                }
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
        if c.pos() != last + 1 {
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
                Voxel { x: 0, y: 0, z: 1, color: 11, normal: 101 },
                Voxel { x: 0, y: 0, z: 2, color: 12, normal: 102 },
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
    fn rejects_truncated() {
        let d = sample();
        assert_eq!(
            VxlFile::parse(&d[..d.len() - 10]).unwrap_err(),
            VxlError::Truncated
        );
    }
}
