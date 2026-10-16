//! CSF (compiled string file) reader — `RA2MD.CSF` and friends.
//!
//! Spec with evidence: `docs/formats/csf.md`.

#[derive(Debug)]
pub enum CsfError {
    BadMagic,
    Truncated,
    BadString,
}

impl std::fmt::Display for CsfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsfError::BadMagic => write!(f, "not a CSF file"),
            CsfError::Truncated => write!(f, "truncated CSF file"),
            CsfError::BadString => write!(f, "invalid CSF string entry"),
        }
    }
}

impl std::error::Error for CsfError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsfEntry {
    pub label: String,
    /// `None` for "NRTS" (null) strings.
    pub value: Option<String>,
    /// "WRTS" records carry an extra ASCII reference after the value — the
    /// EVA/voice sound file stem in retail data (e.g. `ceva001e`).
    pub sound: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CsfFile {
    pub version: u32,
    pub entries: Vec<CsfEntry>,
}

impl CsfFile {
    pub fn parse(data: &[u8]) -> Result<CsfFile, CsfError> {
        if data.len() < 16 || &data[0..4] != b" FSC" {
            return Err(CsfError::BadMagic);
        }
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let num_labels = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let num_strings = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

        let mut r = Reader { data, pos: 16 };
        // Retail files carry 8 reserved zero bytes after the counts.
        let reserved = r.take(8)?;
        if reserved != [0; 8] {
            return Err(CsfError::BadString);
        }
        let mut entries = Vec::with_capacity(num_strings);
        for _ in 0..num_labels {
            r.expect(b" LBL")?;
            let num_strings = r.u32()? as usize;
            let name = r.label_name()?;
            for _ in 0..num_strings {
                let (value, sound) = r.string_value()?;
                entries.push(CsfEntry {
                    label: name.clone(),
                    value,
                    sound,
                });
            }
        }
        if entries.len() != num_strings {
            // Retail files have num_strings == num_labels; a mismatch means the
            // parse walked the wrong path.
            return Err(CsfError::BadString);
        }
        Ok(CsfFile { version, entries })
    }

    /// Case-insensitive label lookup.
    pub fn get(&self, label: &str) -> Option<&str> {
        let want = label.to_ascii_uppercase();
        self.entries
            .iter()
            .find(|e| e.label.to_ascii_uppercase() == want)
            .and_then(|e| e.value.as_deref())
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], CsfError> {
        if self.pos + n > self.data.len() {
            return Err(CsfError::Truncated);
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32, CsfError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn expect(&mut self, magic: &[u8; 4]) -> Result<(), CsfError> {
        if self.take(4)? != magic {
            return Err(CsfError::BadString);
        }
        Ok(())
    }

    fn label_name(&mut self) -> Result<String, CsfError> {
        let len = self.u32()? as usize;
        let raw = self.take(len)?;
        Ok(String::from_utf8_lossy(raw).into_owned())
    }

    /// Returns `(value, sound)`; "WRTS" records carry the trailing sound stem.
    fn string_value(&mut self) -> Result<(Option<String>, Option<String>), CsfError> {
        let magic = self.take(4)?;
        match magic {
            // " RTS" = plain value, "WRTS" = value + ASCII sound reference,
            // "NRTS" = empty string. Values are inverted UTF-16 code units.
            b" RTS" | b"WRTS" => {
                let len = self.u32()? as usize;
                let raw = self.take(len * 2)?;
                let units: Vec<u16> = raw
                    .chunks_exact(2)
                    .map(|c| !u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let value = String::from_utf16_lossy(&units);
                let sound = if magic == b"WRTS" {
                    let slen = self.u32()? as usize;
                    let sraw = self.take(slen)?;
                    Some(String::from_utf8_lossy(sraw).into_owned())
                } else {
                    None
                };
                Ok((Some(value), sound))
            }
            b"NRTS" => Ok((None, None)),
            _ => Err(CsfError::BadString),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b" FSC");
        v.extend_from_slice(&3u32.to_le_bytes());
        v.extend_from_slice(&2u32.to_le_bytes()); // labels
        v.extend_from_slice(&2u32.to_le_bytes()); // strings
        v.extend_from_slice(&[0; 8]); // reserved
        // label 1
        v.extend_from_slice(b" LBL");
        v.extend_from_slice(&1u32.to_le_bytes());
        v.extend_from_slice(&4u32.to_le_bytes());
        v.extend_from_slice(b"TEST");
        v.extend_from_slice(b" RTS");
        let value: Vec<u16> = "Hi".encode_utf16().map(|u| !u).collect();
        v.extend_from_slice(&(value.len() as u32).to_le_bytes());
        for u in value {
            v.extend_from_slice(&u.to_le_bytes());
        }
        // label 2 (null string)
        v.extend_from_slice(b" LBL");
        v.extend_from_slice(&1u32.to_le_bytes());
        v.extend_from_slice(&5u32.to_le_bytes());
        v.extend_from_slice(b"EMPTY");
        v.extend_from_slice(b"NRTS");
        v
    }

    #[test]
    fn round_trip() {
        let csf = CsfFile::parse(&build()).unwrap();
        assert_eq!(csf.version, 3);
        assert_eq!(csf.entries.len(), 2);
        assert_eq!(csf.get("test"), Some("Hi"));
        assert_eq!(csf.get("EMPTY"), None);
        assert!(csf.get("missing").is_none());
    }

    #[test]
    fn rejects_bad_magic() {
        let mut data = build();
        data[0] = b'X';
        assert!(matches!(CsfFile::parse(&data), Err(CsfError::BadMagic)));
    }
}
