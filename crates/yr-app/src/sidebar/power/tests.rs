use super::*;
use crate::sidebar::{self, text::Typeface, Category};
use std::path::{Path, PathBuf};
use yr_assets::AssetFs;
use yr_formats::{mix_id, MixArchive, Palette, ShpFile};
use yr_sim::{Command, Role, OWNER_ENEMY, OWNER_PLAYER};

#[test]
fn levels_handle_empty_deficit_balance_surplus_and_extreme_totals() {
    for height in [0, 227, 256] {
        assert!(Meter::new(height, (200, 100)).is_none());
    }
    for height in [257, 300, 800, 2160] {
        for power in [
            (0, 0),
            (0, 100),
            (200, 0),
            (200, 100),
            (200, 200),
            (200, 300),
            (i64::MAX, i64::MAX - 1),
            (i64::MIN, i64::MAX),
        ] {
            let meter = Meter::new(height, power).unwrap();
            assert!(0 <= meter.used && meter.used <= meter.reserve);
            assert!(meter.reserve <= meter.output && meter.output <= meter.rows);
            assert!((0..=meter.rows).contains(&meter.demand));
            assert!(meter.y(meter.rows - 1) >= TOP);
            assert!(meter.y(0) + PIP_HEIGHT < height as i32 - 26);
        }
    }
    let meter = Meter::new(800, (200, 100)).unwrap();
    assert_eq!((meter.output, meter.used, meter.reserve), (91, 46, 55));
    let frames: Vec<_> = (0..meter.output).map(|r| meter.pip(r)).collect();
    assert_eq!(frames.iter().filter(|&&p| p == 0).count(), 46);
    assert_eq!(frames.iter().filter(|&&p| p == 2).count(), 9);
    assert_eq!(frames.iter().filter(|&&p| p == 1).count(), 36);
    let meter = Meter::new(800, (200, 300)).unwrap();
    assert!(meter.shortage && meter.demand > meter.output);
    assert!((0..meter.output).all(|r| meter.pip(r) == 0));
    assert_eq!(tooltip((0, 100)), ["0 output", "100 used", "100 short"]);
    assert_eq!(tooltip((200, 100)), ["200 output", "100 used", "100 spare"]);
    assert_eq!(tooltip((0, 0)), ["0 output", "0 used", "0 spare"]);
}

#[test]
fn fallback_and_demand_marker_stay_in_the_channel_on_small_windows() {
    for width in [8, WIDTH, 1280] {
        for height in [256, 257, 300, 800] {
            for totals in [(200, 100), (0, 100), (0, 0)] {
                let mut frame = vec![0; (width * height * 4) as usize];
                let x = width.saturating_sub(WIDTH) as i32 + LEFT;
                let meter = Meter::new(height, totals);
                if let Some(meter) = &meter {
                    draw_meter(
                        &mut Canvas {
                            frame: &mut frame,
                            width,
                            height,
                            face: Typeface(None),
                        },
                        meter,
                        &[],
                    );
                }
                for (p, pixel) in frame.as_chunks::<4>().0.iter().enumerate() {
                    if pixel[3] == 0 {
                        continue;
                    }
                    let (px, py) = ((p as u32 % width) as i32, (p as u32 / width) as i32);
                    assert!((x..x + 17).contains(&px));
                    assert!((TOP..height as i32 - 26).contains(&py));
                    assert!(contains(width, height, (f64::from(px), f64::from(py))));
                }
                if let Some(meter) = meter.filter(|_| totals == (0, 100) && width >= WIDTH) {
                    let at =
                        ((meter.y(meter.demand - 1) as u32 * width + x as u32 + 13) * 4) as usize;
                    assert_eq!(&frame[at..at + 4], &RED, "zero-output deficit is visible");
                }
                for pos in [
                    (f64::from(x - 1), 228.0),
                    (f64::from(x + 17), 228.0),
                    (f64::from(x), 227.0),
                    (f64::from(x), f64::from(height.saturating_sub(26))),
                ] {
                    assert!(!contains(width, height, pos));
                }
            }
        }
    }
}

fn panel(session: &Session) -> Vec<u8> {
    let mut frame = vec![0; (WIDTH * 800 * 4) as usize];
    sidebar::draw(&mut frame, WIDTH, 800, session, Category::Structures, None);
    frame
}

fn verify(session: &Session, name: &str, dir: &Path, raw: &ShpFile<'_>, palette: &Palette) {
    let hash = session.sim.state_hash();
    let saved = crate::snapshot::encode(session).unwrap();
    let mut frame = panel(session);
    let meter = Meter::new(800, session.sim.power()).unwrap();
    let mut colors = [0; 3];
    for row in 0..meter.output {
        let phase = meter.pip(row);
        colors[phase] += 1;
        for (p, index) in raw
            .decode_frame_canvas(phase)
            .unwrap()
            .into_iter()
            .enumerate()
        {
            let at = (((meter.y(row) + p as i32 / 12) * WIDTH as i32 + LEFT + p as i32 % 12) * 4)
                as usize;
            assert_eq!(
                &frame[at..at + 4],
                &palette.rgba(index),
                "{name} row {row} pixel {p}"
            );
        }
    }
    let mut restored = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    crate::snapshot::restore(&mut restored, &saved).unwrap();
    let restored_frame = panel(&restored);
    assert_eq!(restored.sim.power(), session.sim.power());
    for y in TOP..774 {
        let at = ((y * WIDTH as i32 + LEFT) * 4) as usize;
        assert_eq!(
            &restored_frame[at..at + 17 * 4],
            &frame[at..at + 17 * 4],
            "meter reconstructs from saved totals at row {y}"
        );
    }
    if let Some(out) = std::env::var_os("OPENYR_POWER_DUMP") {
        std::fs::create_dir_all(&out).unwrap();
        let out = PathBuf::from(out);
        std::fs::write(
            out.join(format!("{name}.png")),
            yr_render::png::encode_rgba(WIDTH, 800, &frame),
        )
        .unwrap();
        crate::snapshot::save(session, &out.join(format!("{name}.oyrsave"))).unwrap();
        sidebar::draw_tooltip(
            &mut frame,
            WIDTH,
            800,
            session,
            Category::Structures,
            (8.0, 550.0),
        );
        std::fs::write(
            out.join(format!("{name}-tooltip.png")),
            yr_render::png::encode_rgba(WIDTH, 800, &frame),
        )
        .unwrap();
    }
    assert_eq!(session.sim.state_hash(), hash);
    assert_eq!(crate::snapshot::encode(session).unwrap(), saved);
    println!(
        "{name}: tick {}, power {:?}, red/green/yellow pips {colors:?}, hash {hash:016x}",
        session.sim.tick(),
        session.sim.power()
    );
}

fn build_structure(session: &mut Session, id: &str) -> u32 {
    let slot = session.buildables.iter().position(|b| b.id == id).unwrap();
    session.step(&[session.produce(slot).unwrap()]);
    let deadline = session.sim.tick() + 2000;
    while session.ready_structure() != Some(slot) {
        session.step(&[]);
        assert!(session.sim.tick() < deadline);
    }
    let yard = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == OWNER_PLAYER && b.role == Role::ConstructionYard)
        .unwrap()
        .cell;
    let spot = (0..16)
        .find_map(|r| {
            (-r..=r).find_map(|dy| {
                (-r..=r).find_map(|dx| {
                    let x = u16::try_from(i32::from(yard.0) + dx).ok()?;
                    let y = u16::try_from(i32::from(yard.1) + dy).ok()?;
                    session.can_place(slot, (x, y)).then_some((x, y))
                })
            })
        })
        .unwrap();
    session.step(&[session.place(slot, spot).unwrap()]);
    session
        .sim
        .buildings()
        .iter()
        .rev()
        .find(|b| b.owner == OWNER_PLAYER && usize::from(b.kind) == slot)
        .unwrap()
        .id
}

#[test]
fn retail_power_source_pixels_placement_destruction_ownership_and_save_load() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let fs = AssetFs::open(&dir).unwrap();
    let mix = MixArchive::parse(fs.read("SIDEC01.MIX").unwrap()).unwrap();
    let raw = ShpFile::parse(mix.get_by_id(mix_id("POWERP.SHP")).unwrap()).unwrap();
    let pal = Palette::parse(mix.get_by_id(mix_id("SIDEBAR.PAL")).unwrap()).unwrap();
    assert_eq!((raw.width, raw.height, raw.frames.len()), (12, 2, 5));
    let mut session = crate::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    session.sim.set_ai(None);
    assert_eq!(session.sidebar_art.power.len(), 5);
    for i in 0..5 {
        let source: Vec<_> = raw
            .decode_frame_canvas(i)
            .unwrap()
            .into_iter()
            .flat_map(|p| pal.rgba(p))
            .collect();
        assert_eq!(session.sidebar_art.power[i].pixels, source);
    }
    let initial = session.sim.power();
    assert_eq!(initial.0, 200);
    verify(&session, "initial", &dir, &raw, &pal);
    let enemy_power: Vec<_> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == OWNER_ENEMY && b.power != 0)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    session.step(&enemy_power);
    assert_eq!(
        session.sim.power(),
        initial,
        "enemy power stays outside the meter"
    );
    let added_plant = build_structure(&mut session, "GAPOWR");
    assert_eq!(session.sim.power(), (400, initial.1));
    verify(&session, "expanded", &dir, &raw, &pal);
    build_structure(&mut session, "AMRADR");
    build_structure(&mut session, "GATECH");
    assert_eq!(session.sim.power(), (400, 235));
    session.step(&[Command::Destroy { id: added_plant }]);
    assert_eq!(session.sim.power(), (200, 235));
    assert!(session.sim.low_power());
    verify(&session, "shortage", &dir, &raw, &pal);
    let plants: Vec<_> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == OWNER_PLAYER && b.power > 0)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    session.step(&plants);
    assert_eq!(session.sim.power(), (0, 235));
    assert!(session.sim.low_power());
    verify(&session, "no-output", &dir, &raw, &pal);
    build_structure(&mut session, "GAPOWR");
    build_structure(&mut session, "GAPOWR");
    assert_eq!(session.sim.power(), (400, 235));
    assert!(!session.sim.low_power());
    verify(&session, "recovered", &dir, &raw, &pal);
}
