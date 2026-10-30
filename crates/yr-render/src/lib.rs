//! Software map renderer shared by `mix-dump render-map` (PNG) and the
//! `yr-app` window: draws a real map (terrain, overlays, buildings,
//! infantry, voxel vehicles) into an RGBA buffer through the VFS.
//!
//! Geometry: a cell (x, y) has its 60x30 diamond centred at
//! `((x - y) * 30, (x + y) * 15 - level * 15)`. Tiles are painted in
//! increasing `x + y` order; each tile's TMP "extra" overlay is placed
//! relative to that tile's diamond inside its template. Objects are
//! drawn in the same painter order. GPU sprite batching comes later;
//! this rasteriser is the correctness reference.

mod buildings;
pub mod depth;
pub mod png;
pub mod sprites;
mod voxels;

use std::collections::HashMap;
use std::fs;
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, MapFile, Palette, ShpFile, TileSetIndex, TmpFile};
use yr_rules::Rules;

/// A rendered map: `width` x `height` RGBA8 pixels plus a summary line.
pub struct RenderedMap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub summary: String,
    /// Iso origin: map pixel `(0, 0)` is cell-space `(-origin_x, -origin_y)`
    /// before downscaling. Kept so the app can map sim cells <-> pixels.
    pub origin_x: i32,
    pub origin_y: i32,
    /// Downscale factor the buffer was rendered at (1, 2 or 4).
    pub scale: u32,
}

impl RenderedMap {
    /// Rendered-buffer pixel of cell-space point `(cell_x, cell_y)`: cell
    /// `(x, y)` spans `[x, x + 1)`, so its centre is `(x + 0.5, y + 0.5)`.
    /// Height level is ignored (flat projection).
    pub fn cell_to_px(&self, cell_x: f32, cell_y: f32) -> (f32, f32) {
        let cx = (cell_x - cell_y) * HALF_W as f32;
        let cy = (cell_x + cell_y) * HALF_H as f32;
        let s = self.scale.max(1) as f32;
        (
            (cx - self.origin_x as f32) / s,
            (cy - self.origin_y as f32) / s,
        )
    }

    /// Inverse of [`Self::cell_to_px`]: buffer pixel -> fractional cell.
    pub fn px_to_cell(&self, px: f32, py: f32) -> (f32, f32) {
        let s = self.scale.max(1) as f32;
        let cx = px * s + self.origin_x as f32;
        let cy = py * s + self.origin_y as f32;
        let a = cx / HALF_W as f32;
        let b = cy / HALF_H as f32;
        ((a + b) / 2.0, (b - a) / 2.0)
    }
}

const HALF_W: i32 = 30;
const HALF_H: i32 = 15;
const LEVEL_H: i32 = 15;

/// Render `map_name` (a loose file in the game dir, or a path) at
/// downscale `scale` (1, 2 or 4).
pub fn render_map(gamedir: &str, map_name: &str, scale: u32) -> Result<RenderedMap, String> {
    if !matches!(scale, 1 | 2 | 4) {
        return Err("scale must be 1, 2 or 4".into());
    }
    let mut vfs = AssetFs::open(std::path::Path::new(gamedir)).map_err(|e| e.to_string())?;
    let raw = match vfs.read(map_name) {
        Some(d) => d.to_vec(),
        None => fs::read(map_name).map_err(|e| format!("{map_name}: {e}"))?,
    };
    let map = MapFile::parse(&String::from_utf8_lossy(&raw)).map_err(|e| e.to_string())?;
    let theater = Theater::from_map_name(&map.theater)
        .ok_or_else(|| format!("unknown theater {}", map.theater))?;
    vfs.set_theater(theater);
    let ini_name = theater
        .tileset_inis()
        .iter()
        .find(|n| vfs.read(n).is_some())
        .ok_or("tileset INI not found")?;
    let ini = IniFile::parse(&String::from_utf8_lossy(
        vfs.read(ini_name).ok_or("tileset INI unreadable")?,
    ));
    let index = TileSetIndex::parse(&ini, theater.tile_extension());
    let pal = Palette::parse(
        vfs.read(theater.palette())
            .ok_or("theater palette not found")?,
    )
    .ok_or("palette must be 768 bytes")?;
    let unit_pal = vfs
        .read(theater.unit_palette())
        .and_then(Palette::parse)
        .unwrap_or_else(|| pal.clone());
    let theater_pal = vfs
        .read(theater.theater_palette())
        .and_then(Palette::parse)
        .unwrap_or_else(|| pal.clone());

    // Game data for objects: map INI overrides merge into rules, as in game.
    let mut rules_ini = IniFile::parse(&String::from_utf8_lossy(
        vfs.read("RULESMD.INI").ok_or("rulesmd.ini not found")?,
    ));
    rules_ini.merge_from(&map.ini);
    let art_ini = IniFile::parse(&String::from_utf8_lossy(
        vfs.read("ARTMD.INI").ok_or("artmd.ini not found")?,
    ));
    let rules = Rules::load(&rules_ini, &art_ini);
    let overlay_types = type_list(&rules_ini, "OverlayTypes");

    // Load every template the map references (clear cells use tile 0).
    let mut tmps: HashMap<u32, Option<TmpFile>> = HashMap::new();
    for t in &map.tiles {
        let id = t.tile.unwrap_or(0);
        tmps.entry(id).or_insert_with(|| {
            let name = index.file_for(id)?;
            TmpFile::parse(vfs.read(name)?).ok()
        });
    }
    let missing = tmps.values().filter(|t| t.is_none()).count();

    let cell_screen = |x: u16, y: u16, level: u8| {
        (
            (i32::from(x) - i32::from(y)) * HALF_W,
            (i32::from(x) + i32::from(y)) * HALF_H - i32::from(level) * LEVEL_H,
        )
    };
    // Canvas bounds over diamonds and extras.
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    let mut grow = |ax: i32, ay: i32, bx: i32, by: i32| {
        x0 = x0.min(ax);
        y0 = y0.min(ay);
        x1 = x1.max(bx);
        y1 = y1.max(by);
    };
    for t in &map.tiles {
        let Some(Some(tmp)) = tmps.get(&t.tile.unwrap_or(0)) else {
            continue;
        };
        let Some(tile) = tmp.tiles.get(t.subtile as usize).and_then(Option::as_ref) else {
            continue;
        };
        let (cx, cy) = cell_screen(t.x, t.y, t.level);
        grow(cx - HALF_W, cy, cx + HALF_W, cy + HALF_H * 2);
        if let Some(e) = &tile.extra {
            let (ox, oy) = (cx - HALF_W - tile.x + e.x, cy - tile.y + e.y);
            grow(ox, oy, ox + e.width as i32, oy + e.height as i32);
        }
    }
    if x0 > x1 {
        return Err("map has no drawable tiles".into());
    }
    // Room for tall object art (trees, infantry) above the terrain.
    x0 -= 90;
    y0 -= 200;
    x1 += 90;
    y1 += 40;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let mut rgba = vec![0u8; w * h * 4];
    for px in rgba.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    let put = |rgba: &mut [u8], x: i32, y: i32, index: u8| {
        if index != 0 && x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            let at = (y as usize * w + x as usize) * 4;
            rgba[at..at + 4].copy_from_slice(&pal.rgba(index));
        }
    };
    let mut sorted: Vec<_> = map.tiles.iter().collect();
    sorted.sort_by_key(|t| (u32::from(t.x) + u32::from(t.y), t.x));
    for t in sorted {
        let Some(Some(tmp)) = tmps.get(&t.tile.unwrap_or(0)) else {
            continue;
        };
        let Some(tile) = tmp.tiles.get(t.subtile as usize).and_then(Option::as_ref) else {
            continue;
        };
        let (cx, cy) = cell_screen(t.x, t.y, t.level);
        let (dx, dy) = (cx - HALF_W - x0, cy - y0);
        let mut src = tile.pixels.iter();
        for &(row, xs, width) in &tmp.diamond_rows() {
            for ddx in 0..width {
                let index = *src.next().unwrap_or(&0);
                put(&mut rgba, dx + (xs + ddx) as i32, dy + row as i32, index);
            }
        }
        if let Some(e) = &tile.extra {
            let (ox, oy) = (dx - tile.x + e.x, dy - tile.y + e.y);
            for (p, &index) in e.pixels.iter().enumerate() {
                put(
                    &mut rgba,
                    ox + (p % e.width as usize) as i32,
                    oy + (p / e.width as usize) as i32,
                    index,
                );
            }
        }
    }
    // Object passes in painter order: overlays, [Terrain] objects, placed
    // infantry/aircraft as SHPs; structures and voxel units as markers.
    let levels: HashMap<(u16, u16), u8> = map.tiles.iter().map(|t| ((t.x, t.y), t.level)).collect();
    // SHP bytes are owned here because ShpFile borrows its source data.
    let mut shps: HashMap<String, Option<Vec<u8>>> = HashMap::new();
    let mut vxl_cache: HashMap<String, Option<voxels::VoxelModel>> = HashMap::new();
    let load_shp = |vfs: &AssetFs, name: &str, shps: &mut HashMap<String, Option<Vec<u8>>>| {
        shps.entry(name.to_string())
            .or_insert_with(|| vfs.read(name).map(<[u8]>::to_vec))
            .is_some()
    };
    struct Job {
        key: u32,
        cx: i32,
        cy: i32,
        file: Option<String>,
        frame: usize,
        unit: bool,
        /// Ore/gems: drawn with the theater object palette.
        tiberium: bool,
        marker: Option<[u8; 4]>,
        shadow: bool,
        remap: Option<[u8; 3]>,
        voxel: Option<(String, u16)>,
    }
    let mut jobs: Vec<Job> = Vec::new();
    let mut markers = 0usize;
    let cell_job = |jobs: &mut Vec<Job>,
                    x: u16,
                    y: u16,
                    file: Option<String>,
                    frame: usize,
                    unit: bool,
                    marker: Option<[u8; 4]>| {
        let level = levels.get(&(x, y)).copied().unwrap_or(0);
        let (cx, top) = cell_screen(x, y, level);
        jobs.push(Job {
            key: u32::from(x) + u32::from(y),
            cx,
            cy: top + HALF_H,
            file,
            frame,
            unit,
            tiberium: false,
            marker,
            shadow: false,
            remap: None,
            voxel: None,
        });
    };
    for (i, &o) in map.overlay.iter().enumerate() {
        if o == yr_formats::map::NO_OVERLAY {
            continue;
        }
        let Some(kind) = overlay_types.get(o as usize) else {
            continue;
        };
        let (image, theater_art, _) = art_image(&art_ini, kind);
        let file = art_file(&image, theater_art, theater);
        if load_shp(&vfs, &file, &mut shps) {
            let frame = usize::from(map.overlay_data[i]);
            cell_job(
                &mut jobs,
                (i % 512) as u16,
                (i / 512) as u16,
                Some(file),
                frame,
                false,
                None,
            );
            jobs.last_mut().unwrap().tiberium = rules_ini
                .section(kind)
                .and_then(|s| s.get_bool("Tiberium"))
                .unwrap_or(false);
        }
    }
    for (x, y, kind) in &map.terrain {
        let (image, theater_art, _) = art_image(&art_ini, kind);
        let file = art_file(&image, theater_art, theater);
        if load_shp(&vfs, &file, &mut shps) {
            let key = u32::from(*x) + u32::from(*y);
            cell_job(&mut jobs, *x, *y, Some(file), 0, false, None);
            jobs.last_mut().unwrap().shadow = true;
            jobs.last_mut().unwrap().key = key;
        }
    }
    let mut object = |jobs: &mut Vec<Job>,
                      obj: &yr_formats::MapObject,
                      shps: &mut HashMap<String, Option<Vec<u8>>>,
                      vfs: &AssetFs| {
        let techno = rules.techno(&obj.kind);
        let voxel = techno
            .and_then(|t| t.art.as_ref())
            .map(|a| a.voxel)
            .unwrap_or(false);
        if voxel {
            let image = techno
                .map(|t| t.image.clone())
                .unwrap_or_else(|| obj.kind.clone());
            let ok = vxl_cache
                .entry(image.to_ascii_uppercase())
                .or_insert_with(|| voxels::VoxelModel::load(vfs, &image))
                .is_some();
            if ok {
                cell_job(jobs, obj.x, obj.y, None, 0, true, None);
                let job = jobs.last_mut().unwrap();
                job.voxel = Some((image.to_ascii_uppercase(), obj.facing.unwrap_or(0)));
                job.remap = Some(house_color(&rules_ini, &obj.owner));
                return;
            }
            markers += 1;
            cell_job(jobs, obj.x, obj.y, None, 0, true, Some([0, 200, 255, 255]));
            return;
        }
        if obj.kind.is_empty() {
            markers += 1;
            cell_job(jobs, obj.x, obj.y, None, 0, true, Some([0, 200, 255, 255]));
            return;
        }
        let image = techno
            .map(|t| t.image.clone())
            .unwrap_or_else(|| obj.kind.clone());
        let art = techno.and_then(|t| t.art.as_ref());
        let is_building = techno
            .map(|t| t.kind == yr_rules::TechnoKind::Building)
            .unwrap_or(false);
        if is_building {
            // NewTheater=yes buildings: second letter is the theater
            // letter, generic `G` variant as fallback (docs/formats/map.md).
            let candidates = building_shp_candidates(&image, theater);
            let found = candidates.iter().find(|f| load_shp(vfs, f, shps)).cloned();
            match found {
                Some(file) => {
                    let (fx, fy) = art.and_then(|a| a.foundation).unwrap_or((1, 1));
                    let level = levels.get(&(obj.x, obj.y)).copied().unwrap_or(0);
                    let (cx, top) = cell_screen(obj.x, obj.y, level);
                    jobs.push(Job {
                        key: u32::from(obj.x) + u32::from(obj.y) + u32::from(fx) + u32::from(fy),
                        cx: cx + (i32::from(fx) - i32::from(fy)) * HALF_H,
                        cy: top + HALF_H + (i32::from(fx) + i32::from(fy) - 2) * HALF_H / 2,
                        file: Some(file),
                        frame: 0,
                        unit: true,
                        tiberium: false,
                        marker: None,
                        shadow: true,
                        remap: Some(house_color(&rules_ini, &obj.owner)),
                        voxel: None,
                    });
                }
                None => {
                    markers += 1;
                    eprintln!(
                        "no building art for {} ({})",
                        obj.kind,
                        candidates.join(", ")
                    );
                    cell_job(jobs, obj.x, obj.y, None, 0, false, Some([255, 0, 200, 255]));
                }
            }
        } else if load_shp(
            vfs,
            &art_file(&image, art.map(|a| a.theater).unwrap_or(false), theater),
            shps,
        ) {
            let file = art_file(&image, art.map(|a| a.theater).unwrap_or(false), theater);
            cell_job(jobs, obj.x, obj.y, Some(file), 0, true, None);
            jobs.last_mut().unwrap().shadow = true;
            jobs.last_mut().unwrap().remap = Some(house_color(&rules_ini, &obj.owner));
        } else {
            markers += 1;
            cell_job(jobs, obj.x, obj.y, None, 0, true, Some([0, 200, 255, 255]));
        }
    };
    for obj in map
        .structures
        .iter()
        .chain(&map.units)
        .chain(&map.infantry)
        .chain(&map.aircraft)
    {
        object(&mut jobs, obj, &mut shps, &vfs);
    }
    let drawn_jobs = jobs.iter().filter(|j| j.file.is_some()).count();
    jobs.sort_by_key(|j| j.key);
    for job in &jobs {
        let (cx, cy) = (job.cx - x0, job.cy - y0);
        if let Some(color) = job.marker {
            // Filled 60x30 diamond centred on the cell.
            for row in 0..30i32 {
                let width = if row < 15 {
                    4 + row * 4
                } else {
                    60 - (row - 14) * 4
                };
                for dx in 0..width.max(0) {
                    let (x, y) = (cx - width / 2 + dx, cy - HALF_H + row);
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        let at = (y as usize * w + x as usize) * 4;
                        rgba[at..at + 4].copy_from_slice(&color);
                    }
                }
            }
            continue;
        }
        if let Some((image, facing)) = &job.voxel {
            if let Some(Some(model)) = vxl_cache.get(image) {
                voxels::draw_voxel(
                    &mut rgba, w, h, cx, cy, model, *facing, &unit_pal, job.remap,
                );
            }
            continue;
        }
        let Some(bytes) = job
            .file
            .as_ref()
            .and_then(|f| shps.get(f))
            .and_then(Option::as_ref)
        else {
            continue;
        };
        let Ok(shp) = ShpFile::parse(bytes) else {
            continue;
        };
        let frame = if job.frame < shp.frames.len() {
            job.frame
        } else {
            0
        };
        let Ok(canvas) = shp.decode_frame_canvas(frame) else {
            continue;
        };
        if job.shadow && shp.frames.len() >= 2 {
            let shadow_frame = job.frame + shp.frames.len() / 2;
            if shadow_frame < shp.frames.len() {
                if let Ok(shadow) = shp.decode_frame_canvas(shadow_frame) {
                    let (ox, oy) = (cx - shp.width as i32 / 2, cy - shp.height as i32 / 2);
                    for (p, &index) in shadow.iter().enumerate() {
                        if index == 0 {
                            continue;
                        }
                        let (x, y) = (
                            ox + (p % shp.width as usize) as i32,
                            oy + (p / shp.width as usize) as i32,
                        );
                        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                            let at = (y as usize * w + x as usize) * 4;
                            rgba[at] /= 2;
                            rgba[at + 1] /= 2;
                            rgba[at + 2] /= 2;
                        }
                    }
                }
            }
        }
        let palette = if job.unit {
            &unit_pal
        } else if job.tiberium {
            &theater_pal
        } else {
            &pal
        };
        let (ox, oy) = (cx - shp.width as i32 / 2, cy - shp.height as i32 / 2);
        for (p, &index) in canvas.iter().enumerate() {
            if index == 0 {
                continue;
            }
            let (x, y) = (
                ox + (p % shp.width as usize) as i32,
                oy + (p / shp.width as usize) as i32,
            );
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                let at = (y as usize * w + x as usize) * 4;
                rgba[at..at + 4].copy_from_slice(&remap_rgba(palette, index, job.remap));
            }
        }
    }

    // Optional nearest-neighbour downscale for eyeball-sized output.
    let (ow, oh, rgba) = if scale == 1 {
        (w, h, rgba)
    } else {
        let s = scale as usize;
        let (ow, oh) = (w.div_ceil(s), h.div_ceil(s));
        let mut out = vec![0u8; ow * oh * 4];
        for y in 0..oh {
            for x in 0..ow {
                let src = ((y * s) * w + x * s) * 4;
                out[(y * ow + x) * 4..(y * ow + x) * 4 + 4].copy_from_slice(&rgba[src..src + 4]);
            }
        }
        (ow, oh, out)
    };
    Ok(RenderedMap {
        width: ow as u32,
        height: oh as u32,
        rgba,
        origin_x: x0,
        origin_y: y0,
        scale,
        summary: format!(
            "{map_name} ({}, {} tiles, {} templates, {missing} missing, {drawn_jobs} object SHPs, {markers} markers, {ow}x{oh} @ scale {scale})",
            map.theater,
            map.tiles.len(),
            tmps.len()
        ),
    })
}

/// SHP names to try for building image `image`: NewTheater art swaps the
/// second letter for the theater letter, with the generic `G` variant
/// as fallback (docs/formats/map.md), then the plain image name.
pub(crate) fn building_shp_candidates(image: &str, theater: Theater) -> Vec<String> {
    let mut candidates = Vec::new();
    if image.len() > 1 && image.is_char_boundary(1) && image.is_char_boundary(2) {
        candidates.push(format!(
            "{}{}{}.SHP",
            &image[..1],
            theater.art_letter(),
            &image[2..]
        ));
        candidates.push(format!("{}G{}.SHP", &image[..1], &image[2..]));
    }
    candidates.push(format!("{}.SHP", image.to_ascii_uppercase()));
    candidates
}

/// Palette colour of `index`, with the house remap range (16..=31)
/// shaded from `remap` when given.
pub(crate) fn remap_rgba(pal: &Palette, index: u8, remap: Option<[u8; 3]>) -> [u8; 4] {
    match (remap, index) {
        (Some(base), 16..=31) => {
            let f = 0.45 + f32::from(index - 16) * 0.055;
            let c = |v: u8| (f32::from(v) * f).min(255.0) as u8;
            [c(base[0]), c(base[1]), c(base[2]), 255]
        }
        _ => pal.rgba(index),
    }
}

/// Ordered values of a `rulesmd.ini` type-list section (`0=FOO`, `1=BAR`).
pub fn type_list(ini: &IniFile, section: &str) -> Vec<String> {
    let Some(sec) = ini.section(section) else {
        return Vec::new();
    };
    let mut entries: Vec<(u32, String)> = sec
        .entries
        .iter()
        .filter_map(|(k, v)| {
            let n: u32 = k.trim().parse().ok()?;
            let name = v.split(';').next()?.split(',').next()?.trim().to_string();
            (!name.is_empty()).then_some((n, name))
        })
        .collect();
    entries.sort_by_key(|e| e.0);
    entries.into_iter().map(|e| e.1).collect()
}

/// `(image, theater-extension, new-theater)` for a type: `Image=` in
/// its artmd section, defaulting to the type ID.
fn art_image(art: &IniFile, kind: &str) -> (String, bool, bool) {
    let sec = art.section(kind);
    let image = sec
        .and_then(|s| s.get("Image"))
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| kind.to_string());
    let flag = |k: &str| sec.map(|s| s.get_bool(k).unwrap_or(false)).unwrap_or(false);
    (image, flag("Theater"), flag("NewTheater"))
}

fn art_file(image: &str, theater_art: bool, theater: Theater) -> String {
    if theater_art {
        format!(
            "{}.{}",
            image.to_ascii_uppercase(),
            theater.tile_extension()
        )
    } else {
        format!("{}.SHP", image.to_ascii_uppercase())
    }
}

/// Owner house colour as RGB: the owner's `Color=` name resolved through
/// the `[Colors]` HSV table in rules. Neutral grey when unknown.
pub(crate) fn house_color(rules: &IniFile, owner: &str) -> [u8; 3] {
    let name = rules
        .section(owner)
        .and_then(|s| s.get("Color"))
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
        .unwrap_or_else(|| "LightGrey".to_string());
    let hsv = rules
        .section("Colors")
        .and_then(|s| s.get(&name))
        .and_then(|v| {
            let mut it = v
                .split(';')
                .next()?
                .split(',')
                .map(|p| p.trim().parse::<f32>().ok());
            Some((it.next()??, it.next()??, it.next()??))
        });
    let Some((h, sat, val)) = hsv else {
        return [170, 170, 170];
    };
    let (h, sat, val) = (h / 255.0 * 360.0, sat / 255.0, val / 255.0);
    let c = val * sat;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = val - c;
    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    ]
}

/// Build the sim terrain grid for a parsed map: TMP terrain types and
/// ramps per cell, resource bails on ore/gem overlay cells, walls from
/// wall/fence/sandbag overlays. Grid spans the tile extents (iso
/// coordinates run past `Size=`; see docs/formats/map.md).
pub fn build_terrain(
    fs: &mut yr_assets::AssetFs,
    map: &MapFile,
    overlay_types: &[String],
    theater: yr_assets::Theater,
) -> Result<yr_sim::TerrainGrid, String> {
    let ini_name = theater
        .tileset_inis()
        .iter()
        .find(|n| fs.read(n).is_some())
        .ok_or("tileset INI not found")?;
    let ini = IniFile::parse(&String::from_utf8_lossy(
        fs.read(ini_name).ok_or("tileset INI unreadable")?,
    ));
    let index = TileSetIndex::parse(&ini, theater.tile_extension());
    let mut tmps: std::collections::HashMap<String, TmpFile> = std::collections::HashMap::new();
    let mut rows: Vec<(u16, u16, u8, u8, u8)> = Vec::new();
    for t in &map.tiles {
        let Some(id) = t.tile else { continue };
        let Some(name) = index.file_for(id) else {
            continue;
        };
        let tmp = tmps.entry(name.to_string()).or_insert_with(|| {
            TmpFile::parse(fs.read(name).expect("TMP readable")).expect("TMP parses")
        });
        if let Some(cell) = tmp.tiles.get(t.subtile as usize).and_then(Option::as_ref) {
            rows.push((t.x, t.y, t.level, cell.terrain_type, cell.ramp_type));
        }
    }
    let (w, h) = rows.iter().fold((0usize, 0usize), |(w, h), r| {
        (w.max(r.0 as usize + 1), h.max(r.1 as usize + 1))
    });
    let mut grid = yr_sim::TerrainGrid::from_tiles(0, 0, w, h, rows.into_iter());
    // Overlay type indices follow the rules [OverlayTypes] order.
    for (i, &o) in map.overlay.iter().enumerate() {
        if o == yr_formats::map::NO_OVERLAY {
            continue;
        }
        let Some(kind) = overlay_types.get(o as usize) else {
            continue;
        };
        let (x, y) = ((i % 512) as u16, (i / 512) as u16);
        let upper = kind.to_ascii_uppercase();
        if upper.contains("TIB") || upper.starts_with("GEM") {
            let bails = 2 + (map.overlay_data[i] % 5);
            let gem = upper.starts_with("GEM");
            grid.set_resource(
                x,
                y,
                if gem {
                    yr_sim::terrain::Overlay::Gem
                } else {
                    yr_sim::terrain::Overlay::Ore
                },
                bails,
            );
        } else if ["WALL", "FENCE", "SAND", "BARB", "CYCL"]
            .iter()
            .any(|k| upper.contains(k))
        {
            grid.set_overlay(x, y, yr_sim::terrain::Overlay::Wall);
        }
    }
    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_px_round_trip() {
        // Arrange
        let map = RenderedMap {
            width: 10,
            height: 10,
            rgba: vec![0; 400],
            summary: String::new(),
            origin_x: -90,
            origin_y: -200,
            scale: 2,
        };
        // Act / Assert
        for (x, y) in [(0.0f32, 0.0f32), (10.5, 3.25), (40.0, 40.0)] {
            let (px, py) = map.cell_to_px(x, y);
            let (rx, ry) = map.px_to_cell(px, py);
            assert!(
                (rx - x).abs() < 0.001 && (ry - y).abs() < 0.001,
                "({x},{y}) -> ({px},{py}) -> ({rx},{ry})"
            );
        }
    }
}
