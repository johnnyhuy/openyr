//! `mix-dump render`: draw an SHP (all frames as a sheet), a TMP template
//! (both with a given palette) or a PCX to PNG, for eyeballing decoder output.
//!
//! Asset specs are `<archive.mix>:<nested/path>`, e.g.
//! `ra2.mix:SNOW.MIX/GACNST.SHP`.

use std::fs;
use yr_formats::{Palette, PcxImage, ShpFile, TmpFile};
use yr_render::png;

/// `palette` is required for SHP/TMP; PCX carries its own colours.
pub fn cmd_render(asset: &str, palette: Option<&str>, out: &str) -> Result<(), String> {
    let data = load_spec(asset)?;
    if let Ok(img) = PcxImage::parse(&data) {
        let (w, h) = (img.width, img.height);
        fs::write(out, png::encode_rgba(w, h, &img.to_rgba())).map_err(|e| e.to_string())?;
        println!("rendered {asset} -> {out} ({w}x{h})");
        return Ok(());
    }
    let palette = palette.ok_or("SHP/TMP need a palette argument")?;
    let pal = Palette::parse(&load_spec(palette)?).ok_or("palette must be 768 bytes")?;
    let (w, h, rgba) = if let Ok(tmp) = TmpFile::parse(&data) {
        render_tmp(&tmp, &pal)
    } else if let Ok(shp) = ShpFile::parse(&data) {
        render_shp(&shp, &pal)?
    } else {
        return Err(format!("{asset}: not an SHP or TMP"));
    };
    fs::write(out, png::encode_rgba(w, h, &rgba)).map_err(|e| e.to_string())?;
    println!("rendered {asset} -> {out} ({w}x{h})");
    Ok(())
}

pub(crate) fn load_spec(spec: &str) -> Result<Vec<u8>, String> {
    let (archive, inner) = spec
        .rsplit_once(':')
        .ok_or_else(|| format!("{spec}: expected <archive.mix>:<path>"))?;
    let (data, _) = crate::load(archive)?;
    let (payload, _) = crate::descend(&data, inner)?;
    Ok(payload.to_vec())
}

/// Index 0 is transparent in every Westwood palette-indexed format.
fn put(rgba: &mut [u8], at: usize, index: u8, pal: &Palette) {
    if index != 0 {
        rgba[at * 4..at * 4 + 4].copy_from_slice(&pal.rgba(index));
    }
}

fn render_shp(shp: &ShpFile, pal: &Palette) -> Result<(u32, u32, Vec<u8>), String> {
    let n = shp.frames.len();
    let cols = (n as f64).sqrt().ceil().max(1.0) as usize;
    let rows = n.div_ceil(cols);
    let (cw, ch) = (shp.width.max(1) as usize, shp.height.max(1) as usize);
    let (w, h) = (cols * cw, rows * ch);
    let mut rgba = vec![0u8; w * h * 4];
    for i in 0..n {
        let canvas = shp.decode_frame_canvas(i).map_err(|e| e.to_string())?;
        let (ox, oy) = ((i % cols) * cw, (i / cols) * ch);
        for (p, &index) in canvas.iter().enumerate() {
            let (x, y) = (p % shp.width as usize, p / shp.width as usize);
            put(&mut rgba, (oy + y) * w + ox + x, index, pal);
        }
    }
    Ok((w as u32, h as u32, rgba))
}

fn render_tmp(tmp: &TmpFile, pal: &Palette) -> (u32, u32, Vec<u8>) {
    let (tw, th) = (tmp.tile_width as i32, tmp.tile_height as i32);
    // Bounds over every diamond and extra overlay.
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for t in tmp.tiles.iter().flatten() {
        x0 = x0.min(t.x);
        y0 = y0.min(t.y);
        x1 = x1.max(t.x + tw);
        y1 = y1.max(t.y + th);
        if let Some(e) = &t.extra {
            x0 = x0.min(e.x);
            y0 = y0.min(e.y);
            x1 = x1.max(e.x + e.width as i32);
            y1 = y1.max(e.y + e.height as i32);
        }
    }
    if x0 > x1 {
        return (1, 1, vec![0; 4]);
    }
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let mut rgba = vec![0u8; w * h * 4];
    let rows = tmp.diamond_rows();
    for t in tmp.tiles.iter().flatten() {
        let mut src = t.pixels.iter();
        for &(row, xs, width) in &rows {
            for dx in 0..width {
                let x = (t.x - x0) as usize + (xs + dx) as usize;
                let y = (t.y - y0) as usize + row as usize;
                put(&mut rgba, y * w + x, *src.next().unwrap_or(&0), pal);
            }
        }
        if let Some(e) = &t.extra {
            for (p, &index) in e.pixels.iter().enumerate() {
                let x = (e.x - x0) as usize + p % e.width as usize;
                let y = (e.y - y0) as usize + p / e.width as usize;
                put(&mut rgba, y * w + x, index, pal);
            }
        }
    }
    (w as u32, h as u32, rgba)
}
