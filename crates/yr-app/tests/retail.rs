//! Retail skirmish setup: EB1.MMX becomes a running session with a
//! player base, starting force, enemy base and rules credits; scripted
//! destruction drives it to a win. Skips without the retail install.

use std::path::PathBuf;
use yr_app::session::{setup, Outcome};
use yr_assets::AssetFs;
use yr_formats::{IniFile, MapFile};
use yr_rules::Rules;
use yr_sim::Command;

#[test]
fn eb1_harvester_delivers_to_retail_refinery_with_working_animation() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut session = yr_app::session::load(dir.to_str().unwrap(), "EB1.MMX", 42).unwrap();
    let refinery = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::Refinery)
        .unwrap()
        .clone();
    assert_eq!(session.kinds[refinery.kind as usize], "GAREFN");
    assert_eq!(refinery.size, (4, 3));
    assert_eq!(
        yr_app::animation::building_animation(&session.sim, &refinery).unloading,
        None
    );
    let start_credits = session.sim.credits();
    let harvester = session
        .sim
        .units()
        .iter()
        .find(|u| u.owner == yr_sim::OWNER_PLAYER && u.role == yr_sim::Role::Harvester)
        .unwrap()
        .id;
    session.step(&[Command::Harvest { id: harvester }]);
    let mut began = None;
    let mut delivered = 0;
    let mut bank = yr_render::sprites::SpriteBank::open(dir.to_str().unwrap(), "EB1.MMX").unwrap();
    let remap = bank.house_color("Americans");
    for _ in 0..2000 {
        session.step(&[]);
        let state = yr_app::animation::building_animation(&session.sim, &refinery);
        if let Some(elapsed) = state.unloading {
            if began.is_none() {
                began = Some(session.sim.tick());
                assert_eq!(elapsed, 0, "new unloading clock starts at zero");
                if let Ok(path) = std::env::var("OPENYR_REFINERY_SAVE") {
                    yr_app::snapshot::save(&session, std::path::Path::new(&path)).unwrap();
                }
            }
            let layers = bank.get_building("GAREFN", remap, state).unwrap();
            assert_eq!(
                layers.layers.len(),
                4 + usize::from(elapsed * 200 / 225 < 19)
            );
        }
        for event in session.sim.drain_events() {
            if let yr_sim::Event::Unloaded { id, value, total } = event {
                if session
                    .sim
                    .units()
                    .iter()
                    .any(|u| u.id == id && u.owner == yr_sim::OWNER_PLAYER)
                {
                    delivered += value;
                    assert_eq!(total, start_credits + delivered);
                    assert_eq!(state.unloading, None, "delivery stops the special overlay");
                    println!(
                        "EB1 refinery: tick {}, harvester {id}, delivered {value}, credits {total}",
                        session.sim.tick()
                    );
                }
            }
        }
        if delivered >= 1000 {
            break;
        }
    }
    assert_eq!(
        delivered, 1000,
        "two full loads must reach the retail refinery"
    );
    println!("EB1 refinery: first unloading at tick {}", began.unwrap());
}

#[test]
fn eb1_skirmish_setup_and_scripted_win() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");

    assert!(
        rules.techno("SMCV").is_some(),
        "Soviet fallback exists but must not substitute for AMCV"
    );
    for fault in ["missing", "owner", "art", "harvester"] {
        let mut unsupported_ini = rules_ini.clone();
        if fault == "missing" {
            unsupported_ini
                .sections
                .iter_mut()
                .find(|s| s.name.eq_ignore_ascii_case("VehicleTypes"))
                .unwrap()
                .entries
                .retain(|(_, id)| id != "AMCV");
        } else {
            let fields = match fault {
                "owner" => "[AMCV]\nOwner=Russians",
                "art" => "[AMCV]\nImage=OPENYR_MISSING_START_ART",
                "harvester" => "[CMIN]\nHarvester=no",
                _ => unreachable!(),
            };
            unsupported_ini.merge_from(&IniFile::parse(fields));
        }
        let unsupported = Rules::load(&unsupported_ini, &art_ini);
        let error = setup(&mut fs, &map, &unsupported, &overlay_types, 42)
            .err()
            .expect("unsupported setup returns an error");
        assert!(
            error.contains(if fault == "harvester" { "CMIN" } else { "AMCV" }),
            "{fault}: {error}"
        );
        println!("Recoverable {fault} setup error: {error}");
    }
    let mut session = setup(&mut fs, &map, &rules, &overlay_types, 42).expect("session sets up");
    assert_eq!(session.sim.credits(), 10_000, "rules starting money");
    let player_units = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
        .count();
    let player_bases = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_PLAYER)
        .count();
    let enemy_bases = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_ENEMY)
        .count();
    assert_eq!(player_units, 10, "retail UnitCount=10 must spawn exactly");
    let mut composition = std::collections::BTreeMap::new();
    for unit in session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
    {
        let id = &session.kinds[usize::from(unit.kind)];
        *composition.entry(id.as_str()).or_insert(0) += 1;
        let t = rules.techno(id).unwrap();
        assert!(
            t.owner
                .iter()
                .any(|h| h.eq_ignore_ascii_case(yr_app::session::PLAYER_COUNTRY)),
            "{id} must belong to Americans"
        );
        assert_eq!(unit.max_hp, t.strength.unwrap());
        assert_eq!(
            unit.weapon,
            t.primary
                .as_deref()
                .and_then(|w| yr_sim::WeaponSpec::from_rules(&rules, w))
        );
        assert_eq!(unit.role == yr_sim::Role::Harvester, id == "CMIN");
    }
    assert_eq!(
        composition,
        std::collections::BTreeMap::from([("AMCV", 1), ("CMIN", 1), ("MTNK", 4), ("E1", 4)])
    );
    println!("Americans starting force: {composition:?}");
    // Authored objects keep their original type and are counted separately.
    for obj in map.units.iter().chain(&map.infantry).chain(&map.aircraft) {
        assert!(session
            .sim
            .units()
            .iter()
            .any(|u| u.owner == yr_sim::OWNER_NEUTRAL
                && session.kinds[usize::from(u.kind)] == obj.kind
                && u.x.to_int_floor() == i32::from(obj.x)
                && u.y.to_int_floor() == i32::from(obj.y)));
    }
    let neutral_units = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_NEUTRAL)
        .count();
    assert!(
        neutral_units >= 1,
        "map pre-placed units stay neutral: {neutral_units}"
    );
    let enemy_units = session
        .sim
        .units()
        .iter()
        .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
        .count();
    assert_eq!(enemy_units, 3, "enemy guards spawned");
    assert!(player_bases >= 3, "player base placed: {player_bases}");
    assert!(enemy_bases >= 3, "enemy base placed: {enemy_bases}");
    assert_eq!(session.outcome, Outcome::Ongoing);

    // Scripted win: destroy every enemy entity, step, expect Won.
    let kills: Vec<Command> = session
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
    // Determinism: a second setup with the same seed hashes identically.
    let mut session2 = setup(&mut fs, &map, &rules, &overlay_types, 42).expect("second setup");
    assert_eq!(session.sim.state_hash(), session2.sim.state_hash());
    let saved = yr_app::snapshot::encode(&session).unwrap();
    session2.step(&[]);
    yr_app::snapshot::restore(&mut session2, &saved).unwrap();
    assert_eq!(session.sim.state_hash(), session2.sim.state_hash());

    for count in [0, 6, 12] {
        let mut configured = rules.clone();
        configured.skirmish.unit_count = Some(count);
        let custom = setup(&mut fs, &map, &configured, &overlay_types, 42).unwrap();
        assert_eq!(
            custom
                .sim
                .units()
                .iter()
                .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
                .count(),
            count as usize
        );
        println!("Configured UnitCount={count}: exact generated force retained");
        let before = session2.sim.state_hash();
        assert!(yr_app::snapshot::restore(
            &mut session2,
            &yr_app::snapshot::encode(&custom).unwrap()
        )
        .unwrap_err()
        .contains("map, rules or engine setup"));
        assert_eq!(
            session2.sim.state_hash(),
            before,
            "incompatible setup leaves current match intact"
        );
    }

    assert!(!kills.is_empty());
    session.step(&kills);
    session.step(&[]);
    assert_eq!(session.outcome, Outcome::Won);
    println!("EB1 session: {player_units} player units, {player_bases} player buildings, {enemy_bases} enemy buildings -> Won");
}

/// Headless playthrough: deterministic AI produces, scouts and attacks
/// the player; the player attack-moves everything at the enemy base and
/// the match resolves (Won or Lost) without a crash.
#[test]
fn eb1_ai_builds_scouts_attacks_and_resolves() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    let mut session = setup(&mut fs, &map, &rules, &overlay_types, 7).expect("session sets up");

    let enemy_count = |s: &yr_app::session::Session| {
        s.sim
            .units()
            .iter()
            .filter(|u| u.owner == yr_sim::OWNER_ENEMY)
            .count()
    };
    let before = enemy_count(&session);
    let mut twin = session.sim.clone();
    for _ in 0..1500 {
        session.step(&[]);
        twin.step(&[]);
        assert_eq!(session.sim.state_hash(), twin.state_hash());
    }
    assert!(session.sim.ai().unwrap().scout_sent(), "scout deployed");
    assert!(
        session.sim.ai().unwrap().groups_sent() >= 1,
        "attack group deployed"
    );
    let after = enemy_count(&session);
    assert!(
        after > before,
        "retail production grew the army: {before} -> {after}"
    );
    assert!(
        session.sim.credits_of(yr_sim::OWNER_ENEMY) < 10_000,
        "AI paid for its army"
    );
    assert!(
        session
            .sim
            .units()
            .iter()
            .any(|u| u.owner == yr_sim::OWNER_ENEMY && (u.target.is_some() || u.attack.is_some())),
        "AI marches on the player"
    );

    // Player counter-attack: every armed player unit attack-moves to the
    // enemy base (centre of its first building), re-issued every 10 s.
    let mut ticks = 0u64;
    while session.outcome == Outcome::Ongoing && ticks < 15 * 60 * 15 {
        let orders: Vec<Command> = match session
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_ENEMY)
        {
            Some(b) if ticks.is_multiple_of(150) => {
                let (x, y) = b.centre();
                session
                    .sim
                    .units()
                    .iter()
                    .filter(|u| {
                        u.owner == yr_sim::OWNER_PLAYER
                            && u.weapon.is_some()
                            && u.role == yr_sim::Role::Combat
                    })
                    .map(|u| Command::AttackMove { id: u.id, x, y })
                    .collect()
            }
            _ => Vec::new(),
        };
        session.step(&orders);
        ticks += 1;
    }
    println!(
        "EB1 AI playthrough: {:?} after {ticks} ticks, groups {}",
        session.outcome,
        session.sim.ai().unwrap().groups_sent()
    );
    assert_ne!(
        session.outcome,
        Outcome::Ongoing,
        "match resolved within 15 minutes"
    );
}

/// Production sidebar data on EB1: every allied slot resolves with a
/// retail cost and decoded cameo, the player base hosts a war factory
/// and barracks, and queued builds roll out of them on schedule.
#[test]
fn eb1_sidebar_slots_produce_units() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    let mut session = setup(&mut fs, &map, &rules, &overlay_types, 5).expect("session sets up");

    let slots: Vec<(&str, i64)> = session
        .buildables
        .iter()
        .map(|b| (b.id.as_str(), b.cost))
        .collect();
    // Sandbox unit picks (the British-only sniper is not American), then
    // every Allied structure from `[BuildingTypes]` in rules order.
    assert_eq!(
        slots,
        vec![
            ("MTNK", 700),
            ("FV", 600),
            ("SREF", 1200),
            ("CMIN", 1400),
            ("E1", 200),
            ("GGI", 400),
            ("GAPOWR", 800),
            ("GAREFN", 2000),
            ("GAPILE", 500),
            ("GADEPT", 800),
            ("GATECH", 2000),
            ("GAWEAP", 2000),
            ("NASAM", 1000),
            ("GACSPH", 2500),
            ("GAWEAT", 5000),
            ("ATESLA", 1500),
            ("GASPYSAT", 1500),
            ("GAGAP", 1000),
            ("GAPILL", 500),
            ("GAOREP", 2500),
            ("AMRADR", 1000),
            ("GAROBO", 600),
        ]
    );
    for b in &session.buildables {
        let (w, h, px) = b.cameo.as_ref().unwrap_or_else(|| panic!("{} cameo", b.id));
        assert_eq!((*w, *h), (60, 48), "{} cameo size", b.id);
        assert!(px.chunks(4).any(|p| p[3] != 0), "{} cameo has pixels", b.id);
    }
    let csf = yr_formats::CsfFile::parse(fs.read("RA2MD.CSF").unwrap()).unwrap();
    for b in &session.buildables {
        use yr_app::sidebar::Category;
        let expected_category = match rules.techno(&b.id).unwrap().kind {
            yr_rules::TechnoKind::Infantry => Category::Infantry,
            yr_rules::TechnoKind::Building => {
                if rules_ini.section(&b.id).and_then(|s| s.get("BuildCat")) == Some("Combat") {
                    Category::Defences
                } else {
                    Category::Structures
                }
            }
            _ => Category::Vehicles,
        };
        assert_eq!(
            b.category, expected_category,
            "{} raw retail category",
            b.id
        );
        let label = rules.techno(&b.id).unwrap().ui_name.as_deref().unwrap();
        assert_eq!(Some(b.name.as_str()), csf.get(label), "{} name", b.id);
        assert!(!b.name.starts_with("Name:"), "{} unresolved label", b.id);
        println!(
            "{}: {} (${}, {})",
            b.id,
            b.name,
            b.cost,
            yr_app::sidebar::duration(b.ticks)
        );
    }
    // Retail BuildSpeed: Grizzly 700 credits -> 441 ticks.
    assert_eq!(session.buildables[0].ticks, 441);
    let roles: Vec<yr_sim::Role> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_PLAYER)
        .map(|b| b.role)
        .collect();
    assert!(roles.contains(&yr_sim::Role::WarFactory), "{roles:?}");
    assert!(roles.contains(&yr_sim::Role::Barracks), "{roles:?}");

    let cmds = vec![session.produce(0).unwrap(), session.produce(4).unwrap()];
    session.step(&cmds);
    let mut produced = Vec::new();
    for _ in 0..600 {
        session.step(&[]);
        for e in session.sim.drain_events() {
            if let yr_sim::Event::Produced { id, kind } = e {
                if !session
                    .sim
                    .units()
                    .iter()
                    .any(|u| u.id == id && u.owner == yr_sim::OWNER_PLAYER)
                {
                    continue;
                }
                produced.push((session.sim.tick(), id, kind));
            }
        }
    }
    let kinds: Vec<u16> = produced.iter().map(|p| p.2).collect();
    assert_eq!(kinds, vec![4, 0], "GI first (shorter), then the Grizzly");
    for (_, id, _) in &produced {
        let u = session.sim.units().iter().find(|u| u.id == *id);
        assert_eq!(u.map(|u| u.owner), Some(yr_sim::OWNER_PLAYER));
    }

    // The full sandbox loop: build more, then send the army at the enemy
    // base every 30 s; produced reinforcements turn it into a win.
    let more: Vec<Command> = [0, 0, 4, 4, 1, 5]
        .iter()
        .filter_map(|&i| session.produce(i))
        .collect();
    session.step(&more);
    let mut ticks = 0u64;
    while session.outcome == Outcome::Ongoing && ticks < 10 * 60 * 15 {
        ticks += 1;
        let target = session
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_ENEMY)
            .map(|b| b.centre());
        let orders: Vec<Command> = match target {
            Some((x, y)) if ticks.is_multiple_of(450) => session
                .sim
                .units()
                .iter()
                .filter(|u| {
                    u.owner == yr_sim::OWNER_PLAYER
                        && u.weapon.is_some()
                        && u.role == yr_sim::Role::Combat
                })
                .map(|u| Command::AttackMove { id: u.id, x, y })
                .collect(),
            _ => Vec::new(),
        };
        session.step(&orders);
    }
    println!(
        "EB1 produced-army playthrough: {:?} after {ticks} ticks",
        session.outcome
    );
    assert_eq!(session.outcome, Outcome::Won);
}

fn eb1_session(seed: u64) -> Option<yr_app::session::Session> {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return None;
    }
    let mut fs = AssetFs::open(&dir).expect("install opens");
    let map =
        MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap())).expect("EB1 parses");
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("RULESMD.INI").unwrap()));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(fs.read("ARTMD.INI").unwrap()));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = yr_render::type_list(&rules_ini, "OverlayTypes");
    Some(setup(&mut fs, &map, &rules, &overlay_types, seed).expect("session sets up"))
}

fn slot_of(session: &yr_app::session::Session, id: &str) -> usize {
    session
        .buildables
        .iter()
        .position(|b| b.id == id)
        .unwrap_or_else(|| panic!("{id} slot"))
}

#[test]
fn eb1_sidebar_dims_unaffordable_and_missing_factories_but_keeps_ready_structures() {
    use yr_app::sidebar;
    let Some(mut session) = eb1_session(11) else {
        return;
    };
    session.sim.set_ai(None);
    let tank = slot_of(&session, "MTNK");
    let lab = slot_of(&session, "GATECH");
    let plant = slot_of(&session, "GAPOWR");
    assert!(session.slot_available(tank));
    assert!(!session.slot_available(lab), "missing radar prerequisite");
    assert!(!session.slot_available(usize::MAX));

    let panel = |session: &yr_app::session::Session, name: &str| {
        let before = session.sim.state_hash();
        let mut frame = vec![0; sidebar::WIDTH as usize * 800 * 4];
        sidebar::draw(
            &mut frame,
            sidebar::WIDTH,
            800,
            session,
            sidebar::Category::Vehicles,
            None,
        );
        assert_eq!(
            session.sim.state_hash(),
            before,
            "drawing is presentation only"
        );
        if let Some(dir) = std::env::var_os("OPENYR_SIDEBAR_DUMP") {
            std::fs::create_dir_all(&dir).unwrap();
            let path = PathBuf::from(dir).join(format!("{name}.png"));
            std::fs::write(
                &path,
                yr_render::png::encode_rgba(sidebar::WIDTH, 800, &frame),
            )
            .unwrap();
            println!(
                "{}: tick {} ${}",
                path.display(),
                session.sim.tick(),
                session.sim.credits()
            );
        }
        frame
    };
    let bright = panel(&session, "funded");
    // Both native cameo corners hit the original production id.
    let mut reached = Vec::new();
    for category in sidebar::Category::ALL {
        for (i, slot) in sidebar::slots(&session, category).enumerate() {
            reached.push(slot);
            let (x, y) = sidebar::slot_origin(sidebar::WIDTH, i);
            let (x, y) = (f64::from(x), f64::from(y));
            for (dx, dy) in [(0.0, 0.0), (59.0, 47.0)] {
                assert_eq!(
                    sidebar::slot_at(sidebar::WIDTH, 800, &session, category, (x + dx, y + dy)),
                    Some(slot)
                );
            }
            assert!(y + 48.0 < 800.0, "last row stays visible");
        }
        for pos in [(22.0, 228.0), (83.0, 228.0), (24.0, 276.0), (10.0, 40.0)] {
            assert_eq!(
                sidebar::slot_at(sidebar::WIDTH, 800, &session, category, pos),
                None
            );
        }
    }
    reached.sort_unstable();
    assert_eq!(reached, (0..session.buildables.len()).collect::<Vec<_>>());

    session.sim.set_credits(699);
    assert!(!session.slot_available(tank), "full cost is 700");
    assert!(
        session.produce(tank).is_some(),
        "gradual-payment queuing still works"
    );
    let poor = panel(&session, "unaffordable");
    // Verify the displayed cameo dims, independently of the changed credit header.
    let mut lit = 0;
    for y in 228..276 {
        for x in 23..83 {
            let at = (y * sidebar::WIDTH as usize + x) * 4;
            for c in 0..3 {
                assert_eq!(poor[at + c], bright[at + c] / 3);
                lit += usize::from(bright[at + c] > 0);
            }
        }
    }
    assert!(lit > 100);
    session.sim.set_credits(700);
    assert!(session.slot_available(tank), "exact price is affordable");

    session.sim.set_credits(10_000);
    let factories: Vec<_> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::WarFactory)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    session.step(&factories);
    assert!(
        !session.slot_available(tank),
        "enemy factory cannot host player builds"
    );
    panel(&session, "factory-lost");

    build_ready(&mut session, plant);
    session.sim.set_credits(0);
    let yards: Vec<_> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::ConstructionYard)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    session.step(&yards);
    assert_eq!(session.ready_structure(), Some(plant));
    assert!(
        session.slot_available(plant),
        "already paid; still placeable"
    );
    panel(&session, "ready-without-yard");
}

/// Filtering changes presentation, never the queued kind or paid structure.
#[test]
fn eb1_sidebar_tabs_preserve_orders_and_ready_placement() {
    use yr_app::sidebar::{self, Category};
    let Some(mut session) = eb1_session(11) else {
        return;
    };
    session.sim.set_ai(None);
    let paint = |session: &yr_app::session::Session, stage: &str| {
        let before = yr_app::snapshot::encode(session).unwrap();
        for category in Category::ALL {
            let mut frame = vec![0; sidebar::WIDTH as usize * 800 * 4];
            sidebar::draw(&mut frame, sidebar::WIDTH, 800, session, category, None);
            if let Some(dir) = std::env::var_os("OPENYR_SIDEBAR_DUMP") {
                std::fs::create_dir_all(&dir).unwrap();
                let path =
                    PathBuf::from(dir).join(format!("tabs-{stage}-{}.png", category.file_name()));
                std::fs::write(
                    &path,
                    yr_render::png::encode_rgba(sidebar::WIDTH, 800, &frame),
                )
                .unwrap();
                println!("{}: tick {}", path.display(), session.sim.tick());
            }
            // A paid structure's tab retains its ready marker while hidden.
            if let Some(ready) = sidebar::ready_category(session) {
                let (start_x, start_y) = sidebar::tab_origin(sidebar::WIDTH, ready);
                let (start_x, start_y) = (start_x as usize, start_y as usize);
                let green = frame
                    .chunks(4)
                    .enumerate()
                    .filter(|(i, p)| {
                        let (x, y) = (i % sidebar::WIDTH as usize, i / sidebar::WIDTH as usize);
                        (start_x..start_x + 28).contains(&x)
                            && (start_y..start_y + 27).contains(&y)
                            && *p == [96, 255, 96, 255]
                    })
                    .count();
                assert!(green > 20, "READY survives on every selected tab");
            }
        }
        assert_eq!(
            before,
            yr_app::snapshot::encode(session).unwrap(),
            "tab changes preserve all saved state"
        );
    };
    let click = |session: &yr_app::session::Session, category| {
        sidebar::slot_at(sidebar::WIDTH, 800, session, category, (23.0, 228.0)).unwrap()
    };
    let tank = click(&session, Category::Vehicles);
    let infantry = click(&session, Category::Infantry);
    let plant = click(&session, Category::Structures);
    let defence = click(&session, Category::Defences);
    assert_eq!(session.buildables[tank].id, "MTNK");
    assert_eq!(session.buildables[infantry].id, "E1");
    assert_eq!(session.buildables[plant].id, "GAPOWR");
    assert_eq!(session.buildables[defence].id, "NASAM");
    for category in Category::ALL {
        let ids: Vec<_> = sidebar::slots(&session, category)
            .map(|s| session.buildables[s].id.as_str())
            .collect();
        assert!(!ids.is_empty());
        println!("{}: {}", category.label(), ids.join(", "));
    }
    paint(&session, "funded");
    let commands: Vec<_> = [tank, infantry, plant]
        .into_iter()
        .map(|slot| {
            let command = session.produce(slot).unwrap();
            assert!(matches!(command, Command::Produce { kind, .. } if usize::from(kind) == slot));
            command
        })
        .collect();
    session.step(&commands);
    for _ in 0..10 {
        session.step(&[]);
    }
    paint(&session, "progress");
    let tank_status = session.slot_status(tank);
    let plant_status = session.slot_status(plant);
    assert!(tank_status.1.is_some_and(|(done, _)| done > 0));
    // Right-clicking the same card position on Infantry cancels only GI.
    let cancel = session.cancel(click(&session, Category::Infantry)).unwrap();
    assert!(matches!(cancel, Command::CancelProduce { kind, .. } if usize::from(kind) == infantry));
    session.step(&[cancel]);
    assert_eq!(session.slot_status(infantry).0, 0);
    assert_eq!(session.slot_status(tank).0, tank_status.0);
    assert_eq!(session.slot_status(plant).0, plant_status.0);
    while session.ready_structure() != Some(plant) {
        session.step(&[]);
        assert!(session.sim.tick() < 5000);
    }
    assert_eq!(
        sidebar::ready_category(&session),
        Some(Category::Structures)
    );
    paint(&session, "ready");
    let (_, spot) = spot_near_yard(&session, plant);
    let place = session
        .place(click(&session, Category::Structures), spot)
        .unwrap();
    session.step(&[place]);
    assert_eq!(session.sim.buildings().last().unwrap().kind as usize, plant);
    assert_eq!(sidebar::ready_category(&session), None);

    // Both building tabs share the structure queue and retain their own ids.
    build_ready(&mut session, defence);
    assert_eq!(sidebar::ready_category(&session), Some(Category::Defences));
    paint(&session, "defence-ready");
    let before = session.sim.credits();
    session.step(&[session.cancel(click(&session, Category::Defences)).unwrap()]);
    assert_eq!(
        session.sim.credits(),
        before + session.buildables[defence].cost
    );
    assert_eq!(session.ready_structure(), None);
}

/// Queue structure `slot`, step until it is ready, return the ticks taken.
fn build_ready(session: &mut yr_app::session::Session, slot: usize) -> u64 {
    let cmd = session.produce(slot).expect("structure queues");
    session.step(&[cmd]);
    let mut ticks = 1;
    while session.ready_structure() != Some(slot) {
        session.step(&[]);
        ticks += 1;
        assert!(ticks < 5000, "structure never finished");
    }
    ticks
}

/// First valid origin for `slot` in rings around the player's yard.
fn spot_near_yard(session: &yr_app::session::Session, slot: usize) -> ((u16, u16), (u16, u16)) {
    let yard = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::ConstructionYard)
        .expect("player construction yard")
        .cell;
    for r in 0..16i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                let o = (
                    (i32::from(yard.0) + dx) as u16,
                    (i32::from(yard.1) + dy) as u16,
                );
                if session.can_place(slot, o) {
                    return (yard, o);
                }
            }
        }
    }
    panic!("no valid spot near the yard");
}

/// Issue #46: structures build at the construction yard, wait, and go
/// down only on valid ground; a placed power plant powers, a placed
/// refinery brings its free harvester, a placed war factory produces.
#[test]
fn eb1_structures_build_place_and_work() {
    let Some(mut session) = eb1_session(11) else {
        return;
    };
    // Exercise the long placement sequence without an opposing army.
    session.sim.set_ai(None);
    let plant = slot_of(&session, "GAPOWR");
    let lab = slot_of(&session, "GATECH");
    assert!(session.prerequisites_met(plant), "GACNST owned");
    assert!(!session.prerequisites_met(lab), "no radar yet");
    assert!(
        session.produce(lab).is_none(),
        "missing prerequisite refuses"
    );

    let credits = session.sim.credits();
    let ticks = build_ready(&mut session, plant);
    assert_eq!(session.sim.credits(), credits - 800, "full Cost= drawn");
    assert!(
        ticks >= u64::from(session.buildables[plant].ticks),
        "not instant"
    );
    assert!(
        session.produce(slot_of(&session, "GAREFN")).is_none(),
        "one structure at a time"
    );

    let (yard, good) = spot_near_yard(&session, plant);
    assert!(!session.can_place(plant, yard), "on the construction yard");
    assert!(session.place(plant, yard).is_none());
    assert!(
        !session.can_place(plant, (good.0 + 40, good.1 + 40)),
        "far from the base"
    );
    assert!(!session.can_place(plant, (0, 0)), "off the playable grid");

    // Determinism: the hash reflects the placement.
    let mut unplaced = session.sim.clone();
    let (power_before, _) = session.sim.power();
    let buildings_before = session.sim.buildings().len();
    let cmd = session.place(plant, good).expect("valid spot places");
    session.step(&[cmd]);
    unplaced.step(&[]);
    assert_ne!(session.sim.state_hash(), unplaced.state_hash());
    assert_eq!(session.sim.buildings().len(), buildings_before + 1);
    assert_eq!(
        session.sim.power().0,
        power_before + 200,
        "power plant powers"
    );
    assert_eq!(session.ready_structure(), None);
    let placed = session.sim.buildings().last().unwrap();
    assert_eq!((placed.cell, placed.owner), (good, yr_sim::OWNER_PLAYER));

    // Refinery: placement brings its `FreeUnit=` harvester.
    let refinery = slot_of(&session, "GAREFN");
    build_ready(&mut session, refinery);
    let (_, spot) = spot_near_yard(&session, refinery);
    let units_before: Vec<u32> = session.sim.units().iter().map(|u| u.id).collect();
    session.step(&[session.place(refinery, spot).unwrap()]);
    let free = session
        .sim
        .units()
        .iter()
        .find(|u| !units_before.contains(&u.id))
        .expect("free harvester");
    assert_eq!(session.kinds[usize::from(free.kind)], "CMIN");
    assert_eq!(
        (free.owner, free.role),
        (yr_sim::OWNER_PLAYER, yr_sim::Role::Harvester)
    );
    assert_eq!(
        session.sim.buildings().last().unwrap().role,
        yr_sim::Role::Refinery
    );

    // Cancel after completion refunds the full cost.
    let pile = slot_of(&session, "GAPILE");
    build_ready(&mut session, pile);
    let before = session.sim.credits();
    session.step(&[session.cancel(pile).unwrap()]);
    assert_eq!(session.sim.credits(), before + 500);
    assert_eq!(session.ready_structure(), None);

    // A second war factory placed on the map takes over production once
    // the starting one is gone.
    let weap = slot_of(&session, "GAWEAP");
    build_ready(&mut session, weap);
    let (_, spot) = spot_near_yard(&session, weap);
    session.step(&[session.place(weap, spot).unwrap()]);
    let new_weap = session.sim.buildings().last().unwrap().id;
    let old: Vec<Command> = session
        .sim
        .buildings()
        .iter()
        .filter(|b| {
            b.owner == yr_sim::OWNER_PLAYER
                && b.role == yr_sim::Role::WarFactory
                && b.id != new_weap
        })
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    assert!(!old.is_empty());
    session.step(&old);
    let tank = session.produce(0).expect("tank queues at the new factory");
    session.step(&[tank]);
    let mut rolled = false;
    for _ in 0..2000 {
        session.step(&[]);
        if session
            .sim
            .drain_events()
            .iter()
            .any(|e| matches!(e, yr_sim::Event::Produced { kind: 0, .. }))
        {
            rolled = true;
            break;
        }
    }
    assert!(rolled, "placed war factory produces");
}

/// The retail war factory replacement uses the same queue, cost, footprint
/// checks and production host lookup as a placed player structure.
#[test]
fn eb1_ai_rebuilds_its_destroyed_factory() {
    let Some(mut session) = eb1_session(23) else {
        return;
    };
    let factory = session
        .sim
        .buildings()
        .iter()
        .find(|b| b.owner == yr_sim::OWNER_ENEMY && b.role == yr_sim::Role::WarFactory)
        .unwrap()
        .id;
    session.step(&[Command::Destroy { id: factory }]);
    let mut rebuilt = None;
    for _ in 0..2000 {
        session.step(&[]);
        rebuilt = session
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_ENEMY && b.role == yr_sim::Role::WarFactory)
            .map(|b| b.id);
        if rebuilt.is_some() {
            break;
        }
    }
    assert!(rebuilt.is_some_and(|id| id != factory), "AI rebuilt GAWEAP");
    assert_eq!(
        session.sim.credits(),
        10_000,
        "replacement never spends player money"
    );
    assert!(
        session.sim.credits_of(yr_sim::OWNER_ENEMY) < 9000,
        "replacement paid for"
    );
    println!("EB1 AI rebuilt GAWEAP at tick {}", session.sim.tick());
}

/// Issue #45: interrupted matches keep the same command stream, state hashes
/// and outcome, including AI, harvesting, combat and paid production queues.
#[test]
fn eb1_saved_match_resumes_to_the_same_win() {
    let Some(mut original) = eb1_session(42) else {
        return;
    };
    let mut resumed = eb1_session(42).unwrap();
    let plant = slot_of(&original, "GAPOWR");
    let mut checkpoints = 0;
    for t in 2..=3000 {
        let mut commands = Vec::new();
        if t == 2 {
            commands.extend(
                [0, 0, 0, 4, 4, 4, 1, 5, plant]
                    .iter()
                    .filter_map(|&s| original.produce(s)),
            );
        }
        if let Some(slot) = original.ready_structure() {
            let (_, spot) = spot_near_yard(&original, slot);
            commands.push(original.place(slot, spot).unwrap());
        }
        if t % 450 == 0 {
            if let Some(b) = original
                .sim
                .buildings()
                .iter()
                .find(|b| b.owner == yr_sim::OWNER_ENEMY)
            {
                let (x, y) = b.centre();
                commands.extend(
                    original
                        .sim
                        .units()
                        .iter()
                        .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some())
                        .map(|u| Command::AttackMove { id: u.id, x, y }),
                );
            }
        }
        original.step(&commands);
        resumed.step(&commands);
        if [100, 500, 900, 1350, 3000].contains(&t) {
            let hash = original.sim.state_hash();
            let bytes = yr_app::snapshot::encode(&original).unwrap();
            assert_eq!(hash, original.sim.state_hash(), "saving is read-only");
            resumed.sim = yr_sim::Sim::new(999);
            yr_app::snapshot::restore(&mut resumed, &bytes).unwrap();
            assert_eq!(bytes, yr_app::snapshot::encode(&resumed).unwrap());
            checkpoints += 1;
        }
        assert_eq!(
            original.sim.state_hash(),
            resumed.sim.state_hash(),
            "tick {t}"
        );
        assert_eq!(original.outcome, resumed.outcome, "tick {t}");
    }
    assert_eq!(checkpoints, 5);
    assert_eq!(resumed.outcome, Outcome::Won);
    println!("EB1 save/resume: five checkpoints, identical hashes through tick 3000, Won");

    let bytes = yr_app::snapshot::encode(&original).unwrap();
    let unchanged = resumed.sim.state_hash();
    let mut bad = bytes.clone();
    bad[8..12].copy_from_slice(&99u32.to_le_bytes());
    assert!(yr_app::snapshot::restore(&mut resumed, &bad)
        .unwrap_err()
        .contains("version 99"));
    bad = bytes.clone();
    bad[20] ^= 1;
    assert!(yr_app::snapshot::restore(&mut resumed, &bad)
        .unwrap_err()
        .contains("map, rules or engine setup"));
    bad = bytes.clone();
    bad[28] ^= 1;
    assert!(yr_app::snapshot::restore(&mut resumed, &bad)
        .unwrap_err()
        .contains("hash mismatch"));
    assert_eq!(
        unchanged,
        resumed.sim.state_hash(),
        "failed loads retain the match"
    );
}
