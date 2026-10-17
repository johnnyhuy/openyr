//! Virtual file system over a Yuri's Revenge install.
//!
//! Mounts the game's MIX archives in the engine's lookup order, including
//! archives nested inside other archives, and resolves a filename to the
//! first archive that holds it, as the engine does. Order and evidence:
//! `docs/formats/vfs.md`.
//!
//! Top-level files are memory-mapped (the retail install is over 1 GB), so
//! reads return slices borrowed from the `AssetFs`.

use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use yr_formats::{mix_id, mix_id_td, MixArchive, MixVersion};

/// YR 1.001 base lookup order after loose files, first match wins.
/// `EXPANDMD##`, `ECACHE*` and `ELOCAL*` are expanded at their positions.
const BASE_ORDER: &[&str] = &[
    "LANGMD.MIX",
    "LANGUAGE.MIX",
    "<EXPANDMD>",
    "RA2MD.MIX",
    "RA2.MIX",
    "CACHEMD.MIX",
    "CACHE.MIX",
    "LOCALMD.MIX",
    "LOCAL.MIX",
    "AUDIOMD.MIX",
    "<ECACHE>",
    "<ELOCAL>",
    "CONQMD.MIX",
    "GENERMD.MIX",
    "GENERIC.MIX",
    "ISOGENMD.MIX",
    "ISOGEN.MIX",
    "CONQUER.MIX",
    "CAMEOMD.MIX",
    "CAMEO.MIX",
    "MAPSMD03.MIX",
    "MULTIMD.MIX",
    "THEMEMD.MIX",
    "MOVMD03.MIX",
    // Not in ModEnc's table but present in retail and needed for their
    // contents (RA2 sound bank, neutral/civilian art, loading screens).
    "AUDIO.MIX",
    "NTRLMD.MIX",
    "NEUTRAL.MIX",
    "LOADMD.MIX",
    "LOAD.MIX",
];

/// Map theaters. Each mounts its archives (when present) after the base set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theater {
    Temperate,
    Snow,
    Urban,
    Desert,
    Lunar,
    NewUrban,
}

impl Theater {
    pub const ALL: [Theater; 6] = [
        Theater::Temperate,
        Theater::Snow,
        Theater::Urban,
        Theater::Desert,
        Theater::Lunar,
        Theater::NewUrban,
    ];

    /// Candidate archives in lookup order: YR tile set, RA2 tile set, theater
    /// art (YR, RA2) and the small per-theater archive. Missing ones are
    /// skipped (e.g. retail has no TEMPERATMD.MIX).
    pub fn archives(self) -> &'static [&'static str] {
        match self {
            Theater::Temperate => &["ISOTEMMD.MIX", "ISOTEMP.MIX", "TEMPERATMD.MIX", "TEMPERAT.MIX", "TEM.MIX"],
            Theater::Snow => &["ISOSNOMD.MIX", "ISOSNOW.MIX", "SNOWMD.MIX", "SNOW.MIX", "SNO.MIX"],
            Theater::Urban => &["ISOURBMD.MIX", "ISOURB.MIX", "URBANMD.MIX", "URBAN.MIX", "URB.MIX"],
            Theater::Desert => &["ISODESMD.MIX", "ISODES.MIX", "DESERTMD.MIX", "DESERT.MIX", "DES.MIX"],
            Theater::Lunar => &["ISOLUNMD.MIX", "ISOLUN.MIX", "LUNARMD.MIX", "LUNAR.MIX", "LUN.MIX"],
            Theater::NewUrban => &["ISOUBNMD.MIX", "ISOUBN.MIX", "URBANNMD.MIX", "URBANN.MIX", "UBN.MIX"],
        }
    }

    /// Tile-set INI candidates in lookup order (YR `*md.ini` first).
    pub fn tileset_inis(self) -> &'static [&'static str] {
        match self {
            Theater::Temperate => &["TEMPERATMD.INI", "TEMPERAT.INI"],
            Theater::Snow => &["SNOWMD.INI", "SNOW.INI"],
            Theater::Urban => &["URBANMD.INI", "URBAN.INI"],
            Theater::Desert => &["DESERTMD.INI", "DESERT.INI"],
            Theater::Lunar => &["LUNARMD.INI", "LUNAR.INI"],
            Theater::NewUrban => &["URBANNMD.INI", "URBAN.INI"],
        }
    }

    /// Terrain palette (`ISO*.PAL`) for this theater.
    pub fn palette(self) -> &'static str {
        match self {
            Theater::Temperate => "ISOTEM.PAL",
            Theater::Snow => "ISOSNO.PAL",
            Theater::Urban => "ISOURB.PAL",
            Theater::Desert => "ISODES.PAL",
            Theater::Lunar => "ISOLUN.PAL",
            Theater::NewUrban => "ISOUBN.PAL",
        }
    }

    /// Letter substituted as the second character of `NewTheater=yes`
    /// art filenames (e.g. `GACNST` -> `GTCNST.SHP` in temperate), with
    /// `G` (generic, in GENERIC.MIX) as the engine's fallback.
    pub fn art_letter(self) -> char {
        match self {
            Theater::Temperate => 'T',
            Theater::Snow => 'A',
            Theater::Urban => 'U',
            Theater::Desert => 'D',
            Theater::Lunar => 'L',
            Theater::NewUrban => 'N',
        }
    }

    /// Unit palette (`UNIT*.PAL`) for this theater.
    pub fn unit_palette(self) -> &'static str {
        match self {
            Theater::Temperate => "UNITTEM.PAL",
            Theater::Snow => "UNITSNO.PAL",
            Theater::Urban => "UNITURB.PAL",
            Theater::Desert => "UNITDES.PAL",
            Theater::Lunar => "UNITLUN.PAL",
            Theater::NewUrban => "UNITUBN.PAL",
        }
    }

    /// Tile template file extension (without dot) for this theater.
    pub fn tile_extension(self) -> &'static str {
        match self {
            Theater::Temperate => "TEM",
            Theater::Snow => "SNO",
            Theater::Urban => "URB",
            Theater::Desert => "DES",
            Theater::Lunar => "LUN",
            Theater::NewUrban => "UBN",
        }
    }

    /// Theater name as written in map files (`[Map] Theater=`).
    pub fn from_map_name(name: &str) -> Option<Theater> {
        match name.trim().to_ascii_uppercase().as_str() {
            "TEMPERATE" => Some(Theater::Temperate),
            "SNOW" => Some(Theater::Snow),
            "URBAN" => Some(Theater::Urban),
            "DESERT" => Some(Theater::Desert),
            "LUNAR" => Some(Theater::Lunar),
            "NEWURBAN" => Some(Theater::NewUrban),
            _ => None,
        }
    }
}

enum Backing {
    Mapped(Mmap),
    Owned(Vec<u8>),
}

impl std::ops::Deref for Backing {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Backing::Mapped(m) => m,
            Backing::Owned(v) => v,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Base,
    Theater,
}

struct Mount {
    /// Path-like provenance, e.g. `RA2MD.MIX/CACHEMD.MIX`.
    label: String,
    root: usize,
    classic: bool,
    /// Entry ID -> absolute byte range within `roots[root]`.
    index: HashMap<u32, (usize, usize)>,
    group: Group,
}

/// Where a file resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// `"<loose>"` for files in the game directory, else the archive path.
    pub source: String,
    pub size: usize,
}

pub struct AssetFs {
    roots: Vec<Backing>,
    /// Uppercase filename -> root index, for loose files in the game dir.
    loose: HashMap<String, usize>,
    mounts: Vec<Mount>,
    theater: Option<Theater>,
}

impl AssetFs {
    /// Open a game directory: map its loose files and mount the base cascade.
    pub fn open(dir: &Path) -> io::Result<AssetFs> {
        let mut fs = AssetFs {
            roots: Vec::new(),
            loose: HashMap::new(),
            mounts: Vec::new(),
            theater: None,
        };
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
            let file = File::open(entry.path())?;
            if file.metadata()?.len() == 0 {
                continue;
            }
            // SAFETY: read-only map of game data; the install must not be
            // modified while the AssetFs is alive.
            let map = unsafe { Mmap::map(&file)? };
            fs.roots.push(Backing::Mapped(map));
            fs.loose.insert(name, fs.roots.len() - 1);
        }
        fs.mount_base();
        Ok(fs)
    }

    /// Build from in-memory loose files (tests, tools).
    pub fn from_files(files: Vec<(String, Vec<u8>)>) -> AssetFs {
        let mut fs = AssetFs {
            roots: Vec::new(),
            loose: HashMap::new(),
            mounts: Vec::new(),
            theater: None,
        };
        for (name, data) in files {
            fs.roots.push(Backing::Owned(data));
            fs.loose.insert(name.to_ascii_uppercase(), fs.roots.len() - 1);
        }
        fs.mount_base();
        fs
    }

    fn mount_base(&mut self) {
        let mut loose: Vec<String> = self.loose.keys().cloned().collect();
        loose.sort();
        for &name in BASE_ORDER {
            match name {
                // Higher numbers load first (ModEnc).
                "<EXPANDMD>" => {
                    for n in (0..100).rev() {
                        self.mount(&format!("EXPANDMD{n:02}.MIX"), Group::Base);
                    }
                }
                // Alphabetical ascending as listed by the file system (ModEnc).
                "<ECACHE>" | "<ELOCAL>" => {
                    let prefix = &name[1..name.len() - 1];
                    for file in loose.iter().filter(|f| f.starts_with(prefix) && f.ends_with(".MIX")) {
                        self.mount(file, Group::Base);
                    }
                }
                name => {
                    self.mount(name, Group::Base);
                }
            }
        }
    }

    /// Switch the theater: unmount the previous theater's archives and mount
    /// this one's after the base set.
    pub fn set_theater(&mut self, theater: Theater) {
        self.mounts.retain(|m| m.group != Group::Theater);
        for name in theater.archives() {
            self.mount(name, Group::Theater);
        }
        self.theater = Some(theater);
    }

    pub fn theater(&self) -> Option<Theater> {
        self.theater
    }

    /// Mount labels in lookup order (after loose files).
    pub fn mounts(&self) -> impl Iterator<Item = &str> {
        self.mounts.iter().map(|m| m.label.as_str())
    }

    /// Read a file by name: loose files first, then mounts in order.
    pub fn read(&self, name: &str) -> Option<&[u8]> {
        self.find(name).map(|(root, start, len, _)| &self.roots[root][start..start + len])
    }

    /// Where `name` resolves from, without reading it.
    pub fn locate(&self, name: &str) -> Option<Location> {
        self.find(name).map(|(_, _, size, source)| Location { source, size })
    }

    fn find(&self, name: &str) -> Option<(usize, usize, usize, String)> {
        let upper = name.to_ascii_uppercase();
        if let Some(&root) = self.loose.get(&upper) {
            return Some((root, 0, self.roots[root].len(), "<loose>".into()));
        }
        let crc = mix_id(name);
        let td = mix_id_td(name);
        self.mounts.iter().find_map(|m| {
            let hit = m.index.get(&crc).or_else(|| m.classic.then(|| m.index.get(&td)).flatten());
            hit.map(|&(start, len)| (m.root, start, len, m.label.clone()))
        })
    }

    /// Mount `name` (resolved through the current cascade) as an archive.
    /// Returns false when it is absent or not a MIX.
    fn mount(&mut self, name: &str, group: Group) -> bool {
        let upper = name.to_ascii_uppercase();
        if self.mounts.iter().any(|m| m.label.rsplit('/').next() == Some(upper.as_str())) {
            return false;
        }
        let Some((root, start, len, source)) = self.find(name) else {
            return false;
        };
        let bytes = &self.roots[root][start..start + len];
        let Ok(mix) = MixArchive::parse(bytes) else {
            return false;
        };
        let base = bytes.as_ptr() as usize;
        let index = mix
            .entries()
            .iter()
            .filter_map(|e| {
                let payload = mix.get_by_id(e.id)?;
                Some((e.id, (start + (payload.as_ptr() as usize - base), payload.len())))
            })
            .collect();
        let label = if source == "<loose>" { upper } else { format!("{source}/{upper}") };
        self.mounts.push(Mount {
            label,
            root,
            classic: mix.version() == MixVersion::Classic,
            index,
            group,
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a classic MIX holding `files` (names hashed with the CRC scheme).
    fn mix(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut entries: Vec<(u32, &[u8])> = files.iter().map(|(n, d)| (mix_id(n), *d)).collect();
        entries.sort_by_key(|e| e.0);
        let body_len: usize = entries.iter().map(|e| e.1.len()).sum();
        let mut out = Vec::new();
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((body_len as u32).to_le_bytes());
        let mut offset = 0u32;
        for (id, d) in &entries {
            out.extend(id.to_le_bytes());
            out.extend(offset.to_le_bytes());
            out.extend((d.len() as u32).to_le_bytes());
            offset += d.len() as u32;
        }
        for (_, d) in &entries {
            out.extend_from_slice(d);
        }
        out
    }

    fn install() -> Vec<(String, Vec<u8>)> {
        let cachemd = mix(&[("UNITTEM.PAL", b"cachemd palette")]);
        let tiles = mix(&[("CLEAR01.TEM", b"tile")]);
        vec![
            ("LANGMD.MIX".into(), mix(&[("A.TXT", b"from langmd")])),
            (
                "RA2MD.MIX".into(),
                mix(&[
                    ("A.TXT", b"from ra2md"),
                    ("B.TXT", b"from ra2md"),
                    ("CACHEMD.MIX", &cachemd),
                    ("ISOTEMMD.MIX", &tiles),
                ]),
            ),
            ("EXPANDMD01.MIX".into(), mix(&[("B.TXT", b"from expandmd01")])),
            ("EXPANDMD02.MIX".into(), mix(&[("B.TXT", b"from expandmd02")])),
            ("RULESMD.INI".into(), b"[General]".to_vec()),
        ]
    }

    #[test]
    fn engine_lookup_order() {
        let fs = AssetFs::from_files(install());
        let read = |n: &str| String::from_utf8(fs.read(n).unwrap().to_vec()).unwrap();
        assert_eq!(read("a.txt"), "from langmd", "LANGMD before RA2MD");
        assert_eq!(read("B.TXT"), "from expandmd02", "EXPANDMD99..00, higher first");
        assert_eq!(read("UNITTEM.PAL"), "cachemd palette", "nested archive mounted");
        assert_eq!(fs.locate("UNITTEM.PAL").unwrap().source, "RA2MD.MIX/CACHEMD.MIX");
        assert_eq!(fs.locate("RULESMD.INI").unwrap().source, "<loose>");
        assert!(fs.read("MISSING.SHP").is_none());
    }

    #[test]
    fn loose_files_win() {
        let mut files = install();
        files.push(("A.TXT".into(), b"loose".to_vec()));
        let fs = AssetFs::from_files(files);
        assert_eq!(fs.read("A.TXT"), Some(&b"loose"[..]));
    }

    #[test]
    fn theater_mounts_swap() {
        let mut fs = AssetFs::from_files(install());
        assert!(fs.read("CLEAR01.TEM").is_none());
        fs.set_theater(Theater::Temperate);
        assert_eq!(fs.read("CLEAR01.TEM"), Some(&b"tile"[..]));
        fs.set_theater(Theater::Snow);
        assert!(fs.read("CLEAR01.TEM").is_none());
        assert_eq!(Theater::from_map_name("newurban"), Some(Theater::NewUrban));
    }
}
