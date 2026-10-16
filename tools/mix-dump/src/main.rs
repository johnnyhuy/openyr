//! mix-dump — inspect and extract MIX archives from a game install.
//!
//! Usage:
//!   mix-dump info <archive.mix>
//!   mix-dump list <archive.mix>
//!   mix-dump extract <archive.mix> <name-or-id> <outdir>

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use yr_formats::{mix_id, mix_id_td, MixArchive, MixVersion};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["info", path] => cmd_info(path),
        ["list", path] => cmd_list(path),
        ["names", path] => cmd_names(path),
        ["extract", path, name, outdir] => cmd_extract(path, name, PathBuf::from(outdir)),
        _ => {
            eprintln!("usage:\n  mix-dump info <archive.mix>\n  mix-dump list <archive.mix>\n  mix-dump names <archive.mix>\n  mix-dump extract <archive.mix> <name-or-id> <outdir>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn load(path: &str) -> Result<(Vec<u8>, usize), String> {
    let data = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let len = data.len();
    Ok((data, len))
}

fn cmd_info(path: &str) -> Result<(), String> {
    let (data, len) = load(path)?;
    let mix = MixArchive::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    println!("{path}");
    println!("  file size   : {len} bytes");
    println!("  variant     : {:?}", mix.version());
    println!("  encrypted   : {}", mix.is_encrypted());
    println!("  checksum    : {}", mix.has_checksum());
    println!("  files       : {}", mix.entries().len());
    let body = mix.body_range();
    println!(
        "  body        : {} bytes @ offset {}",
        body.end - body.start,
        body.start
    );
    if let Some(trailer) = mix.checksum_trailer() {
        println!("  sha1 trailer: {}", hex(trailer));
        println!("  sha1(body)  : {}", hex(&mix.body_sha1()));
        println!("  sha1(header+body): {}", hex(&mix.pre_trailer_sha1()));
    }
    Ok(())
}

fn cmd_list(path: &str) -> Result<(), String> {
    let (data, _) = load(path)?;
    let mix = MixArchive::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    println!("{:10} {:>10} {:>12}  type", "id", "size", "offset");
    let mut entries: Vec<_> = mix.entries().iter().collect();
    entries.sort_by_key(|e| e.size);
    entries.reverse();
    for e in entries {
        let head = mix
            .get_by_id(e.id)
            .map(|d| &d[..d.len().min(12)])
            .unwrap_or(&[]);
        println!(
            "0x{:08x} {:>10} {:>12}  {}",
            e.id,
            e.size,
            e.offset,
            sniff(head)
        );
    }
    Ok(())
}

fn cmd_extract(path: &str, name: &str, outdir: PathBuf) -> Result<(), String> {
    let (data, _) = load(path)?;
    let mix = MixArchive::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    let (id, payload) = match parse_id(name) {
        Some(id) => (id, mix.get_by_id(id)),
        None => {
            let id = match mix.version() {
                MixVersion::Classic => mix_id_td(name),
                MixVersion::Flagged => mix_id(name),
            };
            (id, mix.get(name))
        }
    };
    let payload = payload.ok_or_else(|| format!("{name} (0x{id:08x}) not found"))?;
    fs::create_dir_all(&outdir).map_err(|e| e.to_string())?;
    let outfile = outdir.join(sanitize(name));
    fs::write(&outfile, payload).map_err(|e| e.to_string())?;
    println!(
        "extracted {} (0x{id:08x}, {} bytes) -> {}",
        name,
        payload.len(),
        outfile.display()
    );
    Ok(())
}

/// Accepts decimal (`305419896`) or hex (`0x1234abcd`) IDs.
fn parse_id(name: &str) -> Option<u32> {
    match name.strip_prefix("0x").or_else(|| name.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None if name.chars().all(|c| c.is_ascii_digit()) => name.parse().ok(),
        None => None,
    }
}

/// Known RA2/YR filenames used to reverse-resolve hashed IDs. Extend as the
/// name database grows; see `docs/formats/mix.md`.
const KNOWN_NAMES: &[&str] = &[
    // top-level archives and expansion packs
    "RA2.MIX", "RA2MD.MIX", "LANGUAGE.MIX", "LANGMD.MIX", "THEME.MIX", "THEMEMD.MIX",
    "MULTI.MIX", "MULTIMD.MIX", "MAPS01.MIX", "MAPS02.MIX", "MAPSMD03.MIX", "MOVIES01.MIX",
    "MOVIES02.MIX", "MOVMD03.MIX", "WDT.MIX", "EXPANDMD01.MIX", "SIDEC01.MIX",
    // terrain / asset archives (ra2md.mix contents)
    "TEMPERAT.MIX", "SNOW.MIX", "URBAN.MIX", "URBANN.MIX", "DESERT.MIX", "LUNAR.MIX",
    "NEWURBAN.MIX", "TEMPERATMD.MIX", "SNOWMD.MIX", "URBANMD.MIX", "URBANNMD.MIX",
    "DESERTMD.MIX", "LUNARMD.MIX", "NEWURBANMD.MIX",
    // engine archives
    "AUDIO.MIX", "SOUNDS.MIX", "SOUND.MIX", "CACHE.MIX", "CONQUER.MIX", "LOCAL.MIX",
    "SPEECH01.MIX", "SPEECH02.MIX", "SPEECH03.MIX", "ECACHE01.MIX", "ECACHE02.MIX",
    "ECACHE03.MIX", "ECACHE04.MIX", "ECACHE05.MIX", "ECACHE06.MIX", "ECACHE07.MIX",
    "ECACHE08.MIX", "ECACHE09.MIX", "ECACHE10.MIX",
    // INI / rules
    "RULESMD.INI", "ARTMD.INI", "SOUNDMD.INI", "RA2MD.INI", "RA2.INI", "EVAMD.INI",
    "AI.INI", "AIMD.INI", "MPMAPS.MD.INI", "MULTIMD.INI", "MAPSELMD.INI", "MISSIONMD.INI",
    "RULES.INI", "ART.INI", "SOUND.INI", "THEME.INI",
    // string tables
    "RA2.CSF", "RA2MD.CSF", "LANGUAGE.CSF", "LANGMD.CSF",
    // palettes
    "UNITS.PAL", "TEMPERAT.PAL", "SNOW.PAL", "URBAN.PAL", "DESERT.PAL", "LUNAR.PAL",
    "NEWURBAN.PAL", "LIB.PAL", "ANIM.PAL", "ISO.PAL", "CAMEO.PAL", "GOLDNN.PAL",
    "TIBERIUM.PAL", "MULTI.PAL", "SIDE01.PAL", "SIDE02.PAL", "SIDE03.PAL",
    // common unit/building art and cameos
    "AMMO.PAL", "HPMIX01.MIX", "SEICNC01.MIX", "NTPRD01.MIX",
];

fn cmd_names(path: &str) -> Result<(), String> {
    let (data, _) = load(path)?;
    let mix = MixArchive::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    let known: Vec<(u32, &str)> = KNOWN_NAMES
        .iter()
        .map(|n| (mix_id(n), *n))
        .collect();
    println!("{:10} {:>10}  name", "id", "size");
    for e in mix.entries() {
        let name = known
            .iter()
            .find(|(id, _)| *id == e.id)
            .map(|(_, n)| (*n).to_string())
            .unwrap_or_else(|| {
                let head = mix
                    .get_by_id(e.id)
                    .map(|d| &d[..d.len().min(8)])
                    .unwrap_or(&[]);
                format!("? ({})", sniff(head))
            });
        println!("0x{:08x} {:>10}  {}", e.id, e.size, name);
    }
    Ok(())
}

fn sanitize(name: &str) -> String {
    name.rsplit(['/', '\\']).next().unwrap_or(name).to_string()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn sniff(head: &[u8]) -> &'static str {
    match head {
        [0x20, b'F', b'S', b'C', ..] => "csf",
        [b'B', b'I', b'K', b'i', ..] => "bink video",
        [b'R', b'I', b'F', b'F', ..] => "wav",
        [0x00, 0x00, 0x01..=0x03, 0x00, ..] => "nested mix",
        [b'[', ..] => "ini/ascii",
        _ => "binary",
    }
}
