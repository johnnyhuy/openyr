//! Previous per-voxel formulas retained as an independent pixel oracle.

use super::*;

fn project([x, y, z]: [f32; 3], facing: u16) -> [f32; 3] {
    let ang = f32::from((facing % 256 + 32) % 256) * (std::f32::consts::TAU / 256.0);
    let (sin, cos) = ang.sin_cos();
    let (rx, ry) = (x * cos - y * sin, x * sin + y * cos);
    let horizontal = (rx + ry) * std::f32::consts::FRAC_1_SQRT_2;
    let vertical = 3.0_f32.sqrt() / 2.0;
    [
        (rx - ry) * std::f32::consts::FRAC_1_SQRT_2,
        horizontal * 0.5 - z * vertical,
        horizontal * vertical + z * 0.5,
    ]
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
fn brightness(normal: [f32; 3], pose: &Transform, facing: u16) -> u8 {
    let Some(normal) = posed_normal(normal, pose) else {
        // Isolated voxels, symmetric thin sheets and invalid poses retain
        // their original colour rather than acquiring an arbitrary normal.
        return 255;
    };
    let normal = project(normal, facing);
    let light = normalize([-0.45, -0.65, 0.61]).unwrap();
    let diffuse = normal
        .into_iter()
        .zip(light)
        .map(|(n, l)| n * l)
        .sum::<f32>()
        .clamp(0., 1.);
    ((0.78 + 0.22 * diffuse) * 255.).round() as u8
}

fn reference_points(model: &VoxelModel, facing: u16, frames: &[usize]) -> Vec<Point> {
    let mut points = Vec::new();
    for (i, part) in model.parts.iter().enumerate() {
        for (limb_index, limb) in part.vxl.limbs.iter().enumerate() {
            let Some(m) = pose(part, limb, frames.get(i).copied().unwrap_or(0)) else {
                continue;
            };
            for (voxel_index, v) in limb.voxels.iter().enumerate() {
                let Some(p) = model_point(limb, &m, v) else {
                    continue;
                };
                let p = project(p, facing);
                if p.iter().all(|v| v.is_finite()) {
                    let normal = part.normals[limb_index][voxel_index];
                    points.push((p, v.color, brightness(normal, &m, facing)));
                }
            }
        }
    }
    points
}

const IDENTITY: Transform = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]];

fn poses() -> Vec<Transform> {
    vec![
        IDENTITY,
        [[2., 0., 0., 100.], [0., 1., 0., -100.], [0., 0., 0.5, 99.]],
        [[0., -1., 0., 5.], [1., 0., 0., 10.], [0., 0., 1., 20.]],
        [[-1., 2., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]],
        [[0.; 4]; 3],
        IDENTITY.map(|row| row.map(|v| v * f32::MAX)),
        [[f32::NAN; 4]; 3],
        [[f32::INFINITY; 4]; 3],
    ]
}

#[test]
fn shared_voxel_setup_matches_previous_formulas_at_every_heading() {
    let poses = poses();
    let mut samples = vec![[0.; 3], [f32::NAN; 3], [f32::INFINITY; 3]];
    for x in -3..=3 {
        for y in -3..=3 {
            for z in -3..=3 {
                samples.push([x as f32 / 3., y as f32 / 3., z as f32 / 3.]);
            }
        }
    }
    for facing in (0..256).chain([256, 511, u16::MAX]) {
        let projection = Projection::new(facing);
        for point in &samples {
            let expected = project(*point, facing).map(f32::to_bits);
            assert_eq!(projection.apply(*point).map(f32::to_bits), expected);
        }
        for pose in &poses {
            let setup = lighting::Lighting::new(pose, projection);
            for normal in &samples {
                assert_eq!(setup.brightness(*normal), brightness(*normal, pose, facing));
            }
        }
    }
    println!("259 headings x 8 poses x 346 normals retain every brightness byte; projected coordinate bits match the previous formulas, including invalid/extreme inputs.");
}

fn authored_model() -> VoxelModel {
    use yr_formats::vxl::{Voxel, VxlLimb};
    let mut voxels = Vec::new();
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..5 {
                voxels.push(Voxel {
                    x,
                    y,
                    z,
                    color: 16 + x,
                    normal: 0,
                });
            }
        }
    }
    VoxelModel {
        parts: vec![Part::new(
            VxlFile {
                palette: [0; 768],
                remap: (16, 31),
                limbs: vec![VxlLimb {
                    name: "body".into(),
                    scale: 1.,
                    transform: IDENTITY,
                    min_bounds: [-2., -2., 0.],
                    max_bounds: [3., 3., 5.],
                    size: [5; 3],
                    normals_mode: 4,
                    voxels,
                }],
            },
            Some(yr_formats::HvaFile {
                name: "authored".into(),
                limbs: vec!["body".into()],
                frames: poses().into_iter().map(|pose| vec![pose]).collect(),
            }),
        )],
    }
}

fn compare_model(model: &VoxelModel, pal: &Palette) -> usize {
    let mut checked = 0;
    let poses: std::collections::BTreeSet<_> = (0..16).map(|tick| model.frames_at(tick)).collect();
    for frames in poses {
        for facing in (0..256).step_by(32) {
            let expected = reference_points(model, facing, &frames);
            let actual = projected_points(model, facing, &frames);
            assert_eq!(
                actual, expected,
                "point/depth/color/brightness at {frames:?}/{facing}"
            );
            for remap in [None, Some([220, 30, 30]), Some([30, 80, 220])] {
                let mut old = vec![0; 256 * 256 * 4];
                raster(&mut old, 256, 256, 128, 128, expected.clone(), pal, remap);
                let mut new = vec![0; old.len()];
                raster(&mut new, 256, 256, 128, 128, actual.clone(), pal, remap);
                assert_eq!(new, old);
            }
            let sprite = animated_sprite(model, facing, &frames, pal, [220, 30, 30]);
            if expected.is_empty() {
                assert!(sprite.is_none());
            } else {
                let mut min = [0.0_f32; 2];
                let mut max = [0.0_f32; 2];
                for (point, _, _) in &expected {
                    for axis in 0..2 {
                        min[axis] = min[axis].min(point[axis].floor() - 1.);
                        max[axis] = max[axis].max(point[axis].ceil() + 2.);
                    }
                }
                let [w, h] = std::array::from_fn(|i| max[i] - min[i] + 1.);
                if w > 512. || h > 512. {
                    assert!(sprite.is_none());
                } else {
                    let sprite = sprite.expect("previous bounds admit a live sprite");
                    assert_eq!(
                        (sprite.w, sprite.h, sprite.ax, sprite.ay),
                        (w as u32, h as u32, -min[0] as i32, -min[1] as i32)
                    );
                    let mut old = vec![0; sprite.rgba.len()];
                    raster(
                        &mut old,
                        sprite.w as usize,
                        sprite.h as usize,
                        sprite.ax,
                        sprite.ay,
                        expected,
                        pal,
                        Some([220, 30, 30]),
                    );
                    assert_eq!(old, sprite.rgba);
                }
            }
            checked += 1;
        }
    }
    checked
}

#[test]
fn shared_voxel_setup_preserves_authored_pose_pixels_and_unlit_fallbacks() {
    let model = authored_model();
    let palette = Palette::parse(&(0..768).map(|n| (n % 64) as u8).collect::<Vec<_>>()).unwrap();
    assert_eq!(compare_model(&model, &palette), 64);
    println!("64 authored pose/heading views retain points, depth, ownerless and two house raster buffers, live sprite bytes and unlit fallbacks.");
}

#[test]
fn shared_voxel_setup_preserves_retail_model_pixels() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let palette = Palette::parse(fs.read("UNITTEM.PAL").unwrap()).unwrap();
    let mut checked = 0;
    for id in ["MTNK", "HTNK", "CMIN", "ROBO", "SHAD", "ZEP"] {
        checked += compare_model(&VoxelModel::load(&fs, id).unwrap(), &palette);
    }
    println!("{checked} retail pose/heading views retain exact point, depth, lighting, remap, raster and live sprite bytes across six models.");
}

#[test]
#[ignore = "optional optimized voxel projection timing probe"]
fn voxel_setup_scaling_probe() {
    use std::{hint::black_box, time::Instant};
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    let mut models = vec![("authored", authored_model())];
    if dir.is_dir() {
        let fs = AssetFs::open(&dir).unwrap();
        for id in ["MTNK", "CMIN", "SHAD"] {
            models.push((id, VoxelModel::load(&fs, id).unwrap()));
        }
    }
    for (name, model) in models {
        let requests: Vec<_> = (0..16)
            .map(|tick| (tick as u16 * 16, model.frames_at(tick)))
            .collect();
        for (label, project) in [
            (
                "previous",
                reference_points as fn(&VoxelModel, u16, &[usize]) -> Vec<Point>,
            ),
            ("shared", projected_points),
        ] {
            let start = Instant::now();
            let mut count = 0;
            for _ in 0..16 {
                for (facing, frames) in &requests {
                    count += black_box(project(black_box(&model), *facing, frames)).len();
                }
            }
            println!("{name} {label}: {count} points in {:?}", start.elapsed());
        }
    }
}
