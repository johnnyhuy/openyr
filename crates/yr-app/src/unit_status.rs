//! Shared world status raster. Geometry is native and remains uncalibrated.
use super::SIDEBAR_W;

/// Six-pixel corners track the ground footprint, clipped to the world viewport.
pub(super) fn draw_brackets(
    frame: &mut [u8],
    size: (u32, u32),
    center: (f32, f32),
    half: f32,
    color: [u8; 4],
) {
    let (width, height) = size;
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        != Some(frame.len())
        || !center.0.is_finite()
        || !center.1.is_finite()
        || !half.is_finite()
        || half <= 0.0
    {
        return;
    }
    let world = width.saturating_sub(SIDEBAR_W);
    let rx = half.min(width as f32 + 16.0);
    let ry = (half * 0.5).min(height as f32 + 16.0);
    if center.0 < -rx
        || center.0 > world as f32 + rx
        || center.1 < -ry
        || center.1 > height as f32 + ry
    {
        return;
    }
    let pixel = |frame: &mut [u8], x: i64, y: i64| {
        if x >= 0 && y >= 0 && x < i64::from(world) && y < i64::from(height) {
            let at = ((y as usize * width as usize) + x as usize) * 4;
            frame[at..at + 4].copy_from_slice(&color);
        }
    };
    for sx in [-1, 1] {
        for sy in [-1, 1] {
            let x = center.0.round() as i64 + sx * rx.round() as i64;
            let y = center.1.round() as i64 + sy * ry.round() as i64;
            for step in 0..6 {
                pixel(frame, x - sx * step, y);
                pixel(frame, x, y - sy * step);
            }
        }
    }
}

/// Bordered pips, with a dark lower edge to separate health from terrain.
/// The existing 50/25 percent color thresholds are presentation policy.
pub(super) fn draw_health(
    frame: &mut [u8],
    size: (u32, u32),
    center: (f32, f32),
    width: f32,
    fraction: f32,
) {
    let (fw, fh) = size;
    if (fw as usize)
        .checked_mul(fh as usize)
        .and_then(|n| n.checked_mul(4))
        != Some(frame.len())
        || !center.0.is_finite()
        || !center.1.is_finite()
        || !width.is_finite()
        || !fraction.is_finite()
        || width <= 0.0
    {
        return;
    }
    // Fixed UI-pixel pips; zoom changes the number of pips, not their thickness.
    let count = ((width.clamp(12.0, 126.0) as i32) / 3).max(1);
    let span = count * 3 + 1;
    let world = fw.saturating_sub(SIDEBAR_W);
    if center.0 < -(span as f32)
        || center.0 > world as f32 + span as f32
        || center.1 < -5.0
        || center.1 > fh as f32
    {
        return;
    }
    let x = center.0.round() as i64 - i64::from(span / 2);
    let y = center.1.round() as i64;
    let fraction = fraction.clamp(0.0, 1.0);
    // A living object keeps one pip. Zero health has no colored pixels.
    let filled = (fraction * count as f32).ceil() as i32;
    let (light, dark) = if fraction > 0.5 {
        ([0, 252, 0, 255], [0, 124, 0, 255])
    } else if fraction > 0.25 {
        ([252, 252, 0, 255], [124, 124, 0, 255])
    } else {
        ([252, 0, 0, 255], [124, 0, 0, 255])
    };
    for dx in 0..span {
        for dy in 0..4 {
            let px = x + i64::from(dx);
            let py = y + i64::from(dy);
            if px < 0 || py < 0 || px >= i64::from(world) || py >= i64::from(fh) {
                continue;
            }
            let inside = dy == 1 || dy == 2;
            let pip = dx > 0 && dx < span - 1 && (dx - 1) % 3 < 2;
            let color = if inside && pip && (dx - 1) / 3 < filled {
                if dy == 1 {
                    light
                } else {
                    dark
                }
            } else {
                [0, 0, 0, 255]
            };
            let at = (py as usize * fw as usize + px as usize) * 4;
            frame[at..at + 4].copy_from_slice(&color);
        }
    }
}

#[cfg(test)]
mod tests;
