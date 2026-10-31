//! Geometry-derived lighting until retail normal tables are verified.

use std::collections::HashSet;
use yr_formats::{hva::Transform, vxl::VxlLimb};

/// Estimate the outward density gradient over a 3x3x3 neighbourhood.
/// Storage follows decoded voxels, never the volume claimed by a header.
/// Divide by cell spacing to express gradients in authored model units.
pub(super) fn surface_normals(limb: &VxlLimb) -> Vec<[f32; 3]> {
    let spacing: [f64; 3] = std::array::from_fn(|i| {
        (f64::from(limb.max_bounds[i]) - f64::from(limb.min_bounds[i])) / f64::from(limb.size[i])
    });
    if spacing.iter().any(|s| !s.is_finite() || *s <= 0.) {
        return vec![[0.; 3]; limb.voxels.len()];
    }
    let valid = |p: [i16; 3]| {
        p.into_iter()
            .zip(limb.size)
            .all(|(p, size)| p >= 0 && p < i16::from(size))
    };
    let occupied: HashSet<[i16; 3]> = limb
        .voxels
        .iter()
        .map(|v| [i16::from(v.x), i16::from(v.y), i16::from(v.z)])
        .filter(|p| valid(*p))
        .collect();
    limb.voxels
        .iter()
        .map(|v| {
            let p = [i16::from(v.x), i16::from(v.y), i16::from(v.z)];
            if !valid(p) {
                return [0.; 3];
            }
            let mut gradient = [0_i32; 3];
            for dx in -1_i16..=1 {
                for dy in -1_i16..=1 {
                    for dz in -1_i16..=1 {
                        let distance = dx * dx + dy * dy + dz * dz;
                        if distance == 0 || !occupied.contains(&[p[0] + dx, p[1] + dy, p[2] + dz]) {
                            continue;
                        }
                        for (axis, offset) in [dx, dy, dz].into_iter().enumerate() {
                            // Distances are 1, 2 or 3. Integer weights
                            // cancel exactly inside symmetric surfaces.
                            gradient[axis] -= i32::from(offset * (6 / distance));
                        }
                    }
                }
            }
            normalize(std::array::from_fn(|i| f64::from(gradient[i]) / spacing[i]))
                .unwrap_or([0.; 3])
        })
        .collect()
}

fn normalize(v: [f64; 3]) -> Option<[f32; 3]> {
    if v.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let max = v.iter().map(|v| v.abs()).fold(0., f64::max);
    if max == 0. {
        return None;
    }
    let v = v.map(|v| v / max);
    let length = v.iter().map(|v| v * v).sum::<f64>().sqrt();
    Some(v.map(|v| (v / length) as f32))
}

/// Inverse-transpose normals follow rotation, scale and shear without
/// inheriting translation. f64 intermediates cover finite f32 poses.
fn posed_normal(normal: [f32; 3], pose: &Transform) -> Option<[f32; 3]> {
    let [a, b, c] = pose.map(|row| [row[0], row[1], row[2]].map(f64::from));
    let cross = |u: [f64; 3], v: [f64; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let cofactors = [cross(b, c), cross(c, a), cross(a, b)];
    let det: f64 = a.into_iter().zip(cofactors[0]).map(|(a, c)| a * c).sum();
    if !det.is_finite() || det == 0. {
        return None;
    }
    // Normalization removes |det|; retain its sign for reflected poses.
    normalize(cofactors.map(|row| {
        row.into_iter()
            .zip(normal)
            .map(|(c, n)| c * f64::from(n))
            .sum::<f64>()
            * det.signum()
    }))
}

/// Fixed light above and to the left in the orthographic camera. This
/// is an artistic approximation, not a retail normal-table/VPL lookup.
pub(super) fn brightness(normal: [f32; 3], pose: &Transform, facing: u16) -> u8 {
    let Some(normal) = posed_normal(normal, pose) else {
        // Isolated voxels, symmetric thin sheets and invalid poses retain
        // their original colour rather than acquiring an arbitrary normal.
        return 255;
    };
    let normal = super::project(normal, facing);
    let light = normalize([-0.45, -0.65, 0.61]).unwrap();
    let diffuse = normal
        .into_iter()
        .zip(light)
        .map(|(n, l)| n * l)
        .sum::<f32>()
        .clamp(0., 1.);
    ((0.62 + 0.38 * diffuse) * 255.).round() as u8
}

/// Light ordinary and house-remapped colours equally, preserving alpha.
pub(super) fn shade(mut rgba: [u8; 4], brightness: u8) -> [u8; 4] {
    for channel in &mut rgba[..3] {
        *channel = ((u16::from(*channel) * u16::from(brightness) + 127) / 255) as u8;
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_formats::vxl::Voxel;

    const IDENTITY: Transform = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]];

    fn cube() -> VxlLimb {
        let mut voxels = Vec::new();
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..5 {
                    voxels.push(Voxel {
                        x,
                        y,
                        z,
                        color: 42,
                        normal: 255,
                    });
                }
            }
        }
        VxlLimb {
            name: "cube".into(),
            scale: 1.,
            transform: IDENTITY,
            min_bounds: [0.; 3],
            max_bounds: [5.; 3],
            size: [5; 3],
            normals_mode: 255,
            voxels,
        }
    }

    fn close(a: [f32; 3], b: [f32; 3]) {
        assert!(
            a.into_iter().zip(b).all(|(a, b)| (a - b).abs() < 0.00001),
            "{a:?} != {b:?}"
        );
    }

    #[test]
    fn solid_faces_point_outward_and_corners_respect_authored_spacing() {
        let mut limb = cube();
        let normals = surface_normals(&limb);
        let at = |normals: &[[f32; 3]], x, y, z| {
            let i = limb
                .voxels
                .iter()
                .position(|v| (v.x, v.y, v.z) == (x, y, z))
                .unwrap();
            normals[i]
        };
        for (point, normal) in [
            ([0, 2, 2], [-1., 0., 0.]),
            ([4, 2, 2], [1., 0., 0.]),
            ([2, 0, 2], [0., -1., 0.]),
            ([2, 4, 2], [0., 1., 0.]),
            ([2, 2, 0], [0., 0., -1.]),
            ([2, 2, 4], [0., 0., 1.]),
        ] {
            close(at(&normals, point[0], point[1], point[2]), normal);
        }
        close(at(&normals, 2, 2, 2), [0.; 3]);
        close(at(&normals, 4, 4, 4), [1. / 3_f32.sqrt(); 3]);
        let corner = limb
            .voxels
            .iter()
            .position(|v| (v.x, v.y, v.z) == (4, 4, 4))
            .unwrap();
        limb.max_bounds = [10., 5., 2.5];
        close(
            surface_normals(&limb)[corner],
            normalize([0.5, 1., 2.]).unwrap(),
        );
        // Normal ids and modes are not a geometry input; no proprietary
        // lookup table is inferred from arbitrary byte values.
        limb.normals_mode = 2;
        for v in &mut limb.voxels {
            v.normal = 0;
        }
        close(
            surface_normals(&limb)[corner],
            normalize([0.5, 1., 2.]).unwrap(),
        );
    }

    #[test]
    fn normals_use_inverse_transpose_and_ignore_translation() {
        let n = normalize([1., 1., 0.]).unwrap();
        let pose = [[2., 0., 0., 100.], [0., 1., 0., -100.], [0., 0., 0.5, 99.]];
        close(
            posed_normal(n, &pose).unwrap(),
            normalize([0.5, 1., 0.]).unwrap(),
        );
        let rotated = [[0., -1., 0., 5.], [1., 0., 0., 10.], [0., 0., 1., 20.]];
        close(posed_normal([1., 0., 0.], &rotated).unwrap(), [0., 1., 0.]);
        let reflected = [[-1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]];
        close(
            posed_normal([1., 0., 0.], &reflected).unwrap(),
            [-1., 0., 0.],
        );
        let shear = [[1., 2., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]];
        close(
            posed_normal([1., 0., 0.], &shear).unwrap(),
            normalize([1., -2., 0.]).unwrap(),
        );
    }

    #[test]
    fn light_is_fixed_while_facing_and_authored_pose_change() {
        let top = brightness([0., 0., 1.], &IDENTITY, 0);
        let left = brightness([0., 1., 0.], &IDENTITY, 0);
        let away = brightness([0., -1., 0.], &IDENTITY, 0);
        assert!(top > left && left > away);
        assert_eq!(away, 158); // 62% ambient when the face points away.
        assert_eq!(brightness([0., -1., 0.], &IDENTITY, 128), left);
        let rotated = [[0., -1., 0., 0.], [1., 0., 0., 0.], [0., 0., 1., 0.]];
        assert_eq!(brightness([1., 0., 0.], &rotated, 0), left);
        for facing in [0, 32, 64, 128, 224, 255, u16::MAX] {
            assert_eq!(brightness([0., 0., 1.], &IDENTITY, facing), top);
            assert_eq!(
                brightness([1., 0., 0.], &IDENTITY, facing),
                brightness([1., 0., 0.], &IDENTITY, facing % 256)
            );
        }
        assert_eq!(shade([200, 100, 50, 128], 158), [124, 62, 31, 128]);
        assert_eq!(shade([255, 0, 0, 255], 255), [255, 0, 0, 255]);
    }

    #[test]
    fn invalid_and_ambiguous_geometry_keeps_unlit_colours() {
        let mut limb = cube();
        limb.voxels.truncate(1);
        assert_eq!(surface_normals(&limb), [[0.; 3]]);
        for size in [[0, 5, 5], [5, 0, 5], [5, 5, 0]] {
            limb.size = size;
            assert_eq!(surface_normals(&limb), [[0.; 3]]);
        }
        limb.size = [5; 3];
        limb.max_bounds[0] = f32::NAN;
        assert_eq!(surface_normals(&limb), [[0.; 3]]);
        let mut invalid = IDENTITY;
        for value in [0., f32::NAN, f32::INFINITY] {
            invalid[0][0] = value;
            assert_eq!(brightness([1., 0., 0.], &invalid, 0), 255);
        }
        assert_eq!(brightness([0.; 3], &IDENTITY, 0), 255);
        assert_eq!(brightness([f32::NAN; 3], &IDENTITY, 0), 255);
        let extreme = IDENTITY.map(|row| row.map(|v| v * f32::MAX));
        close(posed_normal([1., 0., 0.], &extreme).unwrap(), [1., 0., 0.]);
    }
}
