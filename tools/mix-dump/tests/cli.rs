#![cfg(unix)]

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let index = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("mix-dump-{}-{nonce}-{index}", std::process::id()));
        fs::create_dir(&dir).unwrap();

        // Enough rows to exceed a pipe buffer when the consumer stops early.
        let count = 2048u16;
        let mut mix = count.to_le_bytes().to_vec();
        mix.extend(u32::from(count).to_le_bytes());
        for id in 1..=u32::from(count) {
            mix.extend(id.to_le_bytes());
            mix.extend((id - 1).to_le_bytes());
            mix.extend(1u32.to_le_bytes());
        }
        mix.extend(vec![b'x'; usize::from(count)]);
        fs::write(dir.join("test.mix"), mix).unwrap();
        fs::write(
            dir.join("rulesmd.ini"),
            "[VehicleTypes]\n0=TEST\n[TEST]\nName=Test Tank\nStrength=100\n",
        )
        .unwrap();
        fs::write(dir.join("artmd.ini"), "[TEST]\nVoxel=yes\n").unwrap();
        Self(dir)
    }

    fn command(&self, verb: &str) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_mix-dump"));
        cmd.arg(verb);
        cmd.arg(if matches!(verb, "rules" | "vfs") {
            self.0.clone()
        } else {
            self.0.join("test.mix")
        });
        cmd
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn closed_stdout() -> Stdio {
    let (reader, writer) = UnixStream::pair().unwrap();
    drop(reader);
    Stdio::from(OwnedFd::from(writer))
}

fn assert_success(output: &Output) {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn inspection_output_is_complete() {
    let fixture = Fixture::new();
    for verb in ["info", "list", "names", "scan", "vfs", "rules"] {
        let output = fixture.command(verb).output().unwrap();
        assert_success(&output);
        let text = String::from_utf8(output.stdout).unwrap();
        match verb {
            "info" => assert!(text.contains("  files       : 2048\n")),
            "list" | "names" => assert_eq!(text.lines().count(), 2049),
            "scan" => assert!(text.contains("2048 unique IDs")),
            "vfs" => assert!(text.starts_with("lookup order")),
            "rules" => assert!(text.starts_with("infantry 0  vehicles 1")),
            _ => unreachable!(),
        }
    }
    let output = fixture.command("rules").arg("TEST").output().unwrap();
    assert_success(&output);
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("Test Tank"));
}

#[test]
fn inspection_commands_accept_an_already_closed_stdout() {
    let fixture = Fixture::new();
    for verb in ["info", "list", "names", "scan", "vfs", "rules"] {
        let output = fixture
            .command(verb)
            .stdout(closed_stdout())
            .output()
            .unwrap();
        assert_success(&output);
    }
    let output = fixture
        .command("rules")
        .arg("TEST")
        .stdout(closed_stdout())
        .output()
        .unwrap();
    assert_success(&output);
}

#[test]
fn a_consumer_can_stop_after_the_header() {
    let fixture = Fixture::new();
    for verb in ["list", "names"] {
        let mut child = fixture
            .command(verb)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        assert!(header.starts_with("id"));
        drop(reader);
        assert_success(&child.wait_with_output().unwrap());
    }
}

#[test]
fn input_errors_still_fail_with_closed_stdout() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("test.mix"), b"invalid").unwrap();
    let output = fixture
        .command("names")
        .stdout(closed_stdout())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .starts_with("error:"));
}

#[test]
fn extraction_finishes_before_accepting_closed_stdout() {
    let fixture = Fixture::new();
    let outdir = fixture.0.join("extracted");
    let output = fixture
        .command("extract")
        .arg("1")
        .arg(&outdir)
        .stdout(closed_stdout())
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(fs::read(outdir.join("1")).unwrap(), b"x");
}

#[test]
fn extraction_errors_still_fail_with_closed_stdout() {
    let fixture = Fixture::new();
    let output = fixture
        .command("extract")
        .arg("1")
        .arg(fixture.0.join("test.mix"))
        .stdout(closed_stdout())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .starts_with("error:"));
}

#[test]
fn other_stdout_errors_still_fail() {
    let fixture = Fixture::new();
    let (_reader, mut writer) = UnixStream::pair().unwrap();
    writer.set_nonblocking(true).unwrap();
    // A full nonblocking socket fails with WouldBlock rather than BrokenPipe.
    loop {
        match writer.write(&[0; 8192]) {
            Ok(n) => assert!(n > 0),
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::WouldBlock);
                break;
            }
        }
    }
    let output = fixture
        .command("names")
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .starts_with("error:"));
}

#[test]
fn invalid_usage_keeps_its_exit_code() {
    let output = Command::new(env!("CARGO_BIN_EXE_mix-dump"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .starts_with("usage:"));
}
