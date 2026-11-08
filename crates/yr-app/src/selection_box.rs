//! Shared visible-world bounds for rubber-band drawing and unit picking.
use super::{set_px, View, SIDEBAR_W};
use yr_render::RenderedMap;
use yr_sim::{Sim, OWNER_PLAYER};

#[derive(Clone, Copy)]
pub(super) struct Bounds {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
    width: u32,
    height: u32,
}

impl Bounds {
    pub(super) fn new(a: (f64, f64), b: (f64, f64), size: (u32, u32)) -> Option<Self> {
        let width = size.0.saturating_sub(SIDEBAR_W);
        let height = size.1;
        if width == 0 || height == 0 || ![a.0, a.1, b.0, b.1].into_iter().all(f64::is_finite) {
            return None;
        }
        let left = a.0.min(b.0).max(0.0);
        let right = a.0.max(b.0).min(f64::from(width));
        let top = a.1.min(b.1).max(0.0);
        let bottom = a.1.max(b.1).min(f64::from(height));
        if left >= f64::from(width) || top >= f64::from(height) || right < left || bottom < top {
            return None;
        }
        Some(Self {
            left,
            right,
            top,
            bottom,
            width,
            height,
        })
    }

    pub(super) fn contains(self, point: (f64, f64)) -> bool {
        point.0 >= self.left
            && point.0 <= self.right
            && point.0 < f64::from(self.width)
            && point.1 >= self.top
            && point.1 <= self.bottom
            && point.1 < f64::from(self.height)
    }

    pub(super) fn ids<'a>(
        self,
        map: &'a RenderedMap,
        sim: &'a Sim,
        view: &'a View,
    ) -> impl Iterator<Item = u32> + 'a {
        sim.units().iter().filter_map(move |u| {
            if u.owner != OWNER_PLAYER {
                return None;
            }
            let (mx, my) = yr_app::interaction::ground_point(
                map,
                sim.terrain(),
                (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0),
            );
            self.contains((
                f64::from((mx - view.cam_x) * view.zoom),
                f64::from((my - view.cam_y) * view.zoom),
            ))
            .then_some(u.id)
        })
    }

    pub(super) fn draw(self, frame: &mut [u8], fw: u32, fh: u32, color: [u8; 4]) {
        let x0 = self.left as i32;
        let x1 = self.right.min(f64::from(self.width - 1)) as i32;
        let y0 = self.top as i32;
        let y1 = self.bottom.min(f64::from(self.height - 1)) as i32;
        for x in x0..=x1 {
            set_px(frame, fw, fh, x, y0, color);
            set_px(frame, fw, fh, x, y1, color);
        }
        for y in y0..=y1 {
            set_px(frame, fw, fh, x0, y, color);
            set_px(frame, fw, fh, x1, y, color);
        }
    }
}
