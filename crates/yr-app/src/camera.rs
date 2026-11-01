//! Native camera policy; timing and bounds are independent of simulation state.

use super::*;

const SPEED: f32 = 600.0;
const EDGE: f64 = 12.0;
const MAX_FRAME: Duration = Duration::from_millis(100);

#[derive(Default)]
pub(super) struct Input {
    frame_at: Option<Instant>,
    pub(super) edge_armed: bool,
}

impl Input {
    fn elapsed(&mut self, now: Instant) -> Duration {
        self.frame_at.replace(now).map_or(Duration::ZERO, |last| {
            now.saturating_duration_since(last).min(MAX_FRAME)
        })
    }
}

impl View {
    fn pan(&mut self, direction: (f32, f32), elapsed: Duration) {
        let length = direction.0.hypot(direction.1);
        if length > 0.0 && !elapsed.is_zero() {
            let step = SPEED * elapsed.as_secs_f32() / (self.zoom * length);
            self.cam_x += direction.0 * step;
            self.cam_y += direction.1 * step;
            self.dirty = true;
        }
    }

    fn constrain(&mut self, map: &yr_render::RenderedMap, viewport: (u32, u32)) {
        fn axis(position: f32, extent: u32, visible: f32) -> f32 {
            let spare = extent as f32 - visible;
            if spare <= 0.0 {
                spare * 0.5
            } else {
                position.clamp(0.0, spare)
            }
        }
        let before = (self.cam_x, self.cam_y);
        self.cam_x = axis(self.cam_x, map.width, viewport.0 as f32 / self.zoom);
        self.cam_y = axis(self.cam_y, map.height, viewport.1 as f32 / self.zoom);
        self.dirty |= before != (self.cam_x, self.cam_y);
    }
}

fn edge_direction(cursor: (f64, f64), viewport: (u32, u32)) -> (f32, f32) {
    let (x, y) = cursor;
    let (w, h) = (f64::from(viewport.0), f64::from(viewport.1));
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.0
        || y < 0.0
        || x >= w
        || y >= h
        || w < EDGE * 2.0
        || h < EDGE * 2.0
    {
        return (0.0, 0.0);
    }
    let axis = |p: f64, size: f64| {
        if p < EDGE {
            -1.0
        } else if p >= size - EDGE {
            1.0
        } else {
            0.0
        }
    };
    (axis(x, w), axis(y, h))
}

impl App {
    pub(super) fn camera_size(&self) -> (u32, u32) {
        self.pixels.as_ref().map_or((WIN_W, WIN_H), |p| {
            (p.texture().width(), p.texture().height())
        })
    }

    pub(super) fn bound_camera(&mut self, size: (u32, u32)) {
        if let Some(map) = &self.map {
            self.view
                .constrain(map, (size.0.saturating_sub(SIDEBAR_W), size.1));
        }
    }

    pub(super) fn zoom_camera(&mut self, zoom: f32) {
        self.cancel_pointer_action();
        self.view.zoom = zoom.clamp(0.25, 4.0);
        self.bound_camera(self.camera_size());
        self.view.dirty = true;
    }

    /// Shared redraw handler. A delayed frame never catches up with a large pan.
    pub(super) fn update_camera(&mut self, now: Instant, size: (u32, u32)) {
        let elapsed = self.camera.elapsed(now);
        self.bound_camera(size);
        if self.menu.visible()
            || !self.pointer.focused
            || !self
                .session
                .as_ref()
                .is_some_and(|s| s.outcome == Outcome::Ongoing)
        {
            return;
        }
        let held = |key| i32::from(self.keys.contains(&key));
        let keyboard = (
            (held(KeyCode::ArrowRight) - held(KeyCode::ArrowLeft)) as f32,
            (held(KeyCode::ArrowDown) - held(KeyCode::ArrowUp)) as f32,
        );
        let direction = if keyboard != (0.0, 0.0) {
            self.cancel_pointer_action();
            keyboard
        } else if self.menu.edge_scroll
            && self.camera.edge_armed
            && self.pointer.inside
            && self.left_down.is_none()
            && self.right_down.is_none()
            && !self.right_panning
            && !self.radar_drag
        {
            edge_direction(self.cursor, (size.0.saturating_sub(SIDEBAR_W), size.1))
        } else {
            (0.0, 0.0)
        };
        self.view.pan(direction, elapsed);
        self.bound_camera(size);
    }
}

#[cfg(test)]
mod tests;
