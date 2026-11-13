//! Classic H navigation, separate from the native Ctrl+H harvest order.
use super::*;

impl App {
    /// Frame installed matches without issuing input or changing saved state.
    pub(super) fn center_match_view(&mut self) {
        let target = self.session.as_ref().and_then(|session| {
            let sim = &session.sim;
            let building = sim
                .buildings()
                .iter()
                .filter(|b| b.owner == yr_sim::OWNER_PLAYER && b.hp > 0)
                .min_by_key(|b| (b.role != yr_sim::Role::ConstructionYard, b.id));
            // A map without a yard still starts at its owned force. Stable IDs
            // avoid depending on entity storage order or consuming simulation RNG.
            building.map(|b| b.centre()).or_else(|| {
                sim.units()
                    .iter()
                    .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.hp > 0)
                    .min_by_key(|u| u.id)
                    .map(|u| (u.x, u.y))
            })
        });
        let size = self.camera_size();
        let viewport = (size.0.saturating_sub(SIDEBAR_W), size.1);
        if let Some((x, y)) = target {
            self.center_world_at((x.raw() as f32 / 256.0, y.raw() as f32 / 256.0), viewport);
        } else if let Some(map) = &self.map {
            self.view.center_map(map, viewport);
        }
        self.bound_camera(size);
    }

    pub(super) fn base_key(&mut self, code: KeyCode, repeat: bool, now: Instant) -> bool {
        if code != KeyCode::KeyH {
            return false;
        }
        if self.control && !self.shift && !self.super_key {
            return false;
        }
        if repeat || self.menu.visible() || !self.pointer.focused || self.shift || self.super_key {
            return true;
        }
        let (Some(session), Some(_)) = (&self.session, &self.map) else {
            return true;
        };
        if session.outcome != Outcome::Ongoing {
            return true;
        }
        let base = session
            .sim
            .buildings()
            .iter()
            .filter(|b| {
                b.owner == yr_sim::OWNER_PLAYER
                    && b.hp > 0
                    && b.role == yr_sim::Role::ConstructionYard
            })
            .min_by_key(|b| b.id)
            .map(|b| b.centre())
            .map(|(x, y)| (x.raw() as f32 / 256.0, y.raw() as f32 / 256.0));
        self.groups.interrupt();
        self.cancel_pointer_action();
        if let Some(cell) = base {
            let size = self.camera_size();
            self.center_world_at(cell, (size.0.saturating_sub(SIDEBAR_W), size.1));
            self.bound_camera(size);
        }
        self.order_feedback.base_navigation(base.is_some(), now);
        self.view.dirty = true;
        true
    }
}
