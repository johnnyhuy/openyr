//! RA2/YR map files (`.map`/`.mpr`/`.yrm`): an INI with packed binary
//! sections. Spec with evidence: `docs/formats/map.md`.

use crate::compress::{base64_decode, lcw_decompress, lzo1x_decompress, unpack_chunks, DecompressError};
use crate::ini::{IniFile, IniSection};

/// Overlay and overlay-data packs cover a fixed 512x512 cell grid.
pub const OVERLAY_GRID: usize = 512;
/// No overlay in this cell.
pub const NO_OVERLAY: u8 = 0xFF;

/// One cell from `[IsoMapPack5]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoTile {
    pub x: u16,
    pub y: u16,
    /// Global tile index across the theater's tile sets; `None` = clear.
    pub tile: Option<u32>,
    /// Cell within the TMP template.
    pub subtile: u8,
    /// Height level.
    pub level: u8,
}

/// A placed object line from `[Structures]`, `[Units]`, `[Infantry]` or
/// `[Aircraft]` (`owner,type,health,x,y,...`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapObject {
    pub owner: String,
    pub kind: String,
    pub health: u16,
    pub x: u16,
    pub y: u16,
    /// Facing (0-255) where the line carries one.
    pub facing: Option<u16>,
}

#[derive(Debug)]
pub enum MapError {
    MissingSection(&'static str),
    BadValue(&'static str),
    Pack(&'static str, DecompressError),
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::MissingSection(s) => write!(f, "map has no [{s}]"),
            MapError::BadValue(k) => write!(f, "map has an invalid {k}"),
            MapError::Pack(s, e) => write!(f, "[{s}]: {e}"),
        }
    }
}

impl std::error::Error for MapError {}

#[derive(Debug, Clone)]
pub struct MapFile {
    /// `[Map] Theater=` (e.g. `TEMPERATE`).
    pub theater: String,
    /// `[Map] Size=` as `(x, y, width, height)`.
    pub size: (i32, i32, i32, i32),
    /// `[Map] LocalSize=`: the playable area.
    pub local_size: (i32, i32, i32, i32),
    pub tiles: Vec<IsoTile>,
    /// Overlay type index per cell (`x + y * 512`), or [`NO_OVERLAY`].
    pub overlay: Vec<u8>,
    /// Overlay frame/data byte per cell.
    pub overlay_data: Vec<u8>,
    /// `[Terrain]`: cell -> terrain type (trees, etc.).
    pub terrain: Vec<(u16, u16, String)>,
    pub structures: Vec<MapObject>,
    pub units: Vec<MapObject>,
    pub infantry: Vec<MapObject>,
    pub aircraft: Vec<MapObject>,
    /// The full INI, for sections not modelled here (rules overrides, etc.).
    pub ini: IniFile,
}

impl MapFile {
    pub fn parse(text: &str) -> Result<MapFile, MapError> {
        let ini = IniFile::parse(text);
        let map = ini.section("Map").ok_or(MapError::MissingSection("Map"))?;
        let rect = |key: &'static str| -> Result<(i32, i32, i32, i32), MapError> {
            let v: Vec<i32> = map
                .get(key)
                .ok_or(MapError::BadValue(key))?
                .split(',')
                .map(|p| p.trim().parse().map_err(|_| MapError::BadValue(key)))
                .collect::<Result<_, _>>()?;
            match v[..] {
                [a, b, c, d] => Ok((a, b, c, d)),
                _ => Err(MapError::BadValue(key)),
            }
        };
        let theater = map.get("Theater").ok_or(MapError::BadValue("Theater"))?.trim().to_string();
        let size = rect("Size")?;
        let local_size = rect("LocalSize").unwrap_or(size);

        let iso = pack(&ini, "IsoMapPack5")?.ok_or(MapError::MissingSection("IsoMapPack5"))?;
        let raw = unpack_chunks(&iso, lzo1x_decompress).map_err(|e| MapError::Pack("IsoMapPack5", e))?;
        // Retail packs are 11-byte entries plus a 4-byte zero trailer
        // (verified on all 234 retail maps; see docs/formats/map.md). Any
        // other shape is a corrupt or hostile pack, not a partial map.
        if raw.len() < 4 || (raw.len() - 4) % 11 != 0 || raw[raw.len() - 4..] != [0, 0, 0, 0] {
            return Err(MapError::BadValue("IsoMapPack5"));
        }
        // At most one entry per overlay-grid cell (retail maps have no
        // duplicate cells); more than that is a hostile pack.
        if raw.len() - 4 > 11 * OVERLAY_GRID * OVERLAY_GRID {
            return Err(MapError::BadValue("IsoMapPack5"));
        }
        let tiles = raw[..raw.len() - 4]
            .as_chunks::<11>().0.iter()
            .map(|c| {
                let tile = i32::from_le_bytes([c[4], c[5], c[6], c[7]]);
                IsoTile {
                    x: u16::from_le_bytes([c[0], c[1]]),
                    y: u16::from_le_bytes([c[2], c[3]]),
                    // 0xFFFF / -1 are the clear tile in retail packs.
                    tile: (tile >= 0 && tile != 0xFFFF).then_some(tile as u32),
                    subtile: c[8],
                    level: c[9],
                }
            })
            // Zero-coordinate padding entries fill the end of the pack.
            .filter(|t| t.x != 0 || t.y != 0)
            .collect();

        let grid = OVERLAY_GRID * OVERLAY_GRID;
        let overlay = overlay_pack(&ini, "OverlayPack", grid, NO_OVERLAY)?;
        let overlay_data = overlay_pack(&ini, "OverlayDataPack", grid, 0)?;

        let terrain = ini
            .section("Terrain")
            .map(|s| {
                s.entries
                    .iter()
                    .filter_map(|(k, v)| {
                        let cell: u32 = k.trim().parse().ok()?;
                        Some(((cell % 1000) as u16, (cell / 1000) as u16, v.trim().to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(MapFile {
            theater,
            size,
            local_size,
            tiles,
            overlay,
            overlay_data,
            terrain,
            structures: objects(ini.section("Structures"), 3, Some(5)),
            units: objects(ini.section("Units"), 3, Some(5)),
            infantry: objects(ini.section("Infantry"), 3, Some(7)),
            aircraft: objects(ini.section("Aircraft"), 3, Some(5)),
            ini,
        })
    }

    pub fn overlay_at(&self, x: u16, y: u16) -> Option<(u8, u8)> {
        let i = usize::from(x) + usize::from(y) * OVERLAY_GRID;
        let o = *self.overlay.get(i)?;
        (o != NO_OVERLAY).then(|| (o, self.overlay_data.get(i).copied().unwrap_or(0)))
    }
}

/// Concatenate a pack section's numbered lines in key order and base64-decode.
fn pack(ini: &IniFile, section: &'static str) -> Result<Option<Vec<u8>>, MapError> {
    let Some(sec) = ini.section(section) else {
        return Ok(None);
    };
    let mut lines: Vec<(u32, &str)> = sec
        .entries
        .iter()
        .filter_map(|(k, v)| Some((k.trim().parse().ok()?, v.as_str())))
        .collect();
    lines.sort_by_key(|l| l.0);
    let text: String = lines.into_iter().map(|l| l.1).collect();
    base64_decode(&text).map(Some).map_err(|e| MapError::Pack(section, e))
}

fn overlay_pack(ini: &IniFile, section: &'static str, len: usize, fill: u8) -> Result<Vec<u8>, MapError> {
    match pack(ini, section)? {
        Some(data) => {
            let v = unpack_chunks(&data, lcw_decompress).map_err(|e| MapError::Pack(section, e))?;
            // Retail packs decode to exactly the 512x512 grid; a different
            // size means the pack is corrupt, not a smaller map.
            if v.len() != len {
                return Err(MapError::BadValue(section));
            }
            Ok(v)
        }
        None => Ok(vec![fill; len]),
    }
}

/// Parse `owner,type,health,x,y,...` object lines. `xy_at` is the index of
/// the x field (y follows); `facing_at` the facing field if present.
fn objects(sec: Option<&IniSection>, xy_at: usize, facing_at: Option<usize>) -> Vec<MapObject> {
    let Some(sec) = sec else { return Vec::new() };
    sec.entries
        .iter()
        .filter_map(|(_, v)| {
            let f: Vec<&str> = v.split(',').map(str::trim).collect();
            Some(MapObject {
                owner: f.first()?.to_string(),
                kind: f.get(1)?.to_string(),
                health: f.get(2)?.parse().ok()?,
                x: f.get(xy_at)?.parse().ok()?,
                y: f.get(xy_at + 1)?.parse().ok()?,
                facing: facing_at.and_then(|i| f.get(i)?.parse().ok()),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode bytes as base64 for building synthetic packs.
    fn b64(data: &[u8]) -> String {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for c in data.chunks(3) {
            let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
            for i in 0..4 {
                if i <= c.len() {
                    s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    s.push('=');
                }
            }
        }
        s
    }

    #[test]
    fn parses_iso_pack_objects_and_terrain() {
        // One tile entry (x=10, y=20, tile 5, sub 1, level 3) as an LZO
        // literal run, wrapped in one chunk.
        let mut entry = Vec::new();
        entry.extend(10u16.to_le_bytes());
        entry.extend(20u16.to_le_bytes());
        entry.extend(5i32.to_le_bytes());
        entry.extend([1, 3, 0]);
        entry.extend([0, 0, 0, 0]); // 4-byte zero trailer
        let mut lzo = vec![17 + 15];
        lzo.extend(&entry);
        lzo.extend([0x11, 0, 0]);
        let mut chunk = Vec::new();
        chunk.extend((lzo.len() as u16).to_le_bytes());
        chunk.extend(15u16.to_le_bytes());
        chunk.extend(&lzo);
        let text = format!(
            "[Map]\nTheater=TEMPERATE\nSize=0,0,50,50\nLocalSize=2,4,46,40\n\
             [IsoMapPack5]\n1={}\n[Terrain]\n20010=TREE01\n\
             [Structures]\n0=Neutral,CAAIRP,256,80,96,64,None,0,0,1,0,0,None,None,None,0,0\n",
            b64(&chunk)
        );
        let map = MapFile::parse(&text).unwrap();
        assert_eq!(map.theater, "TEMPERATE");
        assert_eq!(map.local_size, (2, 4, 46, 40));
        assert_eq!(
            map.tiles,
            vec![IsoTile { x: 10, y: 20, tile: Some(5), subtile: 1, level: 3 }]
        );
        assert_eq!(map.terrain, vec![(10, 20, "TREE01".to_string())]);
        assert_eq!(map.structures[0].kind, "CAAIRP");
        assert_eq!((map.structures[0].x, map.structures[0].y, map.structures[0].facing), (80, 96, Some(64)));
        assert!(map.overlay_at(10, 20).is_none());
    }

    #[test]
    fn rejects_iso_pack_without_zero_trailer() {
        // Same pack as above but with the trailer bytes corrupted.
        let mut entry = Vec::new();
        entry.extend(10u16.to_le_bytes());
        entry.extend(20u16.to_le_bytes());
        entry.extend(5i32.to_le_bytes());
        entry.extend([1, 3, 0, 9, 9, 9, 9]);
        let mut lzo = vec![17 + 15];
        lzo.extend(&entry);
        lzo.extend([0x11, 0, 0]);
        let mut chunk = Vec::new();
        chunk.extend((lzo.len() as u16).to_le_bytes());
        chunk.extend(15u16.to_le_bytes());
        chunk.extend(&lzo);
        let text = format!(
            "[Map]\nTheater=TEMPERATE\nSize=0,0,50,50\n[IsoMapPack5]\n1={}\n",
            b64(&chunk)
        );
        assert!(matches!(MapFile::parse(&text), Err(MapError::BadValue("IsoMapPack5"))));
    }

    #[test]
    fn rejects_short_overlay_pack() {
        // OverlayPack decoding to 3 bytes instead of the 512x512 grid.
        let lcw = [0x83, b'a', b'b', b'c', 0x80];
        let mut chunk = Vec::new();
        chunk.extend((lcw.len() as u16).to_le_bytes());
        chunk.extend(3u16.to_le_bytes());
        chunk.extend(lcw);
        // Minimal valid iso pack: just the trailer.
        let mut iso_lzo = vec![17 + 4, 0, 0, 0, 0, 0x11, 0, 0];
        let mut iso_chunk = Vec::new();
        iso_chunk.extend((iso_lzo.len() as u16).to_le_bytes());
        iso_chunk.extend(4u16.to_le_bytes());
        iso_chunk.append(&mut iso_lzo);
        let text = format!(
            "[Map]\nTheater=TEMPERATE\nSize=0,0,50,50\n[IsoMapPack5]\n1={}\n[OverlayPack]\n1={}\n",
            b64(&iso_chunk),
            b64(&chunk)
        );
        assert!(matches!(MapFile::parse(&text), Err(MapError::BadValue("OverlayPack"))));
    }
}
