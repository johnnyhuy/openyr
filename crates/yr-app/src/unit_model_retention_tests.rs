//! Retail native composition after both model and decoded-sprite eviction.
use super::*;
use std::fs;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    for n in 0..128 {
        let path =
            std::env::temp_dir().join(format!("openyr-retail-models-{}-{n}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Fixture(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("retail fixture: {e}"),
        }
    }
    panic!("retail fixture candidates occupied");
}
fn compose(app: &App, size: (u32, u32)) -> Vec<u8> {
    let mut frame = vec![0; size.0 as usize * size.1 as usize * 4];
    app.compose(&mut frame, size.0, size.1);
    frame
}
#[test]
fn retail_unit_model_eviction_preserves_native_frames_saves_and_input() {
    let dir = std::env::var_os("OPENYR_GAMEDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Games/RA2YR/prefix/drive_c/Westwood/RA2")
        });
    if !dir.is_dir() {
        return;
    }
    let data = dir.to_str().unwrap();
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let ini = |name| yr_formats::IniFile::parse(&String::from_utf8_lossy(fs.read(name).unwrap()));
    let rules = yr_rules::Rules::load(&ini("RULESMD.INI"), &ini("ARTMD.INI"));
    let images: Vec<_> = ["MTNK", "HTNK", "CMIN", "HARV", "SHAD", "ROBO"]
        .iter()
        .map(|id| rules.techno(id).unwrap().image.clone())
        .collect();
    let fixture = fixture();
    let map_path = fixture.0.join("CACHE.MAP");
    let mut map = String::from_utf8_lossy(fs.read("EB1.MMX").unwrap()).into_owned();
    map.push_str("\n[VehicleTypes]\n");
    for i in 0..140 {
        map.push_str(&format!("{}=OPENYRCACHE{i}\n", 10000 + i));
    }
    for i in 0..140 {
        map.push_str(&format!(
            "[OPENYRCACHE{i}]\nImage={}\nStrength=100\nSpeed=4\n",
            images[i % images.len()]
        ));
    }
    fs::write(&map_path, map).unwrap();
    let map_name = map_path.to_str().unwrap();
    let output = std::env::var_os("OPENYR_MODEL_RETENTION_PROOF").map(PathBuf::from);
    if let Some(out) = &output {
        fs::create_dir_all(out).unwrap();
    }
    for size in [(640, 480), (1280, 800)] {
        let mut app = new_app(data.into(), map_name.into(), 1);
        app.frame_size = size;
        app.map = Some(yr_render::render_map_background(data, map_name, 1).unwrap());
        app.sprites = load_sprites(data, map_name);
        assert!(app.sprites.is_some());
        app.session = Some(app.load_session().unwrap());
        let unit = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .find(|u| app.session.as_ref().unwrap().kinds[usize::from(u.kind)] == "MTNK")
            .unwrap();
        let (id, x, y) = (unit.id, unit.x, unit.y);
        app.center_world_at(
            (x.raw() as f32 / 256., y.raw() as f32 / 256.),
            (size.0 - SIDEBAR_W, size.1),
        );
        app.selected.insert(id);
        app.pending = vec![Command::Stop { id }];
        app.pointer.inside = false;
        for phase in 0..2 {
            if phase > 0 {
                for _ in 0..2 {
                    app.session.as_mut().unwrap().step(&[]);
                }
            }
            let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            let hash = app.session.as_ref().unwrap().sim.state_hash();
            let selected = app.selected.clone();
            let pending = app.pending.clone();
            let tick = app.session.as_ref().unwrap().sim.tick();
            let cold = compose(&app, size);
            {
                let mut bank = app.sprites.as_ref().unwrap().borrow_mut();
                for i in 0..140 {
                    let remap = if i % 2 == 0 {
                        [80, 160, 240]
                    } else {
                        [240, 80, 160]
                    };
                    assert!(bank
                        .get_animated_at(
                            &format!("OPENYRCACHE{i}"),
                            (i % 8 * 32) as u8,
                            remap,
                            Default::default(),
                            tick
                        )
                        .is_some());
                }
                // Cheap negative sprite records evict decoded frames independently
                // of the 140 real models that already exceeded the model limit.
                for i in 0..600 {
                    assert!(bank
                        .get(&format!("OPENYRMISSING{i}"), 0, [80, 160, 240])
                        .is_none());
                }
            }
            println!("Retail model churn complete: {size:?}, phase {phase}");
            let rebuilt = compose(&app, size);
            assert_eq!(
                cold, rebuilt,
                "complete native frame {size:?}, phase {phase}"
            );
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                before
            );
            yr_app::snapshot::restore(app.session.as_mut().unwrap(), &before).unwrap();
            assert_eq!(compose(&app, size), cold, "saved native reconstruction");
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                before
            );
            assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
            assert_eq!(app.selected, selected);
            assert_eq!(app.pending, pending);
            if let Some(out) = &output {
                for (name, rgba) in [("cold", &cold), ("rebuilt", &rebuilt)] {
                    fs::write(
                        out.join(format!("{}x{}-p{phase}-{name}.png", size.0, size.1)),
                        yr_render::png::encode_rgba(size.0, size.1, rgba),
                    )
                    .unwrap();
                }
                fs::write(
                    out.join(format!("{}x{}-p{phase}.json", size.0, size.1)),
                    serde_json::to_vec_pretty(&serde_json::json!({
                        "size":size,"phase":phase,"tick":tick,"hash":format!("{hash:016x}"),
                        "snapshot_bytes":before.len(),"aliases":140,"requests":740,"retail":true,
                        "selected":selected,"pending":format!("{pending:?}"),"frames_equal":true
                    }))
                    .unwrap(),
                )
                .unwrap();
            }
        }
    }
    println!("Four retail native cases: 140 real-model aliases and 740 requests per case exceed both production history counts; complete frames, Session bytes/hash, selection and pending orders match after eviction and save restore.");
}
