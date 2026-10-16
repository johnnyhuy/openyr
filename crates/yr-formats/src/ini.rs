//! Westwood INI reader (rulesmd.ini, artmd.ini, maps, ...).
//!
//! Order-preserving: sections and keys keep their source order (the rules
//! merge semantics later depend on it), with case-insensitive lookup.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct IniSection {
    pub name: String,
    /// Entries in source order. Duplicate keys are kept; `get` returns the last.
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

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key)?.trim().to_ascii_uppercase().as_str() {
            "YES" | "TRUE" | "1" => Some(true),
            "NO" | "FALSE" | "0" => Some(false),
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
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
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
    fn duplicate_sections_merge_and_last_key_wins() {
        let ini = IniFile::parse("[A]\nx=1\n[A]\nx=2\ny=3\n");
        assert_eq!(ini.sections.len(), 1);
        assert_eq!(ini.value("A", "x"), Some("2"));
        assert_eq!(ini.value("A", "y"), Some("3"));
    }
}
