//! Pure parsers for Westwood Red Alert 2 / Yuri's Revenge file formats.
//!
//! No I/O and no platform code lives here: callers feed bytes in and get typed
//! data out. Format specs and evidence: `docs/formats/`.

mod bytes;
pub mod csf;
pub mod hash;
pub mod hva;
pub mod ini;
pub mod mix;
pub mod names;
pub mod pal;
pub mod pcx;
pub mod sha1;
pub mod shp;
pub mod tmp;
pub mod vxl;

pub use csf::{CsfEntry, CsfError, CsfFile};
pub use hash::{crc32, mix_id, mix_id_td};
pub use hva::{HvaError, HvaFile};
pub use ini::{IniFile, IniSection};
pub use mix::{MixArchive, MixEntry, MixError, MixVersion};
pub use pal::Palette;
pub use pcx::{PcxError, PcxImage, PcxPixels};
pub use shp::{ShpError, ShpFile, ShpFrame};
pub use tmp::{TmpError, TmpFile, TmpTile};
pub use vxl::{Voxel, VxlError, VxlFile, VxlLimb};
