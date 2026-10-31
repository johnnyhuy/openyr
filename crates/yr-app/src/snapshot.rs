//! Match save envelope and app-side disk I/O. Presentation is rebuilt on load.

use crate::session::{Outcome, Session};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"OYRMATCH";
const VERSION: u32 = 1;
const HEADER: usize = 37;
const LIMIT: usize = yr_sim::save::MAX_SAVE_BYTES + HEADER;

pub(crate) fn fingerprint(bytes: impl Iterator<Item = u8>) -> u64 {
    bytes.fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub fn encode(session: &Session) -> Result<Vec<u8>, String> {
    let mut out = MAGIC.to_vec();
    out.extend(VERSION.to_le_bytes());
    out.extend(session.seed.to_le_bytes());
    out.extend(session.setup_hash.to_le_bytes());
    out.extend(session.sim.state_hash().to_le_bytes());
    out.push(match session.outcome {
        Outcome::Ongoing => 0,
        Outcome::Won => 1,
        Outcome::Lost => 2,
    });
    out.extend(session.sim.save_bytes().map_err(|e| e.to_string())?);
    Ok(out)
}

struct Header {
    seed: u64,
    setup_hash: u64,
    state_hash: u64,
    outcome: Outcome,
}

fn header(bytes: &[u8]) -> Result<Header, String> {
    if bytes.len() > LIMIT || bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err("invalid or truncated OpenYR match save (limit 64 MiB)".into());
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if version != VERSION {
        return Err(format!(
            "unsupported match save version {version}; expected {VERSION}"
        ));
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
    })
}

/// Restore into a freshly rebuilt session for the same map, rules and seed.
/// A failed load leaves the existing session untouched.
pub fn restore(session: &mut Session, bytes: &[u8]) -> Result<(), String> {
    let h = header(bytes)?;
    if h.seed != session.seed || h.setup_hash != session.setup_hash {
        return Err("save does not match this map, rules or engine setup; use matching game data and a compatible OpenYR version".into());
    }
    let sim = yr_sim::Sim::from_save_bytes(&bytes[HEADER..]).map_err(|e| e.to_string())?;
    if sim.state_hash() != h.state_hash {
        return Err("save state hash mismatch; file is damaged".into());
    }
    session.sim = sim;
    session.radar.reset(&session.sim, &session.kinds);
    session.outcome = h.outcome;
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
    let mut session = crate::session::load(gamedir, map_name, header(&bytes)?.seed)?;
    restore(&mut session, &bytes)?;
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
