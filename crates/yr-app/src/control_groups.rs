//! Documented team keys with native timing and transient selection state.

use super::*;

const DOUBLE_PRESS: Duration = Duration::from_millis(400);
const NOTICE_TIME: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(super) struct ControlGroups {
    members: [BTreeSet<u32>; 9],
    recalled: Option<(usize, Instant)>,
    notice: Option<(String, Instant)>,
}

struct Activation {
    center: bool,
    recalled: bool,
}

impl ControlGroups {
    fn activate(
        &mut self,
        index: usize,
        selected: &mut BTreeSet<u32>,
        owned: &BTreeSet<u32>,
        control: bool,
        shift: bool,
        now: Instant,
    ) -> Activation {
        for group in &mut self.members {
            group.retain(|id| owned.contains(id));
        }
        selected.retain(|id| owned.contains(id));
        let mut center = false;
        let recalled = !control && !self.members[index].is_empty();
        let text = if control {
            self.members[index] = selected.clone();
            self.recalled = None;
            if selected.is_empty() {
                format!("Team {} cleared", index + 1)
            } else {
                format!(
                    "Assigned team {} ({} {})",
                    index + 1,
                    selected.len(),
                    if selected.len() == 1 { "unit" } else { "units" }
                )
            }
        } else if self.members[index].is_empty() {
            self.recalled = None;
            format!("Team {} is empty", index + 1)
        } else {
            let group = &self.members[index];
            center = !shift
                && *selected == *group
                && self.recalled.is_some_and(|(last, at)| {
                    last == index && now >= at && now.duration_since(at) <= DOUBLE_PRESS
                });
            if !shift {
                selected.clear();
            }
            selected.extend(group);
            self.recalled = (!shift).then_some((index, now));
            format!(
                "{} team {} ({} {} selected)",
                if shift { "Added" } else { "Selected" },
                index + 1,
                selected.len(),
                if selected.len() == 1 { "unit" } else { "units" }
            )
        };
        self.notice = Some((text, now));
        Activation { center, recalled }
    }

    pub(super) fn interrupt(&mut self) {
        self.recalled = None;
        self.notice = None;
    }

    pub(super) fn message(&self, now: Instant) -> Option<&str> {
        self.notice.as_ref().and_then(|(text, at)| {
            (now.saturating_duration_since(*at) < NOTICE_TIME).then_some(text.as_str())
        })
    }
}

fn slot(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Digit1 => Some(0),
        KeyCode::Digit2 => Some(1),
        KeyCode::Digit3 => Some(2),
        KeyCode::Digit4 => Some(3),
        KeyCode::Digit5 => Some(4),
        KeyCode::Digit6 => Some(5),
        KeyCode::Digit7 => Some(6),
        KeyCode::Digit8 => Some(7),
        KeyCode::Digit9 => Some(8),
        _ => None,
    }
}

impl App {
    /// Called by the live key handler; menus retain their own number input.
    pub(super) fn group_key(
        &mut self,
        code: KeyCode,
        state: ElementState,
        repeat: bool,
        now: Instant,
    ) -> bool {
        let Some(index) = slot(code) else {
            return false;
        };
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
        let owned: BTreeSet<_> = session
            .sim
            .units()
            .iter()
            .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.hp > 0)
            .map(|u| u.id)
            .collect();
        let activation = self.groups.activate(
            index,
            &mut self.selected,
            &owned,
            self.control,
            self.shift,
            now,
        );
        self.cancel_pointer_action();
        self.attack_move = false;
        self.placing = None;
        self.service_mode = ServiceMode::None;
        if activation.center {
            self.center_group();
        }
        if activation.recalled {
            self.selection_audio();
        }
        self.view.dirty = true;
        true
    }

    fn center_group(&mut self) {
        let (Some(map), Some(session)) = (&self.map, &self.session) else {
            return;
        };
        let points = session
            .sim
            .units()
            .iter()
            .filter(|u| {
                u.owner == yr_sim::OWNER_PLAYER && u.hp > 0 && self.selected.contains(&u.id)
            })
            .map(|u| {
                yr_app::interaction::ground_point(
                    map,
                    session.sim.terrain(),
                    (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0),
                )
            });
        let bounds = points.fold(None, |bounds, (x, y)| {
            Some(
                bounds.map_or((x, y, x, y), |(x0, y0, x1, y1): (f32, f32, f32, f32)| {
                    (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
                }),
            )
        });
        if let Some((x0, y0, x1, y1)) = bounds {
            let (width, height) = self.pixels.as_ref().map_or((WIN_W, WIN_H), |p| {
                (p.texture().width(), p.texture().height())
            });
            self.view.cam_x =
                (x0 + x1) * 0.5 - width.saturating_sub(SIDEBAR_W) as f32 / (2.0 * self.view.zoom);
            self.view.cam_y = (y0 + y1) * 0.5 - height as f32 / (2.0 * self.view.zoom);
        }
    }
}

#[cfg(test)]
mod tests;
