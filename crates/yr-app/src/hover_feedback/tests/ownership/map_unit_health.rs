use super::*;

#[test]
fn map_unit_health_applies_to_vehicle_infantry_and_aircraft_setup() {
    for (health, hp) in [(128, 50), (256, 100), (1, 1), (0, 1), (u16::MAX, 100)] {
        let objects = format!("[Units]\n0=Neutral,MTNK,{health},20,20,0\n[Infantry]\n0=Neutral,E1,{health},21,20,0,Guard,0\n[Aircraft]\n0=Neutral,ORCA,{health},22,20,0\n");
        let session = session_with_map_objects(&objects);
        let units = session.sim.units();
        assert_eq!(units.len(), 3);
        for (i, (name, loco)) in [
            ("MTNK", yr_sim::terrain::Loco::Track),
            ("E1", yr_sim::terrain::Loco::Foot),
            ("ORCA", yr_sim::terrain::Loco::Fly),
        ]
        .into_iter()
        .enumerate()
        {
            let u = &units[i];
            assert_eq!(u.id, i as u32 + 1);
            assert_eq!(session.kinds[usize::from(u.kind)], name);
            assert_eq!((u.hp, u.max_hp), (hp, 100), "{name} map health {health}");
            assert_eq!((u.loco, u.owner), (loco, yr_sim::OWNER_NEUTRAL));
            assert_eq!(
                (u.x, u.y),
                (
                    Fix::from_raw((20 + i as i32) * 256 + 128),
                    Fix::from_raw(20 * 256 + 128)
                )
            );
        }
        let bytes = yr_app::snapshot::encode(&session).unwrap();
        let mut restored = session_with_map_objects(&objects);
        yr_app::snapshot::restore(&mut restored, &bytes).unwrap();
        assert_eq!(yr_app::snapshot::encode(&restored).unwrap(), bytes);
        println!("Authored vehicle/infantry/aircraft health {health}: {hp}/100 with type, owner, ids/positions and exact match roundtrip retained.");
    }
}

#[test]
fn map_unit_health_native_pips_and_pending_orders_preserve_session() {
    for (section, kind, suffix) in [
        ("Units", "MTNK", "0"),
        ("Infantry", "E1", "0,Guard,0"),
        ("Aircraft", "ORCA", "0"),
    ] {
        for (health, hp, color, pixels) in [
            (256, 100, [0, 252, 0, 255], 18),
            (128, 50, [252, 252, 0, 255], 10),
            (1, 1, [252, 0, 0, 255], 2),
            (0, 1, [252, 0, 0, 255], 2),
            (u16::MAX, 100, [0, 252, 0, 255], 18),
        ] {
            let objects = format!("[{section}]\n0=Neutral,{kind},{health},20,20,{suffix}\n");
            for size in [(640, 480), (1280, 800)] {
                let (mut app, _, _) = scene(size);
                app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
                    .into_iter()
                    .cycle()
                    .take(1600 * 1200 * 4)
                    .collect();
                app.session = Some(session_with_map_objects(&objects));
                let sim = &mut app.session.as_mut().unwrap().sim;
                assert_eq!((sim.units()[0].hp, sim.units()[0].max_hp), (hp, 100));
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
                let (mx, my) = yr_app::interaction::ground_point(
                    app.map.as_ref().unwrap(),
                    app.session.as_ref().unwrap().sim.terrain(),
                    (20.5, 20.5),
                );
                let center = (
                    (mx - app.view.cam_x) * app.view.zoom,
                    (my - app.view.cam_y) * app.view.zoom,
                );
                let y = (center.1 - 16.0).round() as u32 + 1;
                let x = center.0.round() as u32 - 14;
                let colored = (x..x + 28)
                    .filter(|x| {
                        let at = ((y * size.0 + x) * 4) as usize;
                        frame[at..at + 4] == color
                    })
                    .count();
                assert_eq!(colored, pixels, "{kind} health {health} at {size:?}");
                if health == 128 || (kind == "MTNK" && matches!(health, 1 | 256) && size.0 == 640) {
                    capture(&app, &format!("map-unit-health-{kind}-{health}-{}", size.0));
                }
                assert_eq!(
                    yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                    before
                );
                assert!(app.pending.is_empty());
                assert_eq!(app.selected, BTreeSet::from([own]));
            }
        }
    }
    println!("Thirty native unit-health cases: three unit classes x five map fractions x two layouts; exact green/yellow/red pip rows and native Attack previews preserve complete Session bytes, selection and empty pending orders. Authored fixture art only.");
}
