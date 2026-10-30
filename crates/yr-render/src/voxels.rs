//! Shared voxel projection for map objects and live sprites.
//! Authored bounds and the first HVA pose place every part in model space.

use yr_assets::AssetFs;
use yr_formats::{hva::Transform, Palette, VxlFile};

struct Part {
    vxl: VxlFile,
    hva: Option<yr_formats::HvaFile>,
}

pub(crate) struct VoxelModel {
    parts: Vec<Part>,
}

impl VoxelModel {
    pub(crate) fn load(vfs: &AssetFs, image: &str) -> Option<Self> {
        let load = |suffix: &str| {
            let stem = format!("{image}{suffix}");
            let vxl = VxlFile::parse(vfs.read(&format!("{stem}.VXL"))?).ok()?;
            let hva = vfs
                .read(&format!("{stem}.HVA"))
                .and_then(|data| yr_formats::HvaFile::parse(data).ok());
            Some(Part { vxl, hva })
        };
        let body = load("")?;
        let mut parts = vec![body];
        parts.extend(["TUR", "BARL"].into_iter().filter_map(load));
        Some(Self { parts })
    }
}

/// Size live sprites from their projected pixels, keeping the model origin
/// as the anchor. Bounds are capped before allocation for loose/modded data.
pub(crate) fn sprite(
    model: &VoxelModel,
    facing: u16,
    pal: &Palette,
    remap: [u8; 3],
) -> Option<crate::sprites::Sprite> {
    let points = projected_points(model, facing);
    if points.is_empty() {
        return None;
    }
    let mut min = [0.0_f32; 2];
    let mut max = [0.0_f32; 2];
    for (p, _) in &points {
        for axis in 0..2 {
            min[axis] = min[axis].min(p[axis].floor() - 1.);
            max[axis] = max[axis].max(p[axis].ceil() + 2.);
        }
    }
    let [w, h] = std::array::from_fn(|i| max[i] - min[i] + 1.);
    if w > 512. || h > 512. {
        return None;
    }
    let (w, h, ax, ay) = (w as u32, h as u32, -min[0] as i32, -min[1] as i32);
    let mut rgba = vec![0; w as usize * h as usize * 4];
    raster(
        &mut rgba,
        w as usize,
        h as usize,
        ax,
        ay,
        points,
        pal,
        Some(remap),
    );
    Some(crate::sprites::Sprite { w, h, ax, ay, rgba })
}

fn finite_transform(m: &Transform) -> bool {
    m.iter().flatten().all(|v| v.is_finite())
}

/// HVA translation is in source units; VXL base translation is already
/// scaled. HVA replaces the base pose. See the retail ROBO measurements
/// in docs/formats/vxl.md.
fn pose(part: &Part, limb: &yr_formats::vxl::VxlLimb) -> Option<Transform> {
    let hva = part.hva.as_ref().and_then(|h| {
        let index = h
            .limbs
            .iter()
            .position(|n| n.eq_ignore_ascii_case(&limb.name))?;
        h.frames.first()?.get(index).copied()
    });
    if let Some(mut m) = hva.filter(finite_transform) {
        for row in &mut m {
            row[3] *= limb.scale;
        }
        if finite_transform(&m) {
            return Some(m);
        }
    }
    finite_transform(&limb.transform).then_some(limb.transform)
}

fn model_point(
    limb: &yr_formats::vxl::VxlLimb,
    m: &Transform,
    v: &yr_formats::vxl::Voxel,
) -> Option<[f32; 3]> {
    if !limb.scale.is_finite() || limb.scale <= 0.0 || limb.size.contains(&0) {
        return None;
    }
    let mut local = [0.0; 3];
    for (axis, index) in [v.x, v.y, v.z].into_iter().enumerate() {
        let (min, max) = (limb.min_bounds[axis], limb.max_bounds[axis]);
        if !min.is_finite() || !max.is_finite() || max <= min || index >= limb.size[axis] {
            return None;
        }
        // Sample voxel centres inside the limb's authored box.
        local[axis] = min + (f32::from(index) + 0.5) * (max - min) / f32::from(limb.size[axis]);
    }
    let point = m.map(|row| row[0] * local[0] + row[1] * local[1] + row[2] * local[2] + row[3]);
    point.iter().all(|v| v.is_finite()).then_some(point)
}

/// Fixed orthographic camera: 45 degree azimuth, 30 degree elevation.
/// One model unit keeps the same pixel scale across parts and facings.
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

/// Draw the combined body/turret/barrel with one ground anchor and one
/// depth ordering. Two-pixel samples fill gaps in the projected grid;
/// normal-table lighting and animated HVA poses remain fidelity work.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_voxel(
    rgba: &mut [u8],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    model: &VoxelModel,
    facing: u16,
    pal: &Palette,
    remap: Option<[u8; 3]>,
) {
    raster(
        rgba,
        w,
        h,
        cx,
        cy,
        projected_points(model, facing),
        pal,
        remap,
    );
}

type Point = ([f32; 3], u8);

fn projected_points(model: &VoxelModel, facing: u16) -> Vec<Point> {
    let mut points = Vec::new();
    for part in &model.parts {
        for limb in &part.vxl.limbs {
            let Some(m) = pose(part, limb) else { continue };
            for v in &limb.voxels {
                let Some(p) = model_point(limb, &m, v) else {
                    continue;
                };
                let p = project(p, facing);
                if p.iter().all(|v| v.is_finite()) {
                    points.push((p, v.color));
                }
            }
        }
    }
    points
}

#[allow(clippy::too_many_arguments)]
fn raster(
    rgba: &mut [u8],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    mut points: Vec<Point>,
    pal: &Palette,
    remap: Option<[u8; 3]>,
) {
    points.sort_by(|a, b| a.0[2].total_cmp(&b.0[2]));
    for ([x, y, _], color) in points {
        // Clip before narrowing so hostile finite offsets cannot overflow.
        let (px, py) = (f64::from(cx) + f64::from(x), f64::from(cy) + f64::from(y));
        if px < -2.0 || py < -2.0 || px >= w as f64 || py >= h as f64 {
            continue;
        }
        let (px, py) = (px.round() as i64, py.round() as i64);
        let color = crate::remap_rgba(pal, color, remap);
        for dy in 0..2 {
            for dx in 0..2 {
                let (Ok(x), Ok(y)) = (usize::try_from(px + dx), usize::try_from(py + dy)) else {
                    continue;
                };
                if x < w && y < h {
                    let at = y
                        .checked_mul(w)
                        .and_then(|n| n.checked_add(x))
                        .and_then(|n| n.checked_mul(4));
                    if let Some(pixel) = at.and_then(|at| rgba.get_mut(at..at.checked_add(4)?)) {
                        pixel.copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_formats::vxl::{Voxel, VxlLimb};
    const IDENTITY: Transform = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]];

    fn limb(name: &str) -> VxlLimb {
        VxlLimb {
            name: name.into(),
            scale: 1. / 12.,
            transform: IDENTITY,
            min_bounds: [10., -2., 4.],
            max_bounds: [14., 2., 8.],
            size: [2, 2, 2],
            normals_mode: 4,
            voxels: vec![Voxel {
                x: 0,
                y: 0,
                z: 0,
                color: 42,
                normal: 0,
            }],
        }
    }

    fn part(limbs: Vec<VxlLimb>) -> Part {
        Part {
            vxl: VxlFile {
                palette: [0; 768],
                remap: (16, 31),
                limbs,
            },
            hva: None,
        }
    }

    fn close(a: [f32; 3], b: [f32; 3]) {
        for (a, b) in a.into_iter().zip(b) {
            assert!((a - b).abs() < 0.0001, "{a} != {b}");
        }
    }

    #[test]
    fn authored_bounds_keep_barrel_forward_and_turret_above_hull() {
        let hull = limb("body");
        let mut barrel = limb("barrel");
        barrel.min_bounds = [20., -2., 14.];
        barrel.max_bounds = [24., 2., 18.];
        let v = &hull.voxels[0];
        close(model_point(&hull, &IDENTITY, v).unwrap(), [11., -1., 5.]);
        close(model_point(&barrel, &IDENTITY, v).unwrap(), [21., -1., 15.]);
    }

    #[test]
    fn hva_matches_limb_names_scales_translation_and_replaces_base() {
        let mut a = limb("body");
        a.transform[0][3] = 100.;
        let b = limb("barrel");
        let mut p = part(vec![a.clone(), b.clone()]);
        let rotated = [[0., -1., 0., 24.], [1., 0., 0., -12.], [0., 0., 1., 36.]];
        p.hva = Some(yr_formats::HvaFile {
            name: String::new(),
            limbs: vec!["BARREL".into(), "BODY".into()],
            frames: vec![vec![rotated, IDENTITY]],
        });
        close(
            model_point(&b, &pose(&p, &b).unwrap(), &b.voxels[0]).unwrap(),
            [3., 10., 8.],
        );
        close(
            model_point(&a, &pose(&p, &a).unwrap(), &a.voxels[0]).unwrap(),
            [11., -1., 5.],
        );
        // An unmatched limb retains its base, rather than using another limb's HVA.
        a.name = "missing".into();
        assert_eq!(pose(&p, &a), Some(a.transform));
    }

    #[test]
    fn invalid_hva_falls_back_to_base_and_invalid_geometry_is_skipped() {
        let mut l = limb("body");
        let mut p = part(vec![l.clone()]);
        let mut m = IDENTITY;
        m[0][0] = f32::NAN;
        p.hva = Some(yr_formats::HvaFile {
            name: String::new(),
            limbs: vec!["body".into()],
            frames: vec![vec![m]],
        });
        assert_eq!(pose(&p, &l), Some(IDENTITY));
        l.transform[0][0] = f32::INFINITY;
        assert!(pose(&p, &l).is_none());
        for size in [[0, 2, 2], [2, 0, 2], [2, 2, 0]] {
            l.size = size;
            assert!(model_point(&l, &IDENTITY, &l.voxels[0]).is_none());
        }
        l = limb("body");
        l.max_bounds[0] = f32::NAN;
        assert!(model_point(&l, &IDENTITY, &l.voxels[0]).is_none());
        l = limb("body");
        l.max_bounds[1] = l.min_bounds[1];
        assert!(model_point(&l, &IDENTITY, &l.voxels[0]).is_none());
        l = limb("body");
        l.scale = -1.;
        assert!(model_point(&l, &IDENTITY, &l.voxels[0]).is_none());
    }

    #[test]
    fn fixed_camera_preserves_origin_size_and_wraps_extreme_facings() {
        for facing in [0, 32, 64, 128, 192, 255, u16::MAX] {
            close(project([0.; 3], facing), [0.; 3]);
            close(
                project([2., 4., 6.], facing),
                project([2., 4., 6.], facing % 256),
            );
            let a = project([2., 4., 6.], facing);
            close(project([4., 8., 12.], facing), a.map(|v| v * 2.));
        }
        // Facing 224 cancels the camera's +32 rotation.
        close(
            project([1., 1., 0.], 224),
            [0., std::f32::consts::FRAC_1_SQRT_2, 1.5_f32.sqrt()],
        );
        close(project([0., 0., 2.], 224), [0., -3.0_f32.sqrt(), 1.]);
    }

    #[test]
    fn raster_depth_uses_camera_distance_and_hostile_offsets_clip() {
        let mut far = limb("far");
        far.min_bounds = [0.; 3];
        far.max_bounds = [1.; 3];
        far.size = [1; 3];
        let mut near = far.clone();
        near.name = "near".into();
        near.voxels[0].color = 43;
        // Move along the view ray. Same screen position, greater depth.
        near.transform[0][3] = 6.0_f32.sqrt();
        near.transform[1][3] = 6.0_f32.sqrt();
        near.transform[2][3] = 2.;
        let mut pal = Palette::parse(&[0; 768]).unwrap();
        pal.entries[42] = (63, 0, 0);
        pal.entries[43] = (0, 63, 0);
        let mut model = VoxelModel {
            parts: vec![part(vec![near, far])],
        };
        let mut rgba = vec![0; 32 * 32 * 4];
        draw_voxel(&mut rgba, 32, 32, 16, 16, &model, 224, &pal, None);
        assert!(rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p == &[0, 255, 0, 255]));
        assert!(!rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p == &[255, 0, 0, 255]));
        for l in &mut model.parts[0].vxl.limbs {
            l.transform[0][3] = f32::MAX;
        }
        rgba.fill(0);
        draw_voxel(
            &mut rgba,
            32,
            32,
            i32::MAX,
            i32::MIN,
            &model,
            u16::MAX,
            &pal,
            None,
        );
        assert!(rgba.iter().all(|v| *v == 0));
        assert!(sprite(&model, 0, &pal, [255; 3]).is_none());
        for l in &mut model.parts[0].vxl.limbs {
            l.transform[0][3] = 1024.;
        }
        assert!(sprite(&model, 0, &pal, [255; 3]).is_none());
        draw_voxel(&mut [], 32, 32, 16, 16, &model, 0, &pal, None);
    }

    #[test]
    fn retail_poses_preserve_grizzly_attachments_and_robo_source_units() {
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            eprintln!("skip: retail install not present");
            return;
        }
        let fs = AssetFs::open(&dir).unwrap();
        let model = VoxelModel::load(&fs, "MTNK").unwrap();
        assert_eq!(model.parts.len(), 3);
        let limits: Vec<_> = model
            .parts
            .iter()
            .map(|p| {
                let l = &p.vxl.limbs[0];
                assert_eq!(pose(p, l), Some(IDENTITY));
                let points: Vec<_> = l
                    .voxels
                    .iter()
                    .map(|v| model_point(l, &IDENTITY, v).unwrap())
                    .collect();
                std::array::from_fn::<_, 3, _>(|i| {
                    points.iter().fold((f32::MAX, f32::MIN), |(min, max), p| {
                        (min.min(p[i]), max.max(p[i]))
                    })
                })
            })
            .collect();
        assert!(
            limits[2][0].0 > 7. && limits[2][0].1 > 33.,
            "barrel keeps forward offset: {limits:?}"
        );
        assert!(
            limits[1][2].1 > 27.8 && limits[0][2].1 < 14.,
            "turret keeps authored height: {limits:?}"
        );
        let robo = VoxelModel::load(&fs, "ROBO").unwrap();
        for l in &robo.parts[0].vxl.limbs {
            let m = pose(&robo.parts[0], l).unwrap();
            for (row, base) in m.iter().zip(l.transform) {
                assert!((row[3] - base[3]).abs() < 0.00001);
            }
        }
        let bare = AssetFs::from_files(vec![
            ("MTNK.VXL".into(), fs.read("MTNK.VXL").unwrap().to_vec()),
            ("MTNKTUR.VXL".into(), b"invalid optional part".to_vec()),
            ("MTNK.HVA".into(), vec![255; 24]),
        ]);
        let fallback = VoxelModel::load(&bare, "MTNK").unwrap();
        assert_eq!(fallback.parts.len(), 1);
        assert_eq!(
            pose(&fallback.parts[0], &fallback.parts[0].vxl.limbs[0]),
            Some(IDENTITY)
        );
        assert!(VoxelModel::load(&bare, "missing").is_none());
        let pal = Palette::parse(fs.read("UNITTEM.PAL").unwrap()).unwrap();
        let out = std::env::var("OPENYR_VOXEL_DUMP").ok();
        if let Some(out) = &out {
            std::fs::create_dir_all(out).unwrap();
        }
        for id in ["MTNK", "HTNK", "CMIN", "ROBO", "SHAD", "ZEP"] {
            let model = VoxelModel::load(&fs, id).unwrap();
            for facing in (0..256).step_by(32) {
                let s = sprite(&model, facing, &pal, [220, 30, 30]).expect("bounded retail sprite");
                let mut expected = vec![0; 256 * 256 * 4];
                for y in 0..s.h {
                    for x in 0..s.w {
                        let pixel = &s.rgba[((y * s.w + x) * 4) as usize..][..4];
                        if pixel[3] == 0 {
                            continue;
                        }
                        assert!(
                            x > 0 && y > 0 && x + 1 < s.w && y + 1 < s.h,
                            "{id} sprite clipped at {facing}"
                        );
                        let (dx, dy) = (x as i32 + 128 - s.ax, y as i32 + 128 - s.ay);
                        assert!((0..256).contains(&dx) && (0..256).contains(&dy));
                        let at = (dy as usize * 256 + dx as usize) * 4;
                        expected[at..at + 4].copy_from_slice(pixel);
                    }
                }
                let mut map = vec![0; 256 * 256 * 4];
                draw_voxel(
                    &mut map,
                    256,
                    256,
                    128,
                    128,
                    &model,
                    facing,
                    &pal,
                    Some([220, 30, 30]),
                );
                assert_eq!(
                    map, expected,
                    "{id} sprite and map share ground anchor, facing {facing}"
                );
                if let Some(out) = &out {
                    std::fs::write(
                        format!("{out}/{id}-{facing:03}.png"),
                        crate::png::encode_rgba(256, 256, &map),
                    )
                    .unwrap();
                }
            }
        }
        let shad = VoxelModel::load(&fs, "SHAD").unwrap();
        assert_eq!(shad.parts[0].vxl.limbs.len(), 3);
        let p = &shad.parts[0];
        let rotor = &p.vxl.limbs[0];
        let m = pose(p, rotor).unwrap();
        assert!(m[0][3] < -35. && m[2][3] > 28.);
        println!("retail MTNK body/turret/barrel limits {limits:?}; ROBO source translation matches VXL base; SHAD rotor offset {:?}",m.map(|r|r[3]));
    }
}
