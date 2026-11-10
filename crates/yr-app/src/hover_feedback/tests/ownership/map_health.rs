use super::*;

#[test]
fn map_health_setup_roundtrip_animation_and_native_feedback() {
    for (health, hp, damaged, color) in [
        (256, 100, false, [0, 252, 0, 255]),
        (128, 50, true, [252, 252, 0, 255]),
        (1, 1, true, [252, 0, 0, 255]),
        (0, 1, true, [252, 0, 0, 255]),
        (u16::MAX, 100, false, [0, 252, 0, 255]),
    ] {
        let structures = format!("0=Neutral,GAPOWR,{health},20,20,0\n");
        let session = session_with_map_structures(&structures);
        let b = &session.sim.buildings()[0];
        assert_eq!(
            (b.id, b.hp, b.max_hp, b.owner),
            (1, hp, 100, yr_sim::OWNER_NEUTRAL)
        );
        assert_eq!(
            yr_app::animation::building_animation(&session.sim, b).damaged,
            damaged
        );
        let saved = yr_app::snapshot::encode(&session).unwrap();
        let mut restored = session_with_map_structures(&structures);
        yr_app::snapshot::restore(&mut restored, &saved).unwrap();
        assert_eq!(yr_app::snapshot::encode(&restored).unwrap(), saved);
        assert_eq!(restored.sim.state_hash(), session.sim.state_hash());
        println!("Authored map health={health}: current={hp}, maximum=100, damaged={damaged}; setup/save roundtrip passed.");

        // An armed owned unit exposes the ordinary Attack hover health preview
        // on this neutral map structure through the native world resolver.
        for size in [(640, 480), (1280, 800)] {
            let (mut app, _, _) = scene(size);
            app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                .into_iter()
                .cycle()
                .take(1600 * 1200 * 4)
                .collect();
            app.session = Some(session_with_map_structures(&structures));
            let sim = &mut app.session.as_mut().unwrap().sim;
            let own = sim.next_id_peek();
            sim.step(&[
                Command::Spawn {
                    kind: 0,
                    x: Fix::from_int(2),
                    y: Fix::from_int(2),
                    loco: yr_sim::terrain::Loco::Track,
                    speed: Fix::ONE,
                    hp: 100,
                    armor: 0,
                    weapon: Some(yr_sim::WeaponSpec {
                        source: None,
                        damage: 10,
                        rof_ticks: 10,
                        range: Fix::ONE,
                        projectile_speed: Fix::ZERO,
                        verses: [yr_rules::Decimal::ONE; 11],
                    }),
                },
                Command::AssignOwner {
                    id: own,
                    owner: yr_sim::OWNER_PLAYER,
                },
            ]);
            app.selected = [own].into();
            point_at(&mut app, 1, size);
            assert_eq!(app.cursor_kind(size), Kind::Attack);
            assert_eq!(app.hover_target(size).unwrap().id, 1);
            let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
            app.compose(&mut frame, size.0, size.1);
            let map = app.map.as_ref().unwrap();
            let (mx, my) = yr_app::interaction::ground_point(
                map,
                app.session.as_ref().unwrap().sim.terrain(),
                (20.5, 20.5),
            );
            let center = (
                (mx - app.view.cam_x) * app.view.zoom,
                (my - app.view.cam_y) * app.view.zoom,
            );
            let row_y = (center.1 - 24.0).round() as u32 + 1;
            let first_x = center.0.round() as u32 - 30;
            let light_pips = (first_x..first_x + 61)
                .filter(|x| {
                    let at = ((row_y * size.0 + x) * 4) as usize;
                    frame[at..at + 4] == color
                })
                .count();
            let expected_pips = match hp {
                100 => 40,
                50 => 20,
                _ => 2,
            };
            assert_eq!(light_pips, expected_pips, "health pip row for {health}");
            capture(&app, &format!("map-health-{health}-{}", size.0));
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                before
            );
            assert!(app.pending.is_empty());
        }
    }
    println!("Two native layouts: full/half/low/zero/overfull map health selects green/yellow/red health pips without mutating Session bytes or queuing orders. Authored fixture only, no retail art or window capture.");
}
