//! Westwood audio: AUD files, the AUDIO.BAG/AUDIO.IDX sound bank, and IMA
//! ADPCM WAVs, all decoded to signed 16-bit PCM.
//!
//! Spec with evidence: `docs/formats/audio.md`. Retail uses IMA ADPCM
//! everywhere, in two packings:
//! - Westwood AUD (codec 99): one continuous nibble stream split into
//!   `0xDEAF` chunks; decoder state carries across chunks.
//! - Microsoft IMA (WAV format 0x11, and bag entries with the IMA flag):
//!   fixed-size blocks, each restarting from a 4-byte-per-channel header.

use crate::bytes::Cursor;

const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408,
    449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
    2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630,
    9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];
const INDEX_ADJUST: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

const AUD_CHUNK_MAGIC: u32 = 0x0000_DEAF;
const AUD_CODEC_IMA: u8 = 99;
const AUD_FLAG_STEREO: u8 = 1;
const AUD_FLAG_16BIT: u8 = 2;

const BAG_FLAG_STEREO: u32 = 1;
const BAG_FLAG_PCM: u32 = 2;
const BAG_FLAG_16BIT: u32 = 4;
const BAG_FLAG_IMA: u32 = 8;

#[derive(Debug, PartialEq, Eq)]
pub enum AudioError {
    BadHeader,
    Truncated,
    Unsupported(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::BadHeader => write!(f, "invalid audio header"),
            AudioError::Truncated => write!(f, "truncated audio data"),
            AudioError::Unsupported(what) => write!(f, "unsupported audio: {what}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// Decoded audio: interleaved signed 16-bit samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

/// One channel's IMA ADPCM predictor.
#[derive(Debug, Clone, Copy, Default)]
struct Ima {
    predictor: i32,
    index: i32,
}

impl Ima {
    /// Reference IMA expansion: the difference is summed from step
    /// fractions bit by bit (`step>>3 + step + step>>1 + step>>2`).
    fn expand(&mut self, nibble: u8) -> i16 {
        let step = STEPS[self.index as usize];
        let mut diff = step >> 3;
        if nibble & 4 != 0 {
            diff += step;
        }
        if nibble & 2 != 0 {
            diff += step >> 1;
        }
        if nibble & 1 != 0 {
            diff += step >> 2;
        }
        self.finish(nibble, diff)
    }

    /// Multiplicative expansion `((2*delta + 1) * step) >> 3`, which rounds
    /// differently from [`Ima::expand`]. See `docs/formats/audio.md` for
    /// which packing uses which.
    fn expand_mul(&mut self, nibble: u8) -> i16 {
        let step = STEPS[self.index as usize];
        let diff = ((2 * i32::from(nibble & 7) + 1) * step) >> 3;
        self.finish(nibble, diff)
    }

    fn finish(&mut self, nibble: u8, diff: i32) -> i16 {
        self.predictor = if nibble & 8 != 0 {
            self.predictor - diff
        } else {
            self.predictor + diff
        }
        .clamp(i16::MIN.into(), i16::MAX.into());
        self.index = (self.index + INDEX_ADJUST[nibble as usize]).clamp(0, 88);
        self.predictor as i16
    }
}

/// Westwood `.aud` file header + chunked IMA stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudFile {
    pub sample_rate: u16,
    pub flags: u8,
    pub codec: u8,
    /// Decoded size in bytes as stored in the header.
    pub output_size: u32,
}

impl AudFile {
    pub fn parse_header(data: &[u8]) -> Result<Self, AudioError> {
        let mut c = Cursor::new(data);
        let sample_rate = c.u16().ok_or(AudioError::BadHeader)?;
        let size = c.u32().ok_or(AudioError::BadHeader)? as usize;
        let output_size = c.u32().ok_or(AudioError::BadHeader)?;
        let flags = c.u8().ok_or(AudioError::BadHeader)?;
        let codec = c.u8().ok_or(AudioError::BadHeader)?;
        if sample_rate == 0 || size + 12 != data.len() {
            return Err(AudioError::BadHeader);
        }
        Ok(AudFile {
            sample_rate,
            flags,
            codec,
            output_size,
        })
    }

    pub fn decode(data: &[u8]) -> Result<Pcm, AudioError> {
        let hdr = Self::parse_header(data)?;
        if hdr.codec != AUD_CODEC_IMA || hdr.flags & AUD_FLAG_STEREO != 0 {
            return Err(AudioError::Unsupported(format!(
                "AUD codec {} flags {}",
                hdr.codec, hdr.flags
            )));
        }
        if hdr.flags & AUD_FLAG_16BIT == 0 {
            return Err(AudioError::Unsupported("8-bit IMA AUD".into()));
        }
        let mut ima = Ima::default();
        // Never trust the header for allocation: each input byte yields at
        // most two samples.
        let mut samples = Vec::with_capacity((hdr.output_size as usize / 2).min((data.len() - 12) * 2));
        let mut c = Cursor::at(data, 12);
        while c.pos() < data.len() {
            let in_size = c.u16().ok_or(AudioError::Truncated)? as usize;
            let out_size = c.u16().ok_or(AudioError::Truncated)? as usize;
            if c.u32() != Some(AUD_CHUNK_MAGIC) || out_size > in_size * 4 + 2 {
                return Err(AudioError::BadHeader);
            }
            // A chunk yields out_size / 2 samples, capped by its nibbles.
            // Retail final chunks may claim one sample more than they hold
            // (out = in * 4 + 2; ffmpeg rejects those files), so decoding
            // stops when the nibbles run out.
            let want = samples.len() + out_size / 2;
            for &b in c.take(in_size).ok_or(AudioError::Truncated)? {
                for nibble in [b & 0x0F, b >> 4] {
                    if samples.len() < want {
                        samples.push(ima.expand_mul(nibble));
                    }
                }
            }
        }
        Ok(Pcm {
            sample_rate: hdr.sample_rate.into(),
            channels: 1,
            samples,
        })
    }
}

/// Decode Microsoft IMA ADPCM blocks (WAV format 0x11). Each block holds,
/// per channel, `i16 predictor, u8 index, u8 reserved` (the predictor is the
/// first output sample), then 4-byte groups alternating between channels,
/// each group 8 nibbles low-first.
pub fn decode_ms_ima(data: &[u8], channels: u16, block_align: usize) -> Result<Vec<i16>, AudioError> {
    let ch = channels as usize;
    if ch == 0 || ch > 2 || block_align <= 4 * ch || !(block_align - 4 * ch).is_multiple_of(4 * ch) {
        return Err(AudioError::Unsupported(format!(
            "IMA block {block_align} x {channels} channels"
        )));
    }
    // Two samples per input byte plus one header sample per channel bounds
    // the output regardless of the (untrusted) block size.
    let mut out = Vec::with_capacity(data.len() * 2 + 2 * ch);
    for block in data.chunks(block_align) {
        if block.len() < 4 * ch {
            break;
        }
        let mut state = [Ima::default(); 2];
        let mut chans: [Vec<i16>; 2] = [Vec::new(), Vec::new()];
        for (c, st) in state.iter_mut().enumerate().take(ch) {
            let h = &block[4 * c..4 * c + 4];
            st.predictor = i32::from(i16::from_le_bytes([h[0], h[1]]));
            st.index = i32::from(h[2]).min(88);
            chans[c].push(st.predictor as i16);
        }
        for (g, group) in block[4 * ch..].as_chunks::<4>().0.iter().enumerate() {
            let c = g % ch;
            for &b in group {
                chans[c].push(state[c].expand(b & 0x0F));
                chans[c].push(state[c].expand(b >> 4));
            }
        }
        let n = chans[0].len();
        for i in 0..n {
            for chan in chans.iter().take(ch) {
                out.push(chan.get(i).copied().unwrap_or(0));
            }
        }
    }
    Ok(out)
}

/// Minimal RIFF/WAVE reader for PCM and IMA ADPCM retail WAVs.
pub fn decode_wav(data: &[u8]) -> Result<Pcm, AudioError> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(AudioError::BadHeader);
    }
    let mut c = Cursor::at(data, 12);
    let (mut fmt, mut body, mut frames) = (None, None, None);
    while let (Some(id), Some(len)) = (c.array::<4>(), c.u32()) {
        let chunk = c.take(len as usize).ok_or(AudioError::Truncated)?;
        match &id {
            b"fmt " => fmt = Some(chunk),
            b"data" => body = Some(chunk),
            b"fact" if chunk.len() >= 4 => {
                frames = Some(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as usize)
            }
            _ => {}
        }
        if len % 2 == 1 {
            c.skip(1);
        }
    }
    let (fmt, body) = (fmt.ok_or(AudioError::BadHeader)?, body.ok_or(AudioError::BadHeader)?);
    let mut f = Cursor::new(fmt);
    let tag = f.u16().ok_or(AudioError::BadHeader)?;
    let channels = f.u16().ok_or(AudioError::BadHeader)?;
    let sample_rate = f.u32().ok_or(AudioError::BadHeader)?;
    f.skip(4).ok_or(AudioError::BadHeader)?; // byte rate
    let block_align = f.u16().ok_or(AudioError::BadHeader)? as usize;
    let bits = f.u16().ok_or(AudioError::BadHeader)?;
    let mut samples = match (tag, bits) {
        (1, 16) => body
            .as_chunks::<2>().0.iter()
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect(),
        (1, 8) => body.iter().map(|&b| (i16::from(b) - 128) << 8).collect(),
        (0x11, 4) => decode_ms_ima(body, channels, block_align)?,
        _ => return Err(AudioError::Unsupported(format!("WAV tag {tag:#x}, {bits} bits"))),
    };
    // The fact chunk gives the true frame count; the last block is padded.
    if let Some(n) = frames {
        samples.truncate(n * channels as usize);
    }
    Ok(Pcm {
        sample_rate,
        channels,
        samples,
    })
}

/// One sound in `AUDIO.BAG`, described by `AUDIO.IDX`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BagEntry {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub sample_rate: u32,
    /// 1 stereo, 2 PCM, 4 16-bit, 8 IMA ADPCM.
    pub flags: u32,
    /// IMA block size (per channel set) for IMA entries.
    pub chunk_size: u32,
}

impl BagEntry {
    pub fn channels(&self) -> u16 {
        if self.flags & BAG_FLAG_STEREO != 0 { 2 } else { 1 }
    }

    /// Decode this entry from the full `AUDIO.BAG` bytes.
    pub fn decode(&self, bag: &[u8]) -> Result<Pcm, AudioError> {
        let start = self.offset as usize;
        let data = bag
            .get(start..start + self.size as usize)
            .ok_or(AudioError::Truncated)?;
        let channels = self.channels();
        let samples = if self.flags & BAG_FLAG_IMA != 0 {
            decode_ms_ima(data, channels, self.chunk_size as usize)?
        } else if self.flags & BAG_FLAG_PCM != 0 && self.flags & BAG_FLAG_16BIT != 0 {
            data.as_chunks::<2>().0.iter()
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect()
        } else if self.flags & BAG_FLAG_PCM != 0 {
            data.iter().map(|&b| (i16::from(b) - 128) << 8).collect()
        } else {
            return Err(AudioError::Unsupported(format!("bag flags {:#x}", self.flags)));
        };
        Ok(Pcm {
            sample_rate: self.sample_rate,
            channels,
            samples,
        })
    }
}

/// Parse `AUDIO.IDX`: `"GABA"`, u32 version (2), u32 count, then 36-byte
/// entries `name[16], offset, size, sample_rate, flags, chunk_size`.
pub fn parse_bag_index(data: &[u8]) -> Result<Vec<BagEntry>, AudioError> {
    let mut c = Cursor::new(data);
    if c.take(4) != Some(b"GABA".as_slice()) {
        return Err(AudioError::BadHeader);
    }
    let version = c.u32().ok_or(AudioError::BadHeader)?;
    let count = c.u32().ok_or(AudioError::BadHeader)? as usize;
    if version != 2 || data.len() != 12 + 36 * count {
        return Err(AudioError::BadHeader);
    }
    (0..count)
        .map(|_| {
            let name = crate::bytes::fixed_name(c.take(16).ok_or(AudioError::Truncated)?);
            let mut field = || c.u32().ok_or(AudioError::Truncated);
            Ok(BagEntry {
                name,
                offset: field()?,
                size: field()?,
                sample_rate: field()?,
                flags: field()?,
                chunk_size: field()?,
            })
        })
        .collect()
}

/// Encode PCM as a canonical 16-bit WAV.
pub fn encode_wav(pcm: &Pcm) -> Vec<u8> {
    let data_len = (pcm.samples.len() * 2) as u32;
    let block = pcm.channels * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend(b"RIFF");
    out.extend((36 + data_len).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(pcm.channels.to_le_bytes());
    out.extend(pcm.sample_rate.to_le_bytes());
    out.extend((pcm.sample_rate * u32::from(block)).to_le_bytes());
    out.extend(block.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(data_len.to_le_bytes());
    for s in &pcm.samples {
        out.extend(s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ima_known_steps() {
        let mut ima = Ima::default();
        // step 7: nibble 7 -> diff = 0 + 7 + 3 + 1 = 11, index -> 8
        assert_eq!(ima.expand(7), 11);
        assert_eq!(ima.index, 8);
        // step 16: nibble 8 (sign, no magnitude) -> diff = 2
        assert_eq!(ima.expand(8), 9);
        assert_eq!(ima.index, 7);
    }

    #[test]
    fn expansion_forms_differ_only_in_rounding() {
        let (mut a, mut b) = (Ima::default(), Ima::default());
        a.index = 10; // step 19
        b.index = 10;
        // bitwise: 2 + 19 + 9 + 4 = 34; multiplicative: (15 * 19) >> 3 = 35
        assert_eq!((a.expand(7), b.expand_mul(7)), (34, 35));
    }

    #[test]
    fn ms_ima_block_layout() {
        // Mono block of 8 bytes: header (predictor 100, index 0) + 4 data bytes.
        let block = [100, 0, 0, 0, 0x10, 0, 0, 0];
        let s = decode_ms_ima(&block, 1, 8).unwrap();
        assert_eq!(s.len(), 9);
        assert_eq!(s[0], 100);
        assert_eq!(s[1], 100); // nibble 0: +step>>3 = +0 at step 7
        assert_eq!(s[2], 101); // nibble 1: +0 + 7>>2 = +1
    }

    #[test]
    fn aud_chunks_carry_state() {
        let mut d = Vec::new();
        d.extend(22050u16.to_le_bytes());
        d.extend(16u32.to_le_bytes()); // 2 chunks of (8 header + 0) .. set below
        d.extend(8u32.to_le_bytes()); // output bytes = 4 samples
        d.extend([AUD_FLAG_16BIT, AUD_CODEC_IMA]);
        for b in [0x77u8, 0x07] {
            d.extend(1u16.to_le_bytes());
            d.extend(4u16.to_le_bytes());
            d.extend(AUD_CHUNK_MAGIC.to_le_bytes());
            d.push(b);
        }
        let size = (d.len() - 12) as u32;
        d[2..6].copy_from_slice(&size.to_le_bytes());
        let pcm = AudFile::decode(&d).unwrap();
        assert_eq!(pcm.samples.len(), 4);
        // State carries across the chunk boundary: the 3rd sample continues
        // from the 2nd, not from zero.
        assert!(pcm.samples[2] > pcm.samples[1]);
    }

    #[test]
    fn hostile_sizes_do_not_overallocate() {
        // AUD claiming a 4 GiB output with no chunks.
        let mut d = Vec::new();
        d.extend(22050u16.to_le_bytes());
        d.extend(0u32.to_le_bytes());
        d.extend(u32::MAX.to_le_bytes());
        d.extend([AUD_FLAG_16BIT, AUD_CODEC_IMA]);
        assert!(AudFile::decode(&d).unwrap().samples.is_empty());
        // Huge IMA block size over a tiny buffer.
        // A partial block decodes what it holds: 1 header sample + 8 nibbles.
        assert_eq!(decode_ms_ima(&[0; 8], 1, 65532).unwrap().len(), 9);
    }

    #[test]
    fn wav_roundtrip_pcm() {
        let pcm = Pcm {
            sample_rate: 22050,
            channels: 1,
            samples: vec![0, 1000, -1000, i16::MAX],
        };
        assert_eq!(decode_wav(&encode_wav(&pcm)).unwrap(), pcm);
    }

    #[test]
    fn bag_index_parse() {
        let mut d = b"GABA".to_vec();
        d.extend(2u32.to_le_bytes());
        d.extend(1u32.to_le_bytes());
        let mut name = [0u8; 16];
        name[..8].copy_from_slice(b"ichratta");
        d.extend(name);
        for v in [0u32, 71148, 22050, 12, 512] {
            d.extend(v.to_le_bytes());
        }
        let idx = parse_bag_index(&d).unwrap();
        assert_eq!(idx[0].name, "ichratta");
        assert_eq!((idx[0].flags, idx[0].chunk_size, idx[0].channels()), (12, 512, 1));
    }
}
