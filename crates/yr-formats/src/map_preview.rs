//! Authored map thumbnail, from [Preview] and LZO [PreviewPack].
use crate::{
    bytes::Cursor,
    compress::{base64_decode, lzo1x_decompress, unpack_chunks},
    IniFile, MapError,
};
#[derive(Debug, Clone)]
pub struct MapPreview {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}
impl MapPreview {
    pub fn parse(ini: &IniFile) -> Result<Option<Self>, MapError> {
        let Some(preview) = ini.section("Preview") else {
            return if ini.section("PreviewPack").is_some() {
                Err(MapError::MissingSection("Preview"))
            } else {
                Ok(None)
            };
        };
        let size = preview
            .get("Size")
            .ok_or(MapError::BadValue("Preview.Size"))?;
        if size.len() > 64 {
            return Err(MapError::BadValue("Preview.Size"));
        }
        let values = size
            .split(',')
            .map(|v| v.trim().parse::<i32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| MapError::BadValue("Preview.Size"))?;
        let [_, _, w, h] = values.as_slice() else {
            return Err(MapError::BadValue("Preview.Size"));
        };
        if !(1..=1024).contains(w) || !(1..=1024).contains(h) {
            return Err(MapError::BadValue("Preview.Size"));
        }
        let expected = (*w as usize)
            .checked_mul(*h as usize)
            .and_then(|v| v.checked_mul(3))
            .ok_or(MapError::BadValue("Preview.Size"))?;
        let section = ini
            .section("PreviewPack")
            .ok_or(MapError::MissingSection("PreviewPack"))?;
        let length = section
            .entries
            .iter()
            .try_fold(0usize, |n, (_, v)| {
                n.checked_add(v.len()).filter(|n| *n <= 8 * 1024 * 1024)
            })
            .ok_or(MapError::BadValue("PreviewPack"))?;
        let mut lines = section
            .entries
            .iter()
            .map(|(k, v)| k.parse::<u32>().map(|n| (n, v.as_str())))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| MapError::BadValue("PreviewPack"))?;
        lines.sort_by_key(|(n, _)| *n);
        if lines.windows(2).any(|p| p[0].0 == p[1].0) {
            return Err(MapError::BadValue("PreviewPack"));
        }
        let mut encoded = String::with_capacity(length);
        for (_, line) in lines {
            encoded.push_str(line);
        }
        let packed = base64_decode(&encoded).map_err(|e| MapError::Pack("PreviewPack", e))?;
        // Check every chunk range and the complete output size before decoding.
        let mut c = Cursor::new(&packed);
        let mut total = 0usize;
        while c.pos() < packed.len() {
            let input = c.u16().ok_or(MapError::BadValue("PreviewPack"))? as usize;
            let output = c.u16().ok_or(MapError::BadValue("PreviewPack"))? as usize;
            if input == 0 || output == 0 {
                return Err(MapError::BadValue("PreviewPack"));
            }
            c.skip(input).ok_or(MapError::BadValue("PreviewPack"))?;
            total = total
                .checked_add(output)
                .filter(|n| *n <= expected)
                .ok_or(MapError::BadValue("PreviewPack"))?;
        }
        if total != expected {
            return Err(MapError::BadValue("PreviewPack"));
        }
        let rgb = unpack_chunks(&packed, lzo1x_decompress)
            .map_err(|e| MapError::Pack("PreviewPack", e))?;
        if rgb.len() != expected {
            return Err(MapError::BadValue("PreviewPack"));
        }
        Ok(Some(Self {
            width: *w as u32,
            height: *h as u32,
            rgb,
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_rgb_and_hostile_preview_headers_are_checked() {
        let valid = "[Preview]\nSize=0,0,2,2\n[PreviewPack]\n1=EAAMAB0KFB4KFB4KFB4KFB4RAAA=\n";
        let image = MapPreview::parse(&IniFile::parse(valid)).unwrap().unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.rgb, [10, 20, 30].repeat(4));
        for size in [
            "0,0,-1,2",
            "0,0,4294967295,2",
            "0,0,1025,1",
            "0,0,1,1",
            "0,0,2",
            "0,0,0,0",
        ] {
            assert!(MapPreview::parse(&IniFile::parse(&valid.replace("0,0,2,2", size))).is_err());
        }
        for data in ["", "EAAMAB0=", "AAD//w==", "EAD//w==", "@@@@"] {
            assert!(MapPreview::parse(&IniFile::parse(
                &valid.replace("EAAMAB0KFB4KFB4KFB4KFB4RAAA=", data)
            ))
            .is_err());
        }
        assert!(MapPreview::parse(&IniFile::default()).unwrap().is_none());
    }
}
