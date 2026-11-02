use super::*;
use std::io::Read;
#[test]
fn retail_video_planes_match_optional_independent_oracle_and_export_rgb_frames() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let bytes = fs.read("RA2TS_L.BIK").unwrap().to_vec();
    let shell = crate::frontend::Art::load(&fs);
    drop(fs);
    let meta = BinkFile::parse(&bytes).unwrap();
    let timing = validate(&bytes).unwrap();
    assert_eq!(
        (
            meta.width,
            meta.height,
            meta.frames.len(),
            meta.fps_num,
            meta.fps_den
        ),
        (632, 570, 431, 15, 1)
    );
    let mut decoder = BikStreamingDecoder::new(Cursor::new(&bytes), "RA2TS_L.BIK").unwrap();
    let mut oracle = std::env::var_os("OPENYR_BINK_ORACLE")
        .map(|p| std::io::BufReader::new(std::fs::File::open(p).unwrap()));
    let mut rgb_oracle = std::env::var_os("OPENYR_BINK_RGB_ORACLE")
        .map(|p| std::io::BufReader::new(std::fs::File::open(p).unwrap()));
    let mut hash = 0xcbf29ce484222325u64;
    let mut decoded = 0;
    let mut original_mismatches = Vec::new();
    let mut rgb_max = 0u8;
    let mut rgb_normal_max = 0u8;
    let mut rgb_below_black_pixels = 0u64;
    let mut rgb_error = 0u64;
    let proof = std::env::var_os("OPENYR_FRONT_PROOF").map(std::path::PathBuf::from);
    if let Some(path) = &proof {
        std::fs::create_dir_all(path).unwrap();
    }
    let mut sample_frame = None;
    while let Some(frame) = decoder.next_frame().unwrap() {
        let BikPixels::Yuv(planes) = frame.video.pixels else {
            panic!("expected native YUV");
        };
        let mut flat = Vec::new();
        for p in [&planes.y, &planes.u, &planes.v] {
            for y in 0..p.height as usize {
                flat.extend_from_slice(&p.data[y * p.stride..y * p.stride + p.width as usize]);
            }
        }
        for &value in &flat {
            hash = (hash ^ u64::from(value)).wrapping_mul(0x100000001b3);
        }
        if let Some(oracle) = &mut oracle {
            let mut expected = vec![0; flat.len()];
            oracle.read_exact(&mut expected).unwrap();
            assert_eq!(flat, expected, "YUV frame {decoded}");
        }
        let rgba = limited_rgba(&planes).unwrap();
        if let Some(oracle) = &mut rgb_oracle {
            let mut expected = vec![0; rgba.len()];
            oracle.read_exact(&mut expected).unwrap();
            let luma = planes
                .y
                .data
                .chunks(planes.y.stride)
                .take(meta.height as usize)
                .flat_map(|row| row[..meta.width as usize].iter());
            for (pixel, ((actual, reference), &y)) in rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(expected.as_chunks::<4>().0)
                .zip(luma)
                .enumerate()
            {
                // Separate term truncation has a four-level raw RGB bound.
                // Sub-black luma adds the observed clamp's BT.601 contribution.
                let lift = (u16::from(16u8.saturating_sub(y)) * 255).div_ceil(219);
                if y < 16 {
                    rgb_below_black_pixels += 1;
                }
                for channel in 0..3 {
                    let delta = i16::from(actual[channel]) - i16::from(reference[channel]);
                    assert!(
                        (-4..=4 + lift as i16).contains(&delta),
                        "RGB frame {decoded} pixel {pixel} channel {channel}, Y={y}, delta={delta}"
                    );
                    let error = actual[channel].abs_diff(reference[channel]);
                    rgb_max = rgb_max.max(error);
                    if y >= 16 {
                        rgb_normal_max = rgb_normal_max.max(error);
                    }
                    rgb_error += u64::from(error);
                }
                assert_eq!(actual[3], reference[3], "RGB alpha frame {decoded}");
            }
        }
        if [0, 1, 15, 60, 120, 240, 300, 430].contains(&decoded) {
            if let Some(path) = &proof {
                std::fs::write(
                    path.join(format!("movie-frame-{decoded:03}.png")),
                    yr_render::png::encode_rgba(meta.width, meta.height, &rgba),
                )
                .unwrap();
            }
        }
        let movie_frame = Frame {
            width: meta.width,
            height: meta.height,
            index: decoded as u32,
            sequence: decoded as u64,
            rgba,
        };
        // Original renderer fingerprints cover the visible 632x568 movie area.
        // The lower two source rows are covered by the retail status strip.
        let original_hash = match decoded {
            16 => Some(0x8668bbcc0ff167bdu64),
            18 => Some(0xd2940c751d352169u64),
            24 => Some(0x665e4fc57a05c895u64),
            59 => Some(0xe845ff6273b27b99u64),
            63 => Some(0xc2c42f42131644fdu64),
            76 => Some(0x40e6e5a5742308d9u64),
            128 => Some(0x0451fcd1abc7e3b5u64),
            138 => Some(0x2cc1ed7910a19b71u64),
            203 => Some(0x2e0fc73835e479f9u64),
            206 => Some(0x0a3d6c5fac68d625u64),
            210 => Some(0x3843928266e60255u64),
            221 => Some(0x55945e120cb56619u64),
            224 => Some(0x0ccd8879e0edad79u64),
            265 => Some(0xa279c7a71cd9b631u64),
            275 => Some(0xfc14551331ba22a5u64),
            277 => Some(0x78e30b3b1f008171u64),
            311 => Some(0xcec89b37cd7756e9u64),
            317 => Some(0x6092bbabe9865a11u64),
            319 => Some(0xb9bfd5bdad43b9a9u64),
            326 => Some(0xeb9a5778a57cbad1u64),
            337 => Some(0xa78f86236e9a6b0du64),
            345 => Some(0xba7e5c93a034e11du64),
            350 => Some(0xee558e72a2c41cc9u64),
            351 => Some(0x5fbc84c31b91a919u64),
            357 => Some(0xb6a3bdfc7f1f86cdu64),
            365 => Some(0x34a1b33de6baa2cdu64),
            366 => Some(0x0b52ee6c313addbdu64),
            370 => Some(0x9f5219a1eede7455u64),
            371 => Some(0xc2998403de658965u64),
            416 => Some(0xf0f4671065826391u64),
            417 => Some(0xae1a3a6eccad6d3du64),
            427 => Some(0x33ee710d3d92d295u64),
            _ => None,
        };
        let all = proof.is_some() && std::env::var_os("OPENYR_FRONT_VIDEO").is_some();
        let sampled = [0, 1, 15, 60, 120, 240, 300, 430].contains(&decoded);
        if original_hash.is_some() || (proof.is_some() && (all || sampled)) {
            let state = crate::menu::State {
                front: true,
                page: crate::menu::Page::Main,
                ..Default::default()
            };
            let mut composed = vec![0; 800 * 600 * 4];
            crate::frontend::draw(
                &mut composed,
                800,
                600,
                &shell,
                crate::frontend::Scene {
                    state: &state,
                    movie: Some(&movie_frame),
                    preview: None,
                    phase: decoded as u64,
                    cursor: (-1.0, -1.0),
                },
            );
            if let Some(expected) = original_hash {
                let actual = composed
                    .as_chunks::<{ 800 * 4 }>()
                    .0
                    .iter()
                    .take(568)
                    .flat_map(|row| row[..632 * 4].iter())
                    .fold(0xcbf29ce484222325u64, |h, &byte| {
                        (h ^ u64::from(byte)).wrapping_mul(0x100000001b3)
                    });
                if actual != expected {
                    original_mismatches.push((decoded, actual, expected));
                }
            }
            if let Some(path) = &proof {
                if all {
                    let folder = path.join("movie-sequence");
                    std::fs::create_dir_all(&folder).unwrap();
                    std::fs::write(
                        folder.join(format!("frame-{decoded:05}.png")),
                        yr_render::png::encode_rgba(800, 600, &composed),
                    )
                    .unwrap();
                }
                if sampled || original_hash.is_some() {
                    std::fs::write(
                        path.join(format!("native-movie-frame-{decoded:03}.png")),
                        yr_render::png::encode_rgba(800, 600, &composed),
                    )
                    .unwrap();
                }
            }
        }
        if decoded == 0 {
            sample_frame = Some(Frame {
                width: meta.width,
                height: meta.height,
                index: 0,
                sequence: 0,
                rgba: movie_frame.rgba,
            });
        }
        decoded += 1;
    }
    assert_eq!(decoded, meta.frames.len());
    assert!(
        original_mismatches.is_empty(),
        "original movie mismatches: {original_mismatches:?}"
    );
    if let Some(oracle) = &mut oracle {
        assert_eq!(oracle.read(&mut [0]).unwrap(), 0);
    }
    if let Some(oracle) = &mut rgb_oracle {
        assert_eq!(oracle.read(&mut [0]).unwrap(), 0);
        assert!(
            rgb_normal_max <= 4,
            "Normal-range RGB error {rgb_normal_max}"
        );
    }
    println!("Bink decoded {decoded} frames; YUV FNV {hash:016x}; independent YUV={} RGB={} RGB max={} normal-range max={rgb_normal_max} sub-black pixels={rgb_below_black_pixels} absolute error sum={rgb_error}",oracle.is_some(),rgb_oracle.is_some(),rgb_max);
    if let Some(path) = &proof {
        std::fs::write(path.join("decoder-check.txt"),format!("frames={decoded}\nYUV_FNV={hash:016x}\nindependent_YUV={}\nindependent_RGB={}\nRGB_max_error={rgb_max}\nRGB_normal_range_max_error={rgb_normal_max}\nRGB_below_black_pixels={rgb_below_black_pixels}\nRGB_absolute_error_sum={rgb_error}\n",oracle.is_some(),rgb_oracle.is_some())).unwrap();
    }
    if let Some(path) = &proof {
        let mut full = BikStreamingDecoder::new(Cursor::new(&bytes), "RA2TS_L.BIK")
            .unwrap()
            .with_output_format(infinitier_bik_decoder::BikOutputFormat::Rgba);
        if let BikPixels::Rgba(rgba) = full.next_frame().unwrap().unwrap().video.pixels {
            std::fs::write(
                path.join("full-range-frame-000.png"),
                yr_render::png::encode_rgba(meta.width, meta.height, &rgba),
            )
            .unwrap();
        }
    }
    let mut movie = Movie::new(bytes).unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while movie.current.as_ref().is_none_or(|f| f.sequence < 2) {
        movie.poll();
        assert!(movie.error.is_none(), "{:?}", movie.error);
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        movie.current.as_ref().unwrap().index,
        movie.current.as_ref().unwrap().sequence as u32 % timing.frames
    );
    let zero = sample_frame.unwrap();
    assert_eq!(zero.rgba.len(), 632 * 570 * 4);
}
#[test]
fn malformed_video_payload_reports_an_error_from_the_worker() {
    let mut bytes = b"BIKi".to_vec();
    for value in [48u32, 1, 4, 1, 8, 8, 15, 1, 0, 0, 53, 56] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend([0; 4]);
    let mut movie = Movie::new(bytes).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while movie.error.is_none() {
        movie.poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(movie.current.is_none());
}
