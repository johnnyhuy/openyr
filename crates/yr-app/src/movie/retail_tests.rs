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
    let mut rgb_max = 0u8;
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
            for (&a, b) in rgba.iter().zip(expected) {
                let error = a.abs_diff(b);
                rgb_max = rgb_max.max(error);
                rgb_error += u64::from(error);
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
        if let Some(path) = &proof {
            let all = std::env::var_os("OPENYR_FRONT_VIDEO").is_some();
            if all || [0, 1, 15, 60, 120, 240, 300, 430].contains(&decoded) {
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
                if all {
                    let folder = path.join("movie-sequence");
                    std::fs::create_dir_all(&folder).unwrap();
                    std::fs::write(
                        folder.join(format!("frame-{decoded:05}.png")),
                        yr_render::png::encode_rgba(800, 600, &composed),
                    )
                    .unwrap();
                }
                if [0, 1, 15, 60, 120, 240, 300, 430].contains(&decoded) {
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
    if let Some(oracle) = &mut oracle {
        assert_eq!(oracle.read(&mut [0]).unwrap(), 0);
    }
    if let Some(oracle) = &mut rgb_oracle {
        assert_eq!(oracle.read(&mut [0]).unwrap(), 0);
        assert!(rgb_max <= 3, "RGB error {rgb_max}");
    }
    println!("Bink decoded {decoded} frames; YUV FNV {hash:016x}; independent YUV={} RGB={} RGB max={} absolute error sum={rgb_error}",oracle.is_some(),rgb_oracle.is_some(),rgb_max);
    if let Some(path) = &proof {
        std::fs::write(path.join("decoder-check.txt"),format!("frames={decoded}\nYUV_FNV={hash:016x}\nindependent_YUV={}\nindependent_RGB={}\nRGB_max_error={rgb_max}\nRGB_absolute_error_sum={rgb_error}\n",oracle.is_some(),rgb_oracle.is_some())).unwrap();
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
