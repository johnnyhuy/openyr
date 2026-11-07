//! Authored side-specific repair/sell controls at native pixel coordinates.

use super::{Canvas, Session, Typeface, WIDTH};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ServiceMode {
    #[default]
    None,
    Repair,
    Sell,
}

impl ServiceMode {
    pub fn toggle(&mut self, mode: Self) {
        *self = if *self == mode { Self::None } else { mode };
    }

    /// The building whose footprint contains the cell, if any.
    pub fn target(self, session: &Session, cell: (i32, i32)) -> Option<u32> {
        session
            .sim
            .buildings()
            .iter()
            .find(|b| {
                cell.0 >= i32::from(b.cell.0)
                    && cell.1 >= i32::from(b.cell.1)
                    && cell.0 < i32::from(b.cell.0) + i32::from(b.size.0)
                    && cell.1 < i32::from(b.cell.1) + i32::from(b.size.1)
            })
            .map(|b| b.id)
    }

    pub fn command(self, session: &Session, cell: (i32, i32)) -> Option<yr_sim::Command> {
        let id = self.target(session, cell)?;
        match self {
            Self::None => None,
            Self::Repair => session.repair(id),
            Self::Sell => session.sell(id),
        }
    }
}

fn origin(width: u32, mode: ServiceMode) -> (i32, i32) {
    (
        width.saturating_sub(WIDTH) as i32 + if mode == ServiceMode::Sell { 84 } else { 20 },
        166,
    )
}

/// The transparent corners do not steal clicks from the sidebar surround.
pub fn control_at(width: u32, session: &Session, pos: (f64, f64)) -> Option<ServiceMode> {
    [ServiceMode::Repair, ServiceMode::Sell]
        .into_iter()
        .find(|&mode| {
            let (x, y) = origin(width, mode);
            let images = if mode == ServiceMode::Repair {
                &session.sidebar_art.repair
            } else {
                &session.sidebar_art.sell
            };
            images.first().is_some_and(|image| {
                if !super::contains((x, y), (image.width as i32, image.height as i32), pos) {
                    return false;
                }
                let dx = pos.0.floor() as i32 - x;
                let dy = pos.1.floor() as i32 - y;
                image.pixels[((dy * image.width as i32 + dx) * 4 + 3) as usize] != 0
            })
        })
}

pub(super) fn paint(canvas: &mut Canvas<'_>, session: &Session, mode: ServiceMode) {
    for (button, images) in [
        (ServiceMode::Repair, &session.sidebar_art.repair),
        (ServiceMode::Sell, &session.sidebar_art.sell),
    ] {
        if let Some(image) = images.get(usize::from(mode == button)) {
            canvas.image(origin(canvas.width, button), image);
        }
    }
}

pub fn draw_controls(
    frame: &mut [u8],
    width: u32,
    height: u32,
    session: &Session,
    mode: ServiceMode,
) {
    paint(
        &mut Canvas {
            frame,
            width,
            height,
            face: Typeface(session.sidebar_art.font.as_ref()),
        },
        session,
        mode,
    );
}
