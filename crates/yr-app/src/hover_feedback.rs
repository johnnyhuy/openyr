//! Native target previews derived from the same resolver as world clicks.
use super::*;
use yr_app::cursor::Kind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Target {
    id: u32,
    color: [u8; 4],
}

impl App {
    fn current_attack_targets(&self) -> BTreeSet<u32> {
        let Some(session) = &self.session else {
            return BTreeSet::new();
        };
        if !self.pointer.focused
            || self.menu.visible()
            || session.outcome != Outcome::Ongoing
            || self.placing.is_some()
            || self.service_mode != ServiceMode::None
        {
            return BTreeSet::new();
        }
        session
            .sim
            .units()
            .iter()
            .filter(|u| {
                self.selected.contains(&u.id)
                    && u.owner == yr_sim::OWNER_PLAYER
                    && u.hp > 0
                    && u.weapon.is_some_and(|w| w.damage > 0)
            })
            .filter_map(|u| u.attack)
            .filter(|id| {
                session
                    .sim
                    .units()
                    .iter()
                    .any(|u| u.id == *id && u.hp > 0 && u.owner != yr_sim::OWNER_PLAYER)
                    || session
                        .sim
                        .buildings()
                        .iter()
                        .any(|b| b.id == *id && b.hp > 0 && b.owner != yr_sim::OWNER_PLAYER)
            })
            .collect()
    }

    pub(super) fn draw_attack_targets(&self, frame: &mut [u8], size: (u32, u32)) {
        for id in self.current_attack_targets() {
            self.draw_target_brackets(
                frame,
                size,
                Target {
                    id,
                    color: ENEMY_COL,
                },
            );
        }
    }

    fn hover_target(&self, size: (u32, u32)) -> Option<Target> {
        if !self.pointer.active(size, self.cursor)
            || self.menu.visible()
            || self.in_sidebar(self.cursor)
            || self.left_down.is_some()
            || self.right_down.is_some()
            || self.right_panning
            || self.radar_drag
            || self.placing.is_some()
            || self.service_mode != ServiceMode::None
            || self.edge_cursor(size).is_some()
            || self.session.as_ref()?.outcome != Outcome::Ongoing
        {
            return None;
        }
        let action = self.world_action(self.cursor)?;
        if let Some(id) = action.select {
            return Some(Target {
                id,
                color: PLAYER_COL,
            });
        }
        // Shift alone routes the click to additive selection instead of orders.
        if self.shift && !self.control {
            return None;
        }
        if action.cursor == Kind::Attack {
            if let Some(Command::Attack { target, .. }) = action.commands.first() {
                return Some(Target {
                    id: *target,
                    color: ENEMY_COL,
                });
            }
        }
        None
    }

    pub(super) fn draw_hover_target(&self, frame: &mut [u8], size: (u32, u32)) {
        let Some(target) = self.hover_target(size) else {
            return;
        };
        self.draw_target_brackets(frame, size, target);
    }

    fn draw_target_brackets(&self, frame: &mut [u8], size: (u32, u32), target: Target) {
        let (Some(map), Some(session)) = (&self.map, &self.session) else {
            return;
        };
        let object = session
            .sim
            .units()
            .iter()
            .find(|u| u.id == target.id && u.hp > 0)
            .map(|u| ((u.x, u.y), 14.0 * self.view.zoom))
            .or_else(|| {
                session
                    .sim
                    .buildings()
                    .iter()
                    .find(|b| b.id == target.id && b.hp > 0)
                    .map(|b| (b.centre(), f32::from(b.size.0) + f32::from(b.size.1)))
                    .map(|(cell, span)| {
                        (cell, span * 15.0 / map.scale.max(1) as f32 * self.view.zoom)
                    })
            });
        let Some(((x, y), half)) = object else { return };
        let (x, y) = yr_app::interaction::ground_point(
            map,
            session.sim.terrain(),
            (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
        );
        draw_brackets(
            frame,
            size,
            (
                (x - self.view.cam_x) * self.view.zoom,
                (y - self.view.cam_y) * self.view.zoom,
            ),
            half.max(8.0) + 6.0,
            target.color,
        );
    }
}

/// Six-pixel corners track the ground footprint, clipped to the world viewport.
fn draw_brackets(
    frame: &mut [u8],
    size: (u32, u32),
    center: (f32, f32),
    half: f32,
    color: [u8; 4],
) {
    let (width, height) = size;
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        != Some(frame.len())
        || !center.0.is_finite()
        || !center.1.is_finite()
        || !half.is_finite()
        || half <= 0.0
    {
        return;
    }
    let world = width.saturating_sub(SIDEBAR_W);
    let rx = half.min(width as f32 + 16.0);
    let ry = (half * 0.5).min(height as f32 + 16.0);
    if center.0 < -rx
        || center.0 > world as f32 + rx
        || center.1 < -ry
        || center.1 > height as f32 + ry
    {
        return;
    }
    let pixel = |frame: &mut [u8], x: i64, y: i64| {
        if x >= 0 && y >= 0 && x < i64::from(world) && y < i64::from(height) {
            let at = ((y as usize * width as usize) + x as usize) * 4;
            frame[at..at + 4].copy_from_slice(&color);
        }
    };
    for sx in [-1, 1] {
        for sy in [-1, 1] {
            let x = center.0.round() as i64 + sx * rx.round() as i64;
            let y = center.1.round() as i64 + sy * ry.round() as i64;
            for step in 0..6 {
                pixel(frame, x - sx * step, y);
                pixel(frame, x, y - sy * step);
            }
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod current_tests;
