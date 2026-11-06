//! Physical-window input must belong to the drawn framebuffer before routing.
use super::*;
use yr_app::menu::{Layout, Page};

impl App {
    pub(super) fn invalidate_pointer(&mut self) {
        self.pointer.inside = false;
        self.cursor = (-1.0, -1.0);
        self.last_cursor = self.cursor;
        self.cancel_pointer_action();
        self.view.dirty = true;
    }

    pub(super) fn pointer_moved(&mut self, physical: (f64, f64)) {
        let mapped = if !(physical.0 as f32).is_finite() || !(physical.1 as f32).is_finite() {
            None
        } else {
            self.pixels.as_ref().map_or(Some(physical), |pixels| {
                pixels
                    .window_pos_to_pixel((physical.0 as f32, physical.1 as f32))
                    .ok()
                    .map(|(x, y)| (x as f64, y as f64))
            })
        };
        self.pointer_frame_moved(mapped);
    }

    fn pointer_frame_moved(&mut self, mapped: Option<(f64, f64)>) {
        let Some(position) = mapped else {
            self.invalidate_pointer();
            return;
        };
        self.cursor = position;
        self.pointer.inside = self.pointer.focused;
        if !self.pointer.active(self.camera_size(), position) {
            self.invalidate_pointer();
            return;
        }
        let previous = self.last_cursor;
        self.last_cursor = position;
        self.camera.edge_armed = !self.menu.visible();
        self.view.dirty = true;
        if self.menu.visible() {
            let size = self.camera_size();
            let local = Layout::for_state(&self.menu, size.0, size.1).local(position);
            if self.menu.front && self.menu.page == Page::Skirmish {
                if let Some((x, _)) = local {
                    self.menu.setup.drag_to(x);
                }
            }
            if self.menu.page == Page::ChooseMap {
                if let Some((_, y)) = local {
                    self.menu.maps.drag_to(y);
                }
            }
            return;
        }
        if self.radar_drag {
            if let Some(cell) = self.radar_target(position) {
                self.radar_pan(cell);
            }
        }
        if self.right_down.is_some() {
            let dx = (position.0 - previous.0) as f32;
            let dy = (position.1 - previous.1) as f32;
            if dx.abs() + dy.abs() > 2.0 {
                self.right_panning = true;
            }
            if self.right_panning {
                self.left_down = None;
                self.view.cam_x -= dx / self.view.zoom;
                self.view.cam_y -= dy / self.view.zoom;
                self.bound_camera(self.camera_size());
            }
        }
    }

    pub(super) fn pointer_button(&mut self, state: ElementState, button: MouseButton) {
        if !self.pointer.active(self.camera_size(), self.cursor) {
            self.invalidate_pointer();
            return;
        }
        if self.menu.visible() {
            self.menu_mouse(state, button);
            return;
        }
        match (button, state) {
            (MouseButton::Left, ElementState::Pressed) if self.in_sidebar(self.cursor) => {
                self.sidebar_left_press(self.camera_size());
            }
            (MouseButton::Right, ElementState::Pressed) if self.in_sidebar(self.cursor) => {
                self.sidebar_right_press(self.camera_size());
            }
            (MouseButton::Right, ElementState::Pressed)
                if self.service_mode != ServiceMode::None =>
            {
                self.order_feedback.clear();
                self.service_mode = ServiceMode::None;
                self.view.dirty = true;
            }
            (MouseButton::Right, ElementState::Pressed) if self.placing.is_some() => {
                self.order_feedback.clear();
                self.placing = None;
                self.view.dirty = true;
            }
            (MouseButton::Left, ElementState::Pressed) => self.left_press(),
            (MouseButton::Left, ElementState::Released) => self.left_release(),
            (MouseButton::Right, ElementState::Pressed) => {
                self.right_down = Some(self.cursor);
                self.right_panning = false;
            }
            (MouseButton::Right, ElementState::Released) => self.right_release(),
            _ => {}
        }
    }

    pub(super) fn pointer_wheel(&mut self, delta: MouseScrollDelta) {
        if !self.pointer.active(self.camera_size(), self.cursor) {
            return;
        }
        if self.menu.visible() {
            let y = match delta {
                MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                MouseScrollDelta::PixelDelta(p) => p.y,
            };
            if y.is_finite() && y != 0.0 {
                self.menu_scroll(if y < 0.0 { 1 } else { -1 });
                self.view.dirty = true;
            }
        } else {
            self.match_wheel(delta, self.camera_size());
        }
    }
}

#[cfg(test)]
mod tests;
