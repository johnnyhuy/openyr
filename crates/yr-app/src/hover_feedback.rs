//! Native target previews derived from the same resolver as world clicks.
use super::unit_status::draw_brackets;
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
            self.draw_target_status(
                frame,
                size,
                Target {
                    id,
                    color: ENEMY_COL,
                },
                false,
            );
        }
    }

    fn hover_target(&self, size: (u32, u32)) -> Option<Target> {
        if !self.pointer.active(size, self.cursor)
            || self.menu.visible()
            || self.in_sidebar(self.cursor)
            || self.world_pointer_busy()
            || self.placing.is_some()
            || self.edge_cursor(size).is_some()
            || self.session.as_ref()?.outcome != Outcome::Ongoing
        {
            return None;
        }
        let action = self.world_action(self.cursor)?;
        if self.service_mode != ServiceMode::None {
            let id = action.service_target?;
            return Some(Target {
                id,
                color: if action.commands.is_empty() {
                    ENEMY_COL
                } else {
                    PLAYER_COL
                },
            });
        }
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
        self.draw_target_status(frame, size, target, true);
    }

    fn draw_target_status(
        &self,
        frame: &mut [u8],
        size: (u32, u32),
        target: Target,
        show_health: bool,
    ) {
        let (Some(map), Some(session)) = (&self.map, &self.session) else {
            return;
        };
        let object = session
            .sim
            .units()
            .iter()
            .find(|u| u.id == target.id && u.hp > 0)
            .map(|u| {
                (
                    (u.x, u.y),
                    (14.0 * self.view.zoom).max(5.0),
                    hp_frac(u.hp, u.max_hp),
                )
            })
            .or_else(|| {
                session
                    .sim
                    .buildings()
                    .iter()
                    .find(|b| b.id == target.id && b.hp > 0)
                    .map(|b| {
                        (
                            b.centre(),
                            ((f32::from(b.size.0) + f32::from(b.size.1)) * 15.0
                                / map.scale.max(1) as f32
                                * self.view.zoom)
                                .max(8.0),
                            hp_frac(b.hp, b.max_hp),
                        )
                    })
            });
        let Some(((x, y), half, health)) = object else {
            return;
        };
        let (x, y) = yr_app::interaction::ground_point(
            map,
            session.sim.terrain(),
            (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0),
        );
        let center = (
            (x - self.view.cam_x) * self.view.zoom,
            (y - self.view.cam_y) * self.view.zoom,
        );
        draw_brackets(frame, size, center, half.max(8.0) + 6.0, target.color);
        if show_health {
            // Hover uses the existing status geometry without changing selection.
            unit_status::draw_health(
                frame,
                size,
                (center.0, center.1 - (half.max(8.0) + 6.0) * 0.5 - 6.0),
                half * 2.0,
                health,
            );
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod current_tests;
