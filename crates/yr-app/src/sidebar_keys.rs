//! Production-tab shortcuts documented by the Yuri's Revenge manual.
use super::*;

impl App {
    pub(super) fn sidebar_key(&mut self, code: KeyCode, repeat: bool) -> bool {
        let category = match code {
            KeyCode::KeyQ => sidebar::Category::Structures,
            KeyCode::KeyW => sidebar::Category::Defences,
            KeyCode::KeyE => sidebar::Category::Infantry,
            KeyCode::KeyR => sidebar::Category::Vehicles,
            _ => return false,
        };
        if repeat
            || self.menu.visible()
            || !self.pointer.focused
            || self.control
            || self.shift
            || self.super_key
            || self.map.is_none()
            || !self
                .session
                .as_ref()
                .is_some_and(|s| s.outcome == Outcome::Ongoing)
        {
            return true;
        }
        self.groups.interrupt();
        self.cancel_pointer_action();
        self.sidebar_view.select(category);
        // Follow mouse tabs: dismiss placement/services, retain attack-move.
        self.placing = None;
        self.service_mode = ServiceMode::None;
        self.view.dirty = true;
        true
    }
}
