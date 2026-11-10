//! F camera following documented by the Yuri's Revenge manual.
use super::*;

impl App {
    pub(super) fn follow_key(&mut self, code: KeyCode, repeat: bool, now: Instant) -> bool {
        if code != KeyCode::KeyF {
            return false;
        }
        if repeat
            || self.menu.visible()
            || !self.pointer.focused
            || self.control
            || self.shift
            || self.super_key
            || self.map.is_none()
            || self.keyboard_pan() != (0.0, 0.0)
        {
            return true;
        }
        let Some(session) = &self.session else {
            return true;
        };
        if session.outcome != Outcome::Ongoing {
            return true;
        }
        let target = session
            .sim
            .units()
            .iter()
            .filter(|u| {
                u.owner == yr_sim::OWNER_PLAYER && u.hp > 0 && self.selected.contains(&u.id)
            })
            .map(|u| u.id)
            .min();
        let was_following = self.camera.follow.is_some();
        self.groups.interrupt();
        self.interrupt_type_press();
        self.cancel_pointer_action();
        self.placing = None;
        self.service_mode = ServiceMode::None;
        self.attack_move = false;
        let message = if was_following {
            "Follow stopped"
        } else if let Some(id) = target {
            self.camera.follow = Some(id);
            self.follow_camera(self.camera_size());
            "Following unit"
        } else {
            "Select a unit to follow"
        };
        self.order_feedback.follow(message, now);
        self.view.dirty = true;
        true
    }

    /// Redraw follows authoritative positions without advancing simulation.
    pub(super) fn follow_camera(&mut self, size: (u32, u32)) {
        let Some(id) = self.camera.follow else { return };
        let cell = self
            .session
            .as_ref()
            .filter(|s| s.outcome == Outcome::Ongoing)
            .and_then(|s| {
                s.sim.units().iter().find(|u| {
                    u.id == id
                        && u.owner == yr_sim::OWNER_PLAYER
                        && u.hp > 0
                        && self.selected.contains(&id)
                })
            })
            .map(|u| (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0));
        let (Some(cell), Some(map)) = (cell, &self.map) else {
            self.camera.follow = None;
            return;
        };
        let before = (self.view.cam_x, self.view.cam_y);
        let dirty = self.view.dirty;
        self.view.center_on_ground(
            map,
            self.session.as_ref().and_then(|s| s.sim.terrain()),
            cell,
            (size.0.saturating_sub(SIDEBAR_W), size.1),
        );
        self.bound_camera(size);
        self.view.dirty = dirty || before != (self.view.cam_x, self.view.cam_y);
    }
}
