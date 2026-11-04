//! Retail country loading artwork with the installed map's preview.
//! Asset evidence and remaining calibration: formats/loading-screen.md.

use crate::{
    cursor,
    frontend::Preview,
    menu::{Button, Layout, LoadStage, State},
    sidebar::{art::Image, text::Typeface, Canvas},
};
use yr_assets::AssetFs;
use yr_formats::{CsfFile, Palette, ShpFile, UnicodeFont};

const PREVIEW: (i32, i32, i32, i32) = (500, 380, 216, 166);
const GOLD: [u8; 4] = [248, 232, 96, 255];
const WHITE: [u8; 4] = [224, 232, 240, 255];

#[derive(Default)]
pub struct Art {
    pub cursor: cursor::Art,
    background: Option<Image>,
    font: Option<UnicodeFont>,
    strings: Option<CsfFile>,
}

impl Art {
    pub fn load(fs: &AssetFs) -> Self {
        Self {
            cursor: cursor::Art::load(fs),
            background: fs
                .read("LS800USTATES.SHP")
                .zip(fs.read("MPLSU.PAL"))
                .and_then(|(shp, pal)| decode_background(shp, pal)),
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

/// The supported sandbox has an American player and one computer. Loading
/// bars represent completed preparation stages, not elapsed-time estimates
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
    if let Some(background) = &art.background {
        canvas.image((0, 0), background);
    }
    text(
        &mut canvas,
        (24, 20),
        390,
        art.label("GUI:LoadingEx", "Loading..."),
        GOLD,
    );
    text(
        &mut canvas,
        (24, 90),
        390,
        art.label("Name:Americans", "America"),
        GOLD,
    );
    text(
        &mut canvas,
        (24, 166),
        390,
        art.label("LoadBriefShort:USA", "America"),
        WHITE,
    );
    text(
        &mut canvas,
        (490, 361),
        240,
        art.label("GUI:Battle", "Battle"),
        GOLD,
    );
    text(&mut canvas, (480, 574), 270, &state.loading.name, WHITE);
    canvas.rect(
        (PREVIEW.0, PREVIEW.1),
        (PREVIEW.2, PREVIEW.3),
        [0, 0, 0, 255],
    );
    if let Some(preview) = preview {
        picture(&mut canvas, preview);
    }
    let completed = match state.loading.stage {
        LoadStage::Waiting | LoadStage::Preview | LoadStage::Session => 0,
        LoadStage::Terrain => 1,
        LoadStage::Artwork => 2,
    };
    for (name, y, color) in [
        ("Commander", 506, GOLD),
        ("Computer", 538, [232, 64, 64, 255]),
    ] {
        text(&mut canvas, (24, y), 126, name, color);
        canvas.rect((158, y + 3), (270, 10), [64, 64, 80, 255]);
        canvas.rect((159, y + 4), (268, 8), [0, 0, 0, 255]);
        canvas.rect((159, y + 4), (268 * completed / 3, 8), color);
    }
    let (x, y, _, _) = layout.button(0, Button::Back);
    text(&mut canvas, (x + 8, y + 5), 144, "Esc: Cancel", GOLD);
    if state.keyboard_focus {
        canvas.rect((x + 8, y + 22), (100, 1), GOLD);
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
