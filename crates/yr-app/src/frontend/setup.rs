use super::{Art, Preview};
use crate::{
    menu::{
        setup::{Field, Setup},
        State,
    },
    sidebar::Canvas,
};
const YELLOW: [u8; 4] = [255, 255, 0, 255];
const GRAY: [u8; 4] = [128, 128, 128, 255];

pub(super) fn draw(c: &mut Canvas<'_>, art: &Art, state: &State, preview: Option<&Preview>) {
    for (x, key, label) in [
        (59, "GUI:Players", "Players"),
        (287, "GUI:Side", "Side"),
        (425, "GUI:Color", "Color"),
        (488, "GUI:StartPosition", "Start"),
        (545, "GUI:Team", "Team"),
    ] {
        c.text((x, 34), art.label(key, label), YELLOW, 40);
    }
    // These rows describe the current two-owner Allied sandbox. Additional
    // participants, country changes, teams and color selection remain disabled.
    let rows = preview.map_or(2, |p| {
        p.starts.iter().filter(|s| s.is_some()).count().clamp(2, 8)
    });
    for row in 0..rows {
        let y = 59 + row as i32 * 26;
        for (x, w, label) in [
            (
                59,
                151,
                if row == 0 {
                    "Commander"
                } else if row == 1 {
                    "Computer"
                } else {
                    "Closed"
                },
            ),
            (
                287,
                118,
                if row < 2 {
                    art.label("Name:Americans", "America")
                } else {
                    "---"
                },
            ),
            (
                486,
                39,
                if row == 0 {
                    "1"
                } else if row == 1 {
                    "2"
                } else {
                    "---"
                },
            ),
            (546, 39, "---"),
        ] {
            bevel(c, x, y, w, 25);
            c.rect((x + 2, y + 2), (w - 4, 21), [16, 16, 16, 255]);
            c.text((x + 4, y + 3), label, GRAY, 22);
        }
        bevel(c, 423, y, 45, 25);
        c.rect(
            (425, y + 2),
            (41, 21),
            if row == 0 {
                [200, 160, 0, 255]
            } else if row == 1 {
                [160, 0, 0, 255]
            } else {
                [16, 16, 16, 255]
            },
        );
    }
    for (x, y, key, label) in [
        (71, 286, "GUI:ShortGame", "Short Game"),
        (71, 314, "GUI:MCVRepacks", "MCV Repacks"),
        (71, 341, "GUI:CratesAppear", "Crates Appear"),
        (71, 371, "GUI:Superweapons", "Superweapons"),
        (302, 369, "GUI:BuildOffAlly", "Build Off Ally ConYards"),
    ] {
        if let Some(image) = art.setup_widgets.get(1) {
            c.image((x, y), image);
            c.shade((x, y), (18, 18));
        }
        c.text((x + 25, y), art.label(key, label), GRAY, 44);
    }
    c.text(
        (302, 286),
        art.label("GUI:GameSpeed", "Game Speed"),
        GRAY,
        40,
    );
    c.text((404, 286), "Fixed", GRAY, 16);
    for field in [Field::Credits, Field::Units] {
        let y = Setup::y(field);
        let (key, label) = match field {
            Field::Credits => ("GUI:Credits", "Credits"),
            Field::Units => ("GUI:UnitCount", "Unit Count"),
        };
        c.text(
            (302, if field == Field::Units { 341 } else { 314 }),
            art.label(key, label),
            YELLOW,
            40,
        );
        bevel(c, 402, y - 2, 133, 26);
        // The track reveals the page beneath it. The shared palette quantum
        // matches the observed owner-drawn control's backing.
        for py in y..y + 22 {
            for px in 404..483 {
                let at = ((py as u32 * c.width + px as u32) * 4) as usize;
                for (channel, (mask, step)) in
                    [(248, 8), (252, 4), (248, 8)].into_iter().enumerate()
                {
                    c.frame[at + channel] = (c.frame[at + channel] & mask).saturating_sub(step);
                }
            }
        }
        c.rect((483, y - 1), (1, 24), [160, 188, 192, 255]);
        c.rect((484, y - 1), (1, 24), [104, 120, 128, 255]);
        c.pixel(484, y + 22, [128, 156, 160, 255]);
        c.pixel(483, y + 23, [128, 156, 160, 255]);
        if let Some(image) = art.setup_widgets.get(2) {
            c.image((485, y), image);
        }
        if let Some(image) = art.setup_widgets.first() {
            c.image((state.setup.thumb_x(field), y), image);
        }
        let value = state.setup.value(field).to_string();
        c.text(
            (484 + (48 - c.face.width(&value)) / 2, y + 2),
            &value,
            YELLOW,
            16,
        );
        if state.keyboard_focus && state.setup.focus == Some(field) {
            c.rect((404, y + 21), (79, 1), YELLOW);
        }
    }
    let title = state
        .maps
        .entries
        .iter()
        .find(|m| m.asset.eq_ignore_ascii_case(&state.map_name))
        .map_or(state.map_name.as_str(), |m| m.title.as_str());
    for (text, y) in [(art.label("GUI:Battle", "Battle"), 167), (title, 189)] {
        let text = super::draw::fit(c.face, text, 136);
        let width = c.face.width(&text);
        c.text((650 + (136 - width.min(136)) / 2, y), &text, YELLOW, 28);
    }
}
fn bevel(c: &mut Canvas<'_>, x: i32, y: i32, w: i32, h: i32) {
    let edge = [160, 188, 192, 255];
    let shadow = [104, 120, 128, 255];
    let corner = [128, 156, 160, 255];
    c.rect((x, y), (w - 1, 1), edge);
    c.rect((x, y), (1, h - 1), edge);
    c.rect((x + 1, y + 1), (w - 1, 1), shadow);
    c.rect((x + 1, y + 1), (1, h - 1), shadow);
    c.rect((x, y + h - 2), (w - 1, 1), edge);
    c.rect((x + w - 2, y), (1, h - 1), edge);
    c.rect((x + 1, y + h - 1), (w - 1, 1), shadow);
    c.rect((x + w - 1, y + 1), (1, h - 1), shadow);
    for point in [
        (x + w - 1, y),
        (x + w - 2, y + 1),
        (x + 1, y + h - 2),
        (x, y + h - 1),
    ] {
        c.pixel(point.0, point.1, corner);
    }
}
