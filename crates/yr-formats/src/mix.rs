//! MIX archive reader (Tiberian Dawn, RA/TS/RA2/YR variants).
//!
//! Layout and validation rules are documented in `docs/formats/mix.md`.

use crate::hash::{mix_id, mix_id_td};
use crate::sha1::sha1;
use blowfish::cipher::{BlockDecrypt, KeyInit};
use blowfish::Blowfish;
use num_bigint::BigUint;

const FLAG_CHECKSUM: u32 = 0x0001_0000;
const FLAG_ENCRYPTED: u32 = 0x0002_0000;
const KEY_SOURCE_LEN: usize = 80;
const BLOWFISH_KEY_LEN: usize = 56;
const ENTRY_LEN: usize = 12;

/// Base64 `AihRvNoIbTn85FZRYNZRcT+i6KpU+maCsEqr3Q5q+LDB5tH7Tz2qQ38V`, which
/// decodes to a DER integer header `02 28` followed by this 40-byte modulus.
/// Source: XCC Utilities (Olaf van der Spek, 2000); it is a number, not code.
const WESTWOOD_MODULUS: [u8; 40] = [
    0x51, 0xbc, 0xda, 0x08, 0x6d, 0x39, 0xfc, 0xe4, 0x56, 0x51, 0x60, 0xd6, 0x51, 0x71, 0x3f, 0xa2,
    0xe8, 0xaa, 0x54, 0xfa, 0x66, 0x82, 0xb0, 0x4a, 0xab, 0xdd, 0x0e, 0x6a, 0xf8, 0xb0, 0xc1, 0xe6,
    0xd1, 0xfb, 0x4f, 0x3d, 0xaa, 0x43, 0x7f, 0x15,
];
const WESTWOOD_EXPONENT: u32 = 0x1_0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixEntry {
    pub id: u32,
    /// Offset from the start of the body (already resolved to archive-relative).
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixVersion {
    /// Tiberian Dawn style: no flags word, classic header.
    Classic,
    /// RA/TS/RA2/YR style: optional flags word.
    Flagged,
}

#[derive(Debug)]
pub enum MixError {
    TooSmall,
    BadHeader,
    BadIndex,
    BadKey,
    SizeMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for MixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MixError::TooSmall => write!(f, "file too small for a MIX archive"),
            MixError::BadHeader => write!(f, "invalid MIX header"),
            MixError::BadIndex => write!(f, "invalid MIX index"),
            MixError::BadKey => write!(f, "could not derive MIX Blowfish key"),
            MixError::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "MIX size identity failed: expected {expected}, actual {actual}"
                )
            }
        }
    }
}

impl std::error::Error for MixError {}

/// A parsed MIX archive borrowing the original bytes.
pub struct MixArchive<'a> {
    data: &'a [u8],
    version: MixVersion,
    encrypted: bool,
    checksummed: bool,
    body_start: usize,
    data_size: usize,
    entries: Vec<MixEntry>,
}

impl<'a> MixArchive<'a> {
    /// Parse a MIX archive from raw bytes.
    ///
    /// All structural checks are strict: the size identity from the XCC reader
    /// must hold exactly, so a wrong parse fails loudly instead of returning
    /// garbage slices.
    pub fn parse(data: &'a [u8]) -> Result<Self, MixError> {
        if data.len() < 6 {
            return Err(MixError::TooSmall);
        }
        let flagged = u16::from_le_bytes([data[0], data[1]]) == 0;
        if flagged {
            Self::parse_flagged(data)
        } else {
            Self::parse_classic(data)
        }
    }

    fn parse_classic(data: &'a [u8]) -> Result<Self, MixError> {
        let count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let data_size = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
        if count == 0 || count > 0x0FFF {
            return Err(MixError::BadHeader);
        }
        let index_len = count * ENTRY_LEN;
        let body_start = 6 + index_len;
        let expected = body_start + data_size;
        if data.len() != expected {
            return Err(MixError::SizeMismatch {
                expected,
                actual: data.len(),
            });
        }
        let entries = Self::read_index(data, 6, count, body_start)?;
        Ok(MixArchive {
            data,
            version: MixVersion::Classic,
            encrypted: false,
            checksummed: false,
            body_start,
            data_size,
            entries,
        })
    }

    fn parse_flagged(data: &'a [u8]) -> Result<Self, MixError> {
        let flags = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if flags & !(FLAG_CHECKSUM | FLAG_ENCRYPTED) != 0 {
            return Err(MixError::BadHeader);
        }
        let encrypted = flags & FLAG_ENCRYPTED != 0;
        let checksummed = flags & FLAG_CHECKSUM != 0;

        let (count, data_size, header_index, body_start) = if encrypted {
            if data.len() < 4 + KEY_SOURCE_LEN + 8 {
                return Err(MixError::TooSmall);
            }
            let key = derive_blowfish_key(&data[4..4 + KEY_SOURCE_LEN])?;
            let enc_start = 4 + KEY_SOURCE_LEN;
            // First block gives the count; then decrypt the whole header+index.
            let cipher: Blowfish = Blowfish::new_from_slice(&key).map_err(|_| MixError::BadKey)?;
            let mut first = [0u8; 8];
            first.copy_from_slice(&data[enc_start..enc_start + 8]);
            cipher.decrypt_block((&mut first).into());
            let count = u16::from_le_bytes([first[0], first[1]]) as usize;
            if count == 0 || count > 0x0FFF {
                return Err(MixError::BadHeader);
            }
            let plain_len = 6 + count * ENTRY_LEN;
            let enc_len = (plain_len + 7) & !7;
            if data.len() < enc_start + enc_len {
                return Err(MixError::TooSmall);
            }
            let mut plain = vec![0u8; enc_len];
            plain[..8].copy_from_slice(&first);
            for block in 1..enc_len / 8 {
                let src = enc_start + block * 8;
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[src..src + 8]);
                cipher.decrypt_block((&mut b).into());
                plain[block * 8..block * 8 + 8].copy_from_slice(&b);
            }
            let data_size = u32::from_le_bytes([plain[2], plain[3], plain[4], plain[5]]) as usize;
            let body_start = enc_start + enc_len;
            (count, data_size, plain, body_start)
        } else {
            let off = 4;
            let count = u16::from_le_bytes([data[off], data[off + 1]]) as usize;
            let data_size =
                u32::from_le_bytes([data[off + 2], data[off + 3], data[off + 4], data[off + 5]])
                    as usize;
            if count == 0 || count > 0x0FFF {
                return Err(MixError::BadHeader);
            }
            let body_start = 4 + 6 + count * ENTRY_LEN;
            (count, data_size, Vec::new(), body_start)
        };

        let entries = if encrypted {
            Self::read_index(&header_index, 6, count, body_start)?
        } else {
            Self::read_index(data, 4 + 6, count, body_start)?
        };

        let checksum_len = if checksummed { 20 } else { 0 };
        let expected = body_start + data_size + checksum_len;
        if data.len() != expected {
            return Err(MixError::SizeMismatch {
                expected,
                actual: data.len(),
            });
        }

        Ok(MixArchive {
            data,
            version: MixVersion::Flagged,
            encrypted,
            checksummed,
            body_start,
            data_size,
            entries,
        })
    }

    fn read_index(
        src: &[u8],
        index_off: usize,
        count: usize,
        body_start: usize,
    ) -> Result<Vec<MixEntry>, MixError> {
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let o = index_off + i * ENTRY_LEN;
            if o + ENTRY_LEN > src.len() {
                return Err(MixError::BadIndex);
            }
            let id = u32::from_le_bytes([src[o], src[o + 1], src[o + 2], src[o + 3]]);
            let mut offset =
                u32::from_le_bytes([src[o + 4], src[o + 5], src[o + 6], src[o + 7]]) as usize;
            let size = u32::from_le_bytes([src[o + 8], src[o + 9], src[o + 10], src[o + 11]]);
            offset += body_start;
            entries.push(MixEntry {
                id,
                offset: offset as u32,
                size,
            });
        }
        Ok(entries)
    }

    pub fn version(&self) -> MixVersion {
        self.version
    }

    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    pub fn has_checksum(&self) -> bool {
        self.checksummed
    }

    pub fn entries(&self) -> &[MixEntry] {
        &self.entries
    }

    pub fn body_range(&self) -> std::ops::Range<usize> {
        self.body_start..self.body_start + self.data_size
    }

    /// SHA-1 trailer (if present) as raw bytes.
    pub fn checksum_trailer(&self) -> Option<&[u8]> {
        if self.checksummed {
            self.data.get(self.data.len() - 20..)
        } else {
            None
        }
    }

    /// SHA-1 of the body only, used to discover trailer semantics.
    pub fn body_sha1(&self) -> [u8; 20] {
        sha1(&self.data[self.body_range()])
    }

    /// SHA-1 of everything before the trailer.
    pub fn pre_trailer_sha1(&self) -> [u8; 20] {
        let end = self.data.len() - if self.checksummed { 20 } else { 0 };
        sha1(&self.data[..end])
    }

    pub fn get_by_id(&self, id: u32) -> Option<&'a [u8]> {
        let entry = self.entries.iter().find(|e| e.id == id)?;
        self.data
            .get(entry.offset as usize..entry.offset as usize + entry.size as usize)
    }

    /// Look up a file by name using the RA2/YR hash (or TD hash for classic).
    /// Look up by filename. RA2/YR hash every archive with the CRC32 scheme,
    /// including classic-header ones (retail `AUDIO.MIX` has a classic header
    /// and CRC IDs), so CRC is tried first; classic archives fall back to the
    /// TD/RA1 rotate-add hash.
    pub fn get(&self, name: &str) -> Option<&'a [u8]> {
        self.resolve(name).and_then(|id| self.get_by_id(id))
    }

    /// The entry ID `name` resolves to in this archive, if present.
    pub fn resolve(&self, name: &str) -> Option<u32> {
        let has = |id: u32| self.entries.iter().any(|e| e.id == id);
        let crc = mix_id(name);
        if has(crc) {
            return Some(crc);
        }
        let td = mix_id_td(name);
        (self.version == MixVersion::Classic && has(td)).then_some(td)
    }
}

/// Derive the 56-byte Blowfish key from an 80-byte key source (XCC
/// `get_blowfish_key`): RSA-style modular exponentiation with the Westwood
/// public key, chunks little-endian, low bytes concatenated.
fn derive_blowfish_key(key_source: &[u8]) -> Result<[u8; BLOWFISH_KEY_LEN], MixError> {
    if key_source.len() != KEY_SOURCE_LEN {
        return Err(MixError::BadKey);
    }
    let n = BigUint::from_bytes_be(&WESTWOOD_MODULUS);
    let e = BigUint::from(WESTWOOD_EXPONENT);
    let a = ((n.bits() - 1) / 8) as usize; // 39 output bytes per chunk
    let chunk_in = a + 1; // 40 input bytes per chunk

    let mut out = Vec::with_capacity(a * 2);
    for chunk in key_source.chunks(chunk_in) {
        let x = BigUint::from_bytes_le(chunk);
        let y = x.modpow(&e, &n);
        let mut yb = y.to_bytes_le();
        yb.resize(a, 0);
        out.extend_from_slice(&yb[..a]);
    }
    if out.len() < BLOWFISH_KEY_LEN {
        return Err(MixError::BadKey);
    }
    let mut key = [0u8; BLOWFISH_KEY_LEN];
    key.copy_from_slice(&out[..BLOWFISH_KEY_LEN]);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smallest possible classic MIX: one file of 3 zero bytes.
    fn build_classic() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&1u16.to_le_bytes()); // count
        v.extend_from_slice(&3u32.to_le_bytes()); // data size
        v.extend_from_slice(&0x1234_5678u32.to_le_bytes()); // id
        v.extend_from_slice(&0u32.to_le_bytes()); // offset
        v.extend_from_slice(&3u32.to_le_bytes()); // size
        v.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // body
        v
    }

    #[test]
    fn classic_round_trip() {
        let data = build_classic();
        let mix = MixArchive::parse(&data).unwrap();
        assert_eq!(mix.entries().len(), 1);
        assert_eq!(mix.get_by_id(0x1234_5678), Some(&[0xAA, 0xBB, 0xCC][..]));
        assert!(mix.get_by_id(0).is_none());
    }

    #[test]
    fn classic_rejects_wrong_size() {
        let mut data = build_classic();
        data.push(0);
        assert!(matches!(
            MixArchive::parse(&data),
            Err(MixError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn flagged_plain_round_trip() {
        let mut v = Vec::new();
        v.extend_from_slice(&FLAG_CHECKSUM.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&3u32.to_le_bytes());
        v.extend_from_slice(&mix_id("TEST.BIN").to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&3u32.to_le_bytes());
        v.extend_from_slice(&[1, 2, 3]);
        v.extend_from_slice(&[0; 20]); // trailer
        let mix = MixArchive::parse(&v).unwrap();
        assert!(mix.has_checksum());
        assert!(!mix.is_encrypted());
        assert_eq!(mix.get("test.bin"), Some(&[1, 2, 3][..]));
    }
}
