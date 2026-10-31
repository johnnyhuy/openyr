use super::Art;
use crate::{
    menu::{Button, Layout, Page, State},
    movie::Frame,
    sidebar::{art::Image, text::Typeface, Canvas},
};
pub struct Scene<'a> {
    pub state: &'a State,
    pub movie: Option<&'a Frame>,
    pub preview: Option<&'a super::Preview>,
    pub phase: u64,
    pub cursor: (f64, f64),
}
const YELLOW: [u8; 4] = [255, 232, 0, 255];
const WHITE: [u8; 4] = [220, 232, 240, 255];
pub fn draw(frame: &mut [u8], width: u32, height: u32, art: &Art, scene: Scene<'_>) {
    if (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        != Some(frame.len())
    {
        return;
    }
    let s = scene.state;
    let l = Layout::for_state(s, width, height);
    if l.scale <= 0.0 {
        return;
    }
    let mut pixels = vec![0; 800 * 600 * 4];
    let mut c = Canvas {
        frame: &mut pixels,
        width: 800,
        height: 600,
        face: Typeface(art.font.as_ref()),
    };
    c.rect((0, 0), (800, 600), [12, 12, 16, 255]);
    if matches!(s.page, Page::Main | Page::SinglePlayer | Page::Exit) {
        if let Some(v) = scene.movie {
            // The observed movie is 632x570, including its own frame border.
            picture(&mut c, (0, 0, 632, 570), v.width, v.height, &v.rgba);
        } else if let Some(im) = &art.background {
            c.image((0, 0), im);
        }
    } else if let Some(im) = &art.background {
        c.image((0, 0), im);
    }
    if let Some(im) = &art.status {
        c.image((0, 568), im);
    }
    let map_top = s.page == Page::Skirmish && scene.preview.is_some();
    if let Some(im) = art.top.get(usize::from(map_top)) {
        c.image((632, 0), im);
    }
    if map_top {
        if let Some(map) = scene.preview {
            picture(
                &mut c,
                (644, 40, 144, 140),
                map.width,
                map.height,
                &map.rgba,
            );
        }
    } else if let Some(im) = art
        .warning
        .get(scene.phase as usize % art.warning.len().max(1))
    {
        c.image((670, 45), im);
    }
    for y in (199..535).step_by(42) {
        if let Some(im) = &art.button_back {
            c.image((632, y), im);
        }
        if let Some(im) = art.buttons.first() {
            c.image((638, y), im);
        }
    }
    if let Some(im) = &art.bottom {
        c.image((632, 535), im);
    }
    let title = match s.page {
        Page::Main => art.label("GUI:MainMenu", "Main menu"),
        Page::SinglePlayer => art.label("GUI:SinglePlayer", "Solo play"),
        Page::Skirmish => art.label("GUI:Skirmish", "Match setup"),
        Page::Load => art.label("GUI:LoadGame", "Saved matches"),
        Page::Controls => art.label("GUI:Options", "Controls"),
        _ => "OpenYR",
    };
    centered(&mut c, 632, 8, 168, title, YELLOW);
    if s.page == Page::Skirmish {
        centered(
            &mut c,
            0,
            32,
            632,
            art.label("GUI:Skirmish", "Match setup"),
            YELLOW,
        );
        c.text((32, 82), "EB1.MMX", WHITE, 48);
        c.text((32, 114), "Allied player and computer opponent", WHITE, 64);
        c.text((32, 146), "Starting credits: 10000", WHITE, 48);
        c.text(
            (32, 178),
            "Prototype AI; campaign rules are unavailable.",
            WHITE,
            64,
        );
        c.text(
            (32, 210),
            "Additional maps and factions are unavailable.",
            WHITE,
            64,
        );
        if let Some(map) = scene.preview {
            picture(
                &mut c,
                (32, 260, 568, 280),
                map.width,
                map.height,
                &map.rgba,
            );
        }
    }
    if s.page == Page::Load {
        c.rect((38, 142), (556, 250), [0, 0, 0, 255]);
        if s.slots.is_empty() {
            c.text((48, 154), "No saved matches", WHITE, 48);
        }
        for (index, slot) in s.slots.iter().enumerate().skip(s.scroll).take(10) {
            let y = 150 + (index - s.scroll) as i32 * 24;
            if s.selected == Some(index) {
                c.rect((40, y), (552, 23), [80, 24, 16, 255]);
            }
            c.text((48, y + 1), &fit(c.face, &slot.name, 526), WHITE, 48);
        }
    }
    match &s.page {
        Page::Controls => {
            c.text(
                (32, 80),
                &format!("Audio: {}", if s.muted { "off" } else { "on" }),
                WHITE,
                48,
            );
            c.text((32, 112), &format!("World zoom: {}%", s.zoom), WHITE, 48);
            c.text(
                (32, 160),
                "Choose the game data folder through macOS.",
                WHITE,
                64,
            );
        }
        Page::Confirm(_) => centered(&mut c, 0, 80, 632, "Load this saved match?", YELLOW),
        Page::Exit => centered(&mut c, 0, 380, 632, "Exit OpenYR?", YELLOW),
        Page::Loading => centered(&mut c, 0, 80, 632, "Preparing the match...", YELLOW),
        Page::Message { text, .. } => {
            for (i, line) in text
                .chars()
                .take(360)
                .collect::<Vec<_>>()
                .chunks(48)
                .enumerate()
            {
                c.text(
                    (32, 72 + i as i32 * 24),
                    &line.iter().collect::<String>(),
                    WHITE,
                    48,
                );
            }
        }
        _ => {}
    }
    let hovered = l.hit(s, scene.cursor);
    for (index, (button, enabled)) in s.buttons().into_iter().enumerate() {
        let (x, y, w, h) = l.button(index, button);
        let phase = if s.pressed == Some(button) {
            4
        } else if hovered == Some(button) {
            3
        } else {
            2
        };
        if let Some(im) = art.buttons.get(phase) {
            c.image((x, y), im);
        } else {
            c.rect((x, y), (w, h), [90, 16, 8, 255]);
        }
        if !enabled {
            c.shade((x, y), (w, h));
        }
        let label = match button {
            Button::SinglePlayer => art.label("GUI:SinglePlayer", "Solo play"),
            Button::Internet => art.label("GUI:Internet", "Online"),
            Button::Network => art.label("GUI:Network", "LAN"),
            Button::Movies => art.label("GUI:MoviesAndCredits", "Movies / credits"),
            Button::Campaign => art.label("GUI:NewCampaign", "Campaign"),
            Button::Skirmish => art.label("GUI:Skirmish", "Match setup"),
            Button::MainMenu => art.label("GUI:MainMenu", "Main menu"),
            Button::Start => art.label("GUI:StartGame", "Start match"),
            Button::Folder => "Game data folder",
            Button::Controls => art.label("GUI:Options", "Controls"),
            Button::Load => art.label("GUI:LoadGame", "Restore"),
            Button::Quit => art.label("GUI:ExitGame", "Exit"),
            Button::Back => art.label("GUI:Back", "Return"),
            Button::Mute => {
                if s.muted {
                    "Enable audio"
                } else {
                    "Mute audio"
                }
            }
            Button::ZoomIn => "Zoom in",
            Button::ZoomOut => "Zoom out",
            Button::Accept => art.label("GUI:Yes", "Confirm"),
            Button::Decline => art.label("GUI:No", "Keep current"),
            _ => "",
        };
        let text_y = y + (h - c.face.line_height()) / 2;
        centered(
            &mut c,
            x,
            text_y,
            w,
            label,
            if enabled {
                YELLOW
            } else {
                [128, 128, 128, 255]
            },
        );
        if s.focus == index && enabled {
            c.rect((x + 12, y + h - 6), (w - 24, 1), YELLOW);
        }
    }
    centered(&mut c, 638, 581, 156, "OpenYR 0.1", YELLOW);
    let status = match s.page {
        Page::Main => "Select a mode. Gray actions are unavailable.",
        Page::SinglePlayer => "Saved matches or supported skirmish setup.",
        Page::Skirmish => "Start the supported EB1 skirmish.",
        Page::Load => "Select a saved match. Arrows select; Escape returns.",
        Page::Loading => "Escape cancels and returns.",
        _ => "Escape returns. Tab selects controls.",
    };
    c.text((8, 578), &fit(c.face, status, 610), YELLOW, 100);
    for y in 0..height {
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            if let Some((sx, sy)) = l.local((f64::from(x), f64::from(y))) {
                let src = ((sy as u32 * 800 + sx as u32) * 4) as usize;
                frame[at..at + 4].copy_from_slice(&pixels[src..src + 4]);
            } else {
                frame[at..at + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
}
fn fit(face: Typeface<'_>, text: &str, width: i32) -> String {
    let mut out = String::new();
    for ch in text.chars().take(160) {
        out.push(ch);
        if face.width(&out) > width {
            out.pop();
            break;
        }
    }
    out
}
fn centered(c: &mut Canvas<'_>, x: i32, y: i32, w: i32, text: &str, color: [u8; 4]) {
    let text = fit(c.face, text, w - 12);
    c.text((x + (w - c.face.width(&text)) / 2, y), &text, color, 160);
}
fn picture(c: &mut Canvas<'_>, rect: (i32, i32, i32, i32), w: u32, h: u32, rgba: &[u8]) {
    if w == 0
        || h == 0
        || (w as usize)
            .checked_mul(h as usize)
            .and_then(|v| v.checked_mul(4))
            != Some(rgba.len())
    {
        return;
    }
    let scale = (rect.2 as f64 / f64::from(w)).min(rect.3 as f64 / f64::from(h));
    let tw = (f64::from(w) * scale).floor() as i32;
    let th = (f64::from(h) * scale).floor() as i32;
    let x = rect.0 + (rect.2 - tw) / 2;
    let y = rect.1 + (rect.3 - th) / 2;
    let mut image = Image {
        width: tw as u32,
        height: th as u32,
        pixels: Vec::with_capacity((tw * th * 4) as usize),
    };
    for dy in 0..th {
        for dx in 0..tw {
            let sx = (f64::from(dx) / scale).floor() as u32;
            let sy = (f64::from(dy) / scale).floor() as u32;
            let at = ((sy * w + sx) * 4) as usize;
            image.pixels.extend_from_slice(&rgba[at..at + 4]);
        }
    }
    c.image((x, y), &image);
}
