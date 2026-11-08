use super::{confirmation, message, Art, Button, Layout, Page, State};
use crate::sidebar::{text::Typeface, Canvas};
const YELLOW: [u8; 4] = [255, 232, 0, 255];
const WHITE: [u8; 4] = [220, 232, 240, 255];

pub fn draw(
    frame: &mut [u8],
    width: u32,
    height: u32,
    art: &Art,
    state: &State,
    cursor: (f64, f64),
) {
    if !state.visible() {
        return;
    }
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return;
    };
    if frame.len() != size {
        return;
    }
    let layout = Layout::new(width, height);
    if layout.scale <= 0.0 {
        return;
    }
    let mut pixels = vec![0; (layout.width * layout.height * 4) as usize];
    let mut c = Canvas {
        frame: &mut pixels,
        width: layout.width,
        height: layout.height,
        face: Typeface(art.font.as_ref()),
    };
    c.rect(
        (0, 0),
        (layout.width as i32, layout.height as i32),
        [5, 9, 24, 255],
    );
    let world = layout.width as i32 - 168;
    if let Some(image) = art.backgrounds.iter().find(|im| im.width == world as u32) {
        c.image((0, 0), image);
    }
    if let Some(image) = &art.header {
        c.image((0, layout.height as i32 - 32), image);
    }
    for (image, y) in &art.chrome {
        c.image((world, *y), image);
    }
    for (i, image) in art.tabs.iter().enumerate() {
        c.image((world + 26 + i as i32 * 28, 197), image);
    }
    if let Some(image) = &art.row {
        for y in (227..layout.height as i32).step_by(50) {
            c.image((world, y), image);
        }
    }
    // Keep the metal sides while replacing cameo pockets with the dialog well.
    c.rect(
        (world + 23, 228),
        (125, layout.height as i32 - 254),
        [0, 0, 0, 255],
    );
    if let Some(image) = &art.bottom {
        c.image((world, layout.height as i32 - 26), image);
    }
    let (title, detail) = match &state.page {
        Page::Pause => ("", String::new()),
        Page::Save => (
            art.label("GUI:SaveMissionMenu", "Store match"),
            String::new(),
        ),
        Page::Load => (
            art.label("GUI:LoadMissionMenu", "Restore match"),
            String::new(),
        ),
        Page::Delete => (
            art.label("GUI:DeleteGame", "Delete saved match"),
            String::new(),
        ),
        Page::Controls => (
            art.label("GUI:GameControls", "Controls"),
            format!(
                "Audio: {}    World zoom: {}%",
                if state.muted { "off" } else { "on" },
                state.zoom
            ),
        ),
        Page::Abort => (
            art.label("GUI:AskAbortMission", "Choose how to leave this match"),
            String::new(),
        ),
        Page::Confirm(super::Intent::Save(_)) => (
            art.label("TXT_CONFIRM_SAVE", "Replace this saved match?"),
            String::new(),
        ),
        Page::Confirm(super::Intent::Load(_)) => ("Replace the current match?", String::new()),
        Page::Confirm(super::Intent::Delete(_)) => {
            ("Delete this saved match permanently?", String::new())
        }
        Page::Loading => ("Loading", String::new()),
        Page::Result(true) => (art.label("TXT_VICTORIOUS", "Victory"), String::new()),
        Page::Result(false) => (art.label("TXT_SCENARIO_LOST", "Defeat"), String::new()),
        Page::Message { .. } => ("OpenYR", String::new()),
        Page::Closed
        | Page::Main
        | Page::SinglePlayer
        | Page::Skirmish
        | Page::ChooseMap
        | Page::Exit => ("", String::new()),
    };
    let title = fit(c.face, title, world - 48);
    let x = (world - c.face.width(&title)) / 2;
    c.text((x, 28), &title, YELLOW, 160);
    if let Page::Confirm(intent) = &state.page {
        confirmation::draw(&mut c, layout, intent, state.message_scroll);
    } else if let Page::Message { text, .. } = &state.page {
        let region = message::Region::pause(layout);
        let (first, rows, _) = region.viewport(c.face, text, state.message_scroll);
        for (i, line) in message::lines(c.face, text, region.width)
            .skip(first)
            .take(rows)
            .enumerate()
        {
            c.text(
                (
                    region.left,
                    region.top + i as i32 * message::line_step(c.face),
                ),
                line,
                WHITE,
                usize::MAX,
            );
        }
    } else {
        for (i, line) in detail
            .chars()
            .take(400)
            .collect::<Vec<_>>()
            .chunks(50)
            .take(8)
            .enumerate()
        {
            let line: String = line.iter().collect();
            c.text((40, 72 + i as i32 * 22), &line, WHITE, 50);
        }
    }
    if state.page == Page::Loading {
        for (text, y) in [
            (state.loading.name.as_str(), 72),
            (state.loading.stage.label(), 104),
        ] {
            let text = fit(c.face, text, world - 80);
            c.text((40, y), &text, WHITE, usize::MAX);
        }
    }
    if matches!(state.page, Page::Save | Page::Load | Page::Delete) {
        c.rect((38, 142), (world - 76, 250), [0, 0, 0, 255]);
        for (row, slot) in state.slots.iter().enumerate().skip(state.scroll).take(10) {
            let y = 150 + (row - state.scroll) as i32 * 24;
            if state.selected == Some(row) {
                c.rect((40, y), (world - 80, 23), [28, 62, 108, 255]);
            }
            c.text(
                (48, y + 1),
                &fit(c.face, &slot.name, world - 110),
                WHITE,
                48,
            );
        }
        if state.slots.is_empty() {
            c.text((48, 154), "No saved matches", WHITE, 40);
        }
        if state.page == Page::Save {
            c.rect(
                (38, 414),
                (world - 76, 30),
                if state.name_selected {
                    [32, 56, 88, 255]
                } else {
                    [0, 0, 0, 255]
                },
            );
            c.text(
                (48, 420),
                &editing_name(c.face, &state.name, world - 110),
                YELLOW,
                50,
            );
        }
    }
    let hovered = layout.hit(state, cursor);
    for (i, (button, enabled)) in state.buttons().into_iter().enumerate() {
        let (x, y, w, h) = layout.button(i, button);
        let phase = if state.pressed == Some(button) {
            1
        } else if hovered == Some(button) {
            2
        } else {
            0
        };
        if let Some(image) = art.controls.get(phase) {
            c.image((x, y), image);
        } else {
            c.rect((x, y), (w, h), [24, 64, 100, 255]);
        }
        if !enabled {
            c.shade((x, y), (w, h));
        }
        let label = match button {
            Button::MainMenu => art.label("GUI:MainMenu", "Main menu"),
            Button::SinglePlayer
            | Button::Internet
            | Button::Network
            | Button::Movies
            | Button::Campaign
            | Button::Skirmish
            | Button::Start
            | Button::ChooseMap
            | Button::UseMap
            | Button::GenerateMap
            | Button::Folder => "",
            Button::Controls => art.label("GUI:GameControls", "Controls"),
            Button::Load => art.label("GUI:LoadGame", "Restore"),
            Button::Save => art.label("GUI:SaveGame", "Store"),
            Button::Delete => art.label("GUI:DeleteGame", "Remove"),
            Button::Abort => art.label("GUI:AbortMission", "Leave match"),
            Button::Resume => art.label("GUI:Resume", "Continue"),
            Button::Back => art.label("GUI:Back", "Return"),
            Button::Quit => art.label("GUI:Quit", "Exit"),
            Button::Restart => art.label("GUI:Restart", "Start again"),
            Button::Mute => {
                if state.muted {
                    "Enable audio"
                } else {
                    "Mute audio"
                }
            }
            Button::ZoomOut => "Zoom out",
            Button::EdgeScroll => {
                if state.edge_scroll {
                    "Edge scroll off"
                } else {
                    "Edge scroll on"
                }
            }
            Button::ZoomIn => "Zoom in",
            Button::Accept => art.label("GUI:Yes", "Confirm"),
            Button::Decline => art.label("GUI:No", "Keep current"),
        };
        let label = fit(c.face, label, w - 8);
        let tx = x + (w - c.face.width(&label)) / 2;
        c.text(
            (tx, y + 3),
            &label,
            if enabled {
                YELLOW
            } else {
                [128, 128, 128, 255]
            },
            48,
        );
        if state.focus == i && enabled {
            c.rect((x + 4, y + h - 3), (w - 8, 1), YELLOW);
        }
    }
    let focused = hovered.or_else(|| state.buttons().get(state.focus).map(|b| b.0));
    let message_status;
    let status = match &state.page {
        Page::Pause => match focused {
            Some(Button::Controls) => {
                art.label("STT:OptionInternetGameControls", "Adjust controls")
            }
            Some(Button::Load) => art.label("STT:IGSingleOptButtonLoad", "Restore a saved match"),
            Some(Button::Save) => art.label("STT:IGSingleOptButtonSave", "Store this match"),
            Some(Button::Delete) => "Delete a saved match",
            Some(Button::Abort) => art.label("STT:IGSingleOptButtonAbort", "Leave this match"),
            Some(Button::Resume) => art.label("STT:IDC_RESUME_MISSION", "Continue playing"),
            _ => "",
        },
        Page::Abort => "Unsaved changes will be lost if you restart or quit.",
        Page::Save => "Type a name or select a slot. Escape returns.",
        Page::Load | Page::Delete => "PgUp/PgDn page; Home/End jump; Esc returns.",
        Page::Loading => "Escape cancels this load and keeps the current match.",
        Page::Result(_) => "Choose restart or quit.",
        Page::Confirm(intent) => {
            message_status = confirmation::status(c.face, layout, intent, state.message_scroll);
            &message_status
        }
        Page::Message { text, .. } => {
            let region = message::Region::pause(layout);
            let (first, rows, count) = region.viewport(c.face, text, state.message_scroll);
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
        _ => "Escape returns. Tab and arrow keys select controls.",
    };
    c.text(
        (8, layout.height as i32 - 24),
        &fit(c.face, status, world - 24),
        YELLOW,
        120,
    );
    for y in 0..height {
        let sy = ((f64::from(y) - layout.origin.1) / layout.scale).floor() as i32;
        for x in 0..width {
            let sx = ((f64::from(x) - layout.origin.0) / layout.scale).floor() as i32;
            let dest = ((y * width + x) * 4) as usize;
            if sx >= 0 && sy >= 0 && sx < layout.width as i32 && sy < layout.height as i32 {
                let at = ((sy as u32 * layout.width + sx as u32) * 4) as usize;
                frame[dest..dest + 4].copy_from_slice(&pixels[at..at + 4]);
            } else {
                frame[dest..dest + 4].copy_from_slice(&[0, 0, 0, 255]);
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

fn editing_name(face: Typeface<'_>, name: &str, width: i32) -> String {
    // The editor stores at most 48 characters. Bound rendering independently
    // and keep the end/caret visible without changing the stored name.
    let mut chars: Vec<_> = name.chars().rev().take(48).collect();
    chars.reverse();
    chars.push('|');
    let text: String = chars.into_iter().collect();
    if face.width(&text) <= width {
        return text;
    }
    for (start, _) in text.char_indices().skip(1) {
        let shown = format!("..{}", &text[start..]);
        if face.width(&shown) <= width {
            return shown;
        }
    }
    fit(face, "|", width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_edit_text_keeps_caret_unicode_and_short_names_within_measured_width() {
        let face = Typeface(None);
        assert_eq!(editing_name(face, "Checkpoint", 362), "Checkpoint|");
        for name in ["W".repeat(48), "界é😀".repeat(16), "界".repeat(10000)] {
            let shown = editing_name(face, &name, 362);
            assert!(shown.starts_with("..") && shown.ends_with('|'));
            assert!(face.width(&shown) <= 362);
            let suffix = &shown[2..shown.len() - 1];
            assert!(name.ends_with(suffix));
        }
        assert_eq!(editing_name(face, "W", 8), "|");
        assert_eq!(editing_name(face, "W", 0), "");
    }
}
