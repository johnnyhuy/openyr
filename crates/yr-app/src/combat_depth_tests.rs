use super::*;

/// Real fatal shots exercise world sorting, not just isolated sprite blits.
#[test]
fn retail_combat_art_respects_foreground_bodies_and_reconstructs_saved_flights() {
    let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let dir = dir.to_str().unwrap();
    let mut app = new_app(dir.into(), "EB1.MMX".into(), 1);
    app.map = Some(yr_render::render_map_background(dir, "EB1.MMX", 1).unwrap());
    app.sprites = load_sprites(dir, "EB1.MMX");
    let mut session = app.load_session().unwrap();
    let grid = session.sim.terrain().unwrap().clone();
    let (cx, cy) = (8..grid.height().saturating_sub(8))
        .find_map(|ly| {
            (8..grid.width().saturating_sub(8))
                .map(|lx| grid.cell_at_local(lx, ly))
                .find(|&(x, y)| {
                    (-5..=7).all(|dy| {
                        (-5..=7).all(|dx| {
                            let (x2, y2) = ((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16);
                            grid.passable(yr_sim::terrain::Loco::Track, x2, y2)
                                && grid.cell(x2, y2).unwrap().height
                                    == grid.cell(x, y).unwrap().height
                        })
                    })
                })
        })
        .expect("retail flat patch");
    let kind = |id: &str| session.kinds.iter().position(|k| k == id).unwrap() as u16;
    let (yard, tank) = (kind("GACNST"), kind("MTNK"));
    let weapon = session
        .sim
        .units()
        .iter()
        .find(|u| u.kind == tank)
        .unwrap()
        .weapon
        .unwrap();
    session.sim = yr_sim::Sim::new(42);
    session.sim.set_terrain(grid.clone());
    app.session = Some(session);
    app.view.zoom = 2.0;
    app.center_world_at(
        (f32::from(cx) + 1.5, f32::from(cy) + 1.5),
        (WIN_W - SIDEBAR_W, WIN_H),
    );
    let output = std::env::var_os("OPENYR_COMBAT_DEPTH_PROOF").map(PathBuf::from);
    if let Some(out) = &output {
        std::fs::create_dir_all(out).unwrap();
    }
    for rear in [true, false] {
        app.session.as_mut().unwrap().sim = yr_sim::Sim::new(42);
        app.session.as_mut().unwrap().sim.set_terrain(grid.clone());
        app.session.as_mut().unwrap().outcome = Outcome::Ongoing;
        app.effects = Default::default();
        app.animations = Default::default();
        let spawn = |x, y, hp, weapon| Command::Spawn {
            kind: tank,
            x: Fix::from_raw(x),
            y: Fix::from_raw(y),
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::ZERO,
            hp,
            armor: 0,
            weapon,
        };
        let x = i32::from(cx) * 256 + if rear { -128 } else { 128 };
        let y = i32::from(cy) * 256 + if rear { -32 } else { 4 * 256 + 128 };
        app.pending.extend([
            Command::PlaceBuilding {
                kind: yard,
                x: cx,
                y: cy,
                w: 4,
                h: 4,
                hp: 1000,
                armor: 0,
                power: 0,
                owner: yr_sim::OWNER_PLAYER,
                role: yr_sim::Role::ConstructionYard,
            },
            spawn(x, y, 1000, Some(weapon)),
            Command::AssignOwner {
                id: 2,
                owner: yr_sim::OWNER_PLAYER,
            },
            spawn(x + if rear { 2 } else { 3 } * 256, y, 1, None),
            Command::AssignOwner {
                id: 3,
                owner: yr_sim::OWNER_ENEMY,
            },
            Command::Attack { id: 2, target: 3 },
        ]);
        app.tick_session();
        assert_eq!(app.session.as_ref().unwrap().sim.buildings().len(), 1);
        let mut seen = BTreeSet::new();
        let mut overlap = BTreeSet::new();
        let mut saved_flight = false;
        for age in 0..40 {
            if ![0, 1, 2, 3, 4, 5, 8, 12, 16, 20, 24, 28, 32, 39].contains(&age) {
                app.tick_session();
                continue;
            }
            let session = app.session.as_ref().unwrap();
            let tick = session.sim.tick();
            let save = yr_app::snapshot::encode(session).unwrap();
            let hash = session.sim.state_hash();
            let actual = compose(&app);
            assert_eq!(compose(&app), actual, "redraw cannot advance art");
            assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                save
            );
            if !session.sim.projectiles().is_empty() && !saved_flight {
                yr_app::snapshot::restore(app.session.as_mut().unwrap(), &save).unwrap();
                app.sprites = load_sprites(dir, "EB1.MMX");
                assert_eq!(compose(&app), actual, "fresh cache rebuilds a saved flight");
                saved_flight = true;
            }
            let original = app.session.as_ref().unwrap().sim.clone();
            let mut hidden = serde_json::to_value(&original).unwrap();
            for projectile in hidden["projectiles"].as_array_mut().unwrap() {
                projectile["origin"] = serde_json::Value::Null;
            }
            app.session.as_mut().unwrap().sim = serde_json::from_value(hidden).unwrap();
            let effects = std::mem::take(&mut app.effects.active);
            let baseline = compose(&app);
            app.session.as_mut().unwrap().sim = original;
            app.effects.active = effects;
            let mut old_overlay = baseline.clone();
            let mask = building_mask(&app);
            let mut paint = |name: &str, sprite: &yr_render::sprites::Sprite, x: Fix, y: Fix| {
                seen.insert(name.to_owned());
                let mut alone = baseline.clone();
                let (mx, my) = yr_app::interaction::ground_point(
                    app.map.as_ref().unwrap(),
                    app.session.as_ref().unwrap().sim.terrain(),
                    (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
                );
                let (wx, wy) = (
                    (mx - app.view.cam_x) * app.view.zoom,
                    (my - app.view.cam_y) * app.view.zoom,
                );
                blit_sprite(
                    &mut alone,
                    WIN_W,
                    WIN_H,
                    sprite,
                    wx,
                    wy,
                    app.view.zoom,
                    SpritePass::Effect,
                );
                blit_sprite(
                    &mut old_overlay,
                    WIN_W,
                    WIN_H,
                    sprite,
                    wx,
                    wy,
                    app.view.zoom,
                    SpritePass::Effect,
                );
                if (0..mask.len() / 4).any(|p| {
                    mask[p * 4 + 3] == 255 && alone[p * 4..p * 4 + 3] != baseline[p * 4..p * 4 + 3]
                }) {
                    overlap.insert(name.to_owned());
                }
            };
            let mut bank = app.sprites.as_ref().unwrap().borrow_mut();
            for p in app.session.as_ref().unwrap().sim.projectiles() {
                if let Some(sprite) = p
                    .origin
                    .and_then(|o| o.weapon)
                    .and_then(|s| bank.get_projectile(s))
                {
                    paint("120MM", sprite, p.x, p.y);
                }
            }
            for e in &app.effects.active {
                if let Some(sprite) = bank.get_effect(&e.name, tick - e.born) {
                    paint(&e.name, sprite, e.x, e.y);
                }
            }
            drop(bank);
            let mut changed = 0;
            for p in 0..mask.len() / 4 {
                if mask[p * 4 + 3] != 255 {
                    continue;
                }
                let range = p * 4..p * 4 + 3;
                assert_eq!(
                    &actual[range.clone()],
                    if rear {
                        &baseline[range.clone()]
                    } else {
                        &old_overlay[range.clone()]
                    },
                    "foreground body at tick {tick}, rear={rear}, pixel {p}"
                );
                changed += usize::from(actual[range.clone()] != old_overlay[range]);
            }
            if let Some(out) = &output {
                if changed > 0 || (!rear && age == 8) {
                    let side = if rear { "rear" } else { "front" };
                    for (label, frame) in [("fixed", &actual), ("old-overlay", &old_overlay)] {
                        std::fs::write(
                            out.join(format!("{side}-{label}-t{tick:03}.png")),
                            yr_render::png::encode_rgba(WIN_W, WIN_H, frame),
                        )
                        .unwrap();
                    }
                    std::fs::write(out.join(format!("{side}-t{tick:03}.json")), serde_json::to_vec_pretty(&serde_json::json!({"tick": tick, "state_hash": format!("{hash:016x}"), "rear": rear, "corrected_body_pixels": changed, "effects": app.effects.active.iter().map(|e| &e.name).collect::<Vec<_>>(), "projectiles": app.session.as_ref().unwrap().sim.projectiles()})).unwrap()).unwrap();
                }
            }
            app.tick_session();
        }
        assert!(saved_flight);
        for name in ["GUNFIRE", "120MM", "S_CLSN22", "TWLT070"] {
            assert!(seen.contains(name), "actual shot must show {name}");
        }
        assert!(
            overlap.contains("TWLT070"),
            "retail destruction must overlap the body, rear={rear}"
        );
        println!("rear={rear}, overlapping combat art {overlap:?}");
    }
    // A flight through the rear part of a foundation remains behind its body.
    app.effects = Default::default();
    let baseline = compose(&app);
    let tick = app.session.as_ref().unwrap().sim.tick();
    app.effects.trails.push(yr_app::effects::Trail {
        from: (
            Fix::from_int(i32::from(cx) + 2),
            Fix::from_int(i32::from(cy) + 2),
        ),
        to: (
            Fix::from_raw((i32::from(cx) + 2) * 256 + 160),
            Fix::from_int(i32::from(cy) + 2),
        ),
        born: tick,
        style: yr_render::LineTrail {
            color: [216, 216, 255],
            decrement: 16,
        },
    });
    let actual = compose(&app);
    let mut overlay = baseline.clone();
    yr_app::trails::draw(
        &mut overlay,
        &app.effects.trails[0],
        tick,
        yr_app::trails::View {
            size: (WIN_W, WIN_H),
            viewport_width: WIN_W - SIDEBAR_W,
            map: app.map.as_ref().unwrap(),
            terrain: app.session.as_ref().unwrap().sim.terrain(),
            camera: (app.view.cam_x, app.view.cam_y, app.view.zoom),
        },
    );
    let mask = building_mask(&app);
    let mut occluded = 0;
    for p in 0..mask.len() / 4 {
        if mask[p * 4 + 3] == 255 {
            let range = p * 4..p * 4 + 3;
            assert_eq!(actual[range.clone()], baseline[range.clone()]);
            occluded += usize::from(overlay[range.clone()] != baseline[range]);
        }
    }
    assert!(
        occluded > 0,
        "rear trail must exercise opaque foreground coverage"
    );
    println!("foreground body occludes {occluded} trail pixels");
    if let Some(out) = &output {
        for (label, pixels) in [
            ("fixed", &actual),
            ("overlay", &overlay),
            ("hidden", &baseline),
        ] {
            std::fs::write(
                out.join(format!("trail-depth-{label}-t{tick:03}.png")),
                yr_render::png::encode_rgba(WIN_W, WIN_H, pixels),
            )
            .unwrap();
        }
    }
    app.effects.trails.clear();
    // A building's own impact must remain above its body. Its full retained
    // foundation ties with the body, rather than sorting its centre behind it.
    let b = &app.session.as_ref().unwrap().sim.buildings()[0];
    let (x, y) = b.centre();
    let foundation = yr_app::effects::Foundation {
        cell: b.cell,
        size: b.size,
    };
    let baseline = compose(&app);
    app.effects.active.push(yr_app::effects::Effect {
        name: "S_CLSN22".into(),
        x,
        y,
        born: app.session.as_ref().unwrap().sim.tick() - 4,
        foundation: Some(foundation),
    });
    let actual = compose(&app);
    let mask = building_mask(&app);
    let mut expected = baseline.clone();
    let (mx, my) = yr_app::interaction::ground_point(
        app.map.as_ref().unwrap(),
        app.session.as_ref().unwrap().sim.terrain(),
        (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
    );
    blit_sprite(
        &mut expected,
        WIN_W,
        WIN_H,
        app.sprites
            .as_ref()
            .unwrap()
            .borrow_mut()
            .get_effect("S_CLSN22", 4)
            .unwrap(),
        (mx - app.view.cam_x) * app.view.zoom,
        (my - app.view.cam_y) * app.view.zoom,
        app.view.zoom,
        SpritePass::Effect,
    );
    let mut visible = 0;
    for p in 0..mask.len() / 4 {
        if mask[p * 4 + 3] == 255 {
            let range = p * 4..p * 4 + 3;
            assert_eq!(actual[range.clone()], expected[range.clone()]);
            visible += usize::from(actual[range.clone()] != baseline[range]);
        }
    }
    assert!(
        visible > 0,
        "the struck building cannot hide its own impact"
    );
    app.effects.active.clear();
    // Missing optional art must never fall back to a diamond or hide a body.
    let baseline = compose(&app);
    app.effects.active.push(yr_app::effects::Effect {
        name: "NO_SUCH_OPTIONAL_EFFECT".into(),
        foundation: None,
        x: Fix::from_int(i32::from(cx)),
        y: Fix::from_int(i32::from(cy)),
        born: app.session.as_ref().unwrap().sim.tick(),
    });
    assert_eq!(compose(&app), baseline);
}

fn compose(app: &App) -> Vec<u8> {
    let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
    app.compose(&mut frame, WIN_W, WIN_H);
    frame
}

fn building_mask(app: &App) -> Vec<u8> {
    let session = app.session.as_ref().unwrap();
    let b = &session.sim.buildings()[0];
    let map = app.map.as_ref().unwrap();
    let (mx, my) = yr_app::interaction::ground_point(
        map,
        session.sim.terrain(),
        (f32::from(b.cell.0) + 0.5, f32::from(b.cell.1) + 0.5),
    );
    let mut bank = app.sprites.as_ref().unwrap().borrow_mut();
    let remap = owner_remap(&bank, b.owner, Some(session));
    let art = bank
        .get_building(
            &session.kinds[usize::from(b.kind)],
            remap,
            yr_app::animation::building_animation(&session.sim, b),
        )
        .unwrap();
    let mut mask = vec![0; (WIN_W * WIN_H * 4) as usize];
    for layer in art.layers {
        blit_sprite(
            &mut mask,
            WIN_W,
            WIN_H,
            layer,
            (mx - app.view.cam_x) * app.view.zoom,
            (my - app.view.cam_y) * app.view.zoom,
            app.view.zoom,
            SpritePass::Body,
        );
    }
    // A front unit can cover the building too. Compare only building body
    // pixels untouched by live unit art, including fractional voxel edges.
    let mut units = vec![0; mask.len()];
    for u in session.sim.units() {
        let remap = owner_remap(&bank, u.owner, Some(session));
        let (mx, my) = yr_app::interaction::ground_point(
            map,
            session.sim.terrain(),
            (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0),
        );
        if let Some(sprite) = bank.get_animated_at(
            &session.kinds[usize::from(u.kind)],
            u.facing,
            remap,
            app.animations.get(u.id),
            session.sim.tick(),
        ) {
            blit_sprite(
                &mut units,
                WIN_W,
                WIN_H,
                sprite,
                (mx - app.view.cam_x) * app.view.zoom,
                (my - app.view.cam_y) * app.view.zoom,
                app.view.zoom,
                SpritePass::Body,
            );
        }
    }
    for (building, unit) in mask
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(units.as_chunks::<4>().0)
    {
        if unit[3] != 0 {
            building[3] = 0;
        }
    }
    mask
}
