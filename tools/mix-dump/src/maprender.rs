//! `mix-dump render-map`: draw a real map (terrain, overlays, terrain
//! objects and SHP objects) to PNG through the engine-order VFS.
//!
//! Geometry: a cell (x, y) has its 60x30 diamond centred at
//! `((x - y) * 30, (x + y) * 15 - level * 15)`. Tiles are painted in
//! increasing `x + y` order; each tile's TMP "extra" overlay is placed
//! relative to that tile's diamond inside its template. Overlay-pack
//! cells, `[Terrain]` objects and placed infantry/aircraft are drawn as
//! SHPs in the same painter order. Structures and voxel vehicles are
//! footprint markers for now: building/VXL art is a separate slice.

use crate::png;
use std::collections::HashMap;
use std::fs;
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, MapFile, Palette, ShpFile, TileSetIndex, TmpFile};
use yr_rules::Rules;

const HALF_W: i32 = 30;
const HALF_H: i32 = 15;
const LEVEL_H: i32 = 15;

pub fn cmd_render_map(args: &[&str]) -> Result<(), String> {
    let (gamedir, map_name, out) = match args {
        [g, m, o, ..] => (*g, *m, *o),
        _ => return Err("usage: render-map <gamedir> <map> <out.png> [scale]".into()),
    };
    let scale: u32 = args.get(3).map(|s| s.parse().unwrap_or(2)).unwrap_or(2);
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
    let pal = Palette::parse(vfs.read(theater.palette()).ok_or("theater palette not found")?)
        .ok_or("palette must be 768 bytes")?;
    let unit_pal = vfs
        .read(theater.unit_palette())
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
        let Some(Some(tmp)) = tmps.get(&t.tile.unwrap_or(0)) else { continue };
        let Some(tile) = tmp.tiles.get(t.subtile as usize).and_then(Option::as_ref) else { continue };
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
    for px in rgba.chunks_exact_mut(4) {
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
        let Some(Some(tmp)) = tmps.get(&t.tile.unwrap_or(0)) else { continue };
        let Some(tile) = tmp.tiles.get(t.subtile as usize).and_then(Option::as_ref) else { continue };
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
        marker: Option<[u8; 4]>,
    }
    let mut jobs: Vec<Job> = Vec::new();
    let mut markers = 0usize;
    let cell_job = |jobs: &mut Vec<Job>, x: u16, y: u16, file: Option<String>, frame: usize, unit: bool, marker: Option<[u8; 4]>| {
        let level = levels.get(&(x, y)).copied().unwrap_or(0);
        let (cx, top) = cell_screen(x, y, level);
        jobs.push(Job { key: u32::from(x) + u32::from(y), cx, cy: top + HALF_H, file, frame, unit, marker });
    };
    for (i, &o) in map.overlay.iter().enumerate() {
        if o == yr_formats::map::NO_OVERLAY {
            continue;
        }
        let Some(kind) = overlay_types.get(o as usize) else { continue };
        let (image, theater_art) = art_image(&art_ini, kind);
        let file = art_file(&image, theater_art, theater);
        if load_shp(&vfs, &file, &mut shps) {
            let frame = usize::from(map.overlay_data[i]);
            cell_job(&mut jobs, (i % 512) as u16, (i / 512) as u16, Some(file), frame, false, None);
        }
    }
    for (x, y, kind) in &map.terrain {
        let (image, theater_art) = art_image(&art_ini, kind);
        let file = art_file(&image, theater_art, theater);
        if load_shp(&vfs, &file, &mut shps) {
            cell_job(&mut jobs, *x, *y, Some(file), 0, false, None);
        }
    }
    let mut object = |jobs: &mut Vec<Job>, obj: &yr_formats::MapObject, shps: &mut HashMap<String, Option<Vec<u8>>>, vfs: &AssetFs| {
        let techno = rules.techno(&obj.kind);
        let voxel = techno.and_then(|t| t.art.as_ref()).map(|a| a.voxel).unwrap_or(false);
        if voxel || obj.kind.is_empty() {
            markers += 1;
            cell_job(jobs, obj.x, obj.y, None, 0, true, Some([0, 200, 255, 255]));
            return;
        }
        let image = techno.map(|t| t.image.clone()).unwrap_or_else(|| obj.kind.clone());
        let theater_art = techno.and_then(|t| t.art.as_ref()).map(|a| a.theater).unwrap_or(false);
        let file = art_file(&image, theater_art, theater);
        // Buildings are drawn as footprint markers until building art
        // (foundation + height anchoring) lands; infantry/aircraft as SHP.
        let is_building = techno.map(|t| t.kind == yr_rules::TechnoKind::Building).unwrap_or(false);
        if is_building {
            markers += 1;
            cell_job(jobs, obj.x, obj.y, None, 0, false, Some([255, 0, 200, 255]));
        } else if load_shp(vfs, &file, shps) {
            let frame = (obj.facing.unwrap_or(0) / 32) as usize;
            cell_job(jobs, obj.x, obj.y, Some(file), frame, true, None);
        } else {
            markers += 1;
            cell_job(jobs, obj.x, obj.y, None, 0, true, Some([0, 200, 255, 255]));
        }
    };
    for obj in map.structures.iter().chain(&map.units).chain(&map.infantry).chain(&map.aircraft) {
        object(&mut jobs, obj, &mut shps, &vfs);
    }
    let drawn_jobs = jobs.iter().filter(|j| j.file.is_some()).count();
    jobs.sort_by_key(|j| j.key);
    for job in &jobs {
        let (cx, cy) = (job.cx - x0, job.cy - y0);
        if let Some(color) = job.marker {
            // Filled 60x30 diamond centred on the cell.
            for row in 0..30i32 {
                let width = if row < 15 { 4 + row * 4 } else { 60 - (row - 14) * 4 };
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
        let Some(bytes) = job.file.as_ref().and_then(|f| shps.get(f)).and_then(Option::as_ref) else { continue };
        let Ok(shp) = ShpFile::parse(bytes) else { continue };
        let frame = if job.frame < shp.frames.len() { job.frame } else { 0 };
        let Ok(canvas) = shp.decode_frame_canvas(frame) else { continue };
        let palette = if job.unit { &unit_pal } else { &pal };
        let (ox, oy) = (cx - shp.width as i32 / 2, cy - shp.height as i32 / 2);
        for (p, &index) in canvas.iter().enumerate() {
            if index == 0 {
                continue;
            }
            let (x, y) = (ox + (p % shp.width as usize) as i32, oy + (p / shp.width as usize) as i32);
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                let at = (y as usize * w + x as usize) * 4;
                rgba[at..at + 4].copy_from_slice(&palette.rgba(index));
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
    fs::write(out, png::encode_rgba(ow as u32, oh as u32, &rgba)).map_err(|e| e.to_string())?;
    println!(
        "rendered {map_name} ({}, {} tiles, {} templates, {missing} missing, {drawn_jobs} object SHPs, {markers} markers) -> {out} ({ow}x{oh}, scale {scale})",
        map.theater,
        map.tiles.len(),
        tmps.len()
    );
    Ok(())
}

/// Ordered values of a `rulesmd.ini` type-list section (`0=FOO`, `1=BAR`).
fn type_list(ini: &IniFile, section: &str) -> Vec<String> {
    let Some(sec) = ini.section(section) else { return Vec::new() };
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

/// `(image, theater-art)` for a type: `Image=` in its artmd section,
/// defaulting to the type ID; theater-art files carry the theater extension.
fn art_image(art: &IniFile, kind: &str) -> (String, bool) {
    let sec = art.section(kind);
    let image = sec
        .and_then(|s| s.get("Image"))
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| kind.to_string());
    let theater = sec
        .map(|s| s.get_bool("Theater").unwrap_or(false) || s.get_bool("NewTheater").unwrap_or(false))
        .unwrap_or(false);
    (image, theater)
}

fn art_file(image: &str, theater_art: bool, theater: Theater) -> String {
    if theater_art {
        format!("{}.{}", image.to_ascii_uppercase(), theater.tile_extension())
    } else {
        format!("{}.SHP", image.to_ascii_uppercase())
    }
}
