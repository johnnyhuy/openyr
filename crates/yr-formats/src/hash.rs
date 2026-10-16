//! Filename hashing for MIX archive lookup.
//!
//! RA2/YR use the TS scheme (uppercase + 4-byte tail padding + CRC-32).
//! Tiberian Dawn / Red Alert 1 archives use an older rotate-and-add scheme.
//! See `docs/formats/mix.md` for the evidence behind both.

/// File ID for RA2/YR (and Tiberian Sun) archives.
///
/// Uppercases the name, converts `/` to `\`, pads to a multiple of four bytes
/// with the tail rule below, then takes the standard ZIP CRC-32.
///
/// Tail padding (when `len % 4 != 0`): append one byte with value `len % 4`,
/// then append the first byte of the tail `3 - (len % 4)` times.
pub fn mix_id(name: &str) -> u32 {
    let mut buf: Vec<u8> = name
        .bytes()
        .map(|b| match b.to_ascii_uppercase() {
            b'/' => b'\\',
            b => b,
        })
        .collect();
    let l = buf.len();
    let tail = l & 3;
    if tail != 0 {
        let head = l & !3;
        let first_tail_byte = buf[head];
        buf.push(tail as u8);
        for _ in 0..(3 - tail) {
            buf.push(first_tail_byte);
        }
    }
    crc32(&buf)
}

/// File ID for Tiberian Dawn / Red Alert 1 archives: uppercase, zero-padded
/// 4-byte little-endian groups, `id = id.rotate_left(1) + group`.
pub fn mix_id_td(name: &str) -> u32 {
    let bytes = name.to_ascii_uppercase();
    let bytes = bytes.as_bytes();
    let mut id = 0u32;
    let mut i = 0;
    while i < bytes.len() {
        let mut group = 0u32;
        for _ in 0..4 {
            group >>= 8;
            if i < bytes.len() {
                group |= u32::from(bytes[i]) << 24;
                i += 1;
            }
        }
        id = id.rotate_left(1).wrapping_add(group);
    }
    id
}

/// Standard CRC-32 (ISO 3309 / ZIP): reflected polynomial 0xEDB88320,
/// init 0xFFFFFFFF, final XOR 0xFFFFFFFF.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known-answer tests taken from retail `langmd.mix` (see docs/formats/mix.md):
    // RA2MD.CSF is present in its index under 0xbd835079.
    #[test]
    fn ra2yr_hash_known_answers() {
        assert_eq!(mix_id("RA2MD.CSF"), 0xbd835079);
        assert_eq!(mix_id("ra2md.csf"), 0xbd835079);
        assert_eq!(mix_id("RULESMD.INI"), 0x8218f9f4);
    }

    #[test]
    fn td_hash_known_answers() {
        // XCC local mix database entry id.
        assert_eq!(mix_id_td("local mix database.dat"), 0x54c2d545);
    }

    #[test]
    fn tail_padding_rules() {
        // 9 bytes -> pad 0x01 + first tail byte twice; must equal CRC over
        // the explicit padded form.
        assert_eq!(mix_id("RA2MD.CSF"), crc32(b"RA2MD.CSF\x01FF"));
        // 22 bytes -> pad 0x02 + first tail byte ('A' at index 20) once.
        assert_eq!(
            mix_id("local mix database.dat"),
            crc32(b"LOCAL MIX DATABASE.DAT\x02A")
        );
        // Already aligned: no padding.
        assert_eq!(mix_id("abcd"), crc32(b"ABCD"));
    }

    #[test]
    fn crc32_vector() {
        // "123456789" -> 0xCBF43926 (standard check value).
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
