//! `mix-dump render`: draw an SHP (all frames as a sheet), a TMP template
//! (both with a given palette) or a PCX to PNG, for eyeballing decoder output.
//!
//! Asset specs are `<archive.mix>:<nested/path>`, e.g.
//! `ra2.mix:SNOW.MIX/GACNST.SHP`.

use std::fs;
use std::io::Write;
use yr_formats::{Palette, PcxImage, ShpFile, TmpFile};
use yr_render::png;

/// `palette` is required for SHP/TMP; PCX carries its own colours.
pub fn cmd_render(
    asset: &str,
    palette: Option<&str>,
    out: &str,
    stdout: &mut dyn Write,
) -> crate::CommandResult {
    let data = load_spec(asset)?;
    if let Ok(img) = PcxImage::parse(&data) {
        let (w, h) = (img.width, img.height);
        fs::write(out, png::encode_rgba(w, h, &img.to_rgba())).map_err(|e| e.to_string())?;
        writeln!(stdout, "rendered {asset} -> {out} ({w}x{h})")?;
        return Ok(());
    }
    let palette = palette.ok_or("SHP/TMP need a palette argument")?;
    let pal = Palette::parse(&load_spec(palette)?).ok_or("palette must be 768 bytes")?;
    let (w, h, rgba) = if let Ok(tmp) = TmpFile::parse(&data) {
        render_tmp(&tmp, &pal).map_err(|error| format!("{asset}: {error}"))?
    } else if let Ok(shp) = ShpFile::parse(&data) {
        render_shp(&shp, &pal).map_err(|error| format!("{asset}: {error}"))?
    } else {
        return Err(format!("{asset}: not an SHP or TMP").into());
    };
    fs::write(out, png::encode_rgba(w, h, &rgba)).map_err(|e| e.to_string())?;
    writeln!(stdout, "rendered {asset} -> {out} ({w}x{h})")?;
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

// Inspection sheets use the same byte ceiling as the native map raster.
const MAX_INSPECTION_BYTES: usize = 1024 * 1024 * 1024;

fn rgba_len(width: usize, height: usize) -> Result<usize, String> {
    if width == 0 || height == 0 || u32::try_from(width).is_err() || u32::try_from(height).is_err()
    {
        return Err("inspection raster has unsupported PNG dimensions".into());
    }
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|&bytes| bytes <= MAX_INSPECTION_BYTES)
        .ok_or_else(|| "inspection raster exceeds 1 GiB RGBA budget".into())
}

fn canvas(width: usize, height: usize) -> Result<Vec<u8>, String> {
    let bytes = rgba_len(width, height)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes)
        .map_err(|_| "cannot allocate inspection raster".to_string())?;
    rgba.resize(bytes, 0);
    Ok(rgba)
}

fn render_shp(shp: &ShpFile, pal: &Palette) -> Result<(u32, u32, Vec<u8>), String> {
    let n = shp.frames.len();
    let cols = (n as f64).sqrt().ceil().max(1.0) as usize;
    let rows = n.div_ceil(cols);
    let (cw, ch) = (shp.width.max(1) as usize, shp.height.max(1) as usize);
    let w = cols.checked_mul(cw).ok_or("SHP sheet width overflow")?;
    let h = rows.checked_mul(ch).ok_or("SHP sheet height overflow")?;
    let mut rgba = canvas(w, h)?;
    for i in 0..n {
        if shp.frames[i].is_empty() {
            continue;
        }
        let canvas = shp.decode_frame_canvas(i).map_err(|e| e.to_string())?;
        let (ox, oy) = ((i % cols) * cw, (i / cols) * ch);
        for (p, &index) in canvas.iter().enumerate() {
            let (x, y) = (p % shp.width as usize, p / shp.width as usize);
            put(&mut rgba, (oy + y) * w + ox + x, index, pal);
        }
    }
    Ok((w as u32, h as u32, rgba))
}

fn render_tmp(tmp: &TmpFile, pal: &Palette) -> Result<(u32, u32, Vec<u8>), String> {
    let (tw, th) = (i64::from(tmp.tile_width), i64::from(tmp.tile_height));
    // Bounds over every diamond and extra overlay.
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for t in tmp.tiles.iter().flatten() {
        x0 = x0.min(i64::from(t.x));
        y0 = y0.min(i64::from(t.y));
        x1 = x1.max(i64::from(t.x) + tw);
        y1 = y1.max(i64::from(t.y) + th);
        if let Some(e) = &t.extra {
            x0 = x0.min(i64::from(e.x));
            y0 = y0.min(i64::from(e.y));
            x1 = x1.max(i64::from(e.x) + i64::from(e.width));
            y1 = y1.max(i64::from(e.y) + i64::from(e.height));
        }
    }
    if x0 > x1 {
        return Ok((1, 1, vec![0; 4]));
    }
    let dimension = |n| usize::try_from(n).map_err(|_| "TMP dimensions exceed platform bounds");
    let (w, h) = (dimension(x1 - x0)?, dimension(y1 - y0)?);
    let mut rgba = canvas(w, h)?;
    let offset = |n: i32, origin| {
        usize::try_from(i64::from(n) - origin).map_err(|_| "TMP offset exceeds platform bounds")
    };
    let rows = tmp.diamond_rows();
    for t in tmp.tiles.iter().flatten() {
        let (tx, ty) = (offset(t.x, x0)?, offset(t.y, y0)?);
        let mut src = t.pixels.iter();
        for &(row, xs, width) in &rows {
            for dx in 0..width {
                let x = tx + (xs + dx) as usize;
                let y = ty + row as usize;
                put(&mut rgba, y * w + x, *src.next().unwrap_or(&0), pal);
            }
        }
        if let Some(e) = &t.extra {
            let (ex, ey) = (offset(e.x, x0)?, offset(e.y, y0)?);
            for (p, &index) in e.pixels.iter().enumerate() {
                let x = ex + p % e.width as usize;
                let y = ey + p / e.width as usize;
                put(&mut rgba, y * w + x, index, pal);
            }
        }
    }
    Ok((w as u32, h as u32, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_raster_limits_are_checked_without_allocating_the_boundary() {
        assert_eq!(rgba_len(16384, 16384).unwrap(), MAX_INSPECTION_BYTES);
        assert!(rgba_len(16384, 16385).is_err());
        assert!(rgba_len(usize::MAX, 1).is_err());
        assert!(rgba_len(usize::MAX / 4 + 1, 2).is_err());
        assert!(rgba_len(65535, 65535).is_err());
        assert!(rgba_len(0, 1).is_err());
        assert!(rgba_len(1, 0).is_err());
        assert_eq!(canvas(2, 3).unwrap(), vec![0; 24]);
    }
}
