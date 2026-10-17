//! `mix-dump render-map`: PNG front-end over the shared yr-render rasteriser.

use crate::png;
use std::fs;

pub fn cmd_render_map(args: &[&str]) -> Result<(), String> {
    let (gamedir, map_name, out) = match args {
        [g, m, o, ..] => (*g, *m, *o),
        _ => return Err("usage: render-map <gamedir> <map> <out.png> [scale]".into()),
    };
    let scale: u32 = args.get(3).map(|s| s.parse().unwrap_or(2)).unwrap_or(2);
    let map = yr_render::render_map(gamedir, map_name, scale)?;
    fs::write(out, png::encode_rgba(map.width, map.height, &map.rgba))
        .map_err(|e| e.to_string())?;
    println!("rendered {} -> {out}", map.summary);
    Ok(())
}
