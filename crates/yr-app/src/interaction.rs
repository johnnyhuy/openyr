//! One action resolver for cursor feedback and actual world clicks.

use crate::{cursor::Kind, session::Session, sidebar::ServiceMode};
use yr_sim::{Command, Fix, OWNER_PLAYER};

/// Brief command feedback in framebuffer pixels, independent of world zoom.
pub fn draw_feedback(frame: &mut [u8], width: u32, height: u32, session: &Session, text: &str) {
    use crate::sidebar::{text::Typeface, Canvas, WIDTH};
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        != Some(frame.len())
    {
        return;
    }
    let available = width.saturating_sub(WIDTH + 24).min(512) as i32;
    if available <= 0 {
        return;
    }
    let mut canvas = Canvas {
        frame,
        width,
        height,
        face: Typeface(session.sidebar_art.font.as_ref()),
    };
    let mut fitted = String::new();
    for ch in text.chars().take(80) {
        fitted.push(ch);
        if canvas.face.width(&fitted) > available {
            fitted.pop();
            break;
        }
    }
    canvas.rect(
        (8, 8),
        (
            canvas.face.width(&fitted) + 8,
            canvas.face.line_height() + 8,
        ),
        [0, 0, 0, 255],
    );
    canvas.text((12, 12), &fitted, [255, 255, 0, 255], 80);
}

/// Project a world ground point with the terrain lift used by live art.
/// Missing cells and callers without terrain retain the flat projection.
pub fn ground_point(
    map: &yr_render::RenderedMap,
    grid: Option<&yr_sim::TerrainGrid>,
    cell: (f32, f32),
) -> (f32, f32) {
    let (x, y) = map.cell_to_px(cell.0, cell.1);
    let height = u16::try_from(cell.0.floor() as i32)
        .ok()
        .zip(u16::try_from(cell.1.floor() as i32).ok())
        .and_then(|(x, y)| grid.and_then(|g| g.cell(x, y)))
        .map_or(0, |c| c.height);
    (x, y - f32::from(height) * 15.0 / map.scale.max(1) as f32)
}

/// Invert the same height lift used for live objects. Overlapping ground
/// diamonds pick the frontmost matching cell, as in the draw order.
pub fn cell_at(
    map: &yr_render::RenderedMap,
    grid: Option<&yr_sim::TerrainGrid>,
    pos: (f32, f32),
) -> (f32, f32) {
    let flat = map.px_to_cell(pos.0, pos.1);
    let Some(grid) = grid else {
        return flat;
    };
    for level in (0..=u8::MAX).rev() {
        let candidate = (
            flat.0 + f32::from(level) * 0.5,
            flat.1 + f32::from(level) * 0.5,
        );
        let cell = u16::try_from(candidate.0.floor() as i32)
            .ok()
            .zip(u16::try_from(candidate.1.floor() as i32).ok());
        if cell
            .and_then(|(x, y)| grid.cell(x, y))
            .is_some_and(|c| c.height == level)
        {
            return candidate;
        }
    }
    flat
}

pub struct Action {
    pub cursor: Kind,
    pub commands: Vec<Command>,
    pub select: Option<u32>,
}

impl Action {
    fn new(cursor: Kind) -> Self {
        Self {
            cursor,
            commands: Vec::new(),
            select: None,
        }
    }
}

pub fn at(
    session: &Session,
    selected: &std::collections::BTreeSet<u32>,
    cell: (i32, i32),
    attack_move: bool,
    service: ServiceMode,
) -> Action {
    if service != ServiceMode::None {
        let command = service.command(session, cell);
        let kind = match (service, command.is_some()) {
            (ServiceMode::Repair, true) => Kind::Repair,
            (ServiceMode::Repair, false) => Kind::NoRepair,
            (ServiceMode::Sell, true) => Kind::Sell,
            _ => Kind::NoSell,
        };
        let mut action = Action::new(kind);
        action.commands.extend(command);
        return action;
    }
    orders(&session.sim, selected, cell, attack_move)
}

fn orders(
    sim: &yr_sim::Sim,
    selected: &std::collections::BTreeSet<u32>,
    cell: (i32, i32),
    attack_move: bool,
) -> Action {
    let coordinate = u16::try_from(cell.0).ok().zip(u16::try_from(cell.1).ok());
    let Some(cell) =
        coordinate.filter(|&(x, y)| sim.terrain().is_some_and(|g| g.cell(x, y).is_some()))
    else {
        return Action::new(if selected.is_empty() {
            Kind::Pointer
        } else {
            Kind::Blocked
        });
    };
    let own = sim.units().iter().find(|u| {
        u.owner == OWNER_PLAYER
            && u.hp > 0
            && u.x.to_int_floor() == i32::from(cell.0)
            && u.y.to_int_floor() == i32::from(cell.1)
    });
    if !attack_move {
        if let Some(unit) = own {
            let mut action = Action::new(Kind::Select);
            action.select = Some(unit.id);
            return action;
        }
    }
    let building = sim.buildings().iter().find(|b| {
        b.owner != OWNER_PLAYER
            && b.hp > 0
            && cell.0 >= b.cell.0
            && cell.1 >= b.cell.1
            && u32::from(cell.0) < u32::from(b.cell.0) + u32::from(b.size.0)
            && u32::from(cell.1) < u32::from(b.cell.1) + u32::from(b.size.1)
    });
    let enemy = building.map(|b| b.id).or_else(|| {
        sim.units()
            .iter()
            .find(|u| {
                u.owner != OWNER_PLAYER
                    && u.hp > 0
                    && u.x.to_int_floor() == i32::from(cell.0)
                    && u.y.to_int_floor() == i32::from(cell.1)
            })
            .map(|u| u.id)
    });
    let units: Vec<_> = sim
        .units()
        .iter()
        .filter(|u| u.owner == OWNER_PLAYER && u.hp > 0 && selected.contains(&u.id))
        .collect();
    if units.is_empty() {
        return Action::new(Kind::Pointer);
    }
    let mut action = Action::new(Kind::Blocked);
    if let Some(target) = enemy {
        action.commands.extend(
            units
                .iter()
                .filter(|u| u.weapon.is_some_and(|w| w.damage > 0))
                .map(|u| Command::Attack { id: u.id, target }),
        );
        if !action.commands.is_empty() {
            action.cursor = Kind::Attack;
        }
    } else {
        let (x, y) = (
            Fix::from_int(i32::from(cell.0)) + Fix::from_raw(128),
            Fix::from_int(i32::from(cell.1)) + Fix::from_raw(128),
        );
        for unit in units {
            if sim.can_move_to(unit.id, x, y) {
                action.commands.push(if attack_move {
                    Command::AttackMove { id: unit.id, x, y }
                } else {
                    Command::Move { id: unit.id, x, y }
                });
            }
        }
        if !action.commands.is_empty() {
            action.cursor = if attack_move {
                Kind::AttackMove
            } else {
                Kind::Move
            };
        }
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_sim::{
        terrain::{Loco, TerrainGrid},
        Sim,
    };

    fn scene() -> Sim {
        let mut sim = Sim::new(42);
        sim.set_terrain(TerrainGrid::from_tiles(
            0,
            0,
            8,
            6,
            (0..6).flat_map(|y| (0..8).map(move |x| (x, y, 0, if x == 4 { 15 } else { 13 }, 0))),
        ));
        for (x, owner, weapon) in [
            (
                1,
                OWNER_PLAYER,
                Some(yr_sim::WeaponSpec {
                    source: None,
                    damage: 10,
                    rof_ticks: 10,
                    range: Fix::from_int(1),
                    projectile_speed: Fix::ZERO,
                    verses: [yr_rules::Decimal::ONE; 11],
                }),
            ),
            (2, OWNER_PLAYER, None),
            (6, yr_sim::OWNER_ENEMY, None),
        ] {
            let id = sim.next_id_peek();
            sim.step(&[
                Command::Spawn {
                    kind: 0,
                    x: Fix::from_int(x),
                    y: Fix::from_int(2),
                    loco: Loco::Track,
                    speed: Fix::ONE,
                    hp: 100,
                    armor: 0,
                    weapon,
                },
                Command::AssignOwner { id, owner },
            ]);
        }
        sim
    }

    #[test]
    fn absent_tiles_produce_blocked_cursor_and_no_world_orders() {
        let mut sim = scene();
        sim.set_terrain(TerrainGrid::from_tiles(
            0,
            0,
            8,
            6,
            (0..6).flat_map(|y| {
                (0..8).filter_map(move |x| ((x, y) != (3, 2)).then_some((x, y, 0, 13, 0)))
            }),
        ));
        for attack_move in [false, true] {
            let action = orders(&sim, &[1].into(), (3, 2), attack_move);
            assert_eq!(action.cursor, Kind::Blocked);
            assert!(action.commands.is_empty());
        }
    }

    #[test]
    fn same_resolver_selects_and_filters_click_commands_by_ownership_weapon_and_route() {
        let sim = scene();
        let armed = [1].into();
        let unarmed = [2].into();
        let both = [1, 2].into();
        let select = orders(&sim, &armed, (2, 2), false);
        assert_eq!(
            (select.cursor, select.select, select.commands.len()),
            (Kind::Select, Some(2), 0)
        );
        let attack = orders(&sim, &both, (6, 2), false);
        assert_eq!(attack.cursor, Kind::Attack);
        assert!(matches!(
            attack.commands.as_slice(),
            [Command::Attack { id: 1, target: 3 }]
        ));
        assert_eq!(orders(&sim, &unarmed, (6, 2), false).cursor, Kind::Blocked);
        let movement = orders(&sim, &armed, (3, 2), false);
        assert_eq!(movement.cursor, Kind::Move);
        assert!(matches!(
            movement.commands.as_slice(),
            [Command::Move { id: 1, .. }]
        ));
        assert_eq!(orders(&sim, &armed, (3, 2), true).cursor, Kind::AttackMove);
        assert!(
            orders(&sim, &armed, (6, 4), false).commands.is_empty(),
            "disconnected terrain is not advertised as reachable"
        );
        for cell in [(-1, 0), (8, 0), (0, 6), (i32::MAX, 0)] {
            assert_eq!(orders(&sim, &armed, cell, false).cursor, Kind::Blocked);
        }
        assert_eq!(
            orders(&sim, &Default::default(), (6, 2), false).cursor,
            Kind::Pointer
        );
        assert_eq!(
            orders(&sim, &[3].into(), (3, 2), false).cursor,
            Kind::Pointer,
            "stale/enemy selections cannot issue commands"
        );
        let before = sim.state_hash();
        for _ in 0..3 {
            let _ = orders(&sim, &both, (3, 4), false);
        }
        assert_eq!(sim.state_hash(), before);
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn lifted_ground_centres_hit_their_drawn_cell_at_every_supported_scale() {
        for scale in [1, 2, 4] {
            let map = yr_render::RenderedMap {
                width: 100,
                height: 100,
                rgba: Vec::new(),
                summary: String::new(),
                origin_x: -200,
                origin_y: -100,
                scale,
            };
            let grid = yr_sim::TerrainGrid::from_tiles(
                0,
                0,
                20,
                20,
                (0..20).flat_map(|y| {
                    (0..20).map(move |x| (x, y, if (x, y) == (8, 9) { 4 } else { 0 }, 13, 0))
                }),
            );
            let (x, mut y) = map.cell_to_px(8.5, 9.5);
            y -= 4.0 * 15.0 / scale as f32;
            assert_eq!(ground_point(&map, Some(&grid), (8.5, 9.5)), (x, y));
            assert_eq!(
                map.px_to_cell(x, y),
                (6.5, 7.5),
                "old flat inverse hits the wrong cell"
            );
            assert_eq!(cell_at(&map, Some(&grid), (x, y)), (8.5, 9.5));
        }
    }
}
