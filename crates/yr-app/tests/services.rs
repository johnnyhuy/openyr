//! Retail controls and deterministic service jobs. No display or audio device.

use std::path::PathBuf;
use yr_app::{
    session,
    sidebar::{self, Category, ServiceMode},
    snapshot,
};
use yr_formats::{mix_id, MixArchive, Palette, ShpFile};
use yr_sim::{Command, Sim, OWNER_PLAYER};

#[test]
fn retail_controls_source_pixels_repair_stalls_sale_and_snapshot_resume() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut session = session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    session.sim.set_ai(None);
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let archive = MixArchive::parse(fs.read("SIDEC01.MIX").unwrap()).unwrap();
    let palette = Palette::parse(archive.get_by_id(mix_id("SIDEBAR.PAL")).unwrap()).unwrap();
    let (width, height) = (1280, 800);
    let mut frame = vec![0; (width * height * 4) as usize];
    let before = session.sim.state_hash();
    for mode in [ServiceMode::None, ServiceMode::Repair, ServiceMode::Sell] {
        sidebar::draw(
            &mut frame,
            width,
            height,
            &session,
            Category::Structures,
            None,
        );
        sidebar::draw_controls(&mut frame, width, height, &session, mode);
        for (name, button, x) in [
            ("REPAIR.SHP", ServiceMode::Repair, 20),
            ("SELL.SHP", ServiceMode::Sell, 84),
        ] {
            let shp = ShpFile::parse(archive.get_by_id(mix_id(name)).unwrap()).unwrap();
            assert_eq!((shp.width, shp.height, shp.frames.len()), (64, 31, 2));
            let source = shp
                .decode_frame_canvas(usize::from(mode == button))
                .unwrap();
            for (p, &index) in source.iter().enumerate() {
                if index == 0 {
                    let (px, py) = (
                        width - sidebar::WIDTH + x + p as u32 % 64,
                        166 + p as u32 / 64,
                    );
                    assert_eq!(
                        sidebar::control_at(
                            width,
                            &session,
                            (f64::from(px) + 0.5, f64::from(py) + 0.5)
                        ),
                        None
                    );
                    continue;
                }
                let (px, py) = (
                    width - sidebar::WIDTH + x + p as u32 % 64,
                    166 + p as u32 / 64,
                );
                let at = ((py * width + px) * 4) as usize;
                assert_eq!(
                    &frame[at..at + 4],
                    &palette.rgba(index),
                    "{name} {mode:?} pixel {p}"
                );
                assert_eq!(
                    sidebar::control_at(
                        width,
                        &session,
                        (f64::from(px) + 0.5, f64::from(py) + 0.5)
                    ),
                    Some(button)
                );
            }
        }
        assert_eq!(
            sidebar::control_at(
                width,
                &session,
                (f64::from(width - sidebar::WIDTH + 20), 165.0)
            ),
            None
        );
        if let Some(out) = std::env::var_os("OPENYR_SERVICES_PROOF") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                PathBuf::from(out).join(format!("controls-{mode:?}.png")),
                yr_render::png::encode_rgba(width, height, &frame),
            )
            .unwrap();
        }
    }
    assert_eq!(
        session.sim.state_hash(),
        before,
        "drawing never advances repair clocks"
    );
    let target = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == OWNER_PLAYER && session.kinds[b.kind as usize] == "GAPOWR")
        .unwrap()
        .clone();
    assert!(
        session.repair(target.id).is_none(),
        "full health cannot start a job"
    );
    let enemy = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        .unwrap();
    assert!(session.sell(enemy.id).is_none());
    // A controlled damaged snapshot isolates wrench behavior from combat.
    let bytes = session.sim.save_bytes().unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes[12..]).unwrap();
    for b in json["buildings"].as_array_mut().unwrap() {
        if b["id"] == target.id {
            b["hp"] = (target.max_hp / 2).into();
        }
    }
    let mut damaged = bytes[..12].to_vec();
    damaged.extend(serde_json::to_vec(&json).unwrap());
    session.sim = Sim::from_save_bytes(&damaged).unwrap();
    let idle: Vec<_> = session
        .sim
        .units()
        .iter()
        .map(|u| Command::Stop { id: u.id })
        .collect();
    session.step(&idle);
    let save = |s: &session::Session, name: &str| {
        if let Some(out) = std::env::var_os("OPENYR_SERVICES_PROOF") {
            snapshot::save(s, &PathBuf::from(out).join(format!("{name}.oyrsave"))).unwrap();
        }
    };
    save(&session, "damaged");
    session.sim.set_credits(0);
    let command = ServiceMode::Repair
        .command(
            &session,
            (i32::from(target.cell.0), i32::from(target.cell.1)),
        )
        .unwrap();
    let Command::Repair {
        spec: Some(spec), ..
    } = command
    else {
        panic!("repair recipe");
    };
    assert_eq!(
        (spec.percent.units(), spec.step, spec.interval),
        (150_000, 8, 15)
    );
    session.step(&[command]);
    let hp = |s: &session::Session| {
        s.sim
            .buildings()
            .iter()
            .find(|b| b.id == target.id)
            .unwrap()
            .hp
    };
    for _ in 0..45 {
        session.step(&[]);
    }
    assert_eq!(hp(&session), target.max_hp / 2);
    save(&session, "starved");
    session.sim.set_credits(1000);
    session.step(&[]);
    assert_eq!(hp(&session), target.max_hp / 2 + 8);
    save(&session, "repairing");
    let encoded = snapshot::encode(&session).unwrap();
    let mut resumed = session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    snapshot::restore(&mut resumed, &encoded).unwrap();
    let mut ticks = 0;
    while session.sim.repairing(target.id) {
        session.step(&[]);
        resumed.step(&[]);
        ticks += 1;
        assert_eq!(session.sim.state_hash(), resumed.sim.state_hash());
        assert!(ticks < 2000);
    }
    assert_eq!(hp(&session), target.max_hp);
    let expected = spec.cost * spec.percent.units() * (target.max_hp - target.max_hp / 2)
        / (target.max_hp * 1_000_000);
    assert_eq!(session.sim.credits(), 1000 - expected);
    save(&session, "repaired");
    let credits = session.sim.credits();
    let sell = ServiceMode::Sell
        .command(
            &session,
            (i32::from(target.cell.0), i32::from(target.cell.1)),
        )
        .unwrap();
    session.step(&[sell]);
    assert!(session.sim.buildings().iter().all(|b| b.id != target.id));
    assert_eq!(session.sim.credits(), credits + spec.cost / 2);
    assert!(session.sim.terrain().unwrap().passable(
        yr_sim::terrain::Loco::Track,
        target.cell.0,
        target.cell.1
    ));
    save(&session, "sold");
    println!(
        "GAPOWR {}: {}/{} HP, repair ${expected}, sale ${}, final tick {}, hash {:016x}",
        target.id,
        target.max_hp / 2,
        target.max_hp,
        spec.cost / 2,
        session.sim.tick(),
        session.sim.state_hash()
    );
}
