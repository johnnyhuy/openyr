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

const MAX_FILES: usize = 65_536;
const MAX_STORAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileSetError {
    InvalidCount,
    TooManyFiles,
    TooMuchStorage,
    Allocation,
}

impl std::fmt::Display for TileSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidCount => "invalid theater TilesInSet count",
            Self::TooManyFiles => "theater tile index exceeds 65536 filenames",
            Self::TooMuchStorage => "theater tile index exceeds 16 MiB storage budget",
            Self::Allocation => "cannot allocate bounded theater tile index",
        })
    }
}

impl std::error::Error for TileSetError {}

/// Ordered tile template filenames; `files[tile]` is tile `tile`'s template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TileSetIndex {
    pub files: Vec<String>,
}

impl TileSetIndex {
    /// Build the index from a theater INI. `extension` is the theater's tile
    /// extension without a dot (e.g. `"TEM"`). Sets with `TilesInSet = 0`
    /// (retired sets kept for old-map conversion) occupy no tile numbers.
    pub fn parse(ini: &IniFile, extension: &str) -> Result<TileSetIndex, TileSetError> {
        let mut files = Vec::new();
        let mut storage = 0usize;
        let extension = extension.to_ascii_uppercase();
        for n in 0..10000 {
            let Some(sec) = ini.section(&format!("TileSet{n:04}")) else {
                break;
            };
            let Some(base) = sec.get("FileName") else {
                continue;
            };
            let count = sec.get("TilesInSet").map_or(Ok(0), |v| {
                v.trim()
                    .parse::<u32>()
                    .map_err(|_| TileSetError::InvalidCount)
            })?;
            let count = usize::try_from(count).map_err(|_| TileSetError::TooManyFiles)?;
            files
                .len()
                .checked_add(count)
                .filter(|&total| total <= MAX_FILES)
                .ok_or(TileSetError::TooManyFiles)?;
            if count == 0 {
                continue;
            }
            let base = base.trim().to_ascii_uppercase();
            // Budget filename bytes and String entries before any expansion.
            let per_file = base
                .len()
                .checked_add(extension.len())
                .and_then(|n| n.checked_add(1 + count.to_string().len().max(2)))
                .and_then(|n| n.checked_add(std::mem::size_of::<String>()))
                .ok_or(TileSetError::TooMuchStorage)?;
            storage = count
                .checked_mul(per_file)
                .and_then(|n| storage.checked_add(n))
                .filter(|&n| n <= MAX_STORAGE_BYTES)
                .ok_or(TileSetError::TooMuchStorage)?;
            files
                .try_reserve_exact(count)
                .map_err(|_| TileSetError::Allocation)?;
            for i in 1..=count {
                files.push(format!("{base}{i:02}.{extension}"));
            }
        }
        Ok(TileSetIndex { files })
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
        let idx = TileSetIndex::parse(&ini, "tem").unwrap();
        assert_eq!(idx.files, ["CLEAR01.TEM", "CLIFF01.TEM", "CLIFF02.TEM"]);
        assert_eq!(idx.file_for(2), Some("CLIFF02.TEM"));
        assert_eq!(idx.file_for(3), None);
    }

    #[test]
    fn stops_at_first_missing_set() {
        let ini = IniFile::parse("[TileSet0001]\nFileName=cliff\nTilesInSet=2\n");
        assert!(TileSetIndex::parse(&ini, "TEM").unwrap().files.is_empty());
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn tileset_budget_rejects_hostile_and_malformed_counts_before_expansion() {
        for count in ["4294967295", "65537"] {
            let ini = IniFile::parse(&format!(
                "[TileSet0000]\nFileName=Clear\nTilesInSet={count}\n"
            ));
            assert_eq!(
                TileSetIndex::parse(&ini, "TEM"),
                Err(TileSetError::TooManyFiles)
            );
        }
        for count in ["4294967296", "-1", "not-a-count", "1.5", ""] {
            let ini = IniFile::parse(&format!(
                "[TileSet0000]\nFileName=Clear\nTilesInSet={count}\n"
            ));
            assert_eq!(
                TileSetIndex::parse(&ini, "TEM"),
                Err(TileSetError::InvalidCount)
            );
        }
        let ini = IniFile::parse("[TileSet0000]\nFileName=A\nTilesInSet=32768\n[TileSet0001]\nFileName=B\nTilesInSet=32769\n");
        assert_eq!(
            TileSetIndex::parse(&ini, "TEM"),
            Err(TileSetError::TooManyFiles)
        );
        println!(
            "Huge, aggregate and malformed tile counts return errors before oversized expansion."
        );
    }

    #[test]
    fn tileset_budget_bounds_long_prefix_expansion_and_cumulative_storage() {
        let base = "X".repeat(1024 * 1024);
        let ini = IniFile::parse(&format!("[TileSet0000]\nFileName={base}\nTilesInSet=20\n"));
        assert_eq!(
            TileSetIndex::parse(&ini, "TEM"),
            Err(TileSetError::TooMuchStorage)
        );
        let base = "Y".repeat(1024);
        let ini = IniFile::parse(&format!("[TileSet0000]\nFileName={base}\nTilesInSet=8000\n[TileSet0001]\nFileName={base}\nTilesInSet=8000\n"));
        assert_eq!(
            TileSetIndex::parse(&ini, "TEM"),
            Err(TileSetError::TooMuchStorage)
        );
        println!("Long-prefix and aggregate filename storage exceed the bounded 16 MiB budget and are rejected.");
    }

    #[test]
    fn tileset_budget_accepts_the_count_boundary_and_preserves_numbering() {
        let ini = IniFile::parse(&format!(
            "[TileSet0000]\nFileName=Clear\nTilesInSet={MAX_FILES}\n"
        ));
        let index = TileSetIndex::parse(&ini, "tem").unwrap();
        assert_eq!(index.files.len(), MAX_FILES);
        assert_eq!(index.file_for(0), Some("CLEAR01.TEM"));
        assert_eq!(index.file_for(99), Some("CLEAR100.TEM"));
        assert_eq!(index.file_for(65535), Some("CLEAR65536.TEM"));
        assert_eq!(index.file_for(65536), None);
        println!(
            "Accepted 65536 short template names without truncating or shifting tile indexes."
        );
    }
}
