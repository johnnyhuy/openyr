//! Retail audio resolution and event routing. No device is opened by tests.

use std::path::PathBuf;
use yr_app::audio::mixer::Bus;
use yr_app::audio::{AudioScene, SoundBank};
use yr_assets::AssetFs;
use yr_sim::{Command, Event, OWNER_PLAYER};

fn retail() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    dir.is_dir().then_some(dir)
}

#[test]
fn retail_theme_eva_voices_and_weapon_reports_decode() {
    let Some(dir) = retail() else { return };
    let mut bank = SoundBank::new(AssetFs::open(&dir).unwrap(), "EB1.MMX");
    let theme = bank.theme().expect("normal theme");
    assert_eq!(theme.sample, "BrainFre");
    assert_eq!(theme.playback.pcm.channels, 2);
    for (event, bus) in [
        ("GISelect", Bus::Response),
        ("GIMove", Bus::Response),
        ("GIAttackCommand", Bus::Response),
        ("GrizzlyTankAttack", Bus::Effect),
        ("RhinoTankAttack", Bus::Effect),
        ("EVA_UnitReady", Bus::Eva),
        ("EVA_Training", Bus::Eva),
        ("EVA_LowPower", Bus::Eva),
        ("EVA_Repairing", Bus::Eva),
        ("EVA_StructureSold", Bus::Eva),
        ("EVA_UnitLost", Bus::Eva),
        ("EVA_YouAreVictorious", Bus::Eva),
        ("EVA_YouHaveLost", Bus::Eva),
    ] {
        let cue = bank
            .cue(event, bus)
            .unwrap_or_else(|| panic!("missing {event}"));
        assert_eq!(cue.playback.pcm.sample_rate, 22050);
        assert!(cue.playback.pcm.samples.iter().any(|&s| s != 0), "{event}");
        println!(
            "{event} -> {} ({} samples)",
            cue.sample,
            cue.playback.pcm.samples.len()
        );
    }
    assert!(bank.sample("openyr_missing_sample").is_none());
}

#[test]
fn routing_records_selection_orders_combat_production_and_result_without_changing_sim() {
    let Some(dir) = retail() else { return };
    let gamedir = dir.to_str().unwrap();
    let out = std::env::temp_dir().join(format!("openyr-audio-test-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let mut session = yr_app::session::load(gamedir, "EB1.MMX", 42).unwrap();
    let mut audio =
        AudioScene::capture(gamedir, "EB1.MMX", &session, out.to_str().unwrap()).unwrap();
    let gi = session
        .sim
        .units()
        .iter()
        .find(|u| u.owner == OWNER_PLAYER && session.kinds[usize::from(u.kind)] == "E1")
        .unwrap()
        .id;
    let before = session.sim.state_hash();
    audio.select(&session, [gi]);
    audio.commands(
        &session,
        &[Command::Move {
            id: gi,
            x: yr_sim::Fix::ZERO,
            y: yr_sim::Fix::ZERO,
        }],
    );
    audio.update(
        &session,
        &[
            Event::Fired {
                shooter: gi,
                target: gi,
            },
            Event::Produced { id: gi, kind: 0 },
        ],
    );
    assert_eq!(session.sim.state_hash(), before);
    // Cache the GI's type, then remove it before routing the death event.
    session.step(&[Command::Destroy { id: gi }]);
    let events = session.sim.drain_events();
    audio.update(&session, &events);
    let kills: Vec<_> = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
        .map(|u| Command::Destroy { id: u.id })
        .chain(
            session
                .sim
                .buildings()
                .iter()
                .filter(|b| b.owner == yr_sim::OWNER_ENEMY)
                .map(|b| Command::Destroy { id: b.id }),
        )
        .collect();
    session.step(&kills);
    let events = session.sim.drain_events();
    audio.update(&session, &events);
    audio.finish().unwrap();
    let log = std::fs::read_to_string(out.join("audio-cues.tsv")).unwrap();
    for event in [
        "Theme",
        "GISelect",
        "GIMove",
        "GIAttack",
        "GIDie",
        "EVA_UnitReady",
        "EVA_UnitLost",
        "EVA_YouAreVictorious",
    ] {
        assert!(log.contains(event), "missing {event}: {log}");
    }
    let bytes = std::fs::read(out.join("audio.wav")).unwrap();
    let pcm = yr_formats::audio::decode_wav(&bytes).unwrap();
    assert_eq!(pcm.samples.len(), 44100 / 15 * 2 * 3);
    assert!(pcm.samples.iter().any(|&s| s != 0));
    std::fs::remove_dir_all(out).unwrap();
}
