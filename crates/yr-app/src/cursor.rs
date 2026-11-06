//! Retail mouse canvases. Hotspots/cadence are documented in formats/cursors.md.

use yr_assets::AssetFs;
use yr_formats::{Palette, ShpFile};

const WIDTH: usize = 55;
const HEIGHT: usize = 43;

/// Screen directions in the clockwise order of retail scroll canvases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub const ALL: [Self; 8] = [
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];

    pub fn axes(self) -> (i8, i8) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
        }
    }

    pub fn from_axes(x: i8, y: i8) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.axes() == (x, y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pointer,
    Select,
    Move,
    Blocked,
    Attack,
    AttackMove,
    Repair,
    NoRepair,
    Sell,
    NoSell,
    /// True when both requested camera axes are blocked.
    Scroll(Direction, bool),
}

impl Kind {
    pub const ALL: [Self; 26] = [
        Self::Pointer,
        Self::Select,
        Self::Move,
        Self::Blocked,
        Self::Attack,
        Self::AttackMove,
        Self::Repair,
        Self::NoRepair,
        Self::Sell,
        Self::NoSell,
        Self::Scroll(Direction::North, false),
        Self::Scroll(Direction::NorthEast, false),
        Self::Scroll(Direction::East, false),
        Self::Scroll(Direction::SouthEast, false),
        Self::Scroll(Direction::South, false),
        Self::Scroll(Direction::SouthWest, false),
        Self::Scroll(Direction::West, false),
        Self::Scroll(Direction::NorthWest, false),
        Self::Scroll(Direction::North, true),
        Self::Scroll(Direction::NorthEast, true),
        Self::Scroll(Direction::East, true),
        Self::Scroll(Direction::SouthEast, true),
        Self::Scroll(Direction::South, true),
        Self::Scroll(Direction::SouthWest, true),
        Self::Scroll(Direction::West, true),
        Self::Scroll(Direction::NorthWest, true),
    ];

    pub fn frames(self) -> (usize, usize) {
        match self {
            Self::Pointer => (0, 1),
            Self::Select => (18, 13),
            Self::Move => (31, 10),
            Self::Blocked => (41, 1),
            Self::Attack => (53, 5),
            Self::AttackMove => (404, 9),
            Self::Repair => (170, 20),
            Self::NoRepair => (190, 1),
            Self::Sell => (129, 10),
            Self::NoSell => (149, 1),
            Self::Scroll(direction, blocked) => {
                (2 + direction as usize + usize::from(blocked) * 8, 1)
            }
        }
    }

    pub fn frame(self, tick: u64) -> usize {
        let (first, count) = self.frames();
        first + (tick / 2 % count as u64) as usize
    }

    pub fn hotspot(self) -> (i32, i32) {
        if let Self::Scroll(direction, _) = self {
            // Native inward placement keeps the whole canvas visible at an
            // edge. Original scroll hotspots still require observation.
            let (x, y) = direction.axes();
            (27 + i32::from(x) * 27, 21 + i32::from(y) * 21)
        } else if self == Self::Pointer {
            (0, 0)
        } else {
            (27, 21)
        }
    }
}

#[derive(Default)]
pub struct Art {
    frames: Vec<Vec<u8>>,
}

impl Art {
    pub fn load(fs: &AssetFs) -> Self {
        // Yuri's Revenge extends RA2's sheet under a different extension.
        let data = fs.read("MOUSE.SHA").or_else(|| fs.read("MOUSE.SHP"));
        let palette = fs.read("MOUSEPAL.PAL");
        match data
            .zip(palette)
            .and_then(|(data, pal)| Self::decode(data, pal))
        {
            Some(art) => art,
            None => {
                eprintln!("OpenYR: cursor art unavailable; keeping the system pointer");
                Self::default()
            }
        }
    }

    fn decode(data: &[u8], palette: &[u8]) -> Option<Self> {
        let shp = ShpFile::parse(data).ok()?;
        // Bound total decoded UI memory before allocating any canvases.
        if (usize::from(shp.width), usize::from(shp.height)) != (WIDTH, HEIGHT)
            || !matches!(shp.frames.len(), 450 | 517)
        {
            return None;
        }
        let palette = Palette::parse(palette)?;
        let frames = (0..shp.frames.len())
            .map(|i| {
                Some(
                    shp.decode_frame_canvas(i)
                        .ok()?
                        .into_iter()
                        .flat_map(|index| {
                            if index == 0 {
                                [0; 4]
                            } else {
                                palette.rgba(index)
                            }
                        })
                        .collect(),
                )
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self { frames })
    }

    pub fn available(&self) -> bool {
        !self.frames.is_empty()
    }

    /// UI pixels and hotspot coordinates never pass through the map zoom.
    pub fn draw(&self, frame: &mut [u8], size: (u32, u32), pos: (f64, f64), kind: Kind, tick: u64) {
        let (width, height) = size;
        let Some(bytes) = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
        else {
            return;
        };
        if bytes != frame.len()
            || !pos.0.is_finite()
            || !pos.1.is_finite()
            || pos.0 < 0.0
            || pos.1 < 0.0
            || pos.0 >= f64::from(width)
            || pos.1 >= f64::from(height)
        {
            return;
        }
        let Some(image) = self.frames.get(kind.frame(tick)) else {
            return;
        };
        let hotspot = kind.hotspot();
        let (x, y) = (
            pos.0.floor() as i64 - i64::from(hotspot.0),
            pos.1.floor() as i64 - i64::from(hotspot.1),
        );
        for sy in 0..HEIGHT {
            for sx in 0..WIDTH {
                let (dx, dy) = (x + sx as i64, y + sy as i64);
                let source = &image[(sy * WIDTH + sx) * 4..(sy * WIDTH + sx + 1) * 4];
                if source[3] != 0
                    && dx >= 0
                    && dy >= 0
                    && dx < i64::from(width)
                    && dy < i64::from(height)
                {
                    let at = (dy as usize * width as usize + dx as usize) * 4;
                    frame[at..at + 4].copy_from_slice(source);
                }
            }
        }
    }
}

/// Per-window pointer ownership. Leaving/focus loss always restores the OS cursor.
#[derive(Default)]
pub struct Presence {
    pub inside: bool,
    pub focused: bool,
}
impl Presence {
    pub fn custom(&self, art: &Art, size: (u32, u32), pos: (f64, f64)) -> bool {
        self.active(size, pos) && art.available()
    }
    /// Shared ownership gate for custom drawing and pointer input.
    pub fn active(&self, size: (u32, u32), pos: (f64, f64)) -> bool {
        self.inside
            && self.focused
            && pos.0.is_finite()
            && pos.1.is_finite()
            && pos.0 >= 0.0
            && pos.1 >= 0.0
            && pos.0 < f64::from(size.0)
            && pos.1 < f64::from(size.1)
    }
}

#[cfg(test)]
mod tests;
