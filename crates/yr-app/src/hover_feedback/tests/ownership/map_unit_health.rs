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
