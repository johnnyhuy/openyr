use super::*;
use std::ffi::OsString;
use std::path::PathBuf;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "openyr-save-storage-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn roundtrip(name: OsString) {
    let dir = Scratch::new();
    let path = dir.0.join(&name);
    let before = b"existing save bytes";
    let after = b"replacement save bytes\0\xff";
    // Establish that the actual filesystem accepts the destination basename.
    std::fs::write(&path, before).unwrap();
    let result = write(&path, after);
    assert!(result.is_ok(), "valid target {name:?}: {result:?}");
    assert_eq!(std::fs::read(&path).unwrap(), after);
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
    std::fs::remove_file(&path).unwrap();
    write(&path, before).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[test]
fn legal_255_byte_ascii_target_can_be_saved_and_replaced() {
    roundtrip(format!("{}.oyrsave", "W".repeat(247)).into());
    println!(
        "Valid 255-byte ASCII destination: new save and atomic replacement preserve exact bytes."
    );
}

#[test]
fn legal_254_byte_unicode_target_can_be_saved_and_replaced() {
    roundtrip(format!("{}.oyrsave", "界".repeat(82)).into());
    println!(
        "Valid 254-byte Unicode destination: new save and atomic replacement preserve exact bytes."
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_target_does_not_expand_the_temporary_name() {
    use std::os::unix::ffi::OsStringExt;
    let mut name = vec![0xff; 128];
    name.extend_from_slice(b".oyrsave");
    roundtrip(OsString::from_vec(name));
    println!(
        "Valid non-UTF-8 Unix basename: no lossy temporary-name expansion or changed destination."
    );
}

#[test]
fn occupied_temporary_candidates_are_never_truncated_and_retries_are_bounded() {
    let dir = Scratch::new();
    let next = AtomicU64::new(0);
    let mut occupied = Vec::new();
    for _ in 0..TEMP_ATTEMPTS {
        let (path, mut file) = temporary(&dir.0, OsStr::new("target.oyrsave"), &next).unwrap();
        file.write_all(b"keep occupied candidate").unwrap();
        occupied.push(path);
    }
    next.store(0, Ordering::Relaxed);
    assert_eq!(
        temporary(&dir.0, OsStr::new("target.oyrsave"), &next)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(next.load(Ordering::Relaxed), TEMP_ATTEMPTS as u64);
    for path in &occupied {
        assert_eq!(std::fs::read(path).unwrap(), b"keep occupied candidate");
    }
    // Starting at a collision skips occupied names and creates a new sibling.
    next.store(TEMP_ATTEMPTS as u64 - 1, Ordering::Relaxed);
    let (created, _) = temporary(&dir.0, OsStr::new("target.oyrsave"), &next).unwrap();
    assert!(!occupied.contains(&created));
    assert!(created.file_name().unwrap().len() < 64);
    assert_eq!(
        std::fs::read_dir(&dir.0).unwrap().count(),
        TEMP_ATTEMPTS + 1
    );
    println!("Occupied candidates remain intact, collisions retry, and exhaustion returns after 32 attempts.");
}

#[cfg(unix)]
#[test]
fn occupied_symlink_candidates_are_not_followed() {
    let dir = Scratch::new();
    let protected = dir.0.join("protected.oyrsave");
    std::fs::write(&protected, b"protected save").unwrap();
    let next = AtomicU64::new(0);
    let (occupied, file) = temporary(&dir.0, OsStr::new("target.oyrsave"), &next).unwrap();
    drop(file);
    std::fs::remove_file(&occupied).unwrap();
    std::os::unix::fs::symlink(&protected, &occupied).unwrap();
    next.store(0, Ordering::Relaxed);
    let (created, _) = temporary(&dir.0, OsStr::new("target.oyrsave"), &next).unwrap();
    assert_ne!(created, occupied);
    assert!(std::fs::symlink_metadata(occupied).unwrap().is_symlink());
    assert_eq!(std::fs::read(protected).unwrap(), b"protected save");
    println!("Temporary symlink collision is skipped without modifying its target.");
}

#[test]
fn rename_failure_removes_only_the_owned_temporary_file() {
    let dir = Scratch::new();
    let target = dir.0.join("directory.oyrsave");
    std::fs::create_dir(&target).unwrap();
    let marker = target.join("keep");
    std::fs::write(&marker, b"keep destination").unwrap();
    let unrelated = dir.0.join(".unrelated.pending");
    std::fs::write(&unrelated, b"keep unrelated pending").unwrap();
    assert!(write(&target, b"new bytes")
        .unwrap_err()
        .starts_with("cannot save"));
    assert_eq!(std::fs::read(marker).unwrap(), b"keep destination");
    assert_eq!(std::fs::read(unrelated).unwrap(), b"keep unrelated pending");
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 2);
    println!("Failed replacement preserves destination/unrelated files and removes only its own temporary sibling.");
}

#[test]
fn temporary_candidate_cannot_publish_the_destination_before_replacement() {
    let dir = Scratch::new();
    let next = AtomicU64::new(0);
    let (destination, file) = temporary(&dir.0, OsStr::new("target.oyrsave"), &next).unwrap();
    drop(file);
    std::fs::remove_file(&destination).unwrap();
    next.store(0, Ordering::Relaxed);
    let (created, _) = temporary(&dir.0, destination.file_name().unwrap(), &next).unwrap();
    assert_ne!(created, destination);
    assert!(!destination.exists());
    println!(
        "A destination matching the internal naming pattern remains absent until replacement."
    );
}
