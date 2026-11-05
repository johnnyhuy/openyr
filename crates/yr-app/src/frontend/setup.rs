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
    let rows = preview.map_or(2, |p| {
        p.starts.iter().filter(|s| s.is_some()).count().clamp(2, 8)
    });
    for row in 0..rows {
        let y = 59 + row as i32 * 26;
        let player = state.setup.config.players.get(row);
        let country = player.map(|p| p.country);
        let country_key = country.map(|c| format!("Name:{}", c.id()));
        let name = player.map_or("Closed", |p| p.name.as_str());
        let country_name = country.map_or("---", |c| {
            art.label(country_key.as_deref().unwrap(), c.name())
        });
        let start = player.and_then(|p| p.start).map_or_else(
            || if row < 2 { "Auto".into() } else { "---".into() },
            |p| (p + 1).to_string(),
        );
        let team = player
            .filter(|p| p.team != 0)
            .map_or_else(|| "---".to_string(), |p| p.team.to_string());
        for (x, w, label, field) in [
            (59, 151, name, Field::Name(row)),
            (287, 118, country_name, Field::Country(row)),
            (486, 39, start.as_str(), Field::Start(row)),
            (546, 39, team.as_str(), Field::Team(row)),
        ] {
            bevel(c, x, y, w, 25);
            let focused = state.setup.focus == Some(field);
            let selected = focused && matches!(field, Field::Name(_)) && state.setup.name_selected;
            c.rect(
                (x + 2, y + 2),
                (w - 4, 21),
                if selected {
                    [32, 56, 88, 255]
                } else {
                    [16, 16, 16, 255]
                },
            );
            let fitted = super::draw::fit(
                c.face,
                label,
                w - if matches!(field, Field::Name(_)) {
                    10
                } else {
                    20
                },
            );
            c.text(
                (x + 4, y + 3),
                &fitted,
                if focused { YELLOW } else { GRAY },
                32,
            );
            if player.is_some() && !matches!(field, Field::Name(_)) {
                c.text((x + w - 10, y + 4), "v", GRAY, 1);
            }
            if state.keyboard_focus && focused {
                c.rect((x + 3, y + 23), (w - 6, 1), YELLOW);
            }
        }
        bevel(c, 423, y, 45, 25);
        let rgb = player.map_or([16; 3], |p| art.colors[usize::from(p.color)]);
        c.rect((425, y + 2), (41, 21), [rgb[0], rgb[1], rgb[2], 255]);
        if state.setup.focus == Some(Field::Color(row)) {
            c.rect((425, y + 22), (41, 1), YELLOW);
        }
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
            if y != 286 {
                c.shade((x, y), (18, 18));
            } else if state.setup.config.short_game {
                c.text((x + 4, y + 1), "X", YELLOW, 1);
            }
        }
        c.text(
            (x + 25, y),
            art.label(key, label),
            if y == 286 { YELLOW } else { GRAY },
            44,
        );
    }
    c.text(
        (302, 286),
        art.label("GUI:GameSpeed", "Game Speed"),
        GRAY,
        40,
    );
    bevel(c, 402, 282, 133, 26);
    c.rect((404, 284), (129, 22), [16, 16, 16, 255]);
    c.text((408, 286), state.setup.config.speed.label(), YELLOW, 16);
    c.text((520, 286), "v", GRAY, 1);
    for field in [Field::Credits, Field::Units] {
        let y = Setup::y(field);
        let (key, label) = match field {
            Field::Credits => ("GUI:Credits", "Credits"),
            Field::Units => ("GUI:UnitCount", "Unit Count"),
            _ => continue,
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
        if let Some(image) = &art.setup_well {
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
    if let (Some(popup), Some((x, y, w, h))) = (state.setup.popup, state.setup.popup_rect()) {
        c.rect((x, y), (w, h), [16, 16, 16, 255]);
        for (index, label) in state.setup.choices(popup.field).iter().enumerate() {
            let enabled = state.setup.available(popup.field, index);
            let at = y + index as i32 * 22;
            if popup.selected == index {
                c.rect((x + 1, at + 1), (w - 2, 20), [64, 24, 16, 255]);
            }
            c.text(
                (x + 4, at + 3),
                &super::draw::fit(c.face, label, w - 8),
                if enabled { YELLOW } else { GRAY },
                32,
            );
        }
        c.rect((x, y), (w, 1), [160, 188, 192, 255]);
        c.rect((x, y), (1, h), [160, 188, 192, 255]);
        c.rect((x + w - 1, y), (1, h), [104, 120, 128, 255]);
        c.rect((x, y + h - 1), (w, 1), [104, 120, 128, 255]);
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
