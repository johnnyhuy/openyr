//! HVA (hierarchical voxel animation) reader, per-frame limb transforms
//! that pair with a VXL model of the same name.
//!
//! Spec with evidence: `docs/formats/vxl.md` (HVA section).

use crate::bytes::{fixed_name, Cursor};

#[derive(Debug, PartialEq, Eq)]
pub enum HvaError {
    Truncated,
    BadHeader,
}

impl std::fmt::Display for HvaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HvaError::Truncated => write!(f, "truncated HVA file"),
            HvaError::BadHeader => write!(f, "invalid HVA header"),
        }
    }
}

impl std::error::Error for HvaError {}

/// Row-major 3x4 transform (rotation | translation).
pub type Transform = [[f32; 4]; 3];

#[derive(Debug, Clone, PartialEq)]
pub struct HvaFile {
    /// Free-form name field from the header (often the source path).
    pub name: String,
    /// Limb names, used to match transforms to VXL limbs.
    pub limbs: Vec<String>,
    /// `frames[f][l]` = transform of limb `l` in frame `f`.
    pub frames: Vec<Vec<Transform>>,
}

impl HvaFile {
    pub fn parse(data: &[u8]) -> Result<Self, HvaError> {
        let mut c = Cursor::new(data);
        let name = fixed_name(c.take(16).ok_or(HvaError::Truncated)?);
        let frame_count = c.u32().ok_or(HvaError::Truncated)? as usize;
        let limb_count = c.u32().ok_or(HvaError::Truncated)? as usize;
        if frame_count == 0 || limb_count == 0 {
            return Err(HvaError::BadHeader);
        }
        let need = limb_count
            .checked_mul(frame_count)
            .and_then(|n| n.checked_mul(48))
            .and_then(|n| n.checked_add(24 + 16 * limb_count));
        if need.is_none_or(|n| data.len() < n) {
            return Err(HvaError::Truncated);
        }
        let limbs = (0..limb_count)
            .map(|_| c.take(16).map(fixed_name).ok_or(HvaError::Truncated))
            .collect::<Result<Vec<_>, _>>()?;
        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let mut row = Vec::with_capacity(limb_count);
            for _ in 0..limb_count {
                let m = c.f32s::<12>().ok_or(HvaError::Truncated)?;
                row.push([
                    [m[0], m[1], m[2], m[3]],
                    [m[4], m[5], m[6], m[7]],
                    [m[8], m[9], m[10], m[11]],
                ]);
            }
            frames.push(row);
        }
        Ok(HvaFile {
            name,
            limbs,
            frames,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frames() {
        let mut d = Vec::new();
        let mut name = [0u8; 16];
        name[..5].copy_from_slice(b"htank");
        d.extend(name);
        d.extend(2u32.to_le_bytes());
        d.extend(1u32.to_le_bytes());
        let mut limb = [0u8; 16];
        limb[..4].copy_from_slice(b"body");
        d.extend(limb);
        for f in 0..2 {
            for k in 0..12 {
                d.extend((f as f32 * 100.0 + k as f32).to_le_bytes());
            }
        }
        let hva = HvaFile::parse(&d).unwrap();
        assert_eq!(hva.name, "htank");
        assert_eq!(hva.limbs, vec!["body"]);
        assert_eq!(hva.frames.len(), 2);
        assert_eq!(hva.frames[1][0][2], [108., 109., 110., 111.]);
        assert!(HvaFile::parse(&d[..d.len() - 1]).is_err());
    }
}
