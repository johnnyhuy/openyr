use super::*;

fn shp(count: u16, bodies: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    for n in [0u16, 2, 2, count] {
        bytes.extend(n.to_le_bytes());
    }
    for frame in 0..count {
        for n in [0u16, 0, 2, 2] {
            bytes.extend(n.to_le_bytes());
        }
        for n in [
            1u32,
            0,
            0,
            if frame < bodies {
                8 + u32::from(count) * 24 + u32::from(frame) * 4
            } else {
                0
            },
        ] {
            bytes.extend(n.to_le_bytes());
        }
    }
    for frame in 0..bodies {
        bytes.extend([0, 1, 2, 3 + frame as u8]);
    }
    bytes
}

fn fixture(count: usize, image: &str) -> (AssetFs, Rules, IniFile, Palette) {
    let art = IniFile::parse(
        "[STATIC]\n[ROTATING]\nRotates=yes\n[ANIMATED]\nAnimPalette=yes\n[VOXEL]\nVoxel=yes\n",
    );
    let mut text = String::from("[VehicleTypes]\n");
    for n in 0..count {
        text.push_str(&format!("{n}=T{n}\n"));
    }
    for n in 0..count {
        text.push_str(&format!("[T{n}]\nPrimary=G{n}\n[G{n}]\nProjectile=P\n"));
    }
    text.push_str(&format!("[P]\nImage={image}\n"));
    let rules = Rules::load(&IniFile::parse(&text), &art);
    let palette: Vec<_> = (0..768).map(|n| (n % 64) as u8).collect();
    let fs = AssetFs::from_files(vec![
        ("STATIC.SHP".into(), shp(1, 1)),
        ("ROTATING.SHP".into(), shp(64, 32)),
        ("ANIMATED.SHP".into(), shp(6, 3)),
        ("VOXEL.VXL".into(), voxel()),
        ("VOXEL.HVA".into(), hva()),
        ("ANIM.PAL".into(), palette.clone()),
    ]);
    (fs, rules, art, Palette::parse(&palette).unwrap())
}

fn voxel() -> Vec<u8> {
    // One column containing one opaque voxel, with identity limb transform.
    let mut bytes = b"Voxel Animation\0".to_vec();
    for n in [1u32, 1, 1, 13] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend([16, 31]);
    bytes.extend([0; 768]);
    bytes.extend(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
    bytes.extend([0; 12]);
    for n in [0u32, 4] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend([0, 1, 42, 0, 1]);
    for n in [0u32, 4, 8] {
        bytes.extend(n.to_le_bytes());
    }
    for n in [
        1f32, 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -2., -2., -2., 2., 2., 2.,
    ] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend([1, 1, 1, 4]);
    bytes
}

fn hva() -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes.extend(2u32.to_le_bytes());
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
    for shift in [0f32, 3.] {
        for n in [1f32, 0., 0., shift, 0., 1., 0., 0., 0., 0., 1., 0.] {
            bytes.extend(n.to_le_bytes());
        }
    }
    bytes
}

fn bounds(cache: &Projectiles) {
    assert!(cache.bodies.len() <= MAX_SOURCES);
    assert_eq!(cache.body_order.len(), cache.bodies.len());
    assert!(cache
        .voxel_models
        .keys()
        .all(|source| cache.bodies.contains_key(source)));
    assert!(cache.frames.len() <= MAX_FRAMES);
    assert_eq!(cache.frames.len(), cache.order.len());
    assert!(cache.voxel_frames.len() <= MAX_FRAMES);
    assert_eq!(cache.voxel_frames.len(), cache.voxel_order.len());
}

fn signature(sprite: &Sprite) -> (u32, u32, i32, i32, Vec<u8>) {
    (
        sprite.w,
        sprite.h,
        sprite.ax,
        sprite.ay,
        sprite.rgba.clone(),
    )
}

#[test]
fn missing_source_history_does_not_disable_later_projectile_bodies() {
    for image in ["STATIC", "ROTATING", "ANIMATED", "VOXEL"] {
        let (fs, rules, art, pal) = fixture(1, image);
        let source = rules.weapons.get("G0").unwrap().source_id();
        let mut cache = Projectiles::default();
        let mut fresh = Projectiles::default();
        let expected = signature(
            fresh
                .get_at(&fs, &rules, &art, &pal, source, Some((1, 1)), 2)
                .unwrap(),
        );
        for round in 0..3 {
            for n in 0..MAX_SOURCES {
                let missing = (round * MAX_SOURCES + n) as u64;
                assert_ne!(missing, source);
                assert!(cache.get(&fs, &rules, &art, &pal, missing, None).is_none());
            }
            let sprite = cache
                .get_at(&fs, &rules, &art, &pal, source, Some((1, 1)), 2)
                .expect("later valid bodies must survive missing-source churn");
            assert_eq!(signature(sprite), expected, "{image}, round {round}");
            bounds(&cache);
        }
    }
}

#[test]
fn positive_source_churn_reconstructs_evicted_body_pixels_and_clocks() {
    for image in ["STATIC", "ROTATING", "ANIMATED", "VOXEL"] {
        let (fs, rules, art, pal) = fixture(150, image);
        let mut cache = Projectiles::default();
        for n in 0..150 {
            let source = rules.weapons.get(&format!("G{n}")).unwrap().source_id();
            assert!(
                cache
                    .get_at(&fs, &rules, &art, &pal, source, Some((-1, 1)), 5)
                    .is_some(),
                "{image} source {n}"
            );
            bounds(&cache);
        }
        let source = rules.weapons.get("G0").unwrap().source_id();
        assert!(
            !cache.bodies.contains_key(&source),
            "old metadata must be evicted"
        );
        assert!(!cache.frames.keys().any(|key| key.0 == source));
        assert!(!cache.voxel_models.contains_key(&source));
        assert!(!cache.voxel_frames.keys().any(|key| key.0 == source));
        let mut fresh = Projectiles::default();
        for (motion, tick) in [
            (Some((-1, -1)), 0),
            (Some((1, -1)), 8),
            (Some((1, 1)), u64::MAX),
        ] {
            assert_eq!(
                signature(
                    cache
                        .get_at(&fs, &rules, &art, &pal, source, motion, tick)
                        .unwrap()
                ),
                signature(
                    fresh
                        .get_at(&fs, &rules, &art, &pal, source, motion, tick)
                        .unwrap()
                ),
                "{image} motion {motion:?} tick {tick}"
            );
        }
        let order = cache.body_order.clone();
        for _ in 0..200 {
            assert!(cache
                .get_at(&fs, &rules, &art, &pal, source, Some((1, 1)), 2)
                .is_some());
        }
        assert_eq!(cache.body_order, order, "hits do not grow or reorder FIFO");
        bounds(&cache);
        println!("{image}: 150 valid sources and reconstruction preserve pixels, anchors, facing and clocks within 128 sources/64 frames");
    }
}
