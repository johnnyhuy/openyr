use super::*;

fn map_with(x: u16, y: u16) -> String {
    let mut raw = Vec::new();
    raw.extend(x.to_le_bytes());
    raw.extend(y.to_le_bytes());
    raw.extend(0i32.to_le_bytes());
    raw.extend([0, 0, 0]);
    raw.extend([0; 4]);
    let mut lzo = vec![17 + raw.len() as u8];
    lzo.extend(&raw);
    lzo.extend([0x11, 0, 0]);
    let mut chunk = Vec::new();
    chunk.extend((lzo.len() as u16).to_le_bytes());
    chunk.extend((raw.len() as u16).to_le_bytes());
    chunk.extend(lzo);
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    for part in chunk.chunks(3) {
        let a = part[0];
        let b = part.get(1).copied().unwrap_or(0);
        let c = part.get(2).copied().unwrap_or(0);
        encoded.push(ALPHABET[(a >> 2) as usize] as char);
        encoded.push(ALPHABET[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        encoded.push(if part.len() > 1 {
            ALPHABET[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if part.len() > 2 {
            ALPHABET[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    format!(
        "[Map]\nTheater=TEMPERATE\nSize=0,0,2,2\nLocalSize=0,0,2,2\n[IsoMapPack5]\n1={encoded}\n"
    )
}

#[test]
fn map_extent_budget_rejects_huge_packed_coordinates_before_grid_allocation() {
    for (x, y) in [(u16::MAX, u16::MAX), (512, 511), (u16::MAX, 4)] {
        assert!(matches!(
            MapFile::parse(&map_with(x, y)),
            Err(MapError::TerrainExtent)
        ));
    }
    println!("Packed hostile coordinates reject with TerrainExtent before any coordinate-sized grid allocation.");
}

#[test]
fn map_extent_budget_preserves_boundaries_narrow_extents_and_padding() {
    for (x, y, expected) in [
        (511, 511, (512, 512)),
        (u16::MAX, 3, (65536, 4)),
        (512, 0, (513, 1)),
        (0, 0, (0, 0)),
    ] {
        let map = MapFile::parse(&map_with(x, y)).unwrap();
        assert_eq!(map.terrain_dimensions().unwrap(), expected);
    }
    let mut map = MapFile::parse(&map_with(1, 1)).unwrap();
    map.tiles[0].x = u16::MAX;
    map.tiles[0].y = u16::MAX;
    assert!(matches!(
        map.terrain_dimensions(),
        Err(MapError::TerrainExtent)
    ));
    println!("Accepted exact 262144-cell and long/narrow declared extents; programmatic mutation is revalidated.");
}
