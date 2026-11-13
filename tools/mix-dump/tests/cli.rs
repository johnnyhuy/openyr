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
    // A closed pipe makes BrokenPipe independent of socket teardown timing.
    let (reader, writer) = io::pipe().unwrap();
    drop(reader);
    Stdio::from(writer)
}

fn assert_success(output: &Output) {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

fn archive(payloads: &[&[u8]]) -> Vec<u8> {
    let mut mix = u16::try_from(payloads.len())
        .unwrap()
        .to_le_bytes()
        .to_vec();
    let body_len: usize = payloads.iter().map(|p| p.len()).sum();
    mix.extend(u32::try_from(body_len).unwrap().to_le_bytes());
    let mut offset = 0u32;
    for (i, payload) in payloads.iter().enumerate() {
        mix.extend(u32::try_from(i + 1).unwrap().to_le_bytes());
        mix.extend(offset.to_le_bytes());
        let len = u32::try_from(payload.len()).unwrap();
        mix.extend(len.to_le_bytes());
        offset += len;
    }
    for payload in payloads {
        mix.extend_from_slice(payload);
    }
    mix
}

#[test]
fn scan_fails_for_an_invalid_archive_after_printing_the_census() {
    let fixture = Fixture::new();
    let invalid = fixture.0.join("invalid.mix");
    fs::write(&invalid, b"invalid").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mix-dump"))
        .arg("scan")
        .arg(invalid)
        .arg(fixture.0.join("test.mix"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("2048 unique IDs"), "{text}");
    assert!(
        text.contains("errors (1):\n  invalid.mix: not a MIX"),
        "{text}"
    );
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: archive scan found 1 parser error(s)\n"
    );
}

#[test]
fn scan_reports_nested_asset_errors_and_continues_to_valid_assets() {
    let fixture = Fixture::new();
    let nested = archive(&[b"RIFF", b" FSC"]);
    fs::write(
        fixture.0.join("test.mix"),
        archive(&[&nested, b"RIFF", b"[valid]\nvalue=1\n"]),
    )
    .unwrap();
    let output = fixture.command("scan").output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("test.mix/0x00000001/0x00000001: wav:"),
        "{text}"
    );
    assert!(
        text.contains("test.mix/0x00000001/0x00000002: csf:"),
        "{text}"
    );
    assert!(text.contains("test.mix/0x00000002: wav:"), "{text}");
    assert!(text.contains("test.mix/0x00000003 (16 B)"), "{text}");
    assert!(text.contains("errors (3):"), "{text}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: archive scan found 3 parser error(s)\n"
    );
}

#[test]
fn scan_fails_for_a_malformed_csf() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("test.mix"), archive(&[b" FSC"])).unwrap();
    let output = fixture.command("scan").output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("test.mix/0x00000001: csf: not a CSF file"),
        "{text}"
    );
    assert!(text.contains("errors (1):"), "{text}");
}

#[test]
fn wav_rejects_malformed_headers_without_replacing_output_and_scan_keeps_valid_neighbors() {
    use yr_formats::audio::{encode_wav, Pcm};
    let fixture = Fixture::new();
    let valid = encode_wav(&Pcm {
        sample_rate: 22050,
        channels: 2,
        samples: vec![100, -100, 200, -200],
    });
    let input = format!("{}:0x00000001", fixture.0.join("input.mix").display());
    let destination = fixture.0.join("protected.wav");
    for (field, bytes) in [
        (4, 0u32.to_le_bytes().to_vec()),
        (4, u32::MAX.to_le_bytes().to_vec()),
        (22, 0u16.to_le_bytes().to_vec()),
        (24, 0u32.to_le_bytes().to_vec()),
        (28, 1u32.to_le_bytes().to_vec()),
        (32, 1u16.to_le_bytes().to_vec()),
        (40, 6u32.to_le_bytes().to_vec()),
    ] {
        let mut malformed = valid.clone();
        malformed[field..field + bytes.len()].copy_from_slice(&bytes);
        fs::write(fixture.0.join("input.mix"), archive(&[&malformed])).unwrap();
        fs::write(&destination, b"preserve earlier export").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_mix-dump"))
            .arg("wav")
            .arg(&input)
            .arg(&destination)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.starts_with("error: ") && !stderr.contains("panicked"),
            "{stderr}"
        );
        assert_eq!(fs::read(&destination).unwrap(), b"preserve earlier export");

        fs::write(fixture.0.join("test.mix"), archive(&[&malformed, &valid])).unwrap();
        let output = fixture.command("scan").output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("errors (1):"), "{text}");
        assert!(text.contains("test.mix/0x00000001: wav:"), "{text}");
        assert!(!text.contains("test.mix/0x00000002: wav:"), "{text}");
    }
    fs::write(fixture.0.join("input.mix"), archive(&[&valid])).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mix-dump"))
        .arg("wav")
        .arg(&input)
        .arg(&destination)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(fs::read(destination).unwrap(), valid);
}

#[test]
fn scan_rejects_malformed_shp_rows_and_retains_valid_neighbors() {
    let fixture = Fixture::new();
    let shp = |row: &[u8]| {
        let mut data = [0u16, 5, 1, 1]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        for value in [0u16, 0, 5, 1] {
            data.extend(value.to_le_bytes());
        }
        data.extend(3u32.to_le_bytes());
        data.extend([0; 8]);
        data.extend(32u32.to_le_bytes());
        data.extend(((row.len() + 2) as u16).to_le_bytes());
        data.extend(row);
        data
    };
    let rows: [&[u8]; 6] = [
        &[7, 0],
        &[7, 0, 6],
        &[7, 0, 2],
        &[0, 5, 6],
        &[7, 0, 4],
        &[7, 0, 5],
    ];
    let payloads: Vec<_> = rows.into_iter().map(shp).collect();
    fs::write(
        fixture.0.join("test.mix"),
        archive(&payloads.iter().map(Vec::as_slice).collect::<Vec<_>>()),
    )
    .unwrap();
    let output = fixture.command("scan").output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    for id in 1..=4 {
        assert!(
            text.contains(&format!("test.mix/0x{id:08x}: shp frame 0:")),
            "{text}"
        );
    }
    assert!(text.contains("test.mix/0x00000005 (37 B)"), "{text}");
    assert!(text.contains("test.mix/0x00000006 (37 B)"), "{text}");
    assert!(text.contains("errors (4):"), "{text}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: archive scan found 4 parser error(s)\n"
    );
    println!("SHP scan rejects missing counts, overflowing runs, short rows and extra literals; complete rows and retail one-pixel transparent padding stay valid.");
}

#[test]
fn scan_rejects_vxl_height_overruns_and_retains_valid_neighbors() {
    fn model(skip: u8) -> Vec<u8> {
        let mut bytes = b"Voxel Animation\0".to_vec();
        for value in [1u32, 1, 1, 11] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([16, 31]);
        bytes.extend([0; 768]);
        bytes.extend([0; 28]);
        bytes.extend(0i32.to_le_bytes());
        bytes.extend(2i32.to_le_bytes());
        bytes.extend([skip, 0, 0]);
        for value in [0u32, 4, 8] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0; 76]);
        bytes.extend([1, 1, 4, 4]);
        bytes
    }

    let fixture = Fixture::new();
    let valid = model(4);
    let invalid = model(5);
    fs::write(fixture.0.join("test.mix"), archive(&[&invalid, &valid])).unwrap();
    let output = fixture.command("scan").output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("test.mix/0x00000001: vxl: VXL limb 0 has invalid span data"),
        "{text}"
    );
    assert!(text.contains("test.mix/0x00000002 (933 B)"), "{text}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: archive scan found 1 parser error(s)\n"
    );
    println!("VXL scan rejects an empty run past z=4, retains the valid exact-height neighbor and exits 1 with one limb diagnostic.");
}

#[test]
fn scan_counts_errors_beyond_the_diagnostic_limit() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("test.mix"), archive(&[&b"RIFF"[..]; 43])).unwrap();
    let output = fixture.command("scan").output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("errors (43):"), "{text}");
    assert_eq!(text.matches(": wav:").count(), 40, "{text}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: archive scan found 43 parser error(s)\n"
    );
}

#[test]
fn scan_accepts_closed_stdout_even_when_assets_are_invalid() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("test.mix"), archive(&[b"RIFF"])).unwrap();
    let output = fixture
        .command("scan")
        .stdout(closed_stdout())
        .output()
        .unwrap();
    assert_success(&output);
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
