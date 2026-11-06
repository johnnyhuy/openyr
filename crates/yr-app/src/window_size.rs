//! Bounded logical framebuffer sizes that fit compact physical windows.
use super::*;

pub(super) fn frame_for_surface(physical: (u32, u32), scale: f64) -> Option<(u32, u32)> {
    if physical.0 == 0 || physical.1 == 0 || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let axis = |pixels: u32, maximum: u32| {
        ((f64::from(pixels) / scale).floor() as u32).clamp(1, pixels.min(maximum))
    };
    Some((axis(physical.0, WIN_W), axis(physical.1, WIN_H)))
}

impl App {
    pub(super) fn resize_window(&mut self, physical: (u32, u32), scale: f64) -> Result<(), String> {
        self.invalidate_pointer();
        let Some(frame) = frame_for_surface(physical, scale) else {
            return Ok(());
        };
        if let Some(pixels) = &mut self.pixels {
            pixels
                .resize_surface(physical.0, physical.1)
                .map_err(|e| e.to_string())?;
            if (pixels.texture().width(), pixels.texture().height()) != frame {
                pixels
                    .resize_buffer(frame.0, frame.1)
                    .map_err(|e| e.to_string())?;
            }
        }
        self.frame_size = frame;
        self.view.dirty = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
