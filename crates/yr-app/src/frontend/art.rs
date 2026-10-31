use crate::{cursor, sidebar::art::Image};
use yr_assets::AssetFs;
use yr_formats::{CsfFile, Palette, ShpFile, UnicodeFont};
#[derive(Default)]
pub struct Art {
    pub cursor: cursor::Art,
    pub(super) font: Option<UnicodeFont>,
    pub(super) strings: Option<CsfFile>,
    pub(super) top: Vec<Image>,
    pub(super) warning: Vec<Image>,
    pub(super) bottom: Option<Image>,
    pub(super) status: Option<Image>,
    pub(super) background: Option<Image>,
    pub(super) buttons: Vec<Image>,
    pub(super) button_back: Option<Image>,
}
impl Art {
    pub fn load(fs: &AssetFs) -> Self {
        let mut art = Self {
            cursor: cursor::Art::load(fs),
            font: fs
                .read("GAME.FNT")
                .and_then(|b| UnicodeFont::parse(b).ok())
                .filter(|f| f.stride <= 8 && f.lines <= 32 && f.line_height <= 40),
            strings: fs.read("RA2MD.CSF").and_then(|b| CsfFile::parse(b).ok()),
            ..Self::default()
        };
        let Some(shell) = fs.read("SHELL.PAL").and_then(Palette::parse) else {
            return art;
        };
        let Some(shell2) = fs.read("SHELL2.PAL").and_then(Palette::parse) else {
            return art;
        };
        let Some(button) = fs.read("SDBTNANM.PAL").and_then(Palette::parse) else {
            return art;
        };
        let read = |name, w, h, n, palette: &Palette| -> Vec<Image> {
            let Some(s) = fs.read(name).and_then(|b| ShpFile::parse(b).ok()) else {
                return Vec::new();
            };
            if (s.width, s.height, s.frames.len()) != (w, h, n) {
                return Vec::new();
            }
            (0..n)
                .map(|i| {
                    Some(Image {
                        width: w.into(),
                        height: h.into(),
                        pixels: s
                            .decode_frame_canvas(i)
                            .ok()?
                            .into_iter()
                            .flat_map(|v| if v == 0 { [0; 4] } else { palette.rgba(v) })
                            .collect(),
                    })
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default()
        };
        art.top = read("SDTP.SHP", 168, 199, 2, &shell);
        art.warning = read("SDWRNANM.SHP", 92, 53, 91, &shell2);
        art.bottom = read("SDBTM.SHP", 168, 65, 1, &shell).pop();
        art.status = read("LWSCRNL.SHP", 632, 32, 1, &shell).pop();
        art.background = read("MNSCRNL.SHP", 632, 568, 1, &shell).pop();
        art.buttons = read("SDBTNANM.SHP", 156, 42, 17, &button);
        art.button_back = read("SDBTNBKGD.SHP", 168, 42, 1, &shell2).pop();
        art
    }
    pub(super) fn label<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.strings
            .as_ref()
            .and_then(|s| s.get(key))
            .unwrap_or(fallback)
    }
}
