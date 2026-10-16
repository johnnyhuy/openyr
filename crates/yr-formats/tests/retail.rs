//! Retail corpus tests: parse the real Yuri's Revenge archives from the local
//! testbed and check invariants that only hold if the parse is correct.
//!
//! Corpus: `~/Games/RA2YR/prefix/drive_c/Westwood/RA2` (user-supplied, never
//! committed). All tests skip when the corpus is absent.

use std::fs;
use std::path::PathBuf;
use yr_formats::{mix_id, CsfFile, IniFile, MixArchive};

fn corpus() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var("HOME").ok()?)
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    dir.is_dir().then_some(dir)
}

fn read(name: &str) -> Option<Vec<u8>> {
    let dir = corpus()?;
    let p = dir.join(name);
    p.is_file().then(|| fs::read(&p).ok()).flatten()
}

#[test]
fn theme_mix_is_classic_with_wav_themes() {
    let Some(data) = read("theme.mix") else {
        eprintln!("skip: retail corpus not present");
        return;
    };
    let mix = MixArchive::parse(&data).expect("theme.mix parses as classic MIX");
    assert_eq!(mix.entries().len(), 16);
    assert!(!mix.is_encrypted());
    assert!(!mix.has_checksum());
    for e in mix.entries() {
        let head = &mix.get_by_id(e.id).unwrap()[..4];
        assert_eq!(head, b"RIFF", "theme entry 0x{:08x} is WAV", e.id);
    }
}

#[test]
fn langmd_mix_csf_known_answer() {
    let Some(data) = read("langmd.mix") else {
        eprintln!("skip: retail corpus not present");
        return;
    };
    let mix = MixArchive::parse(&data).expect("langmd.mix parses");
    assert_eq!(mix.entries().len(), 10);
    assert!(mix.has_checksum());
    assert!(!mix.is_encrypted());

    // The known-answer hash test: RA2MD.CSF is in this archive under the CRC id.
    let csf_raw = mix.get("ra2md.csf").expect("RA2MD.CSF present");
    assert_eq!(
        mix.get_by_id(mix_id("RA2MD.CSF")).map(|d| d.len()),
        Some(csf_raw.len())
    );

    let csf = CsfFile::parse(csf_raw).expect("CSF parses");
    assert_eq!(csf.version, 3);
    assert_eq!(csf.entries.len(), 5211);
    // Sanity: real labels resolve to real strings.
    assert!(csf.entries.iter().all(|e| !e.label.is_empty()));
    assert!(csf.entries.iter().any(|e| e.value.as_deref().map(|v| !v.is_empty()).unwrap_or(false)));
    // WRTS records carry the EVA sound stem (e.g. VOX:ceva001 -> ceva001e).
    let vox = csf
        .entries
        .iter()
        .find(|e| e.label.starts_with("VOX:"))
        .expect("VOX labels present");
    assert_eq!(vox.sound.as_deref(), Some("ceva001e"));
    assert!(vox.value.as_deref().unwrap().starts_with("Warning:"));
}

#[test]
fn langmd_checksum_trailer_matches_body() {
    let Some(data) = read("langmd.mix") else {
        eprintln!("skip: retail corpus not present");
        return;
    };
    let mix = MixArchive::parse(&data).unwrap();
    let trailer = mix.checksum_trailer().unwrap();
    assert_eq!(trailer.len(), 20);
    // Discovered semantics (see docs/formats/mix.md): SHA-1 of the body only.
    assert_eq!(trailer, mix.body_sha1().as_slice());
}

#[test]
fn encrypted_archives_parse() {
    for name in [
        "ra2.mix",
        "ra2md.mix",
        "language.mix",
        "expandmd01.mix",
        "maps01.mix",
        "multi.mix",
        "wdt.mix",
    ] {
        let Some(data) = read(name) else {
            eprintln!("skip: retail corpus not present");
            return;
        };
        let mix = MixArchive::parse(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(mix.is_encrypted(), "{name} should be encrypted");
        assert!(
            mix.entries().iter().all(|e| e.size > 0),
            "{name} entries have sizes"
        );
        eprintln!(
            "{name}: {} files, {} encrypted",
            mix.entries().len(),
            mix.is_encrypted()
        );
    }
}

#[test]
fn rulesmd_ini_is_reachable() {
    let mut found = None;
    for name in ["expandmd01.mix", "ra2md.mix"] {
        let Some(data) = read(name) else {
            eprintln!("skip: retail corpus not present");
            return;
        };
        let mix = MixArchive::parse(&data).unwrap();
        if let Some(ini_raw) = mix.get("rulesmd.ini") {
            found = Some((name, ini_raw.to_vec()));
            break;
        }
    }
    let (where_, raw) = found.expect("RULESMD.INI found in the YR install");
    let text = String::from_utf8_lossy(&raw);
    let ini = IniFile::parse(&text);
    assert!(
        ini.section("General").is_some(),
        "rulesmd.ini in {where_} has [General]"
    );
    eprintln!(
        "rulesmd.ini in {where_}: {} sections, {} bytes",
        ini.sections.len(),
        raw.len()
    );
}
