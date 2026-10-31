//! Bink 1 container metadata and packet bounds. No codec or I/O here.
//! Observations and references: docs/formats/bink.md.
use crate::bytes::Cursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinkError {
    Header,
    Size,
    Dimensions,
    Rate,
    Index,
    Audio,
}
impl std::fmt::Display for BinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid Bink {:?}", self)
    }
}
impl std::error::Error for BinkError {}

#[derive(Debug, Clone, Copy)]
pub struct BinkTrack {
    pub sample_rate: u16,
    pub flags: u16,
    pub id: u32,
}
#[derive(Debug, Clone, Copy)]
pub struct BinkFrame {
    pub offset: usize,
    pub size: usize,
    pub keyframe: bool,
}
#[derive(Debug)]
pub struct BinkFile<'a> {
    pub revision: u8,
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub flags: u32,
    pub tracks: Vec<BinkTrack>,
    pub frames: Vec<BinkFrame>,
    data: &'a [u8],
}
impl<'a> BinkFile<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, BinkError> {
        use BinkError::*;
        let mut c = Cursor::new(data);
        if c.take(3) != Some(b"BIK".as_slice()) {
            return Err(Header);
        }
        let revision = c.u8().ok_or(Header)?;
        if !matches!(revision, b'f' | b'g' | b'h' | b'i') {
            return Err(Header);
        }
        let size = c.u32().ok_or(Header)? as usize;
        if size.checked_add(8) != Some(data.len()) {
            return Err(Size);
        }
        let count = c.u32().ok_or(Header)? as usize;
        let max_frame = c.u32().ok_or(Header)? as usize;
        let _repeat_count = c.u32().ok_or(Header)?;
        let width = c.u32().ok_or(Header)?;
        let height = c.u32().ok_or(Header)?;
        if width == 0 || height == 0 || width > 32767 || height > 32767 {
            return Err(Dimensions);
        }
        let fps_num = c.u32().ok_or(Header)?;
        let fps_den = c.u32().ok_or(Header)?;
        if fps_num == 0 || fps_den == 0 {
            return Err(Rate);
        }
        let flags = c.u32().ok_or(Header)?;
        let track_count = c.u32().ok_or(Header)? as usize;
        if track_count > 256 {
            return Err(Audio);
        }
        let audio_size = track_count.checked_mul(12).ok_or(Audio)?;
        let index_size = count
            .checked_add(1)
            .and_then(|n| n.checked_mul(4))
            .ok_or(Index)?;
        let table_end = c
            .pos()
            .checked_add(audio_size)
            .and_then(|v| v.checked_add(index_size))
            .ok_or(Index)?;
        if count == 0 || table_end > data.len() || max_frame == 0 || max_frame > data.len() {
            return Err(Index);
        }
        // Validate counts against the input before reserving either table.
        c.skip(track_count * 4).ok_or(Audio)?;
        let mut tracks = Vec::with_capacity(track_count);
        for _ in 0..track_count {
            tracks.push(BinkTrack {
                sample_rate: c.u16().ok_or(Audio)?,
                flags: c.u16().ok_or(Audio)?,
                id: 0,
            });
        }
        for t in &mut tracks {
            t.id = c.u32().ok_or(Audio)?;
        }
        let mut frames = Vec::with_capacity(count);
        let mut current = c.u32().ok_or(Index)?;
        for _ in 0..count {
            let next = c.u32().ok_or(Index)?;
            let offset = (current & !1) as usize;
            let end = (next & !1) as usize;
            let size = end
                .checked_sub(offset)
                .filter(|n| *n > 0 && *n <= max_frame)
                .ok_or(Index)?;
            if offset < table_end || end > data.len() {
                return Err(Index);
            }
            frames.push(BinkFrame {
                offset,
                size,
                keyframe: current & 1 != 0,
            });
            current = next;
        }
        if (current & !1) as usize != data.len() {
            return Err(Index);
        }
        Ok(Self {
            revision,
            width,
            height,
            fps_num,
            fps_den,
            flags,
            tracks,
            frames,
            data,
        })
    }
    pub fn packet(&self, index: usize) -> Option<&'a [u8]> {
        let f = self.frames.get(index)?;
        self.data.get(f.offset..f.offset + f.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = b"BIKi".to_vec();
        for v in [56u32, 2, 4, 2, 8, 8, 15, 1, 0, 0, 57, 60, 64] {
            b.extend(v.to_le_bytes());
        }
        b.extend([1; 8]);
        b
    }
    #[test]
    fn packets_strip_key_bits_and_preserve_the_rational_clock() {
        let b = fixture();
        let f = BinkFile::parse(&b).unwrap();
        assert_eq!((f.width, f.height, f.fps_num, f.fps_den), (8, 8, 15, 1));
        assert_eq!(f.frames.len(), 2);
        assert!(f.frames[0].keyframe);
        assert_eq!(f.packet(1), Some([1; 4].as_slice()));
        assert!(f.packet(2).is_none());
    }
    #[test]
    fn hostile_headers_indices_and_truncation_are_rejected_before_allocation() {
        let b = fixture();
        for i in 0..b.len() {
            assert!(BinkFile::parse(&b[..i]).is_err());
        }
        for at in [4, 8, 12, 20, 24, 40, 44, 48, 52] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(BinkFile::parse(&bad).is_err(), "field {at}");
        }
        for at in [8, 12, 20, 24, 28, 32] {
            let mut bad = b.clone();
            bad[at..at + 4].fill(0);
            assert!(BinkFile::parse(&bad).is_err(), "field {at}");
        }
        let mut bad = b;
        bad[48..52].copy_from_slice(&56u32.to_le_bytes());
        assert!(BinkFile::parse(&bad).is_err());
    }
}
