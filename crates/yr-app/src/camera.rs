//! Native camera policy; timing and bounds are independent of simulation state.

use super::*;
use yr_app::cursor::{Direction, Kind};

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

    fn available_scroll(
        &self,
        requested: Direction,
        map: &yr_render::RenderedMap,
        viewport: (u32, u32),
    ) -> Option<Direction> {
        let (x, y) = requested.axes();
        let axis = |direction: i8, position: f32, extent: u32, visible: u32| {
            let limit = extent as f32 - visible as f32 / self.zoom;
            if limit <= 0.0
                || direction < 0 && position <= 0.0
                || direction > 0 && position >= limit
            {
                0
            } else {
                direction
            }
        };
        Direction::from_axes(
            axis(x, self.cam_x, map.width, viewport.0),
            axis(y, self.cam_y, map.height, viewport.1),
        )
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

    /// The same guarded, bounds-aware intent drives both cursor and panning.
    pub(super) fn edge_scroll(&self, size: (u32, u32)) -> Option<(Direction, bool)> {
        if !self.menu.edge_scroll
            || self.menu.visible()
            || !self.camera.edge_armed
            || !self.pointer.inside
            || !self.pointer.focused
            || self.left_down.is_some()
            || self.right_down.is_some()
            || self.right_panning
            || self.radar_drag
            || self.keys.iter().any(|k| {
                matches!(
                    k,
                    KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::ArrowUp
                        | KeyCode::ArrowDown
                )
            })
            || !self
                .session
                .as_ref()
                .is_some_and(|s| s.outcome == Outcome::Ongoing)
        {
            return None;
        }
        let viewport = (size.0.saturating_sub(SIDEBAR_W), size.1);
        let (x, y) = edge_direction(self.cursor, viewport);
        let requested = Direction::from_axes(x as i8, y as i8)?;
        let available = self
            .view
            .available_scroll(requested, self.map.as_ref()?, viewport);
        Some((available.unwrap_or(requested), available.is_none()))
    }

    pub(super) fn edge_cursor(&self, size: (u32, u32)) -> Option<Kind> {
        self.edge_scroll(size)
            .map(|(direction, blocked)| Kind::Scroll(direction, blocked))
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
        } else if let Some((direction, false)) = self.edge_scroll(size) {
            let (x, y) = direction.axes();
            (f32::from(x), f32::from(y))
        } else {
            (0.0, 0.0)
        };
        self.view.pan(direction, elapsed);
        self.bound_camera(size);
    }
}

#[cfg(test)]
mod tests;
