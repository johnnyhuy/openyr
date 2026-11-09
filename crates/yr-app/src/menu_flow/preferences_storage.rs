//! Exclusive sibling replacement for the app's interface preferences.
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
        let path = parent.join(format!(
            ".openyr-interface.{}.{serial}.pending",
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
        "no unused temporary preference file after 32 attempts",
    ))
}

pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_with_counter(path, bytes, &NEXT_TEMP)
}

fn write_with_counter(path: &Path, bytes: &[u8], next: &AtomicU64) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let destination = path.file_name().ok_or("preference path has no filename")?;
    let (temporary, mut file) = temporary(parent, destination, next).map_err(|e| {
        format!(
            "cannot create temporary preferences in {}: {e}",
            parent.display()
        )
    })?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written.and_then(|()| std::fs::rename(&temporary, path)) {
        let _ = std::fs::remove_file(temporary);
        return Err(format!(
            "cannot save preferences {}: {error}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "openyr-interface-storage-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&dir).unwrap();
            Self(dir)
        }
        fn candidate(&self, serial: usize) -> PathBuf {
            self.0.join(format!(
                ".openyr-interface.{}.{serial}.pending",
                std::process::id()
            ))
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn preferences_storage_skips_occupied_files_directories_and_links() {
        let dir = Scratch::new();
        let path = dir.0.join("interface.json");
        std::fs::write(dir.candidate(0), b"occupied file").unwrap();
        std::fs::create_dir(dir.candidate(1)).unwrap();
        let unrelated = dir.0.join("unrelated");
        std::fs::write(&unrelated, b"unrelated bytes").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&unrelated, dir.candidate(2)).unwrap();
        #[cfg(not(unix))]
        std::fs::write(dir.candidate(2), b"another occupied file").unwrap();
        write_with_counter(&path, b"new preferences", &AtomicU64::new(0)).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"new preferences");
        assert_eq!(std::fs::read(dir.candidate(0)).unwrap(), b"occupied file");
        assert!(dir.candidate(1).is_dir());
        assert_eq!(std::fs::read(&unrelated).unwrap(), b"unrelated bytes");
        #[cfg(unix)]
        assert!(std::fs::symlink_metadata(dir.candidate(2))
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(!dir.candidate(3).exists());
        println!("Exclusive preference writer skips occupied regular/directory/link candidates and preserves their bytes.");
    }

    #[test]
    fn preferences_storage_bounds_collision_retries_and_preserves_destination() {
        let dir = Scratch::new();
        let path = dir.0.join("interface.json");
        std::fs::write(&path, b"old preferences").unwrap();
        for serial in 0..TEMP_ATTEMPTS {
            std::fs::write(dir.candidate(serial), b"occupied").unwrap();
        }
        let error = write_with_counter(&path, b"new preferences", &AtomicU64::new(0)).unwrap_err();
        assert!(error.contains("32 attempts"));
        assert_eq!(std::fs::read(&path).unwrap(), b"old preferences");
        for serial in 0..TEMP_ATTEMPTS {
            assert_eq!(std::fs::read(dir.candidate(serial)).unwrap(), b"occupied");
        }
        assert!(!dir.candidate(TEMP_ATTEMPTS).exists());
        println!("All 32 occupied candidates stay intact; exhaustion preserves the existing preference destination.");
    }

    #[test]
    fn preferences_storage_removes_only_owned_candidate_after_failed_rename() {
        let dir = Scratch::new();
        let path = dir.0.join("interface.json");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("keep"), b"destination content").unwrap();
        std::fs::write(dir.candidate(0), b"preexisting candidate").unwrap();
        assert!(write_with_counter(&path, b"new preferences", &AtomicU64::new(0)).is_err());
        assert_eq!(
            std::fs::read(path.join("keep")).unwrap(),
            b"destination content"
        );
        assert_eq!(
            std::fs::read(dir.candidate(0)).unwrap(),
            b"preexisting candidate"
        );
        assert!(!dir.candidate(1).exists());
        println!("Failed preference publication cleans its owned candidate and preserves the destination and occupied sibling.");
    }
}
