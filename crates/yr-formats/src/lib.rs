//! Pure parsers for Westwood Red Alert 2 / Yuri's Revenge file formats.
//!
//! No I/O and no platform code lives here: callers feed bytes in and get typed
//! data out. Format specs and evidence: `docs/formats/`.

pub mod csf;
pub mod hash;
pub mod ini;
pub mod mix;
pub mod pal;
pub mod sha1;

pub use csf::{CsfEntry, CsfError, CsfFile};
pub use hash::{crc32, mix_id, mix_id_td};
pub use ini::{IniFile, IniSection};
pub use mix::{MixArchive, MixEntry, MixError, MixVersion};
pub use pal::Palette;
