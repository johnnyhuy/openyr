//! Choices for the supported two-participant skirmish and their save metadata.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Country {
    #[default]
    America,
    Korea,
    France,
    Germany,
    Britain,
    Russia,
    Cuba,
    Libya,
    Iraq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Allied,
    Soviet,
}

impl Country {
    pub const ALL: [Self; 9] = [
        Self::America,
        Self::Korea,
        Self::France,
        Self::Germany,
        Self::Britain,
        Self::Russia,
        Self::Cuba,
        Self::Libya,
        Self::Iraq,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::America => "Americans",
            Self::Korea => "Alliance",
            Self::France => "French",
            Self::Germany => "Germans",
            Self::Britain => "British",
            Self::Russia => "Russians",
            Self::Cuba => "Confederation",
            Self::Libya => "Africans",
            Self::Iraq => "Arabs",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::America => "America",
            Self::Korea => "Korea",
            Self::France => "France",
            Self::Germany => "Germany",
            Self::Britain => "Britain",
            Self::Russia => "Russia",
            Self::Cuba => "Cuba",
            Self::Libya => "Libya",
            Self::Iraq => "Iraq",
        }
    }
    pub fn side(self) -> Side {
        if Self::ALL[..5].contains(&self) {
            Side::Allied
        } else {
            Side::Soviet
        }
    }
    pub fn loading_art(self) -> (&'static str, &'static str) {
        match self {
            Self::America => ("LS800USTATES.SHP", "MPLSU.PAL"),
            Self::Korea => ("LS800KOREA.SHP", "MPLSK.PAL"),
            Self::France => ("LS800FRANCE.SHP", "MPLSF.PAL"),
            Self::Germany => ("LS800GERMANY.SHP", "MPLSG.PAL"),
            Self::Britain => ("LS800UKINGDOM.SHP", "MPLSUK.PAL"),
            Self::Russia => ("LS800RUSSIA.SHP", "MPLSR.PAL"),
            Self::Cuba => ("LS800CUBA.SHP", "MPLSC.PAL"),
            Self::Libya => ("LS800LIBYA.SHP", "MPLSL.PAL"),
            Self::Iraq => ("LS800IRAQ.SHP", "MPLSI.PAL"),
        }
    }
    pub fn brief(self) -> &'static str {
        match self {
            Self::America => "USA",
            Self::Korea => "Korea",
            Self::France => "French",
            Self::Germany => "Germans",
            Self::Britain => "British",
            Self::Russia => "Russia",
            Self::Cuba => "Cuba",
            Self::Libya => "Lybia",
            Self::Iraq => "Iraq",
        }
    }
}

impl Side {
    pub fn buildings(self) -> [&'static str; 5] {
        match self {
            Self::Allied => ["GACNST", "GAPOWR", "GAREFN", "GAWEAP", "GAPILE"],
            Self::Soviet => ["NACNST", "NAPOWR", "NAREFN", "NAWEAP", "NAHAND"],
        }
    }
    pub fn force(self) -> [&'static str; 4] {
        match self {
            Self::Allied => ["AMCV", "CMIN", "MTNK", "E1"],
            Self::Soviet => ["SMCV", "HARV", "HTNK", "E2"],
        }
    }
    pub fn vehicles(self) -> &'static [&'static str] {
        match self {
            Self::Allied => &["MTNK", "FV", "SREF", "CMIN"],
            Self::Soviet => &["HTNK", "HTK", "APOC", "HARV"],
        }
    }
    pub fn infantry(self) -> &'static [&'static str] {
        match self {
            Self::Allied => &["E1", "GGI", "SNIPE"],
            Self::Soviet => &["E2", "SHK", "FLAKT"],
        }
    }
    pub fn sidebar(self) -> &'static str {
        match self {
            Self::Allied => "SIDEC01.MIX",
            Self::Soviet => "SIDEC02.MIX",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
}
impl GameSpeed {
    pub const ALL: [Self; 3] = [Self::Slow, Self::Normal, Self::Fast];
    pub fn label(self) -> &'static str {
        match self {
            Self::Slow => "Slow",
            Self::Normal => "Normal",
            Self::Fast => "Fast",
        }
    }
    pub fn tick(self) -> std::time::Duration {
        std::time::Duration::from_millis(match self {
            Self::Slow => 100,
            Self::Normal => 1000 / 15,
            Self::Fast => 1000 / 30,
        })
    }
}

pub const COLOR_NAMES: [&str; 8] = [
    "Gold",
    "DarkRed",
    "DarkBlue",
    "DarkGreen",
    "Orange",
    "Purple",
    "DarkSky",
    "Magenta",
];
pub const COLOR_LABELS: [&str; 8] = [
    "Yellow", "Red", "Blue", "Green", "Orange", "Purple", "Cyan", "Pink",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participant {
    pub name: String,
    pub country: Country,
    pub color: u8,
    /// Zero means an independent participant; 1..=4 are explicit teams.
    pub team: u8,
    /// Authored waypoint index, independent of compacted start ordering.
    pub start: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchConfig {
    pub players: [Participant; 2],
    pub short_game: bool,
    pub speed: GameSpeed,
}
impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            players: [
                Participant {
                    name: "Commander".into(),
                    country: Country::America,
                    color: 0,
                    team: 0,
                    start: None,
                },
                Participant {
                    name: "Computer".into(),
                    country: Country::America,
                    color: 1,
                    team: 0,
                    start: None,
                },
            ],
            short_game: false,
            speed: GameSpeed::Normal,
        }
    }
}
impl MatchConfig {
    pub fn validate(&self) -> Result<(), String> {
        for p in &self.players {
            if p.name.trim().is_empty()
                || p.name.len() > 96
                || p.name.chars().count() > 32
                || p.name.chars().any(char::is_control)
            {
                return Err("Enter a player name of 1 to 32 characters".into());
            }
            if usize::from(p.color) >= COLOR_NAMES.len()
                || p.team > 4
                || p.start.is_some_and(|s| s >= 8)
            {
                return Err("Invalid player color, team or starting position".into());
            }
        }
        let [a, b] = &self.players;
        if a.color == b.color {
            return Err("Choose different player colors".into());
        }
        if a.team != 0 && a.team == b.team {
            return Err("Choose opposing teams for this two-player match".into());
        }
        if a.start.is_some() && a.start == b.start {
            return Err("Choose different starting positions".into());
        }
        Ok(())
    }
    pub fn color(&self, rules: &yr_formats::IniFile, owner: u8) -> [u8; 3] {
        let Some(player) = self.players.get(usize::from(owner)) else {
            return [170; 3];
        };
        yr_render::named_color(rules, COLOR_NAMES[usize::from(player.color)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn participants_reject_invalid_names_and_conflicting_choices() {
        let mut config = MatchConfig::default();
        assert!(config.validate().is_ok());
        for name in ["", "  ", "bad\nname", &"a".repeat(33), &"界".repeat(33)] {
            config.players[0].name = name.into();
            assert!(config.validate().is_err(), "{name:?}");
        }
        config.players[0].name = "指揮官".into();
        assert!(config.validate().is_ok());
        config.players[0].color = 8;
        assert!(config.validate().is_err());
        config.players[0].color = 1;
        assert!(config.validate().is_err());
        config.players[0].color = 0;
        config.players[0].team = 1;
        config.players[1].team = 1;
        assert!(config.validate().is_err());
        config.players[1].team = 2;
        config.players[0].start = Some(7);
        config.players[1].start = Some(7);
        assert!(config.validate().is_err());
        config.players[1].start = Some(0);
        assert!(config.validate().is_ok());
        config.players[0].start = Some(8);
        assert!(config.validate().is_err());
    }
}
