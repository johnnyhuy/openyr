//! App-side atomic replacement of already encoded save bytes.
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
const TEMP_ATTEMPTS: usize = 32;

fn temporary(
    parent: &Path,
    destination: &OsStr,
    next: &AtomicU64,
) -> std::io::Result<(PathBuf, std::fs::File)> {
    for _ in 0..TEMP_ATTEMPTS {
        let serial = next.fetch_add(1, Ordering::Relaxed);
        // Keep the sibling name short even when the destination reaches NAME_MAX.
        let path = parent.join(format!(
            ".openyr-save.{}.{serial}.pending",
            std::process::id()
        ));
        if path.file_name() == Some(destination) {
            continue;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no unused temporary save file after 32 attempts",
    ))
}

pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("save path has no filename")?;
    let (temp, mut file) = temporary(parent, name, &NEXT_TEMP)
        .map_err(|e| format!("cannot create temporary save in {}: {e}", parent.display()))?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(temp);
        return Err(format!("cannot save {}: {e}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
