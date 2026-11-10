//! Creation-order unit navigation documented by the Yuri's Revenge manual.
use super::*;

impl App {
    pub(super) fn next_unit_key(&mut self, code: KeyCode, repeat: bool, now: Instant) -> bool {
        if code != KeyCode::KeyN {
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
        let mut units: Vec<_> = session
            .sim
            .units()
            .iter()
            .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.hp > 0)
            .map(|u| (u.id, (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0)))
            .collect();
        units.sort_unstable_by_key(|&(id, _)| id);
        let current = units
            .iter()
            .filter(|(id, _)| self.selected.contains(id))
            .map(|&(id, _)| id)
            .max();
        let index = current
            .and_then(|id| units.iter().position(|&(candidate, _)| candidate > id))
            .unwrap_or(0);
        let target = units.get(index).copied();
        let selected = target
            .map(|(id, _)| BTreeSet::from([id]))
            .unwrap_or_default();
        let changed = self.selected != selected;
        self.selected = selected;
        self.interrupt_type_press();
        self.groups.interrupt();
        self.cancel_pointer_action();
        self.placing = None;
        self.service_mode = ServiceMode::None;
        self.attack_move = false;
        if let Some((_, cell)) = target {
            let size = self.camera_size();
            self.center_world_at(cell, (size.0.saturating_sub(SIDEBAR_W), size.1));
            self.bound_camera(size);
        }
        self.order_feedback.unit_cycle(index + 1, units.len(), now);
        if changed && !self.selected.is_empty() {
            self.selection_audio();
        }
        self.view.dirty = true;
        true
    }
}
