use crate::session::StartOptions;
use yr_formats::IniFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Credits,
    Units,
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
        }
    }
    pub fn set(&mut self, field: Field, value: i64) {
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
        }
    }
    pub fn hit((x, y): (i32, i32)) -> Option<Field> {
        if !(404..484).contains(&x) {
            return None;
        }
        [Field::Credits, Field::Units]
            .into_iter()
            .find(|&f| (Self::y(f)..Self::y(f) + 22).contains(&y))
    }
    pub fn thumb_x(&self, field: Field) -> i32 {
        let (min, max, _) = self.range(field);
        404 + i32::from(self.touched[usize::from(field == Field::Units)])
            + ((self.value(field) - min) * 66)
                .checked_div(max - min)
                .unwrap_or(0) as i32
    }
    pub fn press(&mut self, field: Field, x: i32) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
