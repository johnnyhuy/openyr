use super::*;
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn shp(body: usize) -> Vec<u8> {
    let frames = body * 2;
    let mut bytes = Vec::new();
    for v in [0u16, 9, 9, frames as u16] {
        bytes.extend(v.to_le_bytes());
    }
    for i in 0..frames {
        for v in [0u16, 0, 9, 9] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(1u32.to_le_bytes());
        bytes.extend([0; 8]);
        bytes.extend(((8 + 24 * frames + 81 * i) as u32).to_le_bytes());
    }
    let tips = [
        (4, 1),
        (1, 1),
        (1, 4),
        (1, 7),
        (4, 7),
        (7, 7),
        (7, 4),
        (7, 1),
    ];
    for i in 0..frames {
        let mut canvas = [0u8; 81];
        let d = i % 8;
        if i < body {
            let index = if body == 16 && i < 8 {
                48
            } else {
                40 + d as u8
            };
            canvas[4 * 9 + 4] = index;
            canvas[tips[d].1 * 9 + tips[d].0] = index;
        } else {
            canvas[8 * 9 + d] = 1;
        }
        bytes.extend(canvas);
    }
    assert!(ShpFile::parse(&bytes).is_ok());
    bytes
}
fn vxl() -> Vec<u8> {
    // Authored asymmetric 16x3x3 body with a differently colored nose.
    let columns = 48usize;
    let mut spans = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for i in 0..columns {
        starts.extend((spans.len() as i32).to_le_bytes());
        spans.extend([0, 3]);
        for _ in 0..3 {
            spans.extend([if i % 16 < 12 { 40 } else { 48 }, 0]);
        }
        spans.push(3);
        ends.extend(((spans.len() - 1) as i32).to_le_bytes());
    }
    let mut body = starts;
    body.extend(ends);
    body.extend(spans);
    let mut bytes = b"Voxel Animation\0".to_vec();
    for v in [1u32, 1, 1, body.len() as u32] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend([16, 31]);
    bytes.extend([0; 768]);
    let mut name = [0u8; 16];
    name[..4].copy_from_slice(b"body");
    bytes.extend(name);
    bytes.extend([0; 12]);
    bytes.extend(body);
    for v in [0u32, (columns * 4) as u32, (columns * 8) as u32] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend(1f32.to_le_bytes());
    for v in [
        1f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -8., -1.5, 0., 8., 1.5, 3.,
    ] {
        bytes.extend(v.to_le_bytes());
    }
    bytes.extend([16, 3, 3, 4]);
    assert!(yr_formats::VxlFile::parse(&bytes).is_ok());
    bytes
}
fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "openyr-facing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    let mut pal = [0u8; 768];
    pal[3..6].copy_from_slice(&[8, 10, 8]);
    for i in 40..=48 {
        pal[i * 3..i * 3 + 3].copy_from_slice(&[
            10 + (i - 40) as u8 * 5,
            40 - (i - 40) as u8 * 3,
            20,
        ]);
    }
    fs::write(path.join("ISOTEM.PAL"), pal).unwrap();
    fs::write(path.join("UNITTEM.PAL"), pal).unwrap();
    fs::write(path.join("RULESMD.INI"),"[VehicleTypes]\n0=VEH\n[InfantryTypes]\n0=E1\n[AircraftTypes]\n0=ORCA\n[VEH]\nStrength=100\n[E1]\nStrength=100\n[ORCA]\nStrength=100\n").unwrap();
    fs::write(
        path.join("ARTMD.INI"),
        "[VEH]\nVoxel=yes\n[E1]\nSequence=READY\n[READY]\nReady=8,1,1\n[ORCA]\n",
    )
    .unwrap();
    fs::write(path.join("E1.SHP"), shp(16)).unwrap();
    fs::write(path.join("ORCA.SHP"), shp(8)).unwrap();
    fs::write(path.join("VEH.VXL"), vxl()).unwrap();
    fs::write(
        path.join("TEMPERATMD.INI"),
        "[TileSet0000]\nFileName=Clear\nTilesInSet=1\n",
    )
    .unwrap();
    let mut tmp = Vec::new();
    for v in [1u32, 1, 60, 30, 20] {
        tmp.extend(v.to_le_bytes());
    }
    tmp.extend([0; 52]);
    tmp[61] = 13;
    tmp.extend([1; 900]);
    fs::write(path.join("CLEAR01.TEM"), tmp).unwrap();
    Fixture(path)
}
fn map(objects: &str) -> String {
    format!("[Map]\nTheater=TEMPERATE\nSize=0,0,32,32\nLocalSize=0,0,32,32\n[IsoMapPack5]\n1=EwAPACAUABQAAAAAAAAAAAAAAAARAAA=\n{objects}")
}

#[test]
fn map_unit_facing_static_and_live_share_directional_shp_and_voxel_art() {
    let dir = fixture();
    let root = dir.0.to_str().unwrap();
    fs::write(dir.0.join("EMPTY.MAP"), map("")).unwrap();
    let base = render_map(root, "EMPTY.MAP", 1).unwrap();
    let proof = std::env::var_os("OPENYR_MAP_FACING_PROOF").map(PathBuf::from);
    if let Some(p) = &proof {
        fs::create_dir_all(p).unwrap();
    }
    let mut cases = Vec::new();
    for (section, kind, prefix) in [
        ("Units", "VEH", ""),
        ("Infantry", "E1", "0,Guard,"),
        ("Aircraft", "ORCA", ""),
    ] {
        let mut images = Vec::new();
        for raw in [
            "0", "32", "64", "96", "128", "160", "192", "224", "255", "256", "65535", "bad", "",
        ] {
            let text = map(&format!(
                "[{section}]\n0=Neutral,{kind},256,20,20,{prefix}{raw}\n"
            ));
            fs::write(dir.0.join("FACE.MAP"), &text).unwrap();
            let parsed = MapFile::parse(&text).unwrap();
            let obj = parsed
                .units
                .iter()
                .chain(&parsed.infantry)
                .chain(&parsed.aircraft)
                .next()
                .unwrap();
            let expected = match raw {
                "256" | "65535" => 255,
                "bad" | "" => 0,
                _ => raw.parse().unwrap(),
            };
            assert_eq!(obj.initial_facing(), expected);
            let rendered = render_map(root, "FACE.MAP", 1).unwrap();
            assert_eq!(
                (
                    rendered.width,
                    rendered.height,
                    rendered.origin_x,
                    rendered.origin_y
                ),
                (base.width, base.height, base.origin_x, base.origin_y)
            );
            let mut bank = sprites::SpriteBank::open(root, "FACE.MAP").unwrap();
            let remap = bank.house_color("Neutral");
            let sprite = bank.get(kind, expected, remap).unwrap();
            let (cx, cy) = rendered.cell_to_px(20.5, 20.5);
            let mut opaque = 0;
            let mut shadows = 0;
            for (i, p) in sprite.rgba.as_chunks::<4>().0.iter().enumerate() {
                if p[3] != 255 && p[3] != sprites::SHADOW_ALPHA {
                    continue;
                }
                let x = cx as i32 - sprite.ax + (i % sprite.w as usize) as i32;
                let y = cy as i32 - sprite.ay + (i / sprite.w as usize) as i32;
                assert!(
                    x >= 0 && y >= 0 && x < rendered.width as i32 && y < rendered.height as i32
                );
                let at = ((y as u32 * rendered.width + x as u32) * 4) as usize;
                if p[3] == 255 {
                    assert_eq!(&rendered.rgba[at..at + 4], p, "{kind} {raw}");
                    opaque += 1;
                } else {
                    for c in 0..3 {
                        assert_eq!(
                            rendered.rgba[at + c],
                            base.rgba[at + c] / 2,
                            "shadow {kind} {raw}"
                        );
                    }
                    shadows += 1;
                }
            }
            assert!(opaque > 0);
            if kind != "VEH" {
                assert!(shadows > 0);
            }
            images.push(sprite.rgba.clone());
            if let Some(out) = &proof {
                if ["0", "64", "128", "192"].contains(&raw) {
                    fs::write(
                        out.join(format!("{kind}-{raw}-map.png")),
                        png::encode_rgba(rendered.width, rendered.height, &rendered.rgba),
                    )
                    .unwrap();
                    fs::write(
                        out.join(format!("{kind}-{raw}-live.png")),
                        png::encode_rgba(sprite.w, sprite.h, &sprite.rgba),
                    )
                    .unwrap();
                }
            }
            cases.push(format!(
                "{kind}:{raw:?}->{expected}:opaque={opaque}:shadow={shadows}"
            ));
        }
        assert_ne!(images[0], images[4], "opposite headings differ for {kind}");
    }
    if let Some(out) = proof {
        fs::write(out.join("cases.txt"), cases.join("\n")).unwrap();
    }
    println!("39 authored static/live facing cases: asymmetric VXL vehicle, directional Ready SHP infantry and SHP aircraft agree on opaque body anchors and source shadows; overflow/missing/malformed policies are shared. Original-game direction calibration and baked unit lifetime remain open.");
}

#[test]
fn map_unit_facing_invalid_ready_and_short_body_retain_source_shadow() {
    let dir = fixture();
    let root = dir.0.to_str().unwrap();
    fs::write(dir.0.join("EMPTY.MAP"), map("")).unwrap();
    let base = render_map(root, "EMPTY.MAP", 1).unwrap();
    for (section, kind, body, fields) in [
        ("Infantry", "E1", 8, "0,Guard,0"),
        ("Aircraft", "ORCA", 1, "0"),
    ] {
        fs::write(dir.0.join(format!("{kind}.SHP")), shp(body)).unwrap();
        fs::write(
            dir.0.join("FACE.MAP"),
            map(&format!(
                "[{section}]\n0=Neutral,{kind},256,20,20,{fields}\n"
            )),
        )
        .unwrap();
        let rendered = render_map(root, "FACE.MAP", 1).unwrap();
        let mut bank = sprites::SpriteBank::open(root, "FACE.MAP").unwrap();
        let remap = bank.house_color("Neutral");
        let sprite = bank.get(kind, 0, remap).unwrap();
        let (cx, cy) = rendered.cell_to_px(20.5, 20.5);
        let mut checked_shadow = false;
        for (i, p) in sprite.rgba.as_chunks::<4>().0.iter().enumerate() {
            let x = cx as i32 - sprite.ax + (i % sprite.w as usize) as i32;
            let y = cy as i32 - sprite.ay + (i / sprite.w as usize) as i32;
            let at = ((y as u32 * rendered.width + x as u32) * 4) as usize;
            if p[3] == 255 {
                assert_eq!(&rendered.rgba[at..at + 4], p);
            }
            if p[3] == sprites::SHADOW_ALPHA {
                assert!(base.rgba[at] > 0, "shadow must darken visible terrain");
                for c in 0..3 {
                    assert_eq!(rendered.rgba[at + c], base.rgba[at + c] / 2);
                }
                checked_shadow = true;
            }
        }
        assert!(checked_shadow);
    }
    println!("Invalid Ready and short SHP body select bounded fallback bodies and their corresponding shadows over nonblack terrain, never shadow frames as bodies.");
}
