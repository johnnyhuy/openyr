//! `mix-dump render-map`: draw a real map's terrain (iso tiles + template
//! extras, height-offset) to PNG through the engine-order VFS.
//!
//! Geometry: a cell (x, y) has its 60x30 diamond centred at
//! `((x - y) * 30, (x + y) * 15 - level * 15)`. Tiles are painted in
//! increasing `x + y` order; each tile's TMP "extra" overlay is placed
//! relative to that tile's diamond inside its template. Overlay-pack
//! objects (ore, gems, walls) are not drawn yet.

use crate::png;
use std::collections::HashMap;
use std::fs;
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, MapFile, Palette, TileSetIndex, TmpFile};

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
        "rendered {map_name} ({}, {} tiles, {} templates, {missing} missing) -> {out} ({ow}x{oh}, scale {scale})",
        map.theater,
        map.tiles.len(),
        tmps.len()
    );
    Ok(())
}
