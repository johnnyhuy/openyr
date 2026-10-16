//! `mix-dump scan`: walk an archive recursively (nested MIXes included),
//! classify every entry with the strict format parsers, and print a census.
//!
//! The census doubles as a corpus-wide parser check: every SHP frame is
//! decoded, every VXL limb's spans walked, every TMP cell read.

use std::collections::{BTreeMap, HashSet};
use yr_formats::names::NameDb;
use yr_formats::{
    CsfFile, HvaFile, IniFile, MixArchive, Palette, PcxImage, ShpFile, TmpFile, VxlFile,
};

#[derive(Default)]
struct Bucket {
    count: usize,
    named: usize,
    bytes: usize,
    examples: Vec<String>,
}

#[derive(Default)]
struct Census {
    kinds: BTreeMap<&'static str, Bucket>,
    /// Free-form detail counters, e.g. "shp compression 3".
    details: BTreeMap<String, usize>,
    errors: Vec<String>,
}

impl Census {
    fn add(&mut self, kind: &'static str, path: &str, named: bool, len: usize, verbose: bool) {
        let b = self.kinds.entry(kind).or_default();
        b.count += 1;
        b.named += usize::from(named);
        b.bytes += len;
        if b.examples.len() < 4 || verbose {
            b.examples.push(format!("{path} ({len} B)"));
        }
    }

    fn detail(&mut self, key: String) {
        *self.details.entry(key).or_default() += 1;
    }
}

pub fn cmd_scan(paths: &[&str], verbose: bool) -> Result<(), String> {
    let mut archives = Vec::new();
    for path in paths {
        let (data, _) = crate::load(path)?;
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        archives.push((name, data));
    }

    // Pass 1: gather every entry ID and every INI, then derive names.
    let mut ids = HashSet::new();
    let mut inis = Vec::new();
    for (_, data) in &archives {
        collect(data, 0, &mut ids, &mut inis);
    }
    let mut names = NameDb::builtin();
    let builtin = names.len();
    names.learn_from_inis(&inis, &ids);

    // Pass 2: classify with names.
    let mut census = Census::default();
    for (name, data) in &archives {
        walk(data, name, 0, &names, &mut census, verbose);
    }

    println!(
        "{} INIs, {} unique IDs, {} names known ({} built-in, {} learned from INIs)\n",
        inis.len(),
        ids.len(),
        names.len(),
        builtin,
        names.len() - builtin
    );
    println!("{:<12} {:>7} {:>7} {:>12}  examples", "kind", "count", "named", "bytes");
    for (kind, b) in &census.kinds {
        println!(
            "{kind:<12} {:>7} {:>7} {:>12}  {}",
            b.count, b.named, b.bytes, b.examples[0]
        );
        for ex in &b.examples[1..] {
            println!("{:<42}{ex}", "");
        }
    }
    if !census.details.is_empty() {
        println!("\ndetails:");
        for (k, n) in &census.details {
            println!("  {k:<40} {n}");
        }
    }
    if !census.errors.is_empty() {
        println!("\nerrors ({}):", census.errors.len());
        for e in census.errors.iter().take(40) {
            println!("  {e}");
        }
    }
    Ok(())
}

fn collect(data: &[u8], depth: usize, ids: &mut HashSet<u32>, inis: &mut Vec<IniFile>) {
    let Ok(mix) = MixArchive::parse(data) else {
        return;
    };
    for e in mix.entries() {
        ids.insert(e.id);
        let Some(payload) = mix.get_by_id(e.id) else {
            continue;
        };
        if is_ini(payload) {
            inis.push(IniFile::parse(&String::from_utf8_lossy(payload)));
        } else if depth < 4 && MixArchive::parse(payload).is_ok_and(|m| !m.entries().is_empty()) {
            collect(payload, depth + 1, ids, inis);
        }
    }
}

fn walk(data: &[u8], path: &str, depth: usize, names: &NameDb, c: &mut Census, verbose: bool) {
    let Ok(mix) = MixArchive::parse(data) else {
        c.errors.push(format!("{path}: not a MIX"));
        return;
    };
    for e in mix.entries() {
        let Some(payload) = mix.get_by_id(e.id) else {
            c.errors.push(format!("{path}/0x{:08x}: entry out of range", e.id));
            continue;
        };
        let hex_label = format!("{path}/0x{:08x}", e.id);
        let kind = classify(payload, &hex_label, c);
        // Drop recovered names whose extension contradicts the content: those
        // are CRC32 collisions, not real filenames.
        let name = names.get(e.id).filter(|n| {
            let fits = name_fits_kind(n, kind);
            if !fits {
                c.detail(format!("rejected name ({kind} content) {n}"));
            }
            fits
        });
        let label = name.map_or(hex_label, |n| format!("{path}/{n}"));
        c.add(kind, &label, name.is_some(), payload.len(), verbose);
        if kind == "mix" && depth < 4 {
            walk(payload, &label, depth + 1, names, c, verbose);
        }
    }
}

fn classify(d: &[u8], label: &str, c: &mut Census) -> &'static str {
    let head = &d[..d.len().min(4)];
    if head == b"RIFF" {
        return "wav";
    }
    if head == b"BIKi" {
        return "bink";
    }
    if head == b" FSC" {
        return if CsfFile::parse(d).is_ok() { "csf" } else { "csf?" };
    }
    if d.starts_with(b"Voxel Animation\0") {
        return match VxlFile::parse(d) {
            Ok(v) => {
                for l in &v.limbs {
                    c.detail(format!("vxl normals mode {}", l.normals_mode));
                }
                c.detail(format!("vxl limbs {}", v.limbs.len()));
                "vxl"
            }
            Err(e) => {
                c.errors.push(format!("{label}: vxl: {e}"));
                "vxl?"
            }
        };
    }
    if let Ok(mix) = MixArchive::parse(d) {
        if !mix.entries().is_empty() {
            return "mix";
        }
    }
    if is_hva(d) {
        return match HvaFile::parse(d) {
            Ok(_) => "hva",
            Err(e) => {
                c.errors.push(format!("{label}: hva: {e}"));
                "hva?"
            }
        };
    }
    if d.len() == 768 && d.iter().all(|&b| b <= 0x3f) && Palette::parse(d).is_some() {
        return "pal";
    }
    if let Ok(tmp) = TmpFile::parse(d) {
        c.detail(format!("tmp tile {}x{}", tmp.tile_width, tmp.tile_height));
        return "tmp";
    }
    if let Ok(shp) = ShpFile::parse(d) {
        for i in 0..shp.frames.len() {
            let f = shp.frames[i];
            if !f.is_empty() {
                c.detail(format!("shp compression {}", f.compression));
            }
            if let Err(e) = shp.decode_frame(i) {
                c.errors.push(format!("{label}: shp frame {i}: {e}"));
                return "shp?";
            }
        }
        return "shp";
    }
    match d {
        [0x0a, 0x05, 0x01, 0x08, ..] => {
            return match PcxImage::parse(d) {
                Ok(img) => {
                    let kind = match img.pixels {
                        yr_formats::PcxPixels::Indexed { .. } => "8-bit",
                        yr_formats::PcxPixels::Rgb(_) => "24-bit",
                    };
                    c.detail(format!("pcx {kind}"));
                    "pcx"
                }
                Err(e) => {
                    c.errors.push(format!("{label}: pcx: {e}"));
                    "pcx?"
                }
            };
        }
        [b'F', b'o', b'N', b't', ..] | [b'f', b'o', b'n', b'T', ..] => return "fnt",
        // AUD: u16 sample rate (22050 Hz in retail) then sizes, flags, codec.
        [0x22, 0x56, ..] if d.len() > 12 => return "aud",
        _ => {}
    }
    if is_ini(d) {
        return "ini";
    }
    "binary"
}

/// HVA has no magic; its size is fully determined by the header counts.
fn is_hva(d: &[u8]) -> bool {
    if d.len() < 24 || !d[..16].iter().all(|&b| b == 0 || b.is_ascii_graphic() || b == b' ') {
        return false;
    }
    let frames = u32::from_le_bytes([d[16], d[17], d[18], d[19]]) as usize;
    let limbs = u32::from_le_bytes([d[20], d[21], d[22], d[23]]) as usize;
    frames > 0
        && limbs > 0
        && frames < 10_000
        && limbs < 256
        && d.len() == 24 + 16 * limbs + 48 * limbs * frames
}

/// A recovered name is only trusted when its extension agrees with the
/// detected content; a mismatch flags a probable CRC32 collision.
fn name_fits_kind(name: &str, kind: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_uppercase();
    match ext.as_str() {
        "SHP" | "SHA" => kind == "shp",
        "VXL" => kind == "vxl",
        "HVA" => kind == "hva",
        // Theater extensions carry both terrain templates and theater SHPs.
        "TEM" | "SNO" | "URB" | "DES" | "LUN" | "UBN" => kind == "tmp" || kind == "shp",
        "PAL" => kind == "pal",
        "PCX" => kind == "pcx",
        "WAV" => kind == "wav",
        "AUD" => kind == "aud",
        "MIX" => kind == "mix",
        "CSF" => kind == "csf",
        "INI" | "MAP" | "MPR" | "YRM" => kind == "ini",
        "BIK" => kind == "bink",
        "FNT" => kind == "fnt",
        _ => true,
    }
}

/// INI-style text: leading bytes are printable ASCII/whitespace and include a
/// section header, comment or `key=value` (maps may open with bare keys).
fn is_ini(d: &[u8]) -> bool {
    let head = &d[..d.len().min(256)];
    let text = head.trim_ascii_start();
    !text.is_empty()
        && head
            .iter()
            .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace() || b >= 0x80)
        && (text[0] == b'[' || text[0] == b';' || head.contains(&b'='))
}
