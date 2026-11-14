//! Value types parsed from INI text without floating point, so the
//! simulation can consume them deterministically.

use std::fmt;

/// Exact decimal: `units / 10^6`. INI values such as `Range=1.4`,
/// `Verses=75%` or `PercentAtMax=0.5` parse into this without float rounding;
/// conversion to engine units (leptons, fixed-point fractions) is explicit.
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
)]
#[serde(transparent)]
pub struct Decimal(i64);

const SCALE: i64 = 1_000_000;

impl Decimal {
    pub const ZERO: Decimal = Decimal(0);
    pub const ONE: Decimal = Decimal(SCALE);

    pub const fn from_int(v: i64) -> Decimal {
        Decimal(v * SCALE)
    }

    /// Raw `units` (value * 10^6).
    pub const fn units(self) -> i64 {
        self.0
    }

    /// Parse `[-]digits[.digits]`. Up to 6 fractional digits are kept and
    /// more are truncated. A trailing `%` divides by 100 (`75%` = 0.75).
    pub fn parse(text: &str) -> Option<Decimal> {
        let t = text.trim();
        let (t, percent) = match t.strip_suffix('%') {
            Some(rest) => (rest.trim_end(), true),
            None => (t, false),
        };
        let (neg, t) = match t.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, t.strip_prefix('+').unwrap_or(t)),
        };
        let (int, frac) = t.split_once('.').unwrap_or((t, ""));
        if (int.is_empty() && frac.is_empty())
            || !int.bytes().all(|b| b.is_ascii_digit())
            || !frac.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let whole: i64 = if int.is_empty() { 0 } else { int.parse().ok()? };
        let magnitude = whole.checked_mul(SCALE)?;
        let mut units = if neg { -magnitude } else { magnitude };
        let mut place = SCALE / 10;
        for b in frac.bytes().take(6) {
            let digit = i64::from(b - b'0') * place;
            units = if neg {
                units.checked_sub(digit)?
            } else {
                units.checked_add(digit)?
            };
            place /= 10;
        }
        if percent {
            units /= 100;
        }
        Some(Decimal(units))
    }

    /// Cells to leptons (256 per cell), truncating toward zero. The engine's
    /// exact conversion is not yet verified against the oracle.
    pub fn to_leptons(self) -> i64 {
        (i128::from(self.0) * 256 / i128::from(SCALE)) as i64
    }

    /// Integer part, truncating toward zero.
    pub fn trunc(self) -> i64 {
        self.0 / SCALE
    }

    /// `self * v`, truncating toward zero (damage x verses, etc.).
    pub fn mul_int(self, v: i64) -> i64 {
        self.0 * v / SCALE
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        let (int, frac) = (abs / SCALE as u64, abs % SCALE as u64);
        if frac == 0 {
            write!(f, "{sign}{int}")
        } else {
            let digits = format!("{frac:06}");
            write!(f, "{sign}{int}.{}", digits.trim_end_matches('0'))
        }
    }
}

/// The 11 RA2/YR armor types, in `Verses=` order (ModEnc "Verses").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Armor {
    None,
    Flak,
    Plate,
    Light,
    Medium,
    Heavy,
    Wood,
    Steel,
    Concrete,
    Special1,
    Special2,
}

impl Armor {
    pub const ALL: [Armor; 11] = [
        Armor::None,
        Armor::Flak,
        Armor::Plate,
        Armor::Light,
        Armor::Medium,
        Armor::Heavy,
        Armor::Wood,
        Armor::Steel,
        Armor::Concrete,
        Armor::Special1,
        Armor::Special2,
    ];

    /// Case-insensitive (retail uses `Armor=Plate` alongside `plate`).
    pub fn parse(text: &str) -> Option<Armor> {
        let t = text.trim().to_ascii_lowercase();
        Armor::ALL.into_iter().find(|a| a.name() == t)
    }

    pub fn name(self) -> &'static str {
        match self {
            Armor::None => "none",
            Armor::Flak => "flak",
            Armor::Plate => "plate",
            Armor::Light => "light",
            Armor::Medium => "medium",
            Armor::Heavy => "heavy",
            Armor::Wood => "wood",
            Armor::Steel => "steel",
            Armor::Concrete => "concrete",
            Armor::Special1 => "special_1",
            Armor::Special2 => "special_2",
        }
    }

    /// Position in `Verses=`.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Comma-separated list, trimmed, empty items dropped.
pub fn parse_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// A type reference; `none` (any case) and empty mean "no reference".
pub fn parse_ref(text: &str) -> Option<String> {
    let t = text.trim();
    (!t.is_empty() && !t.eq_ignore_ascii_case("none")).then(|| t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_parses_exactly() {
        assert_eq!(Decimal::parse("1.4").unwrap().units(), 1_400_000);
        assert_eq!(Decimal::parse("-2").unwrap(), Decimal::from_int(-2));
        assert_eq!(Decimal::parse("75%").unwrap().units(), 750_000);
        assert_eq!(Decimal::parse(".5").unwrap().units(), 500_000);
        assert_eq!(Decimal::parse("0.1234567").unwrap().units(), 123_456);
        assert!(Decimal::parse("abc").is_none());
        assert!(Decimal::parse("").is_none());
        assert!(Decimal::parse("1.2.3").is_none());
    }

    #[test]
    fn decimal_conversions() {
        let r = Decimal::parse("1.4").unwrap();
        assert_eq!(r.to_leptons(), 358); // 358.4 truncated
        assert_eq!(Decimal::parse("5.75").unwrap().to_leptons(), 1472);
        assert_eq!(Decimal::parse("30%").unwrap().mul_int(100), 30);
        assert_eq!(r.to_string(), "1.4");
        assert_eq!(Decimal::parse("-0.25").unwrap().to_string(), "-0.25");
        assert_eq!(Decimal::from_int(3).to_string(), "3");
    }

    #[test]
    fn decimal_fraction_boundaries_are_checked_before_percentage_scaling() {
        for (text, units) in [
            ("9223372036854.775807", i64::MAX),
            ("+9223372036854.7758079", i64::MAX),
            ("-9223372036854.775808", i64::MIN),
            ("-9223372036854.7758089", i64::MIN),
            ("9223372036854.775807%", i64::MAX / 100),
            ("-9223372036854.775808%", i64::MIN / 100),
            ("75.125%", 751_250),
            ("-75.125%", -751_250),
        ] {
            let value = Decimal::parse(text).unwrap();
            assert_eq!(value.units(), units, "{text}");
            assert_eq!(Decimal::parse(&value.to_string()), Some(value));
            assert_eq!(
                value.to_leptons(),
                (i128::from(units) * 256 / 1_000_000) as i64
            );
        }
        for text in [
            "9223372036854.775808",
            "-9223372036854.775809",
            "9223372036854.999999",
            "-9223372036854.999999",
            "9223372036855",
            "-9223372036855",
            "9223372036854.999999%",
            "-9223372036854.999999%",
            "9223372036854775808",
        ] {
            assert_eq!(Decimal::parse(text), None, "{text}");
        }
    }

    #[test]
    fn armor_order_and_case() {
        assert_eq!(Armor::parse("Plate"), Some(Armor::Plate));
        assert_eq!(Armor::parse("special_2").unwrap().index(), 10);
        assert_eq!(Armor::parse("concrete").unwrap().index(), 8);
        assert_eq!(Armor::parse("titanium"), None);
    }

    #[test]
    fn refs_and_lists() {
        assert_eq!(parse_ref("none"), None);
        assert_eq!(parse_ref(" 105mm "), Some("105mm".into()));
        assert_eq!(parse_list("GAPOWR, ,NAPOWR"), vec!["GAPOWR", "NAPOWR"]);
    }
}
