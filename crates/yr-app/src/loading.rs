//! Retail country loading artwork with the installed map's preview.
//! Asset evidence and remaining calibration: formats/loading-screen.md.

use crate::{
    cursor,
    frontend::Preview,
    match_config::{Country, Side, COLOR_NAMES},
    menu::{Button, Layout, LoadStage, State},
    sidebar::{art::Image, text::Typeface, Canvas},
};
use yr_assets::AssetFs;
use yr_formats::{CsfFile, Palette, PcxImage, ShpFile, UnicodeFont};

const PREVIEW: (i32, i32, i32, i32) = (500, 380, 216, 166);
const FALLBACK_CAPTION: [u8; 4] = [248, 232, 96, 255];
const WHITE: [u8; 4] = [224, 232, 240, 255];

#[derive(Default)]
pub struct Art {
    pub cursor: cursor::Art,
    backgrounds: Vec<(Country, Image)>,
    flags: Vec<(Country, Image)>,
    colors: [[u8; 3]; 8],
    captions: [[u8; 3]; 2],
    font: Option<UnicodeFont>,
    strings: Option<CsfFile>,
}

impl Art {
    pub fn load(fs: &AssetFs) -> Self {
        let rules = yr_formats::IniFile::parse(&String::from_utf8_lossy(
            fs.read("RULESMD.INI").unwrap_or_default(),
        ));
        Self {
            colors: std::array::from_fn(|i| yr_render::named_color(&rules, COLOR_NAMES[i])),
            captions: ["AlliedLoad", "SovietLoad"].map(|name| yr_render::named_color(&rules, name)),
            cursor: cursor::Art::load(fs),
            backgrounds: Country::ALL
                .into_iter()
                .filter_map(|country| {
                    let (shp, pal) = country.loading_art();
                    let image = fs
                        .read(shp)
                        .zip(fs.read(pal))
                        .and_then(|(shp, pal)| decode_background(shp, pal))?;
                    Some((country, image))
                })
                .collect(),
            flags: Country::ALL
                .into_iter()
                .filter_map(|country| {
                    let image = fs
                        .read(country.flag())
                        .and_then(|b| PcxImage::parse(b).ok())?;
                    Some((country, decode_flag(image)?))
                })
                .collect(),
            font: fs
                .read("GAME.FNT")
                .and_then(|bytes| UnicodeFont::parse(bytes).ok())
                .filter(|font| font.stride <= 8 && font.lines <= 32 && font.line_height <= 40),
            strings: fs.read("RA2MD.CSF").and_then(|b| CsfFile::parse(b).ok()),
        }
    }

    fn label<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.strings
            .as_ref()
            .and_then(|strings| strings.get(key))
            .unwrap_or(fallback)
    }
}

fn decode_background(bytes: &[u8], palette: &[u8]) -> Option<Image> {
    let image = ShpFile::parse(bytes).ok()?;
    if (image.width, image.height, image.frames.len()) != (800, 600, 1) {
        return None;
    }
    let palette = Palette::parse(palette)?;
    let pixels = image
        .decode_frame_canvas(0)
        .ok()?
        .into_iter()
        .flat_map(|i| {
            if i == 0 {
                [0, 0, 0, 255]
            } else {
                palette.rgba(i)
            }
        })
        .collect();
    Some(Image {
        width: 800,
        height: 600,
        pixels,
    })
}

/// The supported match has a local player and one computer. Loading
/// progress represents completed preparation stages, not elapsed-time estimates
/// or invented remote-client progress.
pub fn draw(
    frame: &mut [u8],
    width: u32,
    height: u32,
    art: &Art,
    state: &State,
    preview: Option<&Preview>,
) {
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        != Some(frame.len())
    {
        return;
    }
    let layout = Layout::for_state(state, width, height);
    if layout.scale <= 0.0 {
        return;
    }
    let mut pixels = vec![0; 800 * 600 * 4];
    let mut canvas = Canvas {
        frame: &mut pixels,
        width: 800,
        height: 600,
        face: Typeface(art.font.as_ref()),
    };
    canvas.rect((0, 0), (800, 600), [4, 8, 24, 255]);
    let config = &state.loading.config;
    let country = config.players[0].country;
    let known = !state.loading.map_name.is_empty();
    let caption = art.captions[usize::from(country.side() == Side::Soviet)];
    let caption = if !known {
        WHITE
    } else if caption == [0; 3] {
        FALLBACK_CAPTION
    } else {
        [caption[0], caption[1], caption[2], 255]
    };
    if let Some((_, background)) = art.backgrounds.iter().find(|(c, _)| known && *c == country) {
        canvas.image((0, 0), background);
    }
    text(
        &mut canvas,
        (20, 300),
        390,
        art.label("GUI:LoadingEx", "Loading..."),
        caption,
    );
    if known {
        details(
            &mut canvas,
            art.label(&format!("Name:{}", country.specialty()), country.name()),
            art.label(&format!("LoadBrief:{}", country.brief()), country.name()),
            art.label(&format!("Name:{}", country.id()), country.name()),
            caption,
        );
        let player = &config.players[0];
        let rgb = art
            .colors
            .get(usize::from(player.color))
            .copied()
            .unwrap_or([170; 3]);
        let flag = art
            .flags
            .iter()
            .find(|(c, _)| *c == country)
            .map(|(_, image)| image);
        let completed = match state.loading.stage {
            LoadStage::Waiting | LoadStage::Preview | LoadStage::Session => 0,
            LoadStage::Terrain => 1,
            LoadStage::Artwork => 2,
        };
        client(
            &mut canvas,
            flag,
            &player.name,
            [rgb[0], rgb[1], rgb[2], 255],
            completed,
        );
    }
    text(&mut canvas, (480, 574), 270, &state.loading.name, WHITE);
    canvas.rect(
        (PREVIEW.0, PREVIEW.1),
        (PREVIEW.2, PREVIEW.3),
        [0, 0, 0, 255],
    );
    if let Some(preview) = preview {
        picture(&mut canvas, preview);
    }
    let (x, y, _, _) = layout.button(0, Button::Back);
    text(&mut canvas, (x + 8, y + 5), 144, "Esc: Cancel", caption);
    if state.keyboard_focus {
        canvas.rect((x + 8, y + 22), (100, 1), caption);
    }
    for y in 0..height {
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            let color = match layout.local((f64::from(x), f64::from(y))) {
                Some((sx, sy)) => {
                    let src = (sy as usize * 800 + sx as usize) * 4;
                    [
                        pixels[src] & 248,
                        pixels[src + 1] & 252,
                        pixels[src + 2] & 248,
                        255,
                    ]
                }
                None => [0, 0, 0, 255],
            };
            frame[at..at + 4].copy_from_slice(&color);
        }
    }
}

fn decode_flag(image: PcxImage) -> Option<Image> {
    if (image.width, image.height) != (47, 23) {
        return None;
    }
    let mut pixels = image.to_rgba();
    for pixel in pixels.as_chunks_mut::<4>().0 {
        if pixel[..3] == [255, 0, 255] {
            pixel[3] = 0;
        }
    }
    Some(Image {
        width: image.width,
        height: image.height,
        pixels,
    })
}

fn details(canvas: &mut Canvas<'_>, specialty: &str, brief: &str, country: &str, color: [u8; 4]) {
    text(
        canvas,
        (20, 90),
        390,
        &specialty
            .chars()
            .take(160)
            .flat_map(char::to_uppercase)
            .take(160)
            .collect::<String>(),
        [0, 0, 0, 255],
    );
    for (line, text_value) in wrap(canvas.face, brief, 400).iter().enumerate() {
        text(canvas, (20, 158 + line as i32 * 17), 400, text_value, color);
    }
    let width = canvas.face.width(country).min(280) + 1;
    canvas.rect((740 - width - 4, 307), (width + 8, 21), [0, 0, 0, 255]);
    text(canvas, (740 - width, 310), 280, country, color);
}

fn wrap(face: Typeface<'_>, value: &str, width: i32) -> Vec<String> {
    let mut lines = vec![String::new()];
    let bounded: String = value.chars().take(640).collect();
    for word in bounded.split_whitespace().take(160) {
        let current = lines.last_mut().unwrap();
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && face.width(&candidate) > width {
            if lines.len() == 4 {
                break;
            }
            lines.push(word.chars().take(160).collect());
        } else {
            *current = candidate.chars().take(160).collect();
        }
    }
    lines
}

fn client(
    canvas: &mut Canvas<'_>,
    flag: Option<&Image>,
    name: &str,
    color: [u8; 4],
    completed: i32,
) {
    if let Some(flag) = flag {
        canvas.image((117, 323), flag);
    }
    text(canvas, (174, 326), 246, &format!("[{name}]"), color);
    canvas.rect((21, 329), (86, 11), color);
    canvas.rect((22, 330), (84, 9), [0, 0, 0, 255]);
    let filled = 82 * completed.clamp(0, 3) / 3;
    for y in 0..7 {
        let level = if y < 3 { 180 } else { 112 };
        let shaded = [
            ((u16::from(color[0]) * level) / 255) as u8,
            ((u16::from(color[1]) * level) / 255) as u8,
            ((u16::from(color[2]) * level) / 255) as u8,
            255,
        ];
        canvas.rect((23, 331 + y), (filled, 1), shaded);
    }
}

fn text(canvas: &mut Canvas<'_>, point: (i32, i32), width: i32, value: &str, color: [u8; 4]) {
    let mut fitted = String::new();
    for ch in value.chars().filter(|ch| !ch.is_control()).take(160) {
        if canvas.face.width(&fitted) + canvas.face.advance(ch) > width {
            break;
        }
        fitted.push(ch);
    }
    canvas.text(point, &fitted, color, 160);
}

fn picture(canvas: &mut Canvas<'_>, preview: &Preview) {
    if preview.width == 0
        || preview.height == 0
        || (preview.width as usize)
            .checked_mul(preview.height as usize)
            .and_then(|n| n.checked_mul(4))
            != Some(preview.rgba.len())
    {
        return;
    }
    let (sw, sh) = (u64::from(preview.width), u64::from(preview.height));
    let (cw, ch) = (PREVIEW.2 as u64, PREVIEW.3 as u64);
    let (tw, th) = if sw * ch >= sh * cw {
        (cw, sh * cw / sw)
    } else {
        (sw * ch / sh, ch)
    };
    if tw == 0 || th == 0 {
        return;
    }
    let (x, y) = (
        PREVIEW.0 + (cw - tw) as i32 / 2,
        PREVIEW.1 + (ch - th) as i32 / 2,
    );
    for dy in 0..th {
        for dx in 0..tw {
            let at = (((dy * sh / th) * sw + dx * sw / tw) * 4) as usize;
            canvas.pixel(
                x + dx as i32,
                y + dy as i32,
                preview.rgba[at..at + 4].try_into().unwrap(),
            );
        }
    }
}

#[cfg(test)]
mod tests;
