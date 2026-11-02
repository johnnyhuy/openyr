use crate::{cursor, sidebar::art::Image};
use yr_assets::AssetFs;
use yr_formats::{CsfFile, MixArchive, Palette, PcxImage, ShpFile, UnicodeFont};
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
    pub(super) setup_background: Option<Image>,
    pub(super) chooser_background: Option<Image>,
    pub(super) buttons: Vec<Image>,
    pub(super) button_back: Option<Image>,
    pub(super) start_markers: Vec<Image>,
    pub(super) scrollbar: Vec<Image>,
    pub(super) setup_widgets: Vec<Image>,
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
        // LOCALMD.MIX entry 3b7f5065 is the observed 18x18 marker background.
        // Its filename is unresolved; formats/front-preview.md records the evidence.
        let marker = fs
            .read("LOCALMD.MIX")
            .and_then(|bytes| MixArchive::parse(bytes).ok())
            .and_then(|mix| mix.get_by_id(0x3b7f5065))
            .and_then(|bytes| ShpFile::parse(bytes).ok())
            .filter(|s| (s.width, s.height, s.frames.len()) == (18, 18, 1))
            .and_then(|s| s.decode_frame_canvas(0).ok());
        if let (Some(indices), Some(font)) = (marker, &art.font) {
            art.start_markers = ('1'..='8')
                .map(|ch| {
                    let mut pixels: Vec<u8> = indices
                        .iter()
                        .flat_map(|&i| if i == 0 { [0; 4] } else { shell.rgba(i) })
                        .collect();
                    let glyph = font.glyph(ch)?;
                    for y in 0..glyph.height.min(18) {
                        for x in 0..u32::from(glyph.width).min(11) {
                            if glyph.pixel(x, y) {
                                let at = ((y * 18 + x + 7) * 4) as usize;
                                pixels[at..at + 4].copy_from_slice(&[248, 236, 96, 255]);
                            }
                        }
                    }
                    Some(Image {
                        width: 18,
                        height: 18,
                        pixels,
                    })
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default();
        }
        art.scrollbar = fs
            .read("LOCALMD.MIX")
            .and_then(|bytes| MixArchive::parse(bytes).ok())
            .and_then(|mix| {
                [
                    (0xeec2554b, 22),
                    (0x85ff1b15, 22),
                    (0x145c99ef, 30),
                    (0x3085cabc, 2),
                    (0xe50a2b3a, 2),
                ]
                .into_iter()
                .map(|(id, height)| {
                    let image = PcxImage::parse(mix.get_by_id(id)?).ok()?;
                    if (image.width, image.height) != (18, height) {
                        return None;
                    }
                    Some(Image {
                        width: 18,
                        height,
                        pixels: image.to_rgba(),
                    })
                })
                .collect::<Option<Vec<_>>>()
            })
            .unwrap_or_default();
        art.setup_widgets = fs
            .read("LOCALMD.MIX")
            .and_then(|bytes| MixArchive::parse(bytes).ok())
            .and_then(|mix| {
                [
                    (0x5c26a605, 12, 22),
                    (0x429d62c3, 18, 18),
                    (0xb1a77caf, 48, 22),
                ]
                .into_iter()
                .map(|(id, width, height)| {
                    let image = PcxImage::parse(mix.get_by_id(id)?).ok()?;
                    if (image.width, image.height) != (width, height) {
                        return None;
                    }
                    Some(Image {
                        width,
                        height,
                        pixels: image.to_rgba(),
                    })
                })
                .collect::<Option<Vec<_>>>()
            })
            .unwrap_or_default();
        art.top = read("SDTP.SHP", 168, 199, 2, &shell);
        art.warning = read("SDWRNANM.SHP", 92, 53, 91, &shell2);
        art.bottom = read("SDBTM.SHP", 168, 65, 1, &shell).pop();
        art.status = read("LWSCRNL.SHP", 632, 32, 1, &shell).pop();
        art.background = read("MNSCRNL.SHP", 632, 568, 1, &shell).pop();
        art.setup_background = backdrop(fs, 0x1fb68421, 0xd833dd7e);
        art.chooser_background = backdrop(fs, 0x2eec89db, 0x78a45e31);
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

fn backdrop(fs: &AssetFs, image_id: u32, palette_id: u32) -> Option<Image> {
    // The original setup/chooser pair is identified by pixels and archive IDs.
    // Filenames remain unresolved; formats/front-backdrop.md records the evidence.
    let mix = MixArchive::parse(fs.read("NTRLMD.MIX")?).ok()?;
    let palette = Palette::parse(mix.get_by_id(palette_id)?)?;
    let shape = ShpFile::parse(mix.get_by_id(image_id)?).ok()?;
    if (shape.width, shape.height, shape.frames.len()) != (632, 568, 1) {
        return None;
    }
    Some(Image {
        width: 632,
        height: 568,
        pixels: shape
            .decode_frame_canvas(0)
            .ok()?
            .into_iter()
            .flat_map(|index| palette.rgba(index))
            .collect(),
    })
}
