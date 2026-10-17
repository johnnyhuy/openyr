//! Terrain grid: per-cell ground, height and overlay for movement.
//!
//! Ground classes come from the TMP `terrain_type` byte. The mapping was
//! measured on the retail corpus (dominant type per tile-set filename
//! prefix, temperate/urban/snow — see docs/formats/map.md):
//! 9 water, 10 beach, 11 paved, 12 road, 13 clear, 14 rough, 15 cliff.
//! Rarer values (tunnels, bridge pieces) fall back to clear for now.
//!
//! Speed percentages are a first approximation of the original terrain
//! tables; they will be verified against the oracle in the parity issue.

use crate::Fix;

/// Ground class of a cell's base tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    Clear,
    Water,
    Beach,
    Paved,
    Road,
    Rough,
    Cliff,
}

impl Ground {
    pub fn from_raw(raw: u8) -> Ground {
        match raw {
            9 => Ground::Water,
            10 => Ground::Beach,
            11 => Ground::Paved,
            12 => Ground::Road,
            14 => Ground::Rough,
            15 => Ground::Cliff,
            _ => Ground::Clear,
        }
    }
}

/// How a unit moves; decides passability and speed factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loco {
    Foot,
    Track,
    Ship,
    Fly,
}

/// What sits on the cell from the overlay packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Ore,
    Gem,
    Wall,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ground: Ground,
    pub height: u8,
    /// TMP ramp_type; non-zero means the cell slopes.
    pub ramp: u8,
    pub overlay: Overlay,
    pub building: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { ground: Ground::Clear, height: 0, ramp: 0, overlay: Overlay::None, building: false }
    }
}

/// Speed percentage (of the unit's base speed) or `None` = impassable.
fn speed_pct(ground: Ground, loco: Loco) -> Option<i32> {
    use Ground::*;
    use Loco::*;
    Some(match (ground, loco) {
        (_, Fly) => 100,
        (Water, Ship) => 100,
        (Water, _) => return None,
        (Cliff, _) => return None,
        (_, Ship) => return None,
        (Clear, _) => 100,
        (Rough, Foot) => 80,
        (Rough, Track) => 60,
        (Beach, Foot) => 70,
        (Beach, Track) => 40,
        (Road, Foot) => 100,
        (Road, Track) => 110,
        (Paved, Foot) => 100,
        (Paved, Track) => 120,
    })
}

/// Cell grid over a map rectangle. Coordinates are map cells; the grid
/// remembers the rectangle origin from the map's `Size=`.
#[derive(Debug, Clone)]
pub struct TerrainGrid {
    origin_x: i32,
    origin_y: i32,
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    hash: u64,
}

impl TerrainGrid {
    pub fn new(origin_x: i32, origin_y: i32, width: usize, height: usize) -> TerrainGrid {
        let mut g = TerrainGrid {
            origin_x,
            origin_y,
            width,
            height,
            cells: vec![Cell::default(); width * height],
            hash: 0,
        };
        g.rehash();
        g
    }

    /// Build from map tiles: `(x, y, level, raw_terrain_type, ramp)`.
    pub fn from_tiles(
        origin_x: i32,
        origin_y: i32,
        width: usize,
        height: usize,
        tiles: impl Iterator<Item = (u16, u16, u8, u8, u8)>,
    ) -> TerrainGrid {
        let mut g = TerrainGrid::new(origin_x, origin_y, width, height);
        for (x, y, level, raw, ramp) in tiles {
            if let Some(c) = g.cell_mut(x, y) {
                c.height = level;
                c.ground = Ground::from_raw(raw);
                c.ramp = ramp;
            }
        }
        g.rehash();
        g
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        let (lx, ly) = (i32::from(x) - self.origin_x, i32::from(y) - self.origin_y);
        if lx < 0 || ly < 0 || lx as usize >= self.width || ly as usize >= self.height {
            return None;
        }
        Some(ly as usize * self.width + lx as usize)
    }

    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index(x, y).map(|i| &self.cells[i])
    }

    pub fn cell_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.index(x, y).map(|i| &mut self.cells[i])
    }

    pub fn set_overlay(&mut self, x: u16, y: u16, overlay: Overlay) {
        if let Some(c) = self.cell_mut(x, y) {
            c.overlay = overlay;
        }
        self.rehash();
    }

    /// Mark a building footprint (or clear it with `false`).
    pub fn set_building(&mut self, x: u16, y: u16, w: u16, h: u16, present: bool) {
        for dy in 0..h {
            for dx in 0..w {
                if let Some(c) = self.cell_mut(x + dx, y + dy) {
                    c.building = present;
                }
            }
        }
        self.rehash();
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Scenario fingerprint, mixed into the sim state hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    fn rehash(&mut self) {
        let mut h = crate::hash::Hasher::new();
        h.write_u64(self.cells.len() as u64);
        for c in &self.cells {
            h.write_u64(u64::from(c.height));
            h.write_u64(c.ground as u64);
            h.write_u64(c.overlay as u64);
            h.write_u64(u64::from(c.building));
        }
        self.hash = h.finish();
    }

    /// Speed factor for `loco` entering this cell (`None` = cannot enter).
    /// Walls and buildings block all ground movement; flying ignores
    /// everything except buildings' airspace (allowed in this model).
    pub fn speed_factor(&self, loco: Loco, x: u16, y: u16) -> Option<Fix> {
        let c = self.cell(x, y)?;
        if c.building && loco != Loco::Fly {
            return None;
        }
        if c.overlay == Overlay::Wall && loco != Loco::Fly {
            return None;
        }
        speed_pct(c.ground, loco).map(|pct| Fix::from_raw(pct * 256 / 100))
    }

    pub fn passable(&self, loco: Loco, x: u16, y: u16) -> bool {
        self.speed_factor(loco, x, y).is_some()
    }

    /// Can a ground unit step from `(x0, y0)` to adjacent `(x1, y1)`?
    /// Height steps over 1 level need a ramp on either side.
    pub fn can_step(&self, loco: Loco, x0: u16, y0: u16, x1: u16, y1: u16) -> bool {
        if !self.passable(loco, x1, y1) {
            return false;
        }
        if loco == Loco::Fly || loco == Loco::Ship {
            return true;
        }
        let (Some(a), Some(b)) = (self.cell(x0, y0), self.cell(x1, y1)) else {
            return false;
        };
        let dh = (i16::from(a.height) - i16::from(b.height)).abs();
        dh <= 1 || a.ramp != 0 || b.ramp != 0
    }

    /// Count cells per ground class (debug/tests).
    pub fn ground_counts(&self) -> [(Ground, usize); 7] {
        let mut out = [
            (Ground::Clear, 0),
            (Ground::Water, 0),
            (Ground::Beach, 0),
            (Ground::Paved, 0),
            (Ground::Road, 0),
            (Ground::Rough, 0),
            (Ground::Cliff, 0),
        ];
        for c in &self.cells {
            for e in &mut out {
                if e.0 == c.ground {
                    e.1 += 1;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> TerrainGrid {
        // 3x3: water west, road middle, cliff east; heights rise east.
        let mut tiles = Vec::new();
        for y in 0..3u16 {
            tiles.push((0, y, 0, 9, 0));
            tiles.push((1, y, 0, 12, 0));
            tiles.push((2, y, 3, 15, 0));
        }
        TerrainGrid::from_tiles(0, 0, 3, 3, tiles.into_iter())
    }

    #[test]
    fn known_cells_passability() {
        let g = grid();
        assert_eq!(g.speed_factor(Loco::Track, 0, 0), None, "vehicles cannot enter water");
        assert_eq!(g.speed_factor(Loco::Ship, 0, 0), Some(Fix::ONE));
        assert_eq!(g.speed_factor(Loco::Foot, 2, 0), None, "cliff blocks infantry");
        let road = g.speed_factor(Loco::Track, 1, 0).unwrap();
        assert!(road > Fix::ONE, "roads are fast for tracks: {road:?}");
        assert!(g.passable(Loco::Fly, 2, 2), "aircraft ignore terrain");
    }

    #[test]
    fn walls_buildings_and_height_steps_block() {
        let mut g = grid();
        g.set_overlay(1, 1, Overlay::Wall);
        assert!(!g.passable(Loco::Foot, 1, 1));
        g.set_building(1, 0, 1, 1, true);
        assert!(!g.can_step(Loco::Foot, 1, 1, 1, 0));
        // Height step of 3 without ramps is not climbable.
        assert!(!g.can_step(Loco::Foot, 1, 0, 2, 0));
        if let Some(c) = g.cell_mut(2, 0) {
            c.ramp = 1;
        }
        // Manually re-mark: cliff ground still impassable, ramp does not help.
        assert!(!g.can_step(Loco::Foot, 1, 0, 2, 0), "cliff ground wins over ramp");
    }
}
