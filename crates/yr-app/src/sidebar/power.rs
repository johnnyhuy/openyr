//! Power presentation observes the same owner totals as production and radar.

use super::{Canvas, Session, GOLD, TOP, WIDTH};

const LEFT: i32 = 3;
const PIP_WIDTH: i32 = 12;
const PIP_HEIGHT: i32 = 2;
const PITCH: i32 = 3;
const CAP_HEIGHT: u32 = 26;
const RED: [u8; 4] = [240, 48, 32, 255];
const GREEN: [u8; 4] = [32, 224, 48, 255];
const YELLOW: [u8; 4] = [240, 216, 32, 255];

struct Meter {
    bottom: i32,
    rows: i32,
    output: i32,
    used: i32,
    reserve: i32,
    demand: i32,
    shortage: bool,
}

impl Meter {
    fn new(height: u32, (produced, drained): (i64, i64)) -> Option<Self> {
        let bottom = height.min(i32::MAX as u32).saturating_sub(CAP_HEIGHT) as i32;
        let rows = (bottom - TOP).max(0) / PITCH;
        if rows == 0 {
            return None;
        }
        let produced = i128::from(produced.max(0));
        let drained = i128::from(drained.max(0));
        // Provisional display scale. Keep both values in range;
        // redraws and save/load need no UI history.
        let mut scale = 400i128;
        while scale < produced.max(drained) {
            scale *= 2;
        }
        let level = |amount: i128| ((amount * i128::from(rows) + scale - 1) / scale) as i32;
        Some(Self {
            bottom,
            rows,
            output: level(produced),
            used: level(produced.min(drained)),
            // The manual identifies yellow and green as surplus. Its exact
            // boundary is unmeasured; use the first tenth of output for yellow.
            reserve: level(produced.min(drained + (produced + 9) / 10)),
            demand: level(drained),
            shortage: drained > produced,
        })
    }

    fn y(&self, row: i32) -> i32 {
        self.bottom - (row + 1) * PITCH
    }

    fn pip(&self, row: i32) -> usize {
        if row < self.used {
            0
        } else if row < self.reserve {
            2
        } else {
            1
        }
    }
}

pub(super) fn contains(width: u32, height: u32, cursor: (f64, f64)) -> bool {
    let Some(meter) = Meter::new(height, (0, 0)) else {
        return false;
    };
    let x = width.saturating_sub(WIDTH) as i32 + LEFT;
    super::contains((x, TOP), (PIP_WIDTH + 5, meter.bottom - TOP), cursor)
        && cursor.0 < f64::from(width)
}

pub(super) fn tooltip((produced, drained): (i64, i64)) -> Vec<String> {
    let (produced, drained) = (produced.max(0), drained.max(0));
    vec![
        format!("{produced} output"),
        format!("{drained} used"),
        if produced < drained {
            format!("{} short", drained - produced)
        } else {
            format!("{} spare", produced - drained)
        },
    ]
}

pub(super) fn draw(canvas: &mut Canvas<'_>, session: &Session) {
    let Some(meter) = Meter::new(canvas.height, session.sim.power()) else {
        return;
    };
    draw_meter(canvas, &meter, &session.sidebar_art.power);
}

fn draw_meter(canvas: &mut Canvas<'_>, meter: &Meter, art: &[super::art::Image]) {
    let x = canvas.width.saturating_sub(WIDTH) as i32 + LEFT;
    let pip_width = art.first().map_or(PIP_WIDTH, |image| image.width as i32);
    for row in 0..meter.output.min(meter.rows) {
        let pip = meter.pip(row);
        let origin = (x, meter.y(row));
        if let Some(image) = art.get(pip) {
            canvas.image(origin, image);
        } else {
            canvas.rect(origin, (PIP_WIDTH, PIP_HEIGHT), [RED, GREEN, YELLOW][pip]);
        }
    }
    // A separate demand tick remains visible above output during a shortage,
    // including when no generators survive. It never exaggerates output.
    if meter.demand > 0 {
        canvas.rect(
            (x + pip_width + 1, meter.y(meter.demand - 1)),
            (4, PIP_HEIGHT),
            if meter.shortage { RED } else { GOLD },
        );
    }
}

#[cfg(test)]
mod tests;
