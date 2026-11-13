//! CSF (compiled string file) reader, via `RA2MD.CSF` and friends.
//!
//! Spec with evidence: `docs/formats/csf.md`.

use std::sync::Arc;

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
    /// Values belonging to one label share its immutable decoded name.
    pub label: Arc<str>,
    /// `None` for "NRTS" (null) strings.
    pub value: Option<String>,
    /// "WRTS" records carry an extra ASCII reference after the value, the
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
        // A label needs 12 bytes and even a null value needs four. Validate
        // both declared counts together before reserving or decoding names.
        let minimum = num_labels
            .checked_mul(12)
            .and_then(|n| num_strings.checked_mul(4).and_then(|s| n.checked_add(s)))
            .ok_or(CsfError::BadString)?;
        if minimum > data.len() - r.pos {
            return Err(CsfError::BadString);
        }
        let mut entries = Vec::with_capacity(num_strings);
        for _ in 0..num_labels {
            r.expect(b" LBL")?;
            let count = r.u32()? as usize;
            if count > num_strings - entries.len() {
                return Err(CsfError::BadString);
            }
            let name: Arc<str> = r.label_name()?.into();
            if count > (data.len() - r.pos) / 4 {
                return Err(CsfError::Truncated);
            }
            for _ in 0..count {
                let (value, sound) = r.string_value()?;
                entries.push(CsfEntry {
                    label: name.clone(),
                    value,
                    sound,
                });
            }
        }
        if entries.len() != num_strings {
            // Label-local value counts must agree with the table total.
            return Err(CsfError::BadString);
        }
        Ok(CsfFile { version, entries })
    }

    /// Case-insensitive label lookup.
    pub fn get(&self, label: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.label.eq_ignore_ascii_case(label))
            .and_then(|e| e.value.as_deref())
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], CsfError> {
        let end = self.pos.checked_add(n).ok_or(CsfError::Truncated)?;
        let s = self.data.get(self.pos..end).ok_or(CsfError::Truncated)?;
        self.pos = end;
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
                let raw = self.take(len.checked_mul(2).ok_or(CsfError::Truncated)?)?;
                let units: Vec<u16> = raw
                    .as_chunks::<2>()
                    .0
                    .iter()
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
    fn multi_value_labels_share_storage_instead_of_multiplying_text() {
        let name = "L".repeat(4096);
        let count = 128u32;
        let mut data = b" FSC".to_vec();
        for n in [3u32, 1, count, 0, 0] {
            data.extend(n.to_le_bytes());
        }
        data.extend(b" LBL");
        data.extend(count.to_le_bytes());
        data.extend((name.len() as u32).to_le_bytes());
        data.extend(name.as_bytes());
        for _ in 0..count {
            data.extend(b"NRTS");
        }
        let table = CsfFile::parse(&data).unwrap();
        assert_eq!(table.entries.len(), count as usize);
        for entry in &table.entries {
            assert_eq!(&*entry.label, name.as_str());
            assert!(entry.value.is_none());
            assert!(entry.sound.is_none());
            assert_eq!(entry.label.as_ptr(), table.entries[0].label.as_ptr());
        }
        let cloned = table.clone();
        assert_eq!(
            cloned.entries[0].label.as_ptr(),
            table.entries[0].label.as_ptr()
        );
        println!(
            "{} input bytes retain one {}-byte label for {} entries and table clones",
            data.len(),
            name.len(),
            count
        );
    }

    fn multi_value_table() -> Vec<u8> {
        let mut data = b" FSC".to_vec();
        for n in [3u32, 2, 3, 0, 0] {
            data.extend(n.to_le_bytes());
        }
        for (label, values) in [("Name:Unit", 2u32), ("Empty", 1)] {
            data.extend(b" LBL");
            data.extend(values.to_le_bytes());
            data.extend((label.len() as u32).to_le_bytes());
            data.extend(label.as_bytes());
            if values == 2 {
                for (magic, value) in [(b"WRTS", "Elite \u{1f680}"), (b" RTS", "Second")] {
                    data.extend(magic);
                    let units: Vec<_> = value.encode_utf16().collect();
                    data.extend((units.len() as u32).to_le_bytes());
                    for unit in units {
                        data.extend((!unit).to_le_bytes());
                    }
                    if magic == b"WRTS" {
                        data.extend(5u32.to_le_bytes());
                        data.extend(b"voice");
                    }
                }
            } else {
                data.extend(b"NRTS");
            }
        }
        data
    }

    #[test]
    fn csf_multi_value_records_retain_unicode_sound_order_and_first_lookup() {
        let data = multi_value_table();
        let table = CsfFile::parse(&data).unwrap();
        assert_eq!(table.entries.len(), 3);
        assert_eq!(table.get("NAME:UNIT"), Some("Elite \u{1f680}"));
        assert_eq!(table.entries[0].sound.as_deref(), Some("voice"));
        assert_eq!(table.entries[1].value.as_deref(), Some("Second"));
        assert!(table.entries[1].sound.is_none());
        assert!(Arc::ptr_eq(
            &table.entries[0].label,
            &table.entries[1].label
        ));
        assert_eq!(&*table.entries[2].label, "Empty");
        assert_eq!(table.get("empty"), None);
        for len in 0..data.len() {
            assert!(CsfFile::parse(&data[..len]).is_err(), "prefix {len}");
        }
    }

    #[test]
    fn csf_hostile_counts_and_lengths_fail_before_expansion() {
        let good = multi_value_table();
        for (offset, value) in [
            (8, u32::MAX),  // table label count
            (12, u32::MAX), // table value count
            (12, 2),        // too few declared values
            (12, 4),        // too many declared values
            (28, u32::MAX), // label-local value count
            (32, u32::MAX), // label byte length
            (49, u32::MAX), // first UTF-16 length
            (69, u32::MAX), // sound-reference byte length
        ] {
            let mut data = good.clone();
            data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(CsfFile::parse(&data).is_err(), "field {offset}={value}");
        }
        let mut empty = b" FSC".to_vec();
        for n in [3u32, 0, 0, 0, 0] {
            empty.extend(n.to_le_bytes());
        }
        assert!(CsfFile::parse(&empty).unwrap().entries.is_empty());
        // Zero-value labels remain supported, even when no entries are emitted.
        empty[8..12].copy_from_slice(&1u32.to_le_bytes());
        empty.extend(b" LBL");
        empty.extend([0; 8]);
        assert!(CsfFile::parse(&empty).unwrap().entries.is_empty());
        let mut r = Reader {
            data: &[],
            pos: usize::MAX,
        };
        assert!(matches!(r.take(1), Err(CsfError::Truncated)));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut data = build();
        data[0] = b'X';
        assert!(matches!(CsfFile::parse(&data), Err(CsfError::BadMagic)));
    }

    #[test]
    fn hostile_string_count_does_not_overallocate() {
        let mut data = b" FSC".to_vec();
        for v in [3u32, 0, u32::MAX] {
            data.extend(v.to_le_bytes());
        }
        data.extend([0; 8]);
        assert!(matches!(CsfFile::parse(&data), Err(CsfError::BadString)));
    }
}
