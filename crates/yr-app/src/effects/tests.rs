use super::*;
use std::path::PathBuf;
use yr_sim::{Command, OWNER_PLAYER};

#[test]
fn retail_effects_follow_shots_deaths_and_leave_sim_and_saves_unchanged() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let dir = dir.to_str().unwrap();
    let mut session = crate::session::load(dir, "EB1.MMX", 42).unwrap();
    session.sim.set_ai(None);
    let mut bank = SpriteBank::open(dir, "EB1.MMX").unwrap();
    let mut scene = EffectScene::default();
    let tank = session
        .sim
        .units()
        .iter()
        .find(|u| {
            u.owner == OWNER_PLAYER
                && u.weapon.is_some()
                && bank
                    .muzzle_effect(&session.kinds[usize::from(u.kind)], u.facing)
                    .as_deref()
                    == Some("GUNFIRE")
                && bank
                    .destruction_effect(&session.kinds[usize::from(u.kind)])
                    .is_some()
        })
        .unwrap()
        .clone();
    let kind = &session.kinds[usize::from(tank.kind)];
    assert_eq!(
        bank.muzzle_effect(kind, tank.facing).as_deref(),
        Some("GUNFIRE")
    );
    for facing in [0, 32, 64, 96, 128, 160, 192, 224] {
        let name = bank.muzzle_effect("E1", facing).unwrap();
        assert!(name.starts_with("MGUN-"));
        assert!(bank.get_effect(&name, 0).is_some());
    }
    scene.remember(&session);
    session.step(&[Command::Destroy { id: tank.id }]);
    let hash = session.sim.state_hash();
    let save = crate::snapshot::encode(&session).unwrap();
    scene.update(
        &session,
        &[
            Event::Fired {
                shooter: tank.id,
                target: 0,
            },
            Event::Died { id: tank.id },
        ],
        &mut bank,
    );
    assert_eq!(
        scene.active.len(),
        2,
        "a shooter which dies in the same tick retains both effects"
    );
    assert_eq!((scene.active[0].x, scene.active[0].y), (tank.x, tank.y));
    assert_eq!(scene.active[0].name, "GUNFIRE");
    assert_eq!(scene.active[1].name, "TWLT070");
    for _ in 0..10 {
        assert!(bank.get_effect(&scene.active[0].name, 0).is_some());
    }
    assert_eq!(session.sim.state_hash(), hash);
    assert_eq!(crate::snapshot::encode(&session).unwrap(), save);
    scene.remember(&session);
    let survivor = session
        .sim
        .units()
        .iter()
        .find(|u| {
            u.owner == OWNER_PLAYER
                && bank
                    .muzzle_effect(&session.kinds[usize::from(u.kind)], u.facing)
                    .is_some()
        })
        .unwrap()
        .id;
    let events = vec![
        Event::Fired {
            shooter: survivor,
            target: 0
        };
        300
    ];
    scene.update(&session, &events, &mut bank);
    assert_eq!(scene.active.len(), MAX_EFFECTS);
    for _ in 0..300 {
        session.step(&[]);
    }
    scene.update(&session, &[], &mut bank);
    assert!(
        scene.active.is_empty(),
        "one-shot art expires without any redraws"
    );
    let restored = EffectScene::default();
    assert!(
        restored.active.is_empty(),
        "transient effects are not replayed on load"
    );
}
