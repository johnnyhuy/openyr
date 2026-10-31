//! Radar presentation from retail art, terrain metadata and live sim objects.

use super::{Canvas, WIDTH};
use crate::session::Session;
use std::collections::{BTreeMap, BTreeSet};
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, MapFile, TileSetIndex, TmpFile};
use yr_rules::Rules;
use yr_sim::{Sim, OWNER_PLAYER};

const LAST_FRAME: u8 = 32;
const FRAME_TICKS: u64 = 2;
const INTERIOR: (i32, i32, i32, i32) = (14, 50, 140, 104);

#[derive(Default)]
struct Clock {
    frame: u8,
    enabled: bool,
    tick: Option<u64>,
    remainder: u64,
}

impl Clock {
    fn reset(&mut self, tick: u64, enabled: bool) {
        *self = Self {
            frame: if enabled { LAST_FRAME } else { 0 },
            enabled,
            tick: Some(tick),
            remainder: 0,
        };
    }

    fn observe(&mut self, tick: u64, enabled: bool) {
        if let Some(previous) = self.tick.filter(|&t| t <= tick) {
            if enabled == self.enabled {
                let elapsed = tick.saturating_sub(previous).saturating_add(self.remainder);
                let frames = (elapsed / FRAME_TICKS).min(u64::from(LAST_FRAME)) as u8;
                self.remainder = elapsed % FRAME_TICKS;
                self.frame = if enabled {
                    self.frame.saturating_add(frames).min(LAST_FRAME)
                } else {
                    self.frame.saturating_sub(frames)
                };
            } else {
                self.remainder = 0;
            }
        } else {
            self.reset(tick, enabled);
        }
        self.enabled = enabled;
        self.tick = Some(tick);
    }

    fn ready(&self) -> bool {
        self.enabled && self.frame == LAST_FRAME
    }
}

#[derive(Default)]
struct Terrain {
    width: usize,
    height: usize,
    offset: (i32, i32),
    map_width: i32,
    colors: Vec<[u8; 3]>,
    cells: Vec<Option<(u16, u16)>>,
}

impl Terrain {
    fn load(fs: &AssetFs, map: &MapFile) -> Result<Self, String> {
        let (lx, ly, w, h) = map.local_size;
        if !(1..=512).contains(&w)
            || !(1..=512).contains(&h)
            || !(1..=512).contains(&map.size.2)
            || !(0..=512).contains(&lx)
            || !(0..=1024).contains(&ly)
        {
            return Err("radar rectangle is outside the supported map bounds".into());
        }
        let width = w as usize * 2;
        let height = h as usize;
        let mut out = Self {
            width,
            height,
            offset: (lx * 2, ly),
            map_width: map.size.2,
            colors: vec![[0; 3]; width * height],
            cells: vec![None; width * height],
        };
        let theater = Theater::from_map_name(&map.theater).ok_or("radar theater missing")?;
        let ini = theater
            .tileset_inis()
            .iter()
            .find_map(|name| fs.read(name))
            .ok_or("radar tileset missing")?;
        let ini = IniFile::parse(&String::from_utf8_lossy(ini));
        let index = TileSetIndex::parse(&ini, theater.tile_extension());
        let mut cache = BTreeMap::<u32, Option<TmpFile>>::new();
        for tile in &map.tiles {
            let id = tile.tile.unwrap_or(0);
            let tmp = cache
                .entry(id)
                .or_insert_with(|| TmpFile::parse(fs.read(index.file_for(id)?)?).ok());
            let Some(tile_art) = tmp
                .as_ref()
                .and_then(|t| t.tiles.get(tile.subtile as usize))
                .and_then(Option::as_ref)
            else {
                continue;
            };
            let sx = i32::from(tile.x) - i32::from(tile.y) + out.map_width - 1 - out.offset.0;
            let sy = (i32::from(tile.x) + i32::from(tile.y) - out.map_width - 1).div_euclid(2)
                - out.offset.1;
            let Some(p) = out.index(sx, sy) else {
                continue;
            };
            // Both authored colours contribute. Exact original colour
            // compositing, elevation and lighting still need calibration.
            out.colors[p] = std::array::from_fn(|c| {
                ((u16::from(tile_art.radar_left[c]) + u16::from(tile_art.radar_right[c])) / 2) as u8
            });
            out.cells[p] = Some((tile.x, tile.y));
        }
        Ok(out)
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32)
            .then_some(y.max(0) as usize * self.width + x.max(0) as usize)
    }

    fn project(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (
            x - y + self.map_width as f32 - 1.0 - self.offset.0 as f32,
            (x + y - self.map_width as f32 - 1.0) / 2.0 - self.offset.1 as f32,
        )
    }

    fn fit(&self, width: u32) -> Option<Rect> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        let scale =
            (INTERIOR.2 as f32 / self.width as f32).min(INTERIOR.3 as f32 / self.height as f32);
        let w = (self.width as f32 * scale).floor().max(1.0) as i32;
        let h = (self.height as f32 * scale).floor().max(1.0) as i32;
        Some(Rect {
            x: width.saturating_sub(WIDTH) as i32 + INTERIOR.0 + (INTERIOR.2 - w) / 2,
            y: INTERIOR.1 + (INTERIOR.3 - h) / 2,
            w,
            h,
        })
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}
impl Rect {
    fn contains(self, pos: (f64, f64)) -> bool {
        pos.0 >= f64::from(self.x)
            && pos.0 < f64::from(self.x + self.w)
            && pos.1 >= f64::from(self.y)
            && pos.1 < f64::from(self.y + self.h)
    }
}

#[derive(Default)]
pub struct Radar {
    terrain: Terrain,
    providers: BTreeSet<String>,
    hidden: BTreeSet<String>,
    clock: Clock,
    colors: [[u8; 3]; 3],
}

impl Radar {
    pub(crate) fn load(fs: &AssetFs, map: &MapFile, rules: &Rules) -> Self {
        let terrain = Terrain::load(fs, map).unwrap_or_else(|e| {
            eprintln!("OpenYR: radar unavailable ({e})");
            Terrain::default()
        });
        let mut ini = IniFile::parse(&String::from_utf8_lossy(
            fs.read("RULESMD.INI").unwrap_or(&[]),
        ));
        ini.merge_from(&map.ini);
        Self {
            terrain,
            providers: rules
                .buildings
                .iter()
                .filter(|t| t.radar == Some(true))
                .map(|t| t.id.to_ascii_uppercase())
                .collect(),
            hidden: rules
                .technos()
                .filter(|t| t.radar_invisible == Some(true))
                .map(|t| t.id.to_ascii_uppercase())
                .collect(),
            colors: [
                yr_render::house_color(&ini, "Americans"),
                yr_render::house_color(&ini, "Russians"),
                yr_render::house_color(&ini, ""),
            ],
            clock: Clock::default(),
        }
    }

    fn available(&self, sim: &Sim, kinds: &[String]) -> bool {
        let (produced, drained) = sim.power_of(OWNER_PLAYER);
        produced >= drained
            && sim.buildings().iter().any(|b| {
                b.owner == OWNER_PLAYER
                    && b.hp > 0
                    && kinds
                        .get(usize::from(b.kind))
                        .is_some_and(|id| self.providers.contains(&id.to_ascii_uppercase()))
            })
    }

    pub(crate) fn update(&mut self, sim: &Sim, kinds: &[String]) {
        self.clock.observe(sim.tick(), self.available(sim, kinds));
    }

    pub(crate) fn reset(&mut self, sim: &Sim, kinds: &[String]) {
        self.clock.reset(sim.tick(), self.available(sim, kinds));
    }

    pub fn ready(&self) -> bool {
        self.clock.ready() && self.terrain.width > 0
    }

    /// Active terrain only; letterboxing, chrome and transition frames do
    /// not accept navigation or orders. Returned cells share paint sampling.
    pub fn target(&self, width: u32, pos: (f64, f64)) -> Option<(f32, f32)> {
        if !self.ready() {
            return None;
        }
        let rect = self.terrain.fit(width)?;
        if !rect.contains(pos) {
            return None;
        }
        let x = ((pos.0 - f64::from(rect.x)).floor() * self.terrain.width as f64
            / f64::from(rect.w)) as usize;
        let y = ((pos.1 - f64::from(rect.y)).floor() * self.terrain.height as f64
            / f64::from(rect.h)) as usize;
        self.terrain.cells[y * self.terrain.width + x]
            .map(|(x, y)| (f32::from(x) + 0.5, f32::from(y) + 0.5))
    }

    fn cell_rect(&self, rect: Rect, cell: (f32, f32)) -> Rect {
        let (x, y) = self.terrain.project((cell.0.floor(), cell.1.floor()));
        let scale_x = rect.w as f32 / self.terrain.width as f32;
        let scale_y = rect.h as f32 / self.terrain.height as f32;
        let x0 = (x.floor() * scale_x).ceil() as i32;
        let y0 = (y.floor() * scale_y).ceil() as i32;
        let x1 = ((x.floor() + 1.0) * scale_x).ceil() as i32;
        let y1 = ((y.floor() + 1.0) * scale_y).ceil() as i32;
        Rect {
            x: rect.x + x0,
            y: rect.y + y0,
            w: (x1 - x0).max(1),
            h: (y1 - y0).max(1),
        }
    }

    fn point(&self, rect: Rect, cell: (f32, f32)) -> (i32, i32) {
        let area = self.cell_rect(rect, cell);
        (area.x + (area.w - 1) / 2, area.y + (area.h - 1) / 2)
    }

    fn object_color(&self, owner: u8) -> [u8; 4] {
        let rgb = self.colors[match owner {
            yr_sim::OWNER_PLAYER => 0,
            yr_sim::OWNER_ENEMY => 1,
            _ => 2,
        }];
        [rgb[0], rgb[1], rgb[2], 255]
    }

    fn visible(&self, session: &Session, kind: u16) -> bool {
        session
            .kinds
            .get(usize::from(kind))
            .is_some_and(|id| !self.hidden.contains(&id.to_ascii_uppercase()))
    }

    pub(super) fn draw(&self, canvas: &mut Canvas<'_>, session: &Session) {
        let x0 = canvas.width.saturating_sub(WIDTH) as i32;
        if let Some(image) = session.sidebar_art.radar.get(usize::from(self.clock.frame)) {
            canvas.image((x0, 48), image);
        }
        if !self.ready() {
            return;
        }
        let Some(rect) = self.terrain.fit(canvas.width) else {
            return;
        };
        canvas.rect(
            (x0 + INTERIOR.0 - 1, INTERIOR.1 - 1),
            (INTERIOR.2 + 2, INTERIOR.3 + 2),
            [180, 190, 202, 255],
        );
        canvas.rect(
            (x0 + INTERIOR.0, INTERIOR.1),
            (INTERIOR.2, INTERIOR.3),
            [0, 0, 0, 255],
        );
        for y in 0..rect.h {
            for x in 0..rect.w {
                let sx = x as usize * self.terrain.width / rect.w as usize;
                let sy = y as usize * self.terrain.height / rect.h as usize;
                let rgb = self.terrain.colors[sy * self.terrain.width + sx];
                canvas.pixel(rect.x + x, rect.y + y, [rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        let plot = |canvas: &mut Canvas<'_>, cell, color| {
            let area = self.cell_rect(rect, cell);
            // Paint the whole sampled source cell at larger radar scales;
            // keep at least one pixel for blips when cells are downsampled.
            for y in area.y.max(rect.y)..(area.y + area.h).min(rect.y + rect.h) {
                for x in area.x.max(rect.x)..(area.x + area.w).min(rect.x + rect.w) {
                    canvas.pixel(x, y, color);
                }
            }
        };
        // Resources follow the live terrain, including depletion. Ground
        // metadata stays cached and never captures building/unit artwork.
        if let Some(grid) = session.sim.terrain() {
            for (x, y) in self.terrain.cells.iter().flatten() {
                if let Some(cell) = grid.cell(*x, *y).filter(|c| c.resource > 0) {
                    let color = match cell.overlay {
                        yr_sim::terrain::Overlay::Ore => [220, 200, 32, 255],
                        yr_sim::terrain::Overlay::Gem => [160, 80, 220, 255],
                        _ => continue,
                    };
                    plot(canvas, (f32::from(*x) + 0.5, f32::from(*y) + 0.5), color);
                }
            }
        }
        for b in session
            .sim
            .buildings()
            .iter()
            .filter(|b| self.visible(session, b.kind))
        {
            for y in 0..b.size.1 {
                for x in 0..b.size.0 {
                    plot(
                        canvas,
                        (
                            f32::from(b.cell.0) + f32::from(x) + 0.5,
                            f32::from(b.cell.1) + f32::from(y) + 0.5,
                        ),
                        self.object_color(b.owner),
                    );
                }
            }
        }
        for u in session
            .sim
            .units()
            .iter()
            .filter(|u| self.visible(session, u.kind))
        {
            plot(
                canvas,
                (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0),
                self.object_color(u.owner),
            );
        }
    }

    /// World camera corners in cell space. Paints only the visible part of
    /// the white viewport outline, keeping it inside the active map image.
    pub fn draw_viewport(
        &self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        corners: [(f32, f32); 2],
    ) {
        if !self.ready() {
            return;
        }
        let Some(rect) = self.terrain.fit(width) else {
            return;
        };
        let a = self.point(rect, corners[0]);
        let b = self.point(rect, corners[1]);
        let (x0, x1) = (
            a.0.min(b.0).max(rect.x),
            a.0.max(b.0).min(rect.x + rect.w - 1),
        );
        let (y0, y1) = (
            a.1.min(b.1).max(rect.y),
            a.1.max(b.1).min(rect.y + rect.h - 1),
        );
        if x0 > x1 || y0 > y1 {
            return;
        }
        let mut canvas = Canvas {
            frame,
            width,
            height,
            face: super::text::Typeface(None),
        };
        for x in x0..=x1 {
            canvas.pixel(x, y0, [255; 4]);
            canvas.pixel(x, y1, [255; 4]);
        }
        for y in y0..=y1 {
            canvas.pixel(x0, y, [255; 4]);
            canvas.pixel(x1, y, [255; 4]);
        }
    }
}

#[cfg(test)]
mod tests;
