use crate::sidebar::art::Image;
use yr_assets::AssetFs;
use yr_formats::{mix_id, CsfFile, MixArchive, Palette, ShpFile, UnicodeFont};

#[derive(Default)]
pub struct Art {
    pub(super) font: Option<UnicodeFont>,
    pub(super) strings: Option<CsfFile>,
    pub(super) backgrounds: Vec<Image>,
    pub(super) header: Option<Image>,
    pub(super) controls: Vec<Image>,
    pub(super) chrome: Vec<(Image, i32)>,
    pub(super) tabs: Vec<Image>,
    pub(super) bottom: Option<Image>,
    pub(super) row: Option<Image>,
}
impl Art {
    pub fn load(fs: &AssetFs) -> Self {
        let font = fs
            .read("GAME.FNT")
            .and_then(|b| UnicodeFont::parse(b).ok())
            .filter(|f| f.stride <= 8 && f.lines <= 32 && f.line_height <= 40);
        let strings = fs.read("RA2MD.CSF").and_then(|b| CsfFile::parse(b).ok());
        let mut art = Self {
            font,
            strings,
            ..Self::default()
        };
        let Some(archive) = fs
            .read("SIDEC01.MIX")
            .and_then(|b| MixArchive::parse(b).ok())
        else {
            return art;
        };
        let (Some(ui), Some(side)) = (
            archive
                .get_by_id(mix_id("UIBKGD.PAL"))
                .and_then(Palette::parse),
            archive
                .get_by_id(mix_id("SIDEBAR.PAL"))
                .and_then(Palette::parse),
        ) else {
            return art;
        };
        let named = |name: &str, w, h, n, palette: &Palette| {
            archive
                .get_by_id(mix_id(name))
                .map_or_else(Vec::new, |b| decode(b, w, h, n, palette))
        };
        for (name, w, h) in [
            ("BKGDSM.SHP", 472, 448),
            ("BKGDMD.SHP", 632, 568),
            ("BKGDLG.SHP", 856, 736),
        ] {
            if let Some(image) = named(name, w, h, 1, &ui).pop() {
                art.backgrounds.push(image);
            }
        }
        // The filename has not been reverse-resolved. This scoped ID's retail
        // header and three 125x25 canvases are the menu control evidence.
        art.controls = archive
            .get_by_id(0x2f43_f08b)
            .map_or_else(Vec::new, |b| decode(b, 125, 25, 3, &side));
        art.header = archive
            .get_by_id(0xd508_c1a4)
            .and_then(|b| decode(b, 856, 32, 1, &side).pop());
        for (name, w, h, n, y) in [
            ("CREDITS.SHP", 168, 16, 1, 0),
            ("TOP.SHP", 168, 32, 1, 16),
            ("RADAR.SHP", 168, 110, 33, 48),
            ("SIDE1.SHP", 168, 69, 1, 158),
        ] {
            if let Some(image) = named(name, w, h, n, &side).into_iter().next() {
                art.chrome.push((image, y));
            }
        }
        for i in 0..4 {
            if let Some(image) = named(&format!("TAB{i:02}.SHP"), 28, 27, 5, &side)
                .into_iter()
                .nth(2)
            {
                art.tabs.push(image);
            }
        }
        art.bottom = named("SIDE3.SHP", 168, 26, 1, &side).pop();
        art.row = named("SIDE2.SHP", 168, 50, 1, &side).pop();
        art
    }
    pub(super) fn label<'a>(&'a self, label: &str, fallback: &'a str) -> &'a str {
        self.strings
            .as_ref()
            .and_then(|csf| csf.get(label))
            .unwrap_or(fallback)
    }
}

fn decode(bytes: &[u8], w: u16, h: u16, count: usize, palette: &Palette) -> Vec<Image> {
    let Ok(shp) = ShpFile::parse(bytes) else {
        return Vec::new();
    };
    if (shp.width, shp.height, shp.frames.len()) != (w, h, count) {
        return Vec::new();
    }
    (0..count)
        .map(|i| {
            Some(Image {
                width: u32::from(w),
                height: u32::from(h),
                pixels: shp
                    .decode_frame_canvas(i)
                    .ok()?
                    .into_iter()
                    .flat_map(|p| if p == 0 { [0; 4] } else { palette.rgba(p) })
                    .collect(),
            })
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_menu_art_rejects_hostile_or_unexpected_canvases() {
        let palette = Palette::parse(&[0; 768]).unwrap();
        assert!(decode(&[], 125, 25, 3, &palette).is_empty());
        let mut header = vec![0; 8 + 24];
        header[2..4].copy_from_slice(&u16::MAX.to_le_bytes());
        header[4..6].copy_from_slice(&u16::MAX.to_le_bytes());
        header[6..8].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(decode(&header, 125, 25, 3, &palette).is_empty());
        header[6..8].copy_from_slice(&1u16.to_le_bytes());
        assert!(decode(&header, 125, 25, 3, &palette).is_empty());
    }
}
