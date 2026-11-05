//! Bounded world-segment rasterization for transient projectile line trails.

use crate::{effects::Trail, interaction::ground_point};
use yr_render::RenderedMap;
use yr_sim::TerrainGrid;

pub struct View<'a> {
    pub size: (u32, u32),
    pub viewport_width: u32,
    pub map: &'a RenderedMap,
    pub terrain: Option<&'a TerrainGrid>,
    pub camera: (f32, f32, f32),
}

/// One native world pixel of additive color, scaled with the shared camera.
/// Original raster width, blending and fade cadence remain uncalibrated.
pub fn draw(frame: &mut [u8], trail: &Trail, tick: u64, view: View<'_>) {
    let View {
        size,
        viewport_width,
        map,
        terrain,
        camera,
    } = view;
    let Some(color) = trail.color(tick) else {
        return;
    };
    let (width, height) = size;
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        != Some(frame.len())
    {
        return;
    }
    let (cam_x, cam_y, zoom) = camera;
    let project = |p: (yr_sim::Fix, yr_sim::Fix)| {
        let (x, y) = ground_point(
            map,
            terrain,
            (p.0.raw() as f32 / 256.0, p.1.raw() as f32 / 256.0),
        );
        (f64::from((x - cam_x) * zoom), f64::from((y - cam_y) * zoom))
    };
    raster(
        frame,
        size,
        viewport_width,
        project(trail.from),
        project(trail.to),
        f64::from(zoom / map.scale.max(1) as f32),
        color,
    );
}

fn raster(
    frame: &mut [u8],
    size: (u32, u32),
    viewport_width: u32,
    a: (f64, f64),
    b: (f64, f64),
    scale: f64,
    color: [u8; 3],
) {
    if ![a.0, a.1, b.0, b.1, scale].iter().all(|v| v.is_finite()) || scale <= 0.0 {
        return;
    }
    let radius = scale.clamp(1.0, 16.0) / 2.0;
    let left = (a.0.min(b.0) - radius).floor().max(0.0) as u32;
    let right = ((a.0.max(b.0) + radius).ceil().max(0.0) as u32).min(size.0.min(viewport_width));
    let top = (a.1.min(b.1) - radius).floor().max(0.0) as u32;
    let bottom = ((a.1.max(b.1) + radius).ceil().max(0.0) as u32).min(size.1);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length2 = dx * dx + dy * dy;
    for y in top..bottom {
        for x in left..right {
            let (px, py) = (f64::from(x) + 0.5 - a.0, f64::from(y) + 0.5 - a.1);
            let t = if length2 > 0.0 {
                ((px * dx + py * dy) / length2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            if (px - t * dx).powi(2) + (py - t * dy).powi(2) > radius * radius {
                continue;
            }
            let at = (y as usize * size.0 as usize + x as usize) * 4;
            for (channel, value) in color.into_iter().enumerate() {
                frame[at + channel] = frame[at + channel].saturating_add(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_clip_before_rasterizing_and_preserve_alpha_and_sidebar() {
        let mut frame = vec![10; 12 * 8 * 4];
        raster(
            &mut frame,
            (12, 8),
            8,
            (-1e20, 3.5),
            (1e20, 3.5),
            1.0,
            [20, 50, 255],
        );
        for y in 0..8 {
            for x in 0..12 {
                let at = (y * 12 + x) * 4;
                assert_eq!(
                    &frame[at..at + 4],
                    if y == 3 && x < 8 {
                        &[30, 60, 255, 10]
                    } else {
                        &[10; 4]
                    }
                );
            }
        }
        let before = frame.clone();
        for (a, b, scale) in [
            ((f64::NAN, 0.0), (0.0, 0.0), 1.0),
            ((-1e20, -1e20), (-1e19, -1e19), 1.0),
            ((0.0, 0.0), (8.0, 8.0), 0.0),
        ] {
            raster(&mut frame, (12, 8), 8, a, b, scale, [255; 3]);
        }
        assert_eq!(before, frame);
    }
}
