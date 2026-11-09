use super::*;

#[test]
fn selection_only_cursor_never_promises_a_world_order() {
    let mut sim = scene();
    let mut grid = yr_sim::TerrainGrid::from_tiles(
        0,
        0,
        8,
        6,
        (0..6).flat_map(|y| (0..8).map(move |x| (x, y, 0, 13, 0))),
    );
    grid.set_resource(5, 2, yr_sim::terrain::Overlay::Ore, 40);
    sim.set_terrain(grid);
    sim.step(&[Command::AssignRole {
        id: 2,
        role: Role::Harvester,
    }]);
    let saved = sim.save_bytes().unwrap();
    let hash = sim.state_hash();
    let mut receipts = Vec::new();
    for (selected, cell, attack_move, expected) in [
        ([1].into(), (3, 2), false, Kind::Move),
        ([1].into(), (6, 2), false, Kind::Attack),
        ([2].into(), (5, 2), false, Kind::Harvest),
        ([1].into(), (3, 2), true, Kind::AttackMove),
    ] {
        let ordinary = orders(&sim, &selected, cell, attack_move, false);
        assert_eq!(ordinary.cursor, expected);
        assert!(!ordinary.commands.is_empty());
        let selection = orders(&sim, &selected, cell, attack_move, true);
        assert_eq!(
            selection.cursor,
            Kind::Pointer,
            "selection-only click cannot advertise {expected:?}"
        );
        assert!(selection.commands.is_empty() && selection.select.is_none());
        receipts.push(serde_json::json!({"cell":cell,"selected":selected,"attack_move":attack_move,
            "ordinary_cursor":format!("{:?}",ordinary.cursor),"ordinary_commands":format!("{:?}",ordinary.commands),
            "selection_cursor":format!("{:?}",selection.cursor),"selection_commands":format!("{:?}",selection.commands)}));
    }
    for selected in [
        BTreeSet::new(),
        [1].into(),
        [2].into(),
        [1, 2].into(),
        [3, 99].into(),
    ] {
        for attack_move in [false, true] {
            for cell in [(1, 2), (2, 2), (3, 2), (5, 2), (6, 2), (-1, 0), (8, 0)] {
                let action = orders(&sim, &selected, cell, attack_move, true);
                let expected = match cell {
                    (1, 2) => Some(1),
                    (2, 2) => Some(2),
                    _ => None,
                };
                assert_eq!(action.select, expected);
                assert_eq!(
                    action.cursor,
                    if expected.is_some() {
                        Kind::Select
                    } else {
                        Kind::Pointer
                    }
                );
                assert!(action.commands.is_empty());
            }
        }
    }
    assert_eq!(sim.save_bytes().unwrap(), saved);
    assert_eq!(sim.state_hash(), hash);
    println!("Selection-only resolver: Move/Attack/Harvest/AttackMove suppressed; 70 selection/target/mode cases; complete Sim save bytes and hash unchanged ({hash:016x}).");
    if let Some(dir) = std::env::var_os("OPENYR_SELECTION_CURSOR_PROOF") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("resolver.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "actions":receipts,"matrix_cases":70,"hash":format!("{hash:016x}"),"sim_save_bytes":saved.len(),
            "sim_save_unchanged":true,"session_present":false,"retail_art":false,
        })).unwrap()).unwrap();
    }
}
