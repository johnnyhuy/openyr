//! Theater tile-set index: maps a map cell's global tile number to the TMP
//! template file that supplies its artwork.
//!
//! The theater INI (`temperatmd.ini`, `snowmd.ini`, ...) lists
//! `[TileSet0000]`, `[TileSet0001]`, ... in order, stopping at the first
//! missing number. Each set names a `FileName` base and a `TilesInSet`
//! count; its files are `<base>01`, `<base>02`, ... with the theater's tile
//! extension. Global tile numbers are assigned cumulatively in that order:
//! tile 0 is the first file of the first set. Evidence (retail temperate
//! totals match the largest tile used by temperate maps exactly):
//! `docs/formats/map.md`.

use crate::ini::IniFile;

/// Ordered tile template filenames; `files[tile]` is tile `tile`'s template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TileSetIndex {
    pub files: Vec<String>,
}

impl TileSetIndex {
    /// Build the index from a theater INI. `extension` is the theater's tile
    /// extension without a dot (e.g. `"TEM"`). Sets with `TilesInSet = 0`
    /// (retired sets kept for old-map conversion) occupy no tile numbers.
    pub fn parse(ini: &IniFile, extension: &str) -> TileSetIndex {
        let mut files = Vec::new();
        for n in 0..10000 {
            let Some(sec) = ini.section(&format!("TileSet{n:04}")) else {
                break;
            };
            let Some(base) = sec.get("FileName") else {
                continue;
            };
            let count: u32 = sec
                .get("TilesInSet")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            for i in 1..=count {
                files.push(format!(
                    "{}{i:02}.{}",
                    base.trim().to_ascii_uppercase(),
                    extension.to_ascii_uppercase()
                ));
            }
        }
        TileSetIndex { files }
    }

    pub fn file_for(&self, tile: u32) -> Option<&str> {
        self.files.get(tile as usize).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cumulative_numbering_skips_empty_sets() {
        let ini = IniFile::parse(
            "[TileSet0000]\nFileName=Clear\nTilesInSet=1\n\
             [TileSet0001]\nFileName=blank\nTilesInSet=0\n\
             [TileSet0002]\nFileName=cliff\nTilesInSet=2\n",
        );
        let idx = TileSetIndex::parse(&ini, "tem");
        assert_eq!(idx.files, ["CLEAR01.TEM", "CLIFF01.TEM", "CLIFF02.TEM"]);
        assert_eq!(idx.file_for(2), Some("CLIFF02.TEM"));
        assert_eq!(idx.file_for(3), None);
    }

    #[test]
    fn stops_at_first_missing_set() {
        let ini = IniFile::parse("[TileSet0001]\nFileName=cliff\nTilesInSet=2\n");
        assert!(TileSetIndex::parse(&ini, "TEM").files.is_empty());
    }
}
