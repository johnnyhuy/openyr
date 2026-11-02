//! Match save envelope and app-side disk I/O. Presentation is rebuilt on load.

use crate::session::{Outcome, Session, StartOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"OYRMATCH";
const HEADER: usize = 37;
const MAP_LIMIT: usize = 255;
const OPTIONS_BYTES: usize = 11;
const LIMIT: usize = yr_sim::save::MAX_SAVE_BYTES + HEADER + 2 + MAP_LIMIT + OPTIONS_BYTES;

pub(crate) fn valid_map_reference(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAP_LIMIT
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'_' | b'-' | b'.'))
}

pub(crate) fn fingerprint(bytes: impl Iterator<Item = u8>) -> u64 {
    bytes.fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub fn encode(session: &Session) -> Result<Vec<u8>, String> {
    let mut out = MAGIC.to_vec();
    let map = session.source_map.as_deref();
    if map.is_some_and(|name| !valid_map_reference(name)) {
        return Err("invalid saved map reference".into());
    }
    session.start_options.validate()?;
    let options = session.start_options;
    let configured = options.shared_forces;
    out.extend(
        (if configured {
            3u32
        } else if map.is_some() {
            2u32
        } else {
            1u32
        })
        .to_le_bytes(),
    );
    out.extend(session.seed.to_le_bytes());
    out.extend(session.setup_hash.to_le_bytes());
    out.extend(session.sim.state_hash().to_le_bytes());
    out.push(match session.outcome {
        Outcome::Ongoing => 0,
        Outcome::Won => 1,
        Outcome::Lost => 2,
    });
    if configured || map.is_some() {
        let name = map.unwrap_or("");
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(name.bytes());
    }
    if configured {
        out.push(u8::from(options.credits.is_some()) | (u8::from(options.units.is_some()) << 1));
        out.extend(options.credits.unwrap_or(0).to_le_bytes());
        out.extend(options.units.unwrap_or(0).to_le_bytes());
    }
    out.extend(session.sim.save_bytes().map_err(|e| e.to_string())?);
    Ok(out)
}

struct Header<'a> {
    seed: u64,
    setup_hash: u64,
    state_hash: u64,
    outcome: Outcome,
    map: Option<&'a str>,
    options: StartOptions,
    payload_at: usize,
}

fn header(bytes: &[u8]) -> Result<Header<'_>, String> {
    if bytes.len() > LIMIT || bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err("invalid or truncated OpenYR match save (limit 64 MiB)".into());
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if !matches!(version, 1..=3) {
        return Err(format!(
            "unsupported match save version {version}; expected 1, 2 or 3"
        ));
    }
    let (map, mut payload_at) = if version >= 2 {
        let length = bytes
            .get(HEADER..HEADER + 2)
            .ok_or("truncated saved map length")?;
        let length = usize::from(u16::from_le_bytes(length.try_into().unwrap()));
        if (length == 0 && version == 2) || length > MAP_LIMIT {
            return Err("invalid saved map length".into());
        }
        let end = HEADER + 2 + length;
        let name = std::str::from_utf8(
            bytes
                .get(HEADER + 2..end)
                .ok_or("truncated saved map reference")?,
        )
        .map_err(|_| "invalid saved map encoding")?;
        if !name.is_empty() && !valid_map_reference(name) {
            return Err("invalid saved map reference".into());
        }
        ((!name.is_empty()).then_some(name), end)
    } else {
        (None, HEADER)
    };
    let mut options = StartOptions {
        shared_forces: version >= 3,
        ..StartOptions::default()
    };
    if version == 3 {
        let data = bytes
            .get(payload_at..payload_at + OPTIONS_BYTES)
            .ok_or("truncated saved start options")?;
        let flags = data[0];
        let credits = i64::from_le_bytes(data[1..9].try_into().unwrap());
        let units = u16::from_le_bytes(data[9..11].try_into().unwrap());
        if flags > 3 || (flags & 1 == 0 && credits != 0) || (flags & 2 == 0 && units != 0) {
            return Err("invalid saved start option flags or values".into());
        }
        options.credits = (flags & 1 != 0).then_some(credits);
        options.units = (flags & 2 != 0).then_some(units);
        options.validate()?;
        payload_at += OPTIONS_BYTES;
    }
    if bytes.len() - payload_at > yr_sim::save::MAX_SAVE_BYTES {
        return Err("OpenYR simulation payload exceeds 64 MiB".into());
    }
    let value = |at| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    Ok(Header {
        seed: value(12),
        setup_hash: value(20),
        state_hash: value(28),
        outcome: match bytes[36] {
            0 => Outcome::Ongoing,
            1 => Outcome::Won,
            2 => Outcome::Lost,
            _ => return Err("invalid match outcome in save".into()),
        },
        map,
        options,
        payload_at,
    })
}

/// Restore into a freshly rebuilt session for the same map, rules and seed.
/// A failed load leaves the existing session untouched.
pub fn restore(session: &mut Session, bytes: &[u8]) -> Result<(), String> {
    let h = header(bytes)?;
    if h.seed != session.seed || h.setup_hash != session.setup_hash {
        return Err("save does not match this map, rules or engine setup; use matching game data and a compatible OpenYR version".into());
    }
    let sim = yr_sim::Sim::from_save_bytes(&bytes[h.payload_at..]).map_err(|e| e.to_string())?;
    if sim.state_hash() != h.state_hash {
        return Err("save state hash mismatch; file is damaged".into());
    }
    session.sim = sim;
    session.radar.reset(&session.sim, &session.kinds);
    session.outcome = h.outcome;
    session.start_options = h.options;
    if session.source_map.is_none() {
        session.source_map = h.map.map(str::to_owned);
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    let f =
        std::fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    // Do not trust metadata length, or allocate from a count in the file.
    let mut bytes = Vec::new();
    f.take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    header(&bytes)?;
    Ok(bytes)
}

pub fn load(gamedir: &str, map_name: &str, path: &Path) -> Result<Session, String> {
    let bytes = read(path)?;
    load_bytes(gamedir, map_name, path, &bytes)
}

/// Menu loads follow a saved install-relative map; legacy saves use the
/// currently selected map. Explicit CLI loads retain the supplied map check.
pub fn load_from_menu(
    gamedir: &str,
    fallback_map: &str,
    path: &Path,
) -> Result<(Session, String), String> {
    let bytes = read(path)?;
    let h = header(&bytes)?;
    let map_name = h.map.unwrap_or(fallback_map).to_owned();
    if h.map.is_some() {
        let fs = yr_assets::AssetFs::open(Path::new(gamedir)).map_err(|e| e.to_string())?;
        if fs.read(&map_name).is_none() {
            return Err(format!(
                "saved map {map_name} is missing from the selected game data"
            ));
        }
    }
    let session = load_bytes(gamedir, &map_name, path, &bytes)?;
    Ok((session, map_name))
}

fn load_bytes(gamedir: &str, map_name: &str, path: &Path, bytes: &[u8]) -> Result<Session, String> {
    let h = header(bytes)?;
    let mut session = crate::session::load_with_options(gamedir, map_name, h.seed, h.options)?;
    restore(&mut session, bytes)?;
    println!(
        "OpenYR: loaded {} at tick {}, hash {:016x}, {:?}",
        path.display(),
        session.sim.tick(),
        session.sim.state_hash(),
        session.outcome
    );
    Ok(session)
}

/// Replace a save only after all bytes reach a sibling temporary file.
pub fn save(session: &Session, path: &Path) -> Result<(), String> {
    let bytes = encode(session)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .ok_or("save path has no filename")?
        .to_string_lossy();
    let temp = parent.join(format!(".{name}.{}.pending", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("cannot create {}: {e}", temp.display()))?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(temp);
        return Err(format!("cannot save {}: {e}", path.display()));
    }
    println!(
        "OpenYR: saved {} at tick {}, hash {:016x}, {:?}",
        path.display(),
        session.sim.tick(),
        session.sim.state_hash(),
        session.outcome
    );
    Ok(())
}

pub fn quicksave_path() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("Library/Application Support/OpenYR/quicksave.oyrsave")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named_header(name: &[u8], declared: u16) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(2u32.to_le_bytes());
        bytes.resize(HEADER, 0);
        bytes.extend(declared.to_le_bytes());
        bytes.extend(name);
        bytes
    }

    #[test]
    fn saved_map_header_bounds_lengths_and_rejects_paths() {
        for (name, declared) in [
            (&b""[..], 0),
            (&b""[..], u16::MAX),
            (&b""[..], 256),
            (&b"A"[..], 2),
            (&b"\xff"[..], 1),
            (&b"../A.MAP"[..], 8),
            (&b"C:\\A.MAP"[..], 8),
            (&b"A/B.MAP"[..], 7),
            (&b".hidden"[..], 7),
            (&b"A\0.MAP"[..], 6),
        ] {
            assert!(header(&named_header(name, declared)).is_err(), "{name:?}");
        }
        for length in HEADER..HEADER + 2 {
            assert!(header(&named_header(b"A.MAP", 5)[..length]).is_err());
        }
        for name in ["EB1.MMX", "XMP29U2.MAP", "My map-2.yrm"] {
            let bytes = named_header(name.as_bytes(), name.len() as u16);
            let h = header(&bytes).unwrap();
            assert_eq!(h.map, Some(name));
            assert_eq!(h.payload_at, bytes.len());
        }
    }
}

#[cfg(test)]
mod options_tests {
    use super::*;
    fn configured(flags: u8, credits: i64, units: u16) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(3u32.to_le_bytes());
        bytes.resize(HEADER, 0);
        bytes.extend(0u16.to_le_bytes());
        bytes.push(flags);
        bytes.extend(credits.to_le_bytes());
        bytes.extend(units.to_le_bytes());
        bytes
    }
    #[test]
    fn configured_header_rejects_hostile_and_truncated_options() {
        for bytes in [
            configured(4, 0, 0),
            configured(1, -1, 0),
            configured(1, 1_000_000_001, 0),
            configured(1, 5000, 1),
            configured(2, 1, 5),
            configured(2, 0, 4097),
        ] {
            assert!(header(&bytes).is_err());
        }
        let bytes = configured(3, 5000, 0);
        for end in HEADER..bytes.len() {
            assert!(header(&bytes[..end]).is_err(), "{end}");
        }
        let h = header(&bytes).unwrap();
        assert_eq!(
            h.options,
            StartOptions {
                credits: Some(5000),
                units: Some(0),
                ..StartOptions::default()
            }
        );
        assert_eq!(h.map, None);
        assert_eq!(h.payload_at, HEADER + 2 + OPTIONS_BYTES);
    }
}
