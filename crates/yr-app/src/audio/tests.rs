use super::*;
use crate::match_config::{Country, MatchConfig};
use crate::session::StartOptions;
use crate::snapshot;
use std::path::PathBuf;

struct CaptureDir(PathBuf, bool);
impl CaptureDir {
    fn new() -> Self {
        let retained = std::env::var_os("OPENYR_EVA_PROOF").map(PathBuf::from);
        let parent = retained.clone().unwrap_or_else(std::env::temp_dir);
        std::fs::create_dir_all(&parent).unwrap();
        for attempt in 0..1000 {
            let path = parent.join(format!("eva-{}-{attempt}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path, retained.is_some()),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("cannot create audio capture: {e}"),
            }
        }
        panic!("audio capture directory candidates exhausted");
    }
}
impl Drop for CaptureDir {
    fn drop(&mut self) {
        if !self.1 {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

#[test]
fn retail_eva_uses_player_side_for_fresh_and_disk_restored_matches() {
    let install = PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !install.is_dir() {
        eprintln!("retail EVA check skipped: game install absent");
        return;
    }
    let dir = CaptureDir::new();
    let gamedir = install.to_str().unwrap();
    let eva = bank::read_ini(&AssetFs::open(&install).unwrap(), "EVAMD.INI");
    for (country, opponent, side) in [
        (Country::America, Country::Russia, "Allied"),
        (Country::Russia, Country::America, "Russian"),
    ] {
        let mut config = MatchConfig::default();
        config.players[0].country = country;
        config.players[1].country = opponent;
        let fresh = crate::session::load_with_config(
            gamedir,
            "EB1.MMX",
            42,
            StartOptions::default(),
            config.clone(),
        )
        .unwrap();
        let save = dir.0.join(format!("{side}.oyrsave"));
        snapshot::save(&fresh, &save).unwrap();
        let bytes = std::fs::read(&save).unwrap();
        let restored = snapshot::load(gamedir, "EB1.MMX", &save).unwrap();
        assert_eq!(snapshot::encode(&restored).unwrap(), bytes);
        for (phase, mut session) in [("fresh", fresh), ("restored", restored)] {
            assert_eq!(session.match_config, config);
            let out = dir.0.join(format!("{side}-{phase}"));
            std::fs::create_dir(&out).unwrap();
            let before = snapshot::encode(&session).unwrap();
            let mut scene =
                AudioScene::capture(gamedir, "EB1.MMX", &session, out.to_str().unwrap()).unwrap();
            assert_eq!(snapshot::encode(&session).unwrap(), before);
            let slot = session
                .buildables
                .iter()
                .position(|b| b.id == country.side().force()[3])
                .unwrap();
            let command = session.produce(slot).unwrap();
            scene.commands(&session, std::slice::from_ref(&command));
            assert_eq!(snapshot::encode(&session).unwrap(), before);
            session.sim.drain_events();
            for tick in 0..180 {
                session.step(if tick == 0 {
                    std::slice::from_ref(&command)
                } else {
                    &[]
                });
                let events = session.sim.drain_events();
                let before = snapshot::encode(&session).unwrap();
                scene.update(&session, &events);
                assert_eq!(snapshot::encode(&session).unwrap(), before);
            }
            scene.finish().unwrap();
            let log = std::fs::read_to_string(out.join("audio-cues.tsv")).unwrap();
            assert!(log.contains("\tEVA_Training\t"), "{side}-{phase}: {log}");
            assert!(log.contains("\tEVA_UnitReady\t"), "{side}-{phase}: {log}");
            for line in log.lines().skip(1) {
                let fields: Vec<_> = line.split('\t').collect();
                if fields[1] == "Eva" {
                    assert_eq!(
                        Some(fields[3]),
                        eva.value(fields[2], side),
                        "{side}-{phase}: {line}"
                    );
                    println!("{side}-{phase}: {line}");
                }
            }
            let pcm = yr_formats::audio::decode_wav(&std::fs::read(out.join("audio.wav")).unwrap())
                .unwrap();
            assert_eq!((pcm.sample_rate, pcm.channels), (CAPTURE_RATE, 2));
            assert_eq!(pcm.samples.len(), CAPTURE_RATE as usize * 12 * 2);
            assert!(pcm.samples.iter().any(|s| *s != 0));
            assert_eq!(std::fs::read(&save).unwrap(), bytes);
            println!(
                "{}: 12-second shared mixer capture, save unchanged",
                out.display()
            );
        }
    }
}
