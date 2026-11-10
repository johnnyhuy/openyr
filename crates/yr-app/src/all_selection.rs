//! Battlefield-wide unit selection documented by the Yuri's Revenge manual.
use super::*;

impl App {
    pub(super) fn select_all_key(&mut self, code: KeyCode, repeat: bool, now: Instant) -> bool {
        if code != KeyCode::KeyP {
            return false;
        }
        if repeat
            || self.menu.visible()
            || !self.pointer.focused
            || self.control
            || self.shift
            || self.super_key
            || self.map.is_none()
        {
            return true;
        }
        let Some(session) = &self.session else {
            return true;
        };
        if session.outcome != Outcome::Ongoing {
            return true;
        }
        let selected = session
            .sim
            .units()
            .iter()
            .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.hp > 0)
            .map(|u| u.id)
            .collect();
        let changed = self.selected != selected;
        self.selected = selected;
        self.interrupt_type_press();
        self.groups.interrupt();
        self.cancel_pointer_action();
        self.placing = None;
        self.service_mode = ServiceMode::None;
        self.attack_move = false;
        self.order_feedback.all_selection(self.selected.len(), now);
        if changed && !self.selected.is_empty() {
            self.selection_audio();
        }
        self.view.dirty = true;
        true
    }
}
