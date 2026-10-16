//! Westwood INI reader (rulesmd.ini, artmd.ini, maps, ...).
//!
//! Order-preserving: sections and keys keep their source order (the rules
//! merge semantics later depend on it), with case-insensitive lookup.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct IniSection {
    pub name: String,
    /// Entries in source order. Duplicate keys are kept; `get` returns the last
    /// (list indices reused in retail-era mods overwrite earlier entries, which
    /// is why Ares added `+=`; see docs/formats/rules.md).
    pub entries: Vec<(String, String)>,
}

impl IniSection {
    pub fn get(&self, key: &str) -> Option<&str> {
        let want = key.to_ascii_uppercase();
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| k.to_ascii_uppercase() == want)
            .map(|(_, v)| v.as_str())
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key)?.trim().parse().ok()
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key)?.trim().parse().ok()
    }

    /// Westwood booleans are decided by the first character: `Y`, `T` or `1`
    /// is true, `N`, `F` or `0` is false (so `yes`, `True`, `1` all work and
    /// `Yeah` is true). Anything else is unset.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key)?.trim().bytes().next()?.to_ascii_uppercase() {
            b'Y' | b'T' | b'1' => Some(true),
            b'N' | b'F' | b'0' => Some(false),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IniFile {
    pub sections: Vec<IniSection>,
    index: HashMap<String, usize>,
}

impl IniFile {
    pub fn parse(text: &str) -> IniFile {
        let mut file = IniFile::default();
        let mut current: Option<usize> = None;
        for line in text.lines() {
            let line = strip_comment(line).trim();
            if line.is_empty() {
                continue;
            }
            // The section name runs to the first `]`; anything after it is
            // ignored. Retail rulesmd.ini has `[MirageWH]\t// Supposed to be
            // a heat ray.`, which must still open [MirageWH].
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.split_once(']')).map(|(n, _)| n) {
                let name = name.trim().to_string();
                let key = name.to_ascii_uppercase();
                current = match file.index.get(&key) {
                    // Reopened section: keep appending to the first occurrence.
                    Some(&i) => Some(i),
                    None => {
                        file.sections.push(IniSection {
                            name,
                            entries: Vec::new(),
                        });
                        let i = file.sections.len() - 1;
                        file.index.insert(key, i);
                        Some(i)
                    }
                };
            } else if let Some(eq) = line.find('=') {
                if let Some(i) = current {
                    let key = line[..eq].trim().to_string();
                    let value = line[eq + 1..].trim().to_string();
                    file.sections[i].entries.push((key, value));
                }
            }
            // Lines outside any section that are not comments: ignored
            // (retail INIs are well formed).
        }
        file
    }

    pub fn section(&self, name: &str) -> Option<&IniSection> {
        self.index
            .get(&name.to_ascii_uppercase())
            .map(|&i| &self.sections[i])
    }

    /// Layer `other` on top of this file, as maps override rules: sections
    /// merge, and `other`'s keys are appended so they win on lookup.
    pub fn merge_from(&mut self, other: &IniFile) {
        for sec in &other.sections {
            let key = sec.name.to_ascii_uppercase();
            let i = match self.index.get(&key) {
                Some(&i) => i,
                None => {
                    self.sections.push(IniSection {
                        name: sec.name.clone(),
                        entries: Vec::new(),
                    });
                    self.index.insert(key, self.sections.len() - 1);
                    self.sections.len() - 1
                }
            };
            self.sections[i].entries.extend(sec.entries.iter().cloned());
        }
    }

    /// Convenience for `[Section] Key=Value` two-level lookups.
    pub fn value(&self, section: &str, key: &str) -> Option<&str> {
        self.section(section)?.get(key)
    }
}

/// Westwood INI comments start at `;`. Values do not contain `;` in retail data.
fn strip_comment(line: &str) -> &str {
    match line.find(';') {
        Some(i) => &line[..i],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[General]\r\n; a comment\r\nName = Chrono Miner \r\nCost=2000\r\n\r\n[SMIN]\r\nStrength=600\r\nCost=1750 ; trailing\r\n";

    #[test]
    fn parses_sections_and_keys() {
        let ini = IniFile::parse(SAMPLE);
        assert_eq!(ini.sections.len(), 2);
        assert_eq!(ini.value("general", "name"), Some("Chrono Miner"));
        assert_eq!(ini.value("General", "cost"), Some("2000"));
        assert_eq!(ini.value("smin", "Strength"), Some("600"));
        assert_eq!(ini.value("smin", "cost"), Some("1750"));
        assert!(ini.value("SMIN", "missing").is_none());
    }

    #[test]
    fn value_typed_helpers() {
        let ini = IniFile::parse(SAMPLE);
        let smin = ini.section("SMIN").unwrap();
        assert_eq!(smin.get_i64("Strength"), Some(600));
        assert_eq!(smin.get_f64("Strength"), Some(600.0));
    }

    #[test]
    fn text_after_section_header_is_ignored() {
        // Retail rulesmd.ini: a `//` note after the header must not merge the
        // section's keys into the previous one ([LUNARWH]).
        let ini = IniFile::parse("[LUNARWH]\nVerses=1\n\n[MirageWH]\t// Supposed to be a heat ray.\nVerses=2\n");
        assert_eq!(ini.value("LUNARWH", "Verses"), Some("1"));
        assert_eq!(ini.value("MirageWH", "Verses"), Some("2"));
    }

    #[test]
    fn booleans_use_first_character() {
        let ini = IniFile::parse("[A]\na=yes\nb=True\nc=1\nd=no\ne=False\nf=0\ng=maybe\n");
        let a = ini.section("A").unwrap();
        let got: Vec<_> = ["a", "b", "c", "d", "e", "f", "g"].iter().map(|k| a.get_bool(k)).collect();
        assert_eq!(got, [Some(true), Some(true), Some(true), Some(false), Some(false), Some(false), None]);
    }

    #[test]
    fn merge_layers_map_over_rules() {
        let mut rules = IniFile::parse("[HTNK]\nStrength=400\nCost=900\n");
        let map = IniFile::parse("[HTNK]\nStrength=600\n[NEWSEC]\nx=1\n");
        rules.merge_from(&map);
        assert_eq!(rules.value("HTNK", "Strength"), Some("600"));
        assert_eq!(rules.value("HTNK", "Cost"), Some("900"));
        assert_eq!(rules.value("NEWSEC", "x"), Some("1"));
    }

    #[test]
    fn duplicate_sections_merge_and_last_key_wins() {
        let ini = IniFile::parse("[A]\nx=1\n[A]\nx=2\ny=3\n");
        assert_eq!(ini.sections.len(), 1);
        assert_eq!(ini.value("A", "x"), Some("2"));
        assert_eq!(ini.value("A", "y"), Some("3"));
    }
}
