//! Native allocation policy for the full raster, before optional downscaling.

pub(super) const MAX_RASTER_BYTES: usize = 1024 * 1024 * 1024;

pub(super) fn rgba_len(width: usize, height: usize) -> Result<usize, String> {
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|&bytes| bytes <= MAX_RASTER_BYTES)
        .ok_or_else(|| "map raster exceeds 1 GiB native RGBA budget".into())
}

pub(super) struct Layout {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: usize,
    pub height: usize,
    pub bytes: usize,
}

impl Layout {
    pub fn from_bounds(x0: i64, y0: i64, x1: i64, y1: i64) -> Result<Self, String> {
        if x0 > x1 || y0 > y1 {
            return Err("map has no drawable tiles".into());
        }
        let invalid = || "map raster coordinates exceed supported bounds".to_string();
        let (x0, y0, x1, y1) = (
            x0.checked_sub(90).ok_or_else(invalid)?,
            y0.checked_sub(200).ok_or_else(invalid)?,
            x1.checked_add(90).ok_or_else(invalid)?,
            y1.checked_add(40).ok_or_else(invalid)?,
        );
        let width = x1
            .checked_sub(x0)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(invalid)?;
        let height = y1
            .checked_sub(y0)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(invalid)?;
        let bytes = rgba_len(width, height)?;
        // The renderer's public origins remain i32; reject rather than truncate.
        let origin_x = i32::try_from(x0).map_err(|_| invalid())?;
        let origin_y = i32::try_from(y0).map_err(|_| invalid())?;
        i32::try_from(x1).map_err(|_| invalid())?;
        i32::try_from(y1).map_err(|_| invalid())?;
        Ok(Self {
            origin_x,
            origin_y,
            width,
            height,
            bytes,
        })
    }
}

pub(super) fn extra_origin(
    cx: i64,
    cy: i64,
    tile: &yr_formats::TmpTile,
    extra: &yr_formats::tmp::TmpExtra,
) -> (i64, i64) {
    (
        cx - i64::from(super::HALF_W) - i64::from(tile.x) + i64::from(extra.x),
        cy - i64::from(tile.y) + i64::from(extra.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_budget_checks_ceiling_without_allocating_it() {
        assert_eq!(rgba_len(16384, 16384).unwrap(), MAX_RASTER_BYTES);
        assert!(rgba_len(16384, 16385).is_err());
        assert!(rgba_len(usize::MAX, 2).is_err());
        assert!(rgba_len(usize::MAX / 4 + 1, 1).is_err());
    }

    #[test]
    fn raster_bounds_pad_in_wide_checked_coordinates() {
        let layout = Layout::from_bounds(-30, 30, 30, 60).unwrap();
        assert_eq!(
            (
                layout.origin_x,
                layout.origin_y,
                layout.width,
                layout.height,
                layout.bytes
            ),
            (-120, -170, 240, 270, 259200)
        );
        assert!(Layout::from_bounds(i64::MIN, 0, 0, 30).is_err());
        assert!(Layout::from_bounds(0, 0, i64::MAX, 30).is_err());
        assert!(Layout::from_bounds(i64::from(i32::MAX), 0, i64::from(i32::MAX), 30).is_err());
        assert!(Layout::from_bounds(-1_000_000, 0, 1_000_000, 30).is_err());
        println!("Raster arithmetic rejects oversized bounds and overflowing padding before allocation; 1 GiB ceiling checked without allocating it.");
    }
}
