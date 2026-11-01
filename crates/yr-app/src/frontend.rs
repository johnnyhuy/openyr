//! Retail front-menu shell. Observations and limits: formats/front-menu.md.
mod art;
mod draw;
pub use art::Art;
pub use draw::{draw, Scene};
#[cfg(test)]
mod tests;
#[cfg(test)]
mod warning_tests;

pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub authored: bool,
}
pub fn load_preview(directory: &str, map_name: &str, scale: u32) -> Result<Preview, String> {
    let image = {
        let fs =
            yr_assets::AssetFs::open(std::path::Path::new(directory)).map_err(|e| e.to_string())?;
        let bytes = fs.read(map_name).ok_or("Map is missing")?;
        let map = yr_formats::MapFile::parse(&String::from_utf8_lossy(bytes))
            .map_err(|e| e.to_string())?;
        map.preview().map_err(|e| e.to_string())?
    };
    if let Some(image) = image {
        return Ok(Preview {
            width: image.width,
            height: image.height,
            rgba: image
                .rgb
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            authored: true,
        });
    }
    let map = yr_render::render_map(directory, map_name, scale)?;
    Ok(Preview {
        width: map.width,
        height: map.height,
        rgba: map.rgba,
        authored: false,
    })
}
