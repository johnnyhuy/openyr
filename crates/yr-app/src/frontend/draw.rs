use super::{message, Art};
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
const YELLOW: [u8; 4] = [255, 255, 0, 255];
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
    if s.page == Page::ChooseMap {
        if let Some(im) = art.chooser_background.as_ref().or(art.background.as_ref()) {
            c.image((0, 0), im);
        }
    } else if s.page == Page::Skirmish {
        if let Some(im) = art.setup_background.as_ref().or(art.background.as_ref()) {
            c.image((0, 0), im);
        }
    } else if matches!(s.page, Page::Main | Page::SinglePlayer | Page::Exit) {
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
    let map_top =
        s.page == Page::ChooseMap || (s.page == Page::Skirmish && scene.preview.is_some());
    if let Some(im) = art.top.get(usize::from(map_top)) {
        c.image((632, 0), im);
    }
    if map_top {
        if let Some(map) = scene.preview {
            map_preview(&mut c, map, &art.start_markers, s.page == Page::ChooseMap);
        } else {
            c.rect((643, 37), (146, 114), [0, 0, 0, 255]);
        }
    } else if let Some(im) = art
        .warning
        .get(scene.phase as usize % art.warning.len().max(1))
    {
        c.image((670, 48), im);
    }
    for y in (199..577).step_by(42) {
        if let Some(im) = &art.button_back {
            c.image((632, y), im);
        }
    }
    if let Some(im) = &art.bottom {
        c.image((632, 577), im);
    }
    let title = match s.page {
        Page::Main => art.label("GUI:MainMenu", "Main menu"),
        Page::SinglePlayer => art.label("GUI:SinglePlayer", "Solo play"),
        Page::Skirmish => art.label("GUI:SkirmishGame", "Skirmish Game"),
        Page::ChooseMap => art.label("GUI:ChooseMap", "Choose Map"),
        Page::Load => art.label("GUI:LoadGame", "Saved matches"),
        Page::Delete => art.label("GUI:DeleteGame", "Delete saved match"),
        Page::Controls => art.label("GUI:Options", "Controls"),
        _ => "OpenYR",
    };
    let (title_x, title_width) = if s.page == Page::ChooseMap {
        (634, 164)
    } else {
        (635, 163)
    };
    heading(
        &mut c,
        title_x,
        if map_top { 3 } else { 9 },
        title_width,
        title,
        s.heading_refreshed,
    );
    if s.page == Page::Skirmish {
        super::setup::draw(&mut c, art, s, scene.preview);
    }
    if s.page == Page::ChooseMap {
        chooser(&mut c, art, s);
    }
    if matches!(s.page, Page::Load | Page::Delete) {
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
        Page::Confirm(intent) => {
            let text = match intent {
                crate::menu::Intent::Delete(_) => "Delete this saved match permanently?",
                crate::menu::Intent::Save(_) => "Replace this saved match?",
                crate::menu::Intent::Load(_) => "Load this saved match?",
            };
            centered(&mut c, 0, 80, 632, text, YELLOW);
            crate::menu::confirmation::draw(&mut c, l, intent, s.message_scroll);
        }

        Page::Exit => centered(&mut c, 0, 380, 632, "Exit OpenYR?", YELLOW),
        Page::Loading => {
            centered(&mut c, 0, 80, 632, "Loading", YELLOW);
            let name = fit(c.face, &s.loading.name, 568);
            centered(&mut c, 0, 124, 632, &name, WHITE);
            centered(&mut c, 0, 168, 632, s.loading.stage.label(), YELLOW);
            // Indeterminate activity, not a fabricated percentage of work.
            let x = 64 + (scene.phase % 46) as i32 * 10;
            c.rect((32, 212), (568, 4), [40, 40, 48, 255]);
            c.rect((x, 212), (72, 4), YELLOW);
            centered(
                &mut c,
                0,
                248,
                632,
                "Escape or Back cancels loading.",
                WHITE,
            );
        }
        Page::Message { text, .. } => {
            let (first, rows, _) = message::viewport(c.face, text, s.message_scroll);
            for (i, line) in message::lines(c.face, text)
                .skip(first)
                .take(rows)
                .enumerate()
            {
                c.text(
                    (
                        message::LEFT,
                        message::TOP + i as i32 * message::line_step(c.face),
                    ),
                    line,
                    WHITE,
                    usize::MAX,
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
            Button::ChooseMap => art.label("GUI:ChooseMap", "Choose Map"),
            Button::UseMap => art.label("GUI:UseMap", "Use Map"),
            Button::GenerateMap => art.label("GUI:RandomMap", "Random Map"),
            Button::Folder => "Game data folder",
            Button::Controls => art.label("GUI:Options", "Controls"),
            Button::Load => art.label("GUI:LoadGame", "Restore"),
            Button::Delete => art.label("GUI:DeleteGame", "Delete saved match"),
            Button::Quit => art.label("GUI:ExitGame", "Exit"),
            Button::Back if s.page == Page::ChooseMap => art.label("GUI:Cancel", "Cancel"),
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
            Button::EdgeScroll => {
                if s.edge_scroll {
                    "Edge scroll off"
                } else {
                    "Edge scroll on"
                }
            }
            Button::Accept => art.label("GUI:Yes", "Confirm"),
            Button::Decline => art.label("GUI:No", "Keep current"),
            _ => "",
        };
        let text_y = y + (h - c.face.line_height() + 1) / 2;
        centered(
            &mut c,
            x - 2,
            text_y,
            w,
            label,
            if enabled {
                YELLOW
            } else {
                [128, 128, 128, 255]
            },
        );
        if s.keyboard_focus
            && s.focus == index
            && enabled
            && (s.page != Page::Skirmish || s.setup.focus.is_none())
        {
            c.rect((x + 12, y + h - 6), (w - 24, 1), YELLOW);
        }
    }
    centered(&mut c, 638, 581, 156, "OpenYR 0.1", YELLOW);
    let message_status;
    let setup_status;
    let status = match &s.page {
        Page::Main if hovered.is_some() => "Select a mode. Gray actions are unavailable.",
        Page::SinglePlayer if hovered.is_some() => "Saved matches or supported skirmish setup.",
        Page::Main | Page::SinglePlayer => "",
        Page::Skirmish => {
            setup_status = s.setup.config.validate().err().unwrap_or_else(|| {
                "Two-player Allied/Soviet skirmish. Tab selects; Enter opens choices.".into()
            });
            setup_status.as_str()
        }
        Page::Load | Page::Delete => "PgUp/PgDn page; Home/End jump; Esc returns.",
        Page::ChooseMap => "Select a Battle map. Enter uses it; Escape keeps the current map.",
        Page::Loading => "Escape cancels and returns.",
        Page::Confirm(intent) => {
            message_status = crate::menu::confirmation::status(c.face, l, intent, s.message_scroll);
            &message_status
        }
        Page::Message { text, .. } => {
            let (first, rows, count) = message::viewport(c.face, text, s.message_scroll);
            if count > rows {
                message_status = format!(
                    "Lines {}-{} of {count}. Up/Down or wheel scroll; Escape returns.",
                    first + 1,
                    (first + rows).min(count)
                );
                &message_status
            } else {
                "Escape returns. Tab selects controls."
            }
        }
        _ => "Escape returns. Tab selects controls.",
    };
    c.text((8, 578), &fit(c.face, status, 610), YELLOW, 100);
    for y in 0..height {
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            if let Some((sx, sy)) = l.local((f64::from(x), f64::from(y))) {
                let src = ((sy as u32 * 800 + sx as u32) * 4) as usize;
                // Observed retail shell output truncates channels to RGB565.
                frame[at..at + 4].copy_from_slice(&[
                    pixels[src] & 0xf8,
                    pixels[src + 1] & 0xfc,
                    pixels[src + 2] & 0xf8,
                    pixels[src + 3],
                ]);
            } else {
                frame[at..at + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
}
fn chooser(c: &mut Canvas<'_>, art: &Art, state: &State) {
    control_label(
        c,
        120,
        33,
        388,
        art.label("GUI:SelectEngagement", "Select Engagement"),
        YELLOW,
    );
    control_label(
        c,
        116,
        98,
        197,
        art.label("GUI:GameType", "Game Type"),
        YELLOW,
    );
    control_label(
        c,
        338,
        98,
        197,
        art.label("GUI:GameMap", "Game Map"),
        YELLOW,
    );
    let edge = [160, 188, 192, 255];
    let shadow = [104, 120, 128, 255];
    for (x, w) in [(115, 199), (337, 199)] {
        c.rect((x, 126), (w - 1, 1), edge);
        c.rect((x, 126), (1, 346), edge);
        c.rect((x + 1, 127), (w - 1, 1), shadow);
        c.rect((x + 1, 127), (1, 346), shadow);
        c.rect((x, 471), (w - 1, 1), edge);
        c.rect((x + w - 2, 126), (1, 346), edge);
        c.rect((x + 1, 472), (w - 1, 1), shadow);
        c.rect((x + w - 1, 127), (1, 346), shadow);
    }
    // Observed list backing subtracts one saturated RGB565 quantum.
    for (left, width) in [(117, 195), (339, 175)] {
        for y in 128..471 {
            for x in left..left + width {
                let at = ((y * c.width + x) * 4) as usize;
                for (channel, (mask, step)) in
                    [(248, 8), (252, 4), (248, 8)].into_iter().enumerate()
                {
                    c.frame[at + channel] = (c.frame[at + channel] & mask).saturating_sub(step);
                }
            }
        }
    }
    c.rect((514, 127), (1, 345), edge);
    c.rect((117, 128), (195, 19), [255, 0, 0, 255]);
    for (row, (key, label)) in [
        ("GUI:Battle", "Battle"),
        ("GUI:FreeForAll", "Free For All"),
        ("GUI:TeamGame", "Team Alliance"),
    ]
    .iter()
    .enumerate()
    {
        c.text(
            (119, 128 + row as i32 * 19),
            art.label(key, label),
            if row == 0 {
                YELLOW
            } else {
                [128, 128, 128, 255]
            },
            40,
        );
    }
    for (index, entry) in state
        .maps
        .entries
        .iter()
        .enumerate()
        .skip(state.maps.top)
        .take(crate::menu::maps::VISIBLE)
    {
        let y = 128 + (index - state.maps.top) as i32 * 19;
        if index == state.maps.selected {
            c.rect((339, y), (175, 19), [255, 0, 0, 255]);
        }
        c.text((341, y), &fit(c.face, &entry.title, 171), YELLOW, 128);
    }
    // Leave the track transparent so the page backdrop shows through.
    c.rect((515, 127), (1, 345), shadow);
    c.rect((534, 127), (1, 345), edge);
    c.rect((515, 127), (19, 1), shadow);
    c.rect((515, 471), (20, 1), edge);
    let corner = [128, 156, 160, 255];
    for x in [115, 337] {
        for point in [(x + 198, 126), (x + 197, 127), (x + 1, 471), (x, 472)] {
            c.pixel(point.0, point.1, corner);
        }
    }
    for point in [(515, 126), (514, 127), (515, 471)] {
        c.pixel(point.0, point.1, corner);
    }
    if let [up, down, body, top, bottom] = art.scrollbar.as_slice() {
        c.image((516, 128), up);
        c.image((516, 449), down);
        let (y, height) = state.maps.thumb();
        for row in 0..height {
            for x in 0..18 {
                let at = (((row as u32 % body.height) * 18 + x) * 4) as usize;
                c.pixel(
                    516 + x as i32,
                    y + row,
                    body.pixels[at..at + 4].try_into().unwrap(),
                );
            }
        }
        c.image((516, y), top);
        c.image((516, y + height - 2), bottom);
    }
    if let Some(error) = &state.maps.error {
        c.text((116, 492), &fit(c.face, error, 419), YELLOW, 128);
    } else if state.maps.ready != Some(state.maps.selected) {
        c.text((116, 492), "Loading map preview...", YELLOW, 80);
    }
}

fn control_label(c: &mut Canvas<'_>, x: i32, y: i32, w: i32, text: &str, color: [u8; 4]) {
    let text = fit(c.face, text, w - 12);
    c.text(
        (x + (w - c.face.width(&text) - 1) / 2, y),
        &text,
        color,
        160,
    );
}

pub(super) fn fit(face: Typeface<'_>, text: &str, width: i32) -> String {
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
    c.text((x + w / 2 - c.face.width(&text) / 2, y), &text, color, 160);
}
fn heading(c: &mut Canvas<'_>, x: i32, y: i32, w: i32, text: &str, refreshed: bool) {
    let text = fit(c.face, text, w - 12);
    let Some((tail, _)) = text.char_indices().next_back() else {
        return;
    };
    let x = x + (w - c.face.width(&text)) / 2;
    let prefix = &text[..tail];
    c.text((x, y), prefix, YELLOW, 160);
    let advance: i32 = prefix.chars().map(|ch| c.face.advance(ch)).sum();
    // Initial and refreshed dialog captures differ only in this foreground.
    let blue = if refreshed { 0 } else { 24 };
    c.text((x + advance, y), &text[tail..], [255, 255, blue, 255], 1);
}
fn map_preview(c: &mut Canvas<'_>, map: &super::Preview, markers: &[Image], chooser: bool) {
    // Public UI measurements and renderer samples: formats/front-preview.md.
    let (x, y, width, height) = if chooser {
        (643, 37, 146, 114)
    } else {
        (644, 37, 145, 113)
    };
    c.rect((x, y), (width, height), [0, 0, 0, 255]);
    if map.width == 0
        || map.height == 0
        || (map.width as usize)
            .checked_mul(map.height as usize)
            .and_then(|n| n.checked_mul(4))
            != Some(map.rgba.len())
    {
        return;
    }
    let (sw, sh) = (u64::from(map.width), u64::from(map.height));
    let (cw, ch) = (width as u64, height as u64);
    let (tw, th) = if sw * ch >= sh * cw {
        (cw - 1, sh * cw / sw)
    } else {
        (sw * ch / sh, ch - 1)
    };
    if tw == 0 || th == 0 {
        return;
    }
    let mut image = Image {
        width: tw as u32,
        height: th as u32,
        pixels: Vec::with_capacity((tw * th * 4) as usize),
    };
    for dy in 0..th {
        for dx in 0..tw {
            let (sx, sy) = (dx * sw / tw, dy * sh / th);
            let at = ((sy * sw + sx) * 4) as usize;
            image.pixels.extend_from_slice(&map.rgba[at..at + 4]);
        }
    }
    let image_x = x + (width - tw as i32 + i32::from(chooser)) / 2;
    let image_y = y + (height - th as i32 + i32::from(chooser)) / 2;
    c.image((image_x, image_y), &image);
    // Generated terrain images have different bounds from authored thumbnails.
    if !map.authored {
        return;
    }
    let map_width = i64::from(map.map_rect.2);
    let (local_x, local_y, local_width, local_height) = map.local_rect;
    let header_width = i64::from(local_width) - 1;
    let header_height = i64::from(local_height) + map_width.rem_euclid(2);
    if map_width <= 0 || header_width <= 0 || header_height <= 0 {
        return;
    }
    for (start, marker) in map.starts.iter().zip(markers).take(8) {
        let Some((cell_x, cell_y)) = *start else {
            continue;
        };
        if marker.width != 18 || marker.height != 18 || marker.pixels.len() != 18 * 18 * 4 {
            continue;
        }
        let (cell_x, cell_y) = (i64::from(cell_x), i64::from(cell_y));
        let relative_x = (cell_x - cell_y).div_euclid(2) + map_width / 2 - i64::from(local_x);
        // The chooser uses the projected x directly. Skirmish has the
        // separately observed one-eighth-pixel sampling bias.
        let marker_x = i64::from(image_x)
            + (8 * relative_x * tw as i64 - if chooser { 0 } else { header_width })
                .div_euclid(8 * header_width)
            - 9;
        // Round the scaled absolute coordinates before subtracting. The Heck
        // capture distinguishes this from scaling their integer difference.
        let scale_y = th as f64 / header_height as f64;
        let point_y = (((cell_x + cell_y + 1) / 2) as f64 * scale_y) as f32;
        let origin_y = ((map_width / 2 + i64::from(local_y) + 1) as f64 * scale_y) as f32;
        let marker_y = i64::from(image_y) + (point_y - origin_y).floor() as i64 - 6;
        for (index, color) in marker.pixels.as_chunks::<4>().0.iter().enumerate() {
            let px = marker_x + (index % 18) as i64;
            let py = marker_y + (index / 18) as i64;
            if color[3] != 0
                && (i64::from(x)..i64::from(x + width)).contains(&px)
                && (i64::from(y)..i64::from(y + height)).contains(&py)
            {
                c.pixel(px as i32, py as i32, *color);
            }
        }
    }
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
