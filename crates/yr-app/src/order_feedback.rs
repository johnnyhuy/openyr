//! Brief native match and tactical acknowledgements, outside the simulation.

use super::*;
use yr_app::cursor::Kind;

const LIFETIME: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
enum Target {
    Ground(f32, f32),
    Entity(u32),
}

struct Notice {
    text: String,
    marker: Option<(Target, [u8; 4])>,
    at: Instant,
}

#[derive(Default)]
pub(super) struct Feedback(Option<Notice>);

impl Feedback {
    pub(super) fn loaded(&mut self, path: &std::path::Path, tick: u64, now: Instant) {
        let name: String = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .filter(|ch| !ch.is_control())
            .take(48)
            .collect();
        let name = if name.trim().is_empty() {
            "match"
        } else {
            name.trim()
        };
        self.0 = Some(Notice {
            text: format!("Match loaded (tick {tick}): {name}"),
            marker: None,
            at: now,
        });
    }

    pub(super) fn clear(&mut self) {
        self.0 = None;
    }

    fn current(&self, now: Instant) -> Option<&Notice> {
        self.0
            .as_ref()
            .filter(|n| now >= n.at && now.saturating_duration_since(n.at) < LIFETIME)
    }

    pub(super) fn expire(&mut self, now: Instant) -> bool {
        if self.0.is_some() && self.current(now).is_none() {
            self.clear();
            true
        } else {
            false
        }
    }

    fn record(&mut self, kind: Kind, commands: &[Command], cell: (f32, f32), now: Instant) {
        let target = commands.first().and_then(|command| match *command {
            Command::Move { x, y, .. } | Command::AttackMove { x, y, .. } => Some(Target::Ground(
                x.raw() as f32 / 256.0,
                y.raw() as f32 / 256.0,
            )),
            Command::Attack { target, .. } => Some(Target::Entity(target)),
            _ => None,
        });
        let (text, marker) = if commands.is_empty() {
            (
                "No selected unit can act here".into(),
                Some((
                    Target::Ground(cell.0.floor() + 0.5, cell.1.floor() + 0.5),
                    ENEMY_COL,
                )),
            )
        } else {
            let (name, color) = match kind {
                Kind::Attack => ("Attack", ENEMY_COL),
                Kind::AttackMove => ("Attack-move", [255, 220, 64, 255]),
                _ => ("Move", PLAYER_COL),
            };
            (
                order_text(name, commands.len()),
                target.map(|target| (target, color)),
            )
        };
        self.0 = Some(Notice {
            text,
            marker,
            at: now,
        });
    }

    pub(super) fn message(&self, now: Instant) -> Option<&str> {
        self.current(now).map(|n| n.text.as_str())
    }
}

fn order_text(name: &str, count: usize) -> String {
    format!(
        "{name} order sent ({count} {})",
        if count == 1 { "unit" } else { "units" }
    )
}

impl App {
    pub(super) fn acknowledge_placement_order(
        &mut self,
        command: Option<&Command>,
        cell: (f32, f32),
        now: Instant,
    ) {
        let (text, point, color) = match command {
            Some(Command::PlaceProduced { kind, x, y }) => {
                let (w, h) = self
                    .session
                    .as_ref()
                    .and_then(|s| s.buildables.get(usize::from(*kind)))
                    .and_then(|b| b.footprint())
                    .unwrap_or((1, 1));
                (
                    "Placement order sent",
                    (
                        f32::from(*x) + f32::from(w) / 2.0,
                        f32::from(*y) + f32::from(h) / 2.0,
                    ),
                    PLAYER_COL,
                )
            }
            None if self.placing.is_some() => (
                "Cannot place here",
                (cell.0.floor() + 0.5, cell.1.floor() + 0.5),
                ENEMY_COL,
            ),
            _ => return,
        };
        self.groups.interrupt();
        self.order_feedback.0 = Some(Notice {
            text: text.into(),
            marker: Some((Target::Ground(point.0, point.1), color)),
            at: now,
        });
    }

    pub(super) fn acknowledge_service_order(
        &mut self,
        command: Option<&Command>,
        cell: (f32, f32),
        now: Instant,
    ) {
        let (text, marker) = match command {
            Some(Command::Repair { id, spec }) => (
                if spec.is_some() {
                    "Repair order sent"
                } else {
                    "Cancel repair order sent"
                },
                (Target::Entity(*id), PLAYER_COL),
            ),
            Some(Command::Sell { id, .. }) => {
                ("Sell order sent", (Target::Entity(*id), PLAYER_COL))
            }
            None => {
                let text = match self.service_mode {
                    ServiceMode::Repair => "Cannot repair here",
                    ServiceMode::Sell => "Cannot sell here",
                    ServiceMode::None => return,
                };
                (
                    text,
                    (
                        Target::Ground(cell.0.floor() + 0.5, cell.1.floor() + 0.5),
                        ENEMY_COL,
                    ),
                )
            }
            _ => return,
        };
        self.groups.interrupt();
        self.order_feedback.0 = Some(Notice {
            text: text.into(),
            marker: Some(marker),
            at: now,
        });
    }

    pub(super) fn acknowledge_production_order(
        &mut self,
        command: Option<&Command>,
        right: bool,
        now: Instant,
    ) {
        let text = match command {
            Some(Command::Produce { .. }) => "Build order sent",
            Some(Command::HoldProduce { held: true, .. }) => "Production held",
            Some(Command::HoldProduce { held: false, .. }) => "Production resumed",
            Some(Command::CancelProduce { .. }) => "Build order cancelled",
            None if !right => "Cannot build here",
            _ => return,
        };
        self.groups.interrupt();
        self.order_feedback.0 = Some(Notice {
            text: text.into(),
            marker: None,
            at: now,
        });
    }

    pub(super) fn acknowledge_placement_start(&mut self, now: Instant) {
        self.groups.interrupt();
        self.order_feedback.0 = Some(Notice {
            text: "Select a location to place the structure".into(),
            marker: None,
            at: now,
        });
    }

    pub(super) fn acknowledge_radar_order(
        &mut self,
        commands: &[Command],
        cell: (f32, f32),
        attack_move: bool,
        now: Instant,
    ) {
        self.groups.interrupt();
        self.order_feedback.record(
            if attack_move {
                Kind::AttackMove
            } else {
                Kind::Move
            },
            commands,
            cell,
            now,
        );
    }

    pub(super) fn acknowledge_order(
        &mut self,
        action: &yr_app::interaction::Action,
        pos: (f64, f64),
        now: Instant,
    ) {
        if let Some(cell) = self.window_to_cell(pos) {
            self.groups.interrupt();
            self.order_feedback
                .record(action.cursor, &action.commands, cell, now);
        }
    }

    /// Shared with the live keyboard handler. Repeats and inactive matches do nothing.
    pub(super) fn unit_order_key(
        &mut self,
        code: KeyCode,
        state: ElementState,
        repeat: bool,
        now: Instant,
    ) -> bool {
        if !matches!(code, KeyCode::KeyS | KeyCode::KeyH) {
            return false;
        }
        if state != ElementState::Pressed || repeat || self.menu.visible() || !self.pointer.focused
        {
            return true;
        }
        let Some(session) = &self.session else {
            return true;
        };
        if session.outcome != Outcome::Ongoing {
            return true;
        }
        let harvest = code == KeyCode::KeyH;
        let commands: Vec<_> = session
            .sim
            .units()
            .iter()
            .filter(|u| {
                u.owner == yr_sim::OWNER_PLAYER
                    && u.hp > 0
                    && self.selected.contains(&u.id)
                    && (!harvest || u.role == yr_sim::Role::Harvester)
            })
            .map(|u| {
                if harvest {
                    Command::Harvest { id: u.id }
                } else {
                    Command::Stop { id: u.id }
                }
            })
            .collect();
        let text = if commands.is_empty() {
            if harvest {
                "Select a harvester to harvest"
            } else {
                "Select a unit to stop"
            }
            .into()
        } else {
            order_text(if harvest { "Harvest" } else { "Stop" }, commands.len())
        };
        self.groups.interrupt();
        self.cancel_pointer_action();
        self.placing = None;
        self.service_mode = ServiceMode::None;
        self.attack_move = false;
        self.order_feedback.0 = Some(Notice {
            text,
            marker: None,
            at: now,
        });
        self.pending.extend(commands);
        self.view.dirty = true;
        true
    }

    pub(super) fn draw_order_marker(&self, frame: &mut [u8], size: (u32, u32), now: Instant) {
        let Some((target, color)) = self.order_feedback.current(now).and_then(|n| n.marker) else {
            return;
        };
        let (Some(map), Some(session)) = (&self.map, &self.session) else {
            return;
        };
        let cell = match target {
            Target::Ground(x, y) => Some((x, y)),
            Target::Entity(id) => session
                .sim
                .units()
                .iter()
                .find(|u| u.id == id && u.hp > 0)
                .map(|u| (u.x, u.y))
                .or_else(|| {
                    session
                        .sim
                        .buildings()
                        .iter()
                        .find(|b| b.id == id && b.hp > 0)
                        .map(|b| b.centre())
                })
                .map(|(x, y)| (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0)),
        };
        let Some(cell) = cell else { return };
        let (x, y) = yr_app::interaction::ground_point(map, session.sim.terrain(), cell);
        let (x, y) = (
            (x - self.view.cam_x) * self.view.zoom,
            (y - self.view.cam_y) * self.view.zoom,
        );
        if !x.is_finite()
            || !y.is_finite()
            || x < -16.0
            || y < -16.0
            || x >= size.0.saturating_sub(SIDEBAR_W) as f32 + 16.0
            || y >= size.1 as f32 + 16.0
        {
            return;
        }
        draw_diamond(frame, size.0, size.1, x, y, 14.0, [0, 0, 0, 255]);
        draw_diamond_outline(frame, size.0, size.1, x, y, 12.0, color);
        draw_diamond_outline(frame, size.0, size.1, x, y, 8.0, color);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod production_tests;

#[cfg(test)]
mod radar_tests;

#[cfg(test)]
mod service_tests;

#[cfg(test)]
mod placement_tests;
