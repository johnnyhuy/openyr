//! Finder launch defaults and remembered retail-data location.

use std::io::Read;
use std::path::{Path, PathBuf};

pub const DEFAULT_MAP: &str = "EB1.MMX";
pub const USAGE: &str = "openyr [<gamedir> [<map> [scale]]] [--load <file>] [--save <file>] [--ticks <end-tick>] [--frames <dir> <tick>...] [--passive]";

#[cfg(target_os = "macos")]
const CHOOSE_INSTALL: &str = r#"try
    return POSIX path of (choose folder with prompt "Choose your Red Alert 2: Yuri's Revenge folder for OpenYR. It must contain ra2.mix and ra2md.mix.")
on error number -128
    return ""
end try"#;

#[cfg(target_os = "macos")]
const SHOW_ERROR: &str = "on run argv\ndisplay alert \"OpenYR could not start\" message (item 1 of argv) as critical\nend run";

fn preferences(home: &Path) -> PathBuf {
    home.join("Library/Application Support/OpenYR/game-directory.json")
}

fn is_install(path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    let mut found = [false; 2];
    for entry in entries.flatten() {
        for (i, name) in ["ra2.mix", "ra2md.mix"].iter().enumerate() {
            if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
                && entry
                    .file_type()
                    .is_ok_and(|t| t.is_file() || t.is_symlink())
            {
                found[i] = true;
            }
        }
    }
    found.into_iter().all(|f| f)
}

fn remembered(home: &Path) -> Option<PathBuf> {
    let mut bytes = Vec::new();
    // Preferences are user-editable. Bound reads before parsing JSON.
    std::fs::File::open(preferences(home))
        .ok()?
        .take(16 * 1024 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 16 * 1024 {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

fn discover(home: Option<&Path>, override_path: Option<&Path>) -> Result<Option<PathBuf>, String> {
    if let Some(path) = override_path {
        return if is_install(path) {
            Ok(Some(path.to_path_buf()))
        } else {
            Err(format!(
                "OPENYR_GAMEDIR does not contain ra2.mix and ra2md.mix: {}",
                path.display()
            ))
        };
    }
    Ok(home.and_then(|home| {
        remembered(home)
            .filter(|p| p.is_absolute() && is_install(p))
            .or_else(|| {
                let path = home.join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
                is_install(&path).then_some(path)
            })
    }))
}

fn remember(home: &Path, path: &Path) -> Result<(), String> {
    let file = preferences(home);
    let bytes = serde_json::to_vec(path).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
    let temporary = file.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)
        .and_then(|()| std::fs::rename(&temporary, file))
        .map_err(|e| format!("cannot remember game directory: {e}"))
}

#[cfg(target_os = "macos")]
fn choose_install() -> Result<Option<PathBuf>, String> {
    // Paths are returned as data, never interpolated into a script or shell.
    let output = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", CHOOSE_INSTALL])
        .output()
        .map_err(|e| format!("cannot open game folder chooser: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot open game folder chooser: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let path = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let path = path.strip_suffix('\n').unwrap_or(&path);
    Ok((!path.is_empty()).then(|| PathBuf::from(path)))
}

#[cfg(not(target_os = "macos"))]
fn choose_install() -> Result<Option<PathBuf>, String> {
    Err("game data not found; supply a game directory or set OPENYR_GAMEDIR".into())
}

fn default_directory(interactive: bool) -> Result<Option<String>, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let override_path = std::env::var_os("OPENYR_GAMEDIR").map(PathBuf::from);
    let path = match discover(home.as_deref(), override_path.as_deref())? {
        Some(path) => path,
        None if interactive => {
            let Some(path) = choose_install()? else {
                return Ok(None);
            };
            if !is_install(&path) {
                return Err("the selected folder needs both ra2.mix and ra2md.mix; open OpenYR again to choose another folder".into());
            }
            if let Some(home) = home {
                if let Err(e) = remember(&home, &path) {
                    eprintln!("OpenYR: {e}");
                }
            }
            path
        }
        None => {
            return Err("game data not found; supply a game directory or set OPENYR_GAMEDIR".into())
        }
    };
    path.into_os_string()
        .into_string()
        .map(Some)
        .map_err(|_| "game directory path must be UTF-8".into())
}

/// Explicit directories bypass discovery. Flags alone use defaults without
/// opening a chooser, so headless jobs never wait for desktop input.
pub fn arguments(args: &[String]) -> Result<Option<Vec<String>>, String> {
    complete_arguments(args, || default_directory(args.is_empty()))
}

fn complete_arguments(
    args: &[String],
    default_directory: impl FnOnce() -> Result<Option<String>, String>,
) -> Result<Option<Vec<String>>, String> {
    let mut out = args.to_vec();
    if out.first().is_none_or(|s| s.starts_with('-')) {
        let Some(dir) = default_directory()? else {
            return Ok(None);
        };
        out.insert(0, dir);
    }
    if out.get(1).is_none_or(|s| s.starts_with('-')) {
        out.insert(1, DEFAULT_MAP.to_owned());
    }
    Ok(Some(out))
}

pub fn show_error(message: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", SHOW_ERROR, "--", message])
        .status();
    #[cfg(not(target_os = "macos"))]
    let _ = message;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn native_dialog_scripts_compile_without_opening_ui() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let path = std::env::temp_dir().join(format!("openyr-dialog-{}.scpt", std::process::id()));
        for script in [CHOOSE_INSTALL, SHOW_ERROR] {
            let mut child = Command::new("/usr/bin/osacompile")
                .arg("-o")
                .arg(&path)
                .arg("-")
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            std::fs::remove_file(&path).unwrap();
        }
    }

    #[test]
    fn arguments_preserve_explicit_paths_and_headless_options() {
        let args = |s: &str| s.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
        let defaults = || Ok(Some("/retail path".to_owned()));
        assert_eq!(
            complete_arguments(&[], defaults).unwrap().unwrap(),
            ["/retail path", "EB1.MMX"]
        );
        assert_eq!(
            complete_arguments(&args("--frames proof 300 --passive"), defaults)
                .unwrap()
                .unwrap(),
            [
                "/retail path",
                "EB1.MMX",
                "--frames",
                "proof",
                "300",
                "--passive"
            ]
        );
        for input in [
            "/explicit",
            "/explicit --ticks 300",
            "/explicit MYMAP.MMX 2 --ticks 300",
        ] {
            let result = complete_arguments(&args(input), || {
                panic!("explicit directory must bypass discovery")
            })
            .unwrap()
            .unwrap();
            assert_eq!(result[0], "/explicit");
            assert_eq!(
                result[1],
                if input.contains("MYMAP") {
                    "MYMAP.MMX"
                } else {
                    DEFAULT_MAP
                }
            );
        }
        assert!(complete_arguments(&[], || Ok(None)).unwrap().is_none());
    }

    #[test]
    fn discovery_checks_overrides_remembers_paths_and_recovers_from_stale_preferences() {
        let root = std::env::temp_dir().join(format!("openyr-launcher-{}", std::process::id()));
        let home = root.join("home");
        let standard = home.join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        let custom = root.join("custom \"game\" with spaces");
        for path in [&standard, &custom] {
            std::fs::create_dir_all(path).unwrap();
            for name in ["RA2.MIX", "ra2md.mix"] {
                std::fs::write(path.join(name), []).unwrap();
            }
        }
        assert_eq!(discover(Some(&home), None).unwrap(), Some(standard.clone()));
        remember(&home, &custom).unwrap();
        assert_eq!(discover(Some(&home), None).unwrap(), Some(custom.clone()));
        assert_eq!(
            discover(Some(&home), Some(&standard)).unwrap(),
            Some(standard.clone())
        );
        assert!(discover(Some(&home), Some(&root)).is_err());
        std::fs::remove_file(custom.join("ra2md.mix")).unwrap();
        assert_eq!(discover(Some(&home), None).unwrap(), Some(standard.clone()));
        for bytes in [b"bad json".to_vec(), vec![b' '; 16 * 1024 + 1]] {
            std::fs::write(preferences(&home), bytes).unwrap();
            assert_eq!(discover(Some(&home), None).unwrap(), Some(standard.clone()));
        }
        assert_eq!(discover(None, None).unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
