//! Native visible-type expansion using the shared drawn-ground selection bounds.
//! A second deliberate press within the team double-press window expands the
//! same types across the whole map, as the manual documents for Type Select.
use super::*;

pub(super) const TYPE_DOUBLE_PRESS: Duration = Duration::from_millis(400);

impl App {
    pub(super) fn interrupt_type_press(&mut self) {
        self.type_press = None;
    }
    pub(super) fn type_select_key(
        &mut self,
        code: KeyCode,
        state: ElementState,
        repeat: bool,
        now: Instant,
    ) -> bool {
        if code != KeyCode::KeyT {
            return false;
        }
        if state != ElementState::Pressed
            || repeat
            || self.menu.visible()
            || !self.pointer.focused
            || self.control
            || self.shift
            || self.super_key
        {
            return true;
        }
        let (Some(session), Some(map)) = (&self.session, &self.map) else {
            return true;
        };
        if session.outcome != Outcome::Ongoing {
            return true;
        }
        let size = self.camera_size();
        let Some(bounds) = selection_box::Bounds::new(
            (0.0, 0.0),
            (
                f64::from(size.0.saturating_sub(SIDEBAR_W)),
                f64::from(size.1),
            ),
            size,
        ) else {
            return true;
        };
        let types: BTreeSet<_> = session
            .sim
            .units()
            .iter()
            .filter(|u| {
                u.owner == yr_sim::OWNER_PLAYER && u.hp > 0 && self.selected.contains(&u.id)
            })
            .map(|u| u.kind)
            .collect();
        let global = !types.is_empty()
            && self
                .type_press
                .as_ref()
                .is_some_and(|(at, prev_types, prev_selected)| {
                    *prev_types == types
                        && *prev_selected == self.selected
                        && now >= *at
                        && now.duration_since(*at) <= TYPE_DOUBLE_PRESS
                });
        let selected: BTreeSet<_> = if global {
            session
                .sim
                .units()
                .iter()
                .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.hp > 0 && types.contains(&u.kind))
                .map(|u| u.id)
                .collect()
        } else {
            let visible: BTreeSet<_> = bounds.ids(map, &session.sim, &self.view).collect();
            session
                .sim
                .units()
                .iter()
                .filter(|u| {
                    u.owner == yr_sim::OWNER_PLAYER
                        && u.hp > 0
                        && (self.selected.contains(&u.id)
                            || (types.contains(&u.kind) && visible.contains(&u.id)))
                })
                .map(|u| u.id)
                .collect()
        };
        let changed = self.selected != selected;
        self.selected = selected;
        self.type_press = (!types.is_empty()).then(|| (now, types, self.selected.clone()));
        self.groups.interrupt();
        self.cancel_pointer_action();
        self.placing = None;
        self.service_mode = ServiceMode::None;
        self.attack_move = false;
        self.order_feedback.type_selection(self.selected.len(), now);
        if changed && !self.selected.is_empty() {
            self.selection_audio();
        }
        self.view.dirty = true;
        true
    }
}
