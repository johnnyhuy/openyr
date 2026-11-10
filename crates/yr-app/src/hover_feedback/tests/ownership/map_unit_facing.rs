use super::*;

#[test]
fn map_unit_facing_setup_preserves_headings_and_exact_save_bytes() {
    for (raw, expected) in [
        ("0", 0),
        ("32", 32),
        ("64", 64),
        ("96", 96),
        ("128", 128),
        ("160", 160),
        ("192", 192),
        ("224", 224),
        ("255", 255),
        ("256", 255),
        ("65535", 255),
        ("65536", 0),
        ("18446744073709551615", 0),
        ("", 0),
        ("bad", 0),
        ("-1", 0),
    ] {
        let objects = format!("[Units]\n0=Neutral,MTNK,128,20,20,{raw}\n[Infantry]\n0=Neutral,E1,128,21,20,0,Guard,{raw}\n[Aircraft]\n0=Neutral,ORCA,128,22,20,{raw}\n");
        let session = session_with_map_objects(&objects);
        assert_eq!(session.sim.units().len(), 3);
        for (i, kind) in ["MTNK", "E1", "ORCA"].into_iter().enumerate() {
            let unit = &session.sim.units()[i];
            assert_eq!(unit.facing, expected, "{kind}: {raw:?}");
            assert_eq!(
                (unit.id, unit.owner, unit.hp, unit.max_hp),
                (i as u32 + 1, yr_sim::OWNER_NEUTRAL, 50, 100)
            );
            assert_eq!(session.kinds[usize::from(unit.kind)], kind);
            assert_eq!(
                (unit.x, unit.y),
                (
                    Fix::from_raw((20 + i as i32) * 256 + 128),
                    Fix::from_raw(20 * 256 + 128)
                )
            );
            assert!(unit.target.is_none() && unit.attack.is_none());
        }
        let bytes = yr_app::snapshot::encode(&session).unwrap();
        let mut restored = session_with_map_objects(&objects);
        yr_app::snapshot::restore(&mut restored, &bytes).unwrap();
        assert_eq!(yr_app::snapshot::encode(&restored).unwrap(), bytes);
        println!("Parsed vehicle/infantry/aircraft facing {raw:?} -> {expected}; ids, health, owner, positions, orders and exact match roundtrip retained.");
    }
}
