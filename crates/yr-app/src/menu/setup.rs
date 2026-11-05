use crate::match_config::{Country, GameSpeed, MatchConfig, COLOR_LABELS};
use crate::session::StartOptions;
use yr_formats::IniFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Credits,
    Units,
    Name(usize),
    Country(usize),
    Color(usize),
    Start(usize),
    Team(usize),
    ShortGame,
    Speed,
}

impl Field {
    pub fn rect(self) -> (i32, i32, i32, i32) {
        match self {
            Self::Credits => (404, 314, 80, 22),
            Self::Units => (404, 340, 80, 22),
            Self::Name(row) => (59, 59 + row as i32 * 26, 151, 25),
            Self::Country(row) => (287, 59 + row as i32 * 26, 118, 25),
            Self::Color(row) => (423, 59 + row as i32 * 26, 45, 25),
            Self::Start(row) => (486, 59 + row as i32 * 26, 39, 25),
            Self::Team(row) => (546, 59 + row as i32 * 26, 39, 25),
            Self::ShortGame => (71, 286, 200, 18),
            Self::Speed => (404, 284, 131, 24),
        }
    }
    pub fn slider(self) -> bool {
        matches!(self, Self::Credits | Self::Units)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Popup {
    pub field: Field,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub min_credits: i64,
    pub max_credits: i64,
    pub credits: i64,
    pub increment: i64,
    pub min_units: u16,
    pub max_units: u16,
    pub units: u16,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            min_credits: 5000,
            max_credits: 10000,
            credits: 10000,
            increment: 100,
            min_units: 0,
            max_units: 10,
            units: 10,
        }
    }
}
impl Profile {
    pub fn from_ini(ini: &IniFile) -> Self {
        let default = Self::default();
        let get = |key, fallback| {
            ini.section("MultiplayerDialogSettings")
                .and_then(|s| s.get_i64(key))
                .unwrap_or(fallback)
        };
        let min_credits = get("MinMoney", default.min_credits).clamp(0, 1_000_000_000);
        let max_credits = get("MaxMoney", default.max_credits).clamp(min_credits, 1_000_000_000);
        let min_units = get("MinUnitCount", 0).clamp(0, 4096) as u16;
        let max_units = get("MaxUnitCount", 10).clamp(i64::from(min_units), 4096) as u16;
        Self {
            min_credits,
            max_credits,
            credits: get("Money", default.credits).clamp(min_credits, max_credits),
            increment: get("MoneyIncrement", default.increment).clamp(1, 1_000_000_000),
            min_units,
            max_units,
            units: get("UnitCount", 10).clamp(i64::from(min_units), i64::from(max_units)) as u16,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Setup {
    pub profile: Profile,
    pub options: StartOptions,
    pub focus: Option<Field>,
    pub drag: Option<Drag>,
    pub touched: [bool; 2],
    pub config: MatchConfig,
    pub popup: Option<Popup>,
    pub name_selected: bool,
    pub starts: [bool; 8],
}
#[derive(Debug, Clone, Copy)]
pub struct Drag {
    field: Field,
    x: i32,
    value: i64,
}

impl Setup {
    pub fn value(&self, field: Field) -> i64 {
        match field {
            Field::Credits => self.options.credits.unwrap_or(self.profile.credits),
            Field::Units => i64::from(self.options.units.unwrap_or(self.profile.units)),
            Field::Name(_) => 0,
            Field::Country(row) => self
                .config
                .players
                .get(row)
                .and_then(|p| Country::ALL.iter().position(|c| *c == p.country))
                .unwrap_or(0) as i64,
            Field::Color(row) => self
                .config
                .players
                .get(row)
                .map_or(0, |p| i64::from(p.color)),
            Field::Start(row) => self
                .config
                .players
                .get(row)
                .and_then(|p| p.start)
                .map_or(0, |p| i64::from(p) + 1),
            Field::Team(row) => self
                .config
                .players
                .get(row)
                .map_or(0, |p| i64::from(p.team)),
            Field::ShortGame => i64::from(self.config.short_game),
            Field::Speed => GameSpeed::ALL
                .iter()
                .position(|s| *s == self.config.speed)
                .unwrap_or(1) as i64,
        }
    }
    pub fn range(&self, field: Field) -> (i64, i64, i64) {
        match field {
            Field::Credits => (
                self.profile.min_credits,
                self.profile.max_credits,
                self.profile.increment,
            ),
            Field::Units => (
                i64::from(self.profile.min_units),
                i64::from(self.profile.max_units),
                1,
            ),
            Field::Country(_) => (0, 8, 1),
            Field::Color(_) => (0, 7, 1),
            Field::Start(_) => (0, 8, 1),
            Field::Team(_) => (0, 4, 1),
            Field::ShortGame => (0, 1, 1),
            Field::Speed => (0, 2, 1),
            Field::Name(_) => (0, 0, 1),
        }
    }
    pub fn set(&mut self, field: Field, value: i64) {
        if !field.slider() {
            if value < 0 || !self.available(field, value as usize) {
                return;
            }
            match field {
                Field::Country(row) => {
                    self.config.players[row].country = Country::ALL[value as usize]
                }
                Field::Color(row) => self.config.players[row].color = value as u8,
                Field::Start(row) => {
                    self.config.players[row].start =
                        (value != 0).then_some(value.saturating_sub(1) as u8)
                }
                Field::Team(row) => self.config.players[row].team = value as u8,
                Field::ShortGame => self.config.short_game = value != 0,
                Field::Speed => self.config.speed = GameSpeed::ALL[value as usize],
                _ => {}
            }
            return;
        }
        let (min, max, _) = self.range(field);
        let value = value.clamp(min, max);
        self.options.shared_forces = true;
        self.touched[usize::from(field == Field::Units)] = true;
        match field {
            Field::Credits => {
                self.options.credits = (value != self.profile.credits).then_some(value)
            }
            Field::Units => {
                self.options.units =
                    (value != i64::from(self.profile.units)).then_some(value as u16)
            }
            _ => {}
        }
    }
    pub fn update_profile(&mut self, profile: Profile) {
        let touched = self.touched;
        self.profile = profile;
        if let Some(v) = self.options.credits {
            self.set(Field::Credits, v);
        }
        if let Some(v) = self.options.units {
            self.set(Field::Units, i64::from(v));
        }
        self.touched = touched;
    }
    pub fn adjust(&mut self, field: Field, delta: i32) {
        if !field.slider() {
            let (min, max, _) = self.range(field);
            let mut value = self
                .value(field)
                .saturating_add(i64::from(delta))
                .clamp(min, max);
            for _ in min..=max {
                if self.available(field, value as usize) {
                    self.set(field, value);
                    break;
                }
                let next = value + i64::from(delta.signum());
                if next < min || next > max {
                    break;
                }
                value = next;
            }
            return;
        }
        let (_, _, step) = self.range(field);
        self.set(
            field,
            self.value(field)
                .saturating_add(i64::from(delta).saturating_mul(step)),
        );
    }
    pub fn y(field: Field) -> i32 {
        match field {
            Field::Credits => 314,
            Field::Units => 340,
            other => other.rect().1,
        }
    }
    pub fn hit((x, y): (i32, i32)) -> Option<Field> {
        Self::fields().into_iter().find(|f| {
            let (r, c, w, h) = f.rect();
            (r..r + w).contains(&x) && (c..c + h).contains(&y)
        })
    }
    pub fn thumb_x(&self, field: Field) -> i32 {
        let (min, max, _) = self.range(field);
        404 + i32::from(self.touched[usize::from(field == Field::Units)])
            + ((self.value(field) - min) * 66)
                .checked_div(max - min)
                .unwrap_or(0) as i32
    }
    pub fn press(&mut self, field: Field, x: i32) {
        self.focus = Some(field);
        if !field.slider() {
            self.drag = None;
            match field {
                Field::Name(_) => {
                    self.name_selected = true;
                    self.popup = None;
                }
                Field::ShortGame => {
                    self.config.short_game = !self.config.short_game;
                    self.popup = None;
                }
                _ => {
                    self.popup = Some(Popup {
                        field,
                        selected: self.value(field) as usize,
                    })
                }
            }
            return;
        }
        self.popup = None;
        let thumb = self.thumb_x(field);
        self.focus = Some(field);
        self.touched[usize::from(field == Field::Units)] = true;
        if !(thumb..thumb + 12).contains(&x) {
            let (min, max, step) = self.range(field);
            let scaled = i64::from(x.saturating_sub(411).clamp(0, 66)) * (max - min);
            self.set(field, min + ((scaled + 33 * step) / (66 * step)) * step);
        }
        self.drag = Some(Drag {
            field,
            x,
            value: self.value(field),
        });
    }
    pub fn drag_to(&mut self, x: i32) {
        if let Some(Drag {
            field,
            x: anchor,
            value,
        }) = self.drag
        {
            let (min, max, step) = self.range(field);
            let scaled = (i64::from(x) - i64::from(anchor)) * (max - min);
            let delta = (scaled + scaled.signum() * 33 * step) / (66 * step) * step;
            self.set(field, value.saturating_add(delta));
        }
    }

    pub fn fields() -> Vec<Field> {
        let mut fields = vec![Field::Credits, Field::Units];
        for row in 0..2 {
            fields.extend([
                Field::Name(row),
                Field::Country(row),
                Field::Color(row),
                Field::Start(row),
                Field::Team(row),
            ]);
        }
        fields.extend([Field::ShortGame, Field::Speed]);
        fields
    }
    pub fn set_starts(&mut self, starts: &[Option<(u16, u16)>; 8]) {
        self.starts = starts.map(|s| s.is_some());
        for p in &mut self.config.players {
            if p.start.is_some_and(|s| !self.starts[usize::from(s)]) {
                p.start = None;
            }
        }
    }
    pub fn available(&self, field: Field, index: usize) -> bool {
        match field {
            Field::Country(row) => row < 2 && index < Country::ALL.len(),
            Field::Color(row) => {
                row < 2
                    && index < COLOR_LABELS.len()
                    && usize::from(self.config.players[1 - row].color) != index
            }
            Field::Start(row) => {
                row < 2
                    && (index == 0
                        || index <= 8
                            && self.starts[index - 1]
                            && self.config.players[1 - row].start != Some((index - 1) as u8))
            }
            Field::Team(row) => {
                row < 2
                    && index <= 4
                    && (index == 0 || usize::from(self.config.players[1 - row].team) != index)
            }
            Field::ShortGame => index < 2,
            Field::Speed => index < GameSpeed::ALL.len(),
            _ => false,
        }
    }
    pub fn choices(&self, field: Field) -> Vec<String> {
        match field {
            Field::Country(_) => Country::ALL.iter().map(|c| c.name().to_owned()).collect(),
            Field::Color(_) => COLOR_LABELS.iter().map(|s| (*s).to_owned()).collect(),
            Field::Start(_) => std::iter::once("Auto".to_owned())
                .chain((1..=8).map(|s| s.to_string()))
                .collect(),
            Field::Team(_) => std::iter::once("---".to_owned())
                .chain((1..=4).map(|s| s.to_string()))
                .collect(),
            Field::Speed => GameSpeed::ALL
                .iter()
                .map(|s| s.label().to_owned())
                .collect(),
            _ => Vec::new(),
        }
    }
    pub fn popup_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let popup = self.popup?;
        let (x, y, _, h) = popup.field.rect();
        let w = match popup.field {
            Field::Country(_) => 150,
            Field::Color(_) => 126,
            _ => 90,
        };
        Some((x, y + h, w, self.choices(popup.field).len() as i32 * 22))
    }
    pub fn popup_hit(&mut self, point: Option<(i32, i32)>) {
        let Some(popup) = self.popup else { return };
        if let (Some((x, y)), Some((left, top, w, h))) = (point, self.popup_rect()) {
            if (left..left + w).contains(&x) && (top..top + h).contains(&y) {
                self.set(popup.field, i64::from((y - top) / 22));
            }
        }
        self.popup = None;
    }
    pub fn popup_move(&mut self, delta: i32) {
        let Some(mut popup) = self.popup else { return };
        let count = self.choices(popup.field).len() as i32;
        if count == 0 {
            return;
        }
        for _ in 0..count {
            popup.selected = (popup.selected as i32 + delta).rem_euclid(count) as usize;
            if self.available(popup.field, popup.selected) {
                break;
            }
        }
        self.popup = Some(popup);
    }
    pub fn popup_accept(&mut self) {
        if let Some(popup) = self.popup.take() {
            self.set(popup.field, popup.selected as i64);
        }
    }
    pub fn edit_name(&mut self, text: Option<&str>) {
        let Some(Field::Name(row)) = self.focus else {
            return;
        };
        let Some(player) = self.config.players.get_mut(row) else {
            return;
        };
        if self.name_selected {
            player.name.clear();
            self.name_selected = false;
        }
        if let Some(text) = text {
            for ch in text.chars().filter(|c| !c.is_control()).take(32) {
                if player.name.chars().count() >= 32 || player.name.len() + ch.len_utf8() > 96 {
                    break;
                }
                player.name.push(ch);
            }
        } else {
            player.name.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn participant_popup_skips_conflicts_and_keeps_authored_start_indices() {
        let mut setup = Setup::default();
        setup.set_starts(&[
            None,
            Some((12, 14)),
            None,
            None,
            Some((22, 24)),
            None,
            None,
            None,
        ]);
        setup.set(Field::Start(0), 5);
        assert_eq!(setup.config.players[0].start, Some(4));
        assert!(!setup.available(Field::Start(1), 5));
        assert!(!setup.available(Field::Start(1), 1));
        setup.set(Field::Start(1), 2);
        assert_eq!(setup.config.players[1].start, Some(1));
        setup.press(Field::Color(0), 423);
        setup.popup_move(1);
        assert_eq!(
            setup.popup.unwrap().selected,
            2,
            "opponent red is unavailable"
        );
        setup.popup_accept();
        assert_eq!(setup.config.players[0].color, 2);
        setup.press(Field::Country(1), 287);
        setup.popup_hit(Some((700, 260)));
        assert!(setup.popup.is_none());
        assert_eq!(setup.config.players[1].country, Country::America);
        setup.set_starts(&[Some((1, 1)); 8]);
        setup.set(Field::Team(0), 2);
        assert!(!setup.available(Field::Team(1), 2));
        setup.press(Field::Name(0), 59);
        setup.edit_name(Some("指揮官"));
        assert_eq!(setup.config.players[0].name, "指揮官");
        setup.edit_name(None);
        assert_eq!(setup.config.players[0].name, "指揮");
        setup.edit_name(Some(&"x".repeat(200)));
        assert_eq!(setup.config.players[0].name.chars().count(), 32);
        assert!(setup.config.validate().is_ok());
        setup.set_starts(&[None; 8]);
        assert!(setup.config.players.iter().all(|p| p.start.is_none()));
    }
    #[test]
    fn rules_limits_dragging_and_overrides_remain_bounded() {
        let mut setup = Setup::default();
        setup.press(Field::Credits, 410);
        setup.drag_to(i32::MIN);
        assert_eq!(setup.value(Field::Credits), 5000);
        setup.drag_to(443);
        assert_eq!(setup.value(Field::Credits), 7500);
        setup.drag_to(i32::MAX);
        assert_eq!(setup.options, StartOptions::default());
        setup.adjust(Field::Units, i32::MIN);
        assert_eq!(setup.options.units, Some(0));
        setup.adjust(Field::Units, i32::MAX);
        assert_eq!(setup.options.units, None);
        setup.adjust(Field::Credits, -1);
        assert_eq!(setup.options.credits, Some(9900));
        let hostile=Profile::from_ini(&IniFile::parse("[MultiplayerDialogSettings]\nMinMoney=-1\nMaxMoney=9223372036854775807\nMoney=-7\nMoneyIncrement=0\nMinUnitCount=9000\nMaxUnitCount=-1\nUnitCount=-1\n"));
        assert_eq!(
            (
                hostile.min_credits,
                hostile.max_credits,
                hostile.credits,
                hostile.increment
            ),
            (0, 1_000_000_000, 0, 1)
        );
        assert_eq!(
            (hostile.min_units, hostile.max_units, hostile.units),
            (4096, 4096, 4096)
        );
        setup.update_profile(Profile {
            min_credits: 6000,
            max_credits: 8000,
            credits: 7000,
            ..Profile::default()
        });
        assert_eq!(setup.options.credits, Some(8000));
        assert_eq!(Setup::hit((404, 314)), Some(Field::Credits));
        assert_eq!(Setup::hit((484, 314)), None);
    }
}
