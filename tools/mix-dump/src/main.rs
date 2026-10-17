//! mix-dump — inspect and extract MIX archives from a game install.
//!
//! Usage:
//!   mix-dump info <archive.mix>
//!   mix-dump list <archive.mix>
//!   mix-dump names <archive.mix>
//!   mix-dump scan [-v] <archive.mix>...  recursive content census + name recovery
//!   mix-dump extract <archive.mix> <name-or-id> <outdir>
//!   mix-dump render <archive.mix:path> [<archive.mix:palette.pal>] <out.png>
//!   mix-dump wav <archive.mix:path[/AUDIO.BAG#name]> <out.wav>
//!   mix-dump vfs <gamedir> [--theater <name>] [file...]   engine-order lookup
//!   mix-dump rules <gamedir> [ID...]   typed rules summary / resolved unit
//!   mix-dump render-map <gamedir> <map> <out.png> [scale]   terrain to PNG

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
mod maprender;
mod png;
mod render;
mod rules;
mod scan;
mod vfs;
mod wav;

use yr_formats::names::known_name;
use yr_formats::{mix_id, MixArchive};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["info", path] => cmd_info(path),
        ["list", path] => cmd_list(path),
        ["names", path] => cmd_names(path),
        ["scan", "-v", ref paths @ ..] if !paths.is_empty() => scan::cmd_scan(paths, true),
        ["scan", ref paths @ ..] if !paths.is_empty() => scan::cmd_scan(paths, false),
        ["rules", ref rest @ ..] if !rest.is_empty() => rules::cmd_rules(rest),
        ["render-map", ref rest @ ..] if rest.len() >= 3 => maprender::cmd_render_map(rest),
        ["vfs", ref rest @ ..] if !rest.is_empty() => vfs::cmd_vfs(rest),
        ["wav", asset, out] => wav::cmd_wav(asset, out),
        ["render", asset, out] => render::cmd_render(asset, None, out),
        ["render", asset, palette, out] => render::cmd_render(asset, Some(palette), out),
        ["extract", path, name, outdir] => cmd_extract(path, name, PathBuf::from(outdir)),
        _ => {
            eprintln!("usage:\n  mix-dump info <archive.mix>\n  mix-dump list <archive.mix>\n  mix-dump names <archive.mix>\n  mix-dump scan [-v] <archive.mix>...\n  mix-dump extract <archive.mix> <name-or-id> <outdir>\n  mix-dump render <archive.mix:path> [<archive.mix:palette>] <out.png>\n  mix-dump wav <archive.mix:path[.BAG#name]> <out.wav>\n  mix-dump vfs <gamedir> [--theater <name>] [file...]\n  mix-dump rules <gamedir> [ID...]\n  mix-dump render-map <gamedir> <map> <out.png> [scale]");
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

pub(crate) fn load(path: &str) -> Result<(Vec<u8>, usize), String> {
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

/// `name` may be a nested path through inner archives, e.g.
/// `NTRLMD.MIX/0x88480c72`; each segment is a filename or hex/decimal ID.
fn cmd_extract(path: &str, name: &str, outdir: PathBuf) -> Result<(), String> {
    let (data, _) = load(path)?;
    let (payload, id) = descend(&data, name)?;
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

/// Walk `/`-separated segments through nested archives; returns the payload
/// and the ID of the last segment.
pub(crate) fn descend<'a>(data: &'a [u8], path: &str) -> Result<(&'a [u8], u32), String> {
    let mut current = data;
    let mut id = 0;
    for segment in path.split('/') {
        let mix = MixArchive::parse(current).map_err(|e| format!("{segment}: {e}"))?;
        id = parse_id(segment)
            .or_else(|| mix.resolve(segment))
            .unwrap_or_else(|| mix_id(segment));
        let payload = mix.get_by_id(id);
        current = payload.ok_or_else(|| format!("{segment} (0x{id:08x}) not found"))?;
    }
    Ok((current, id))
}

/// Accepts decimal (`305419896`) or hex (`0x1234abcd`) IDs.
fn parse_id(name: &str) -> Option<u32> {
    match name.strip_prefix("0x").or_else(|| name.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None if name.chars().all(|c| c.is_ascii_digit()) => name.parse().ok(),
        None => None,
    }
}

fn cmd_names(path: &str) -> Result<(), String> {
    let (data, _) = load(path)?;
    let mix = MixArchive::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    println!("{:10} {:>10}  name", "id", "size");
    for e in mix.entries() {
        let name = known_name(e.id)
            .map(str::to_string)
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
