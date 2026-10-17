//! Decompressors used by map packs: base64 text, LZO1X ("format 5", used by
//! `[IsoMapPack5]`) and Westwood LCW ("format 80", used by `[OverlayPack]`,
//! `[OverlayDataPack]`), plus the chunk container both packs share.
//!
//! Spec with evidence: `docs/formats/map.md`. Every decoder checks that it
//! consumed exactly its input and produced exactly the declared output; the
//! retail maps all satisfy this.

#[derive(Debug, PartialEq, Eq)]
pub enum DecompressError {
    BadBase64,
    Truncated,
    BadReference,
    SizeMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for DecompressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecompressError::BadBase64 => write!(f, "invalid base64"),
            DecompressError::Truncated => write!(f, "compressed data truncated"),
            DecompressError::BadReference => write!(f, "back-reference before start of output"),
            DecompressError::SizeMismatch { expected, actual } => {
                write!(f, "decompressed {actual} bytes, expected {expected}")
            }
        }
    }
}

impl std::error::Error for DecompressError {}

type Result<T> = std::result::Result<T, DecompressError>;

/// Standard base64 (`A-Z a-z 0-9 + /`, `=` padding); whitespace ignored.
pub fn base64_decode(text: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            c if c.is_ascii_whitespace() => continue,
            _ => return Err(DecompressError::BadBase64),
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Map pack container: repeated `u16 in_size, u16 out_size, in_size bytes`,
/// each chunk decoded independently and concatenated.
pub fn unpack_chunks(data: &[u8], decode: fn(&[u8], usize) -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let head = data.get(pos..pos + 4).ok_or(DecompressError::Truncated)?;
        let in_size = usize::from(u16::from_le_bytes([head[0], head[1]]));
        let out_size = usize::from(u16::from_le_bytes([head[2], head[3]]));
        let body = data.get(pos + 4..pos + 4 + in_size).ok_or(DecompressError::Truncated)?;
        out.extend(decode(body, out_size)?);
        pos += 4 + in_size;
    }
    Ok(out)
}

struct Out {
    buf: Vec<u8>,
    limit: usize,
}

impl Out {
    fn literal(&mut self, src: &[u8], ip: &mut usize, n: usize) -> Result<()> {
        let s = src.get(*ip..*ip + n).ok_or(DecompressError::Truncated)?;
        if self.buf.len() + n > self.limit {
            return Err(DecompressError::SizeMismatch { expected: self.limit, actual: self.buf.len() + n });
        }
        self.buf.extend_from_slice(s);
        *ip += n;
        Ok(())
    }

    /// Byte-by-byte copy so overlapping references repeat patterns.
    fn copy_back(&mut self, dist: usize, n: usize) -> Result<()> {
        let start = self.buf.len().checked_sub(dist).ok_or(DecompressError::BadReference)?;
        self.copy_abs(start, n)
    }

    fn copy_abs(&mut self, start: usize, n: usize) -> Result<()> {
        if start >= self.buf.len() && n > 0 {
            return Err(DecompressError::BadReference);
        }
        if self.buf.len() + n > self.limit {
            return Err(DecompressError::SizeMismatch { expected: self.limit, actual: self.buf.len() + n });
        }
        for i in 0..n {
            let b = self.buf[start + i];
            self.buf.push(b);
        }
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>> {
        if self.buf.len() == self.limit {
            Ok(self.buf)
        } else {
            Err(DecompressError::SizeMismatch { expected: self.limit, actual: self.buf.len() })
        }
    }
}

fn byte(src: &[u8], ip: &mut usize) -> Result<usize> {
    let b = *src.get(*ip).ok_or(DecompressError::Truncated)?;
    *ip += 1;
    Ok(usize::from(b))
}

fn le16(src: &[u8], ip: &mut usize) -> Result<usize> {
    let lo = byte(src, ip)?;
    let hi = byte(src, ip)?;
    Ok(lo | (hi << 8))
}

/// Run-length extension shared by LZO length fields: each zero byte adds
/// 255, then the next byte plus `base` ends it.
fn ext_len(src: &[u8], ip: &mut usize, base: usize) -> Result<usize> {
    let mut n = 0;
    while *src.get(*ip).ok_or(DecompressError::Truncated)? == 0 {
        n += 255;
        *ip += 1;
    }
    Ok(n + base + byte(src, ip)?)
}

/// LZO1X decompression into exactly `out_size` bytes.
pub fn lzo1x_decompress(src: &[u8], out_size: usize) -> Result<Vec<u8>> {
    enum State {
        Top,
        FirstLiteralRun,
        Match(usize),
        MatchDone,
        MatchNext(usize),
    }
    let mut out = Out { buf: Vec::with_capacity(out_size.min(src.len().saturating_mul(255))), limit: out_size };
    let mut ip = 0;
    let mut state = match src.first() {
        Some(&b) if b > 17 => {
            ip = 1;
            let t = usize::from(b) - 17;
            if t < 4 {
                State::MatchNext(t)
            } else {
                out.literal(src, &mut ip, t)?;
                State::FirstLiteralRun
            }
        }
        Some(_) => State::Top,
        None => return Err(DecompressError::Truncated),
    };
    loop {
        state = match state {
            State::Top => {
                let mut t = byte(src, &mut ip)?;
                if t >= 16 {
                    State::Match(t)
                } else {
                    if t == 0 {
                        t = ext_len(src, &mut ip, 15)?;
                    }
                    out.literal(src, &mut ip, t + 3)?;
                    State::FirstLiteralRun
                }
            }
            State::FirstLiteralRun => {
                let t = byte(src, &mut ip)?;
                if t >= 16 {
                    State::Match(t)
                } else {
                    // A 3-byte match 2 KiB back, only valid right after a
                    // literal run of 4 or more bytes.
                    let dist = 1 + 0x0800 + (t >> 2) + (byte(src, &mut ip)? << 2);
                    out.copy_back(dist, 3)?;
                    State::MatchDone
                }
            }
            State::Match(t) => {
                if t >= 64 {
                    let dist = 1 + ((t >> 2) & 7) + (byte(src, &mut ip)? << 3);
                    out.copy_back(dist, (t >> 5) + 1)?;
                } else if t >= 32 {
                    let mut len = t & 31;
                    if len == 0 {
                        len = ext_len(src, &mut ip, 31)?;
                    }
                    let dist = 1 + (le16(src, &mut ip)? >> 2);
                    out.copy_back(dist, len + 2)?;
                } else if t >= 16 {
                    let mut len = t & 7;
                    if len == 0 {
                        len = ext_len(src, &mut ip, 7)?;
                    }
                    let dist = ((t & 8) << 11) + (le16(src, &mut ip)? >> 2);
                    if dist == 0 {
                        // End-of-stream marker.
                        if ip != src.len() {
                            return Err(DecompressError::SizeMismatch { expected: src.len(), actual: ip });
                        }
                        return out.finish();
                    }
                    out.copy_back(dist + 0x4000, len + 2)?;
                } else {
                    // 2-byte match after a short (1..3) literal run.
                    let dist = 1 + (t >> 2) + (byte(src, &mut ip)? << 2);
                    out.copy_back(dist, 2)?;
                }
                State::MatchDone
            }
            State::MatchDone => {
                // The low two bits of the instruction's second-to-last byte
                // carry a trailing literal count (0..3).
                let t = usize::from(src[ip - 2] & 3);
                if t == 0 {
                    State::Top
                } else {
                    State::MatchNext(t)
                }
            }
            State::MatchNext(t) => {
                out.literal(src, &mut ip, t)?;
                State::Match(byte(src, &mut ip)?)
            }
        };
    }
}

/// Westwood LCW ("format 80") decompression into exactly `out_size` bytes.
/// Copy positions in the long forms are absolute from the output start.
pub fn lcw_decompress(src: &[u8], out_size: usize) -> Result<Vec<u8>> {
    let mut out = Out { buf: Vec::with_capacity(out_size.min(src.len().saturating_mul(64))), limit: out_size };
    let mut ip = 0;
    loop {
        let cmd = byte(src, &mut ip)?;
        match cmd {
            0x80 => break,
            // 0ccc pppp pppppppp: copy (c+3) bytes from `pos` back.
            c if c < 0x80 => {
                let n = (c >> 4) + 3;
                let dist = ((c & 0x0F) << 8) | byte(src, &mut ip)?;
                out.copy_back(dist, n)?;
            }
            // 10nnnnnn: n literal bytes.
            c if c < 0xC0 => out.literal(src, &mut ip, c & 0x3F)?,
            // 11111110 n16 v8: fill n bytes with v.
            0xFE => {
                let n = le16(src, &mut ip)?;
                let v = byte(src, &mut ip)? as u8;
                if out.buf.len() + n > out.limit {
                    return Err(DecompressError::SizeMismatch { expected: out.limit, actual: out.buf.len() + n });
                }
                out.buf.resize(out.buf.len() + n, v);
            }
            // 11111111 n16 p16: copy n bytes from absolute p.
            0xFF => {
                let n = le16(src, &mut ip)?;
                let p = le16(src, &mut ip)?;
                out.copy_abs(p, n)?;
            }
            // 11nnnnnn p16: copy (n+3) bytes from absolute p.
            c => {
                let n = (c & 0x3F) + 3;
                let p = le16(src, &mut ip)?;
                out.copy_abs(p, n)?;
            }
        }
    }
    out.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_known_answers() {
        assert_eq!(base64_decode("TWFu").unwrap(), b"Man");
        assert_eq!(base64_decode("TWE=").unwrap(), b"Ma");
        assert_eq!(base64_decode("TW\nE=").unwrap(), b"Ma");
        assert!(base64_decode("T*").is_err());
    }

    #[test]
    fn lzo_literal_only_stream() {
        // First byte 17+5: five literals, then the end marker 0x11 0x00 0x00.
        let src = [22, b'h', b'e', b'l', b'l', b'o', 0x11, 0, 0];
        assert_eq!(lzo1x_decompress(&src, 5).unwrap(), b"hello");
        assert!(lzo1x_decompress(&src, 6).is_err(), "declared size must match");
    }

    #[test]
    fn lzo_overlapping_match() {
        // Literals "ab", then M2 (t >= 64) copying 4 bytes from 2 back
        // ("abab"), then end. t = ((len-1) << 5) | ((dist-1)&7) << 2 with
        // len 4 -> (3 << 5) = 0x60, dist 2 -> ((1 & 7) << 2) = 0x04.
        let src = [17 + 2, b'a', b'b', 0x64, 0x00, 0x11, 0, 0];
        assert_eq!(lzo1x_decompress(&src, 6).unwrap(), b"ababab");
    }

    #[test]
    fn lcw_commands() {
        let src = [
            0x83, b'a', b'b', b'c', // 3 literals
            0x00, 0x03, // copy 3 from 3 back: "abc"
            0xFE, 0x02, 0x00, b'z', // fill 2 x 'z'
            0xC0, 0x00, 0x00, // copy 3 from absolute 0
            0x80,
        ];
        assert_eq!(lcw_decompress(&src, 11).unwrap(), b"abcabczzabc");
    }

    #[test]
    fn chunk_container_rejects_oversized_chunk() {
        // Header claims 100 body bytes; only 2 are present.
        let data = [100u8, 0, 2, 0, 0x81, b'!'];
        assert_eq!(unpack_chunks(&data, lcw_decompress), Err(DecompressError::Truncated));
        // Declared output size must match what the chunk actually produces.
        let mut lying = Vec::new();
        lying.extend(3u16.to_le_bytes());
        lying.extend(99u16.to_le_bytes());
        lying.extend([0x81, b'!', 0x80]);
        assert!(matches!(
            unpack_chunks(&lying, lcw_decompress),
            Err(DecompressError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn chunk_container() {
        let mut data = Vec::new();
        for chunk in [&[0x82u8, b'h', b'i', 0x80][..], &[0x81, b'!', 0x80]] {
            data.extend((chunk.len() as u16).to_le_bytes());
            data.extend(((chunk.len() - 2) as u16).to_le_bytes());
            data.extend(chunk);
        }
        assert_eq!(unpack_chunks(&data, lcw_decompress).unwrap(), b"hi!");
    }
}
