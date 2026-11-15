use super::*;
use yr_render::sprites::{Sprite, SHADOW_ALPHA};

#[derive(Clone, Copy, Debug)]
enum Pass {
    Body,
    Effect,
    Shadow,
}

impl Pass {
    fn native(self, mask: &mut [bool]) -> SpritePass<'_> {
        match self {
            Self::Body => SpritePass::Body,
            Self::Effect => SpritePass::Effect,
            Self::Shadow => SpritePass::Shadow(mask),
        }
    }
}

// Destination-driven oracle: test every viewport pixel rather than clipping
// source iteration. Keep the previous sampling and blending equations explicit.
#[allow(clippy::too_many_arguments)]
fn reference(
    frame: &mut [u8],
    size: (u32, u32),
    sprite: &Sprite,
    anchor: (f32, f32),
    scale: f32,
    pass: Pass,
    mask: &mut [bool],
) {
    let origin = (
        (anchor.0 - sprite.ax as f32 * scale).round() as i32,
        (anchor.1 - sprite.ay as f32 * scale).round() as i32,
    );
    let extent = (
        (sprite.w as f32 * scale).ceil() as i32,
        (sprite.h as f32 * scale).ceil() as i32,
    );
    for y in 0..size.1 {
        for x in 0..size.0 {
            let dx = i64::from(x) - i64::from(origin.0);
            let dy = i64::from(y) - i64::from(origin.1);
            if dx < 0 || dy < 0 || dx >= i64::from(extent.0) || dy >= i64::from(extent.1) {
                continue;
            }
            let sx = ((dx as f32 + 0.5) / scale) as u32;
            let sy = ((dy as f32 + 0.5) / scale) as u32;
            if sx >= sprite.w || sy >= sprite.h {
                continue;
            }
            let src = ((sy * sprite.w + sx) * 4) as usize;
            let dst = ((y * size.0 + x) * 4) as usize;
            let alpha = u32::from(sprite.rgba[src + 3]);
            match pass {
                Pass::Shadow if alpha == u32::from(SHADOW_ALPHA) && !mask[dst / 4] => {
                    for channel in &mut frame[dst..dst + 3] {
                        *channel /= 2;
                    }
                    mask[dst / 4] = true;
                }
                Pass::Body if alpha == 255 => {
                    frame[dst..dst + 4].copy_from_slice(&sprite.rgba[src..src + 4]);
                }
                Pass::Body if alpha > 0 && alpha != u32::from(SHADOW_ALPHA) => {
                    let da = u32::from(frame[dst + 3]);
                    let weight = alpha * 255 + da * (255 - alpha);
                    for c in 0..3 {
                        frame[dst + c] = ((u32::from(sprite.rgba[src + c]) * alpha * 255
                            + u32::from(frame[dst + c]) * da * (255 - alpha)
                            + weight / 2)
                            / weight) as u8;
                    }
                    frame[dst + 3] = ((weight + 127) / 255) as u8;
                }
                Pass::Effect if alpha > 0 => {
                    for c in 0..3 {
                        frame[dst + c] = ((u32::from(sprite.rgba[src + c]) * alpha
                            + u32::from(frame[dst + c]) * (255 - alpha))
                            / 255) as u8;
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn clipped_sprites_preserve_sampling_blending_and_shadow_union_at_every_edge() {
    let size = (13, 11);
    let mut sprite = Sprite {
        w: 4,
        h: 3,
        ax: 1,
        ay: 2,
        rgba: [0, 1, 127, 128, 129, 254, 255, 255, 128, 0, 255, 255]
            .into_iter()
            .enumerate()
            .flat_map(|(i, alpha)| [23 + i as u8 * 17, 231 - i as u8 * 13, 71, alpha])
            .collect(),
    };
    let background: Vec<_> = (0..size.0 * size.1)
        .flat_map(|i| [100, 80, 60, [0, 80, 255][i as usize % 3]])
        .collect();
    let initial_mask: Vec<_> = (0..size.0 * size.1).map(|i| i % 5 == 0).collect();
    let positions = [-20.0, -7.5, -4.0, -0.5, 0.0, 1.5, 6.0, 12.25, 17.0, 30.0];
    let mut cases = 0;
    for offset in [(1, 2), (-2, -1)] {
        (sprite.ax, sprite.ay) = offset;
        for scale in [0.125, 0.5, 0.75, 1.0, 1.25, 2.0, 4.0] {
            for x in positions {
                for y in positions {
                    for pass in [Pass::Body, Pass::Effect, Pass::Shadow] {
                        let mut expected = background.clone();
                        let mut actual = background.clone();
                        let mut expected_mask = initial_mask.clone();
                        let mut actual_mask = initial_mask.clone();
                        for _ in 0..2 {
                            reference(
                                &mut expected,
                                size,
                                &sprite,
                                (x, y),
                                scale,
                                pass,
                                &mut expected_mask,
                            );
                            blit_sprite(
                                &mut actual,
                                size.0,
                                size.1,
                                &sprite,
                                x,
                                y,
                                scale,
                                pass.native(&mut actual_mask),
                            );
                            assert_eq!(actual, expected, "{offset:?} {scale} ({x},{y}) {pass:?}");
                            assert_eq!(actual_mask, expected_mask);
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    println!("{cases} authored edge/scale/pass cases preserve complete RGBA and shadow masks through two draws.");
}

#[test]
fn sprite_iteration_is_bounded_by_visible_dimensions_in_wide_coordinates() {
    for origin in -10..=10 {
        for extent in 0..=10 {
            for limit in 0..=8 {
                let expected: Vec<_> = (0..i64::from(extent))
                    .filter(|&offset| (0..i64::from(limit)).contains(&(i64::from(origin) + offset)))
                    .collect();
                assert_eq!(
                    clipped_sprite_axis(origin, extent, limit).collect::<Vec<_>>(),
                    expected
                );
            }
        }
    }
    for origin in [
        i32::MIN,
        i32::MIN + 1,
        -4090,
        0,
        639,
        640,
        i32::MAX - 1,
        i32::MAX,
    ] {
        for extent in [0, 1, 4096, i32::MAX] {
            for limit in [0, 1, 640, u32::MAX] {
                let range = clipped_sprite_axis(origin, extent, limit);
                if !range.is_empty() {
                    assert!(range.end - range.start <= i64::from(limit));
                    assert!(range.start >= 0 && range.end <= i64::from(extent));
                    assert!((0..i64::from(limit)).contains(&(i64::from(origin) + range.start)));
                    assert!((0..i64::from(limit)).contains(&(i64::from(origin) + range.end - 1)));
                }
            }
        }
    }
    assert!(clipped_sprite_axis(640, i32::MAX, 640).is_empty());
    assert!(clipped_sprite_axis(i32::MIN, i32::MAX, 640).is_empty());
    assert_eq!(clipped_sprite_axis(-4090, 4096, 640), 4090..4096);
    println!("Iteration never exceeds the viewport; fully offscreen axes are empty, including saturated coordinates.");
}

#[test]
fn nonfinite_sprite_transforms_leave_pixels_and_shadow_coverage_intact() {
    let sprite = Sprite {
        w: 1,
        h: 1,
        ax: 0,
        ay: 0,
        rgba: vec![255; 4],
    };
    for (x, y, scale) in [
        (f32::NAN, 0.0, 1.0),
        (0.0, f32::NAN, 1.0),
        (f32::INFINITY, 0.0, 1.0),
        (0.0, f32::NEG_INFINITY, 1.0),
        (0.0, 0.0, f32::NAN),
        (0.0, 0.0, f32::INFINITY),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, -1.0),
    ] {
        for pass in [Pass::Body, Pass::Effect, Pass::Shadow] {
            let mut frame = vec![100, 80, 60, 255];
            let mut mask = [false];
            blit_sprite(
                &mut frame,
                1,
                1,
                &sprite,
                x,
                y,
                scale,
                pass.native(&mut mask),
            );
            assert_eq!(frame, [100, 80, 60, 255]);
            assert_eq!(mask, [false]);
        }
    }
}
