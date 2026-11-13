//! Westwood WSA container metadata and frame bounds. No codec or I/O here.
//! Observations and references: docs/formats/wsa.md.
use crate::bytes::Cursor;

/// Palette bytes stored after the offset table when the header flag is set.
pub const WSA_PALETTE_BYTES: usize = 768;
/// Header flag bit marking an embedded palette (bit 1).
pub const WSA_FLAG_HAS_PALETTE: u16 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsaError {
    Header,
    Dimensions,
    Index,
}
impl std::fmt::Display for WsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid WSA {:?}", self)
    }
}
impl std::error::Error for WsaError {}

#[derive(Debug, Clone, Copy)]
pub struct WsaFrame {
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug)]
pub struct WsaFile<'a> {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub delta_buffer_size: u16,
    pub flags: u16,
    pub has_palette: bool,
    /// False when the first offset is zero: the clip chains onto the final
    /// frame of a previously played WSA instead of carrying its own start.
    pub has_initial_state: bool,
    /// Extra frame that maps the final image back to the first when the
    /// table carries a nonzero terminal entry beyond the last frame.
    pub loop_frame: Option<WsaFrame>,
    pub frames: Vec<WsaFrame>,
    data: &'a [u8],
    palette_offset: Option<usize>,
}

impl<'a> WsaFile<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, WsaError> {
        use WsaError::*;
        let mut c = Cursor::new(data);
        let count = c.u16().ok_or(Header)? as usize;
        let x = c.u16().ok_or(Header)?;
        let y = c.u16().ok_or(Header)?;
        let width = c.u16().ok_or(Header)?;
        let height = c.u16().ok_or(Header)?;
        let delta_buffer_size = c.u16().ok_or(Header)?;
        let flags = c.u16().ok_or(Header)?;
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err(Dimensions);
        }
        if count == 0 {
            return Err(Index);
        }
        let has_palette = flags & WSA_FLAG_HAS_PALETTE != 0;
        let palette_len = if has_palette { WSA_PALETTE_BYTES } else { 0 };
        // Validate the table size against the input before reserving it.
        let table_size = count
            .checked_add(2)
            .and_then(|n| n.checked_mul(4))
            .ok_or(Index)?;
        let data_start = c.pos().checked_add(table_size).ok_or(Index)?;
        let total_start = data_start.checked_add(palette_len).ok_or(Index)?;
        if total_start > data.len() {
            return Err(Index);
        }
        let mut offsets = Vec::with_capacity(count + 2);
        for _ in 0..count + 2 {
            offsets.push(c.u32().ok_or(Index)? as usize);
        }
        // Stored offsets ignore the palette, so compare them against the
        // file length with the palette bytes removed.
        let stored_len = data.len() - palette_len;
        let has_initial_state = offsets[0] != 0;
        if has_initial_state && offsets[0] != data_start {
            return Err(Index);
        }
        let first = usize::from(!has_initial_state);
        // Chained clips omit frame zero, but their first stored packet must
        // still begin after the metadata (in palette-free coordinates).
        if offsets[first] < data_start {
            return Err(Index);
        }
        for pair in offsets[first..=count].windows(2) {
            if pair[1] <= pair[0] || pair[1] > stored_len {
                return Err(Index);
            }
        }
        // The final entry is either zero (the previous entry ends the data)
        // or the data end itself, leaving a loop frame in between.
        let (data_end, loop_frame) = match offsets[count + 1] {
            0 => (offsets[count], None),
            end => (
                end,
                Some(WsaFrame {
                    offset: offsets[count].checked_add(palette_len).ok_or(Index)?,
                    size: end
                        .checked_sub(offsets[count])
                        .filter(|size| *size > 0)
                        .ok_or(Index)?,
                }),
            ),
        };
        if data_end != stored_len || offsets[count] < data_start {
            return Err(Index);
        }
        let mut frames = Vec::with_capacity(count);
        for i in 0..count {
            if i == 0 && !has_initial_state {
                // Chained clips have no first frame of their own; keep an
                // empty range so frame indices still line up.
                frames.push(WsaFrame {
                    offset: total_start,
                    size: 0,
                });
                continue;
            }
            let offset = offsets[i].checked_add(palette_len).ok_or(Index)?;
            let size = offsets[i + 1]
                .checked_sub(offsets[i])
                .filter(|n| *n > 0)
                .ok_or(Index)?;
            frames.push(WsaFrame { offset, size });
        }
        Ok(Self {
            x,
            y,
            width,
            height,
            delta_buffer_size,
            flags,
            has_palette,
            has_initial_state,
            loop_frame,
            frames,
            data,
            palette_offset: has_palette.then_some(data_start),
        })
    }

    pub fn palette(&self) -> Option<&'a [u8]> {
        let start = self.palette_offset?;
        self.data.get(start..start + WSA_PALETTE_BYTES)
    }

    pub fn frame(&self, index: usize) -> Option<&'a [u8]> {
        let f = self.frames.get(index)?;
        if f.size == 0 {
            return None;
        }
        self.data.get(f.offset..f.offset + f.size)
    }

    pub fn loop_data(&self) -> Option<&'a [u8]> {
        let f = self.loop_frame?;
        self.data.get(f.offset..f.offset + f.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(count: u16, flags: u16, delta: u16) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [count, 10, 20, 4, 3, delta, flags] {
            b.extend(v.to_le_bytes());
        }
        b
    }

    fn fixture() -> Vec<u8> {
        // Two frames with an embedded palette and no loop frame.
        let mut b = header(2, WSA_FLAG_HAS_PALETTE, 100);
        for v in [30u32, 34, 38, 0] {
            b.extend(v.to_le_bytes());
        }
        b.extend([7u8; WSA_PALETTE_BYTES]);
        b.extend([1u8; 8]);
        b
    }

    #[test]
    fn metadata_palette_and_frame_ranges_follow_the_table() {
        let b = fixture();
        let f = WsaFile::parse(&b).unwrap();
        assert_eq!((f.x, f.y, f.width, f.height), (10, 20, 4, 3));
        assert_eq!(f.delta_buffer_size, 100);
        assert!(f.has_palette && f.has_initial_state);
        assert_eq!(f.palette(), Some([7u8; WSA_PALETTE_BYTES].as_slice()));
        assert_eq!(f.frames.len(), 2);
        assert_eq!(f.frame(0), Some([1u8; 4].as_slice()));
        assert_eq!(f.frame(1), Some([1u8; 4].as_slice()));
        assert!(f.frame(2).is_none());
        assert!(f.loop_frame.is_none() && f.loop_data().is_none());
    }

    #[test]
    fn nonzero_terminal_entry_exposes_the_loop_frame() {
        let mut b = header(2, 0, 100);
        for v in [30u32, 34, 40, 44] {
            b.extend(v.to_le_bytes());
        }
        b.extend([2u8; 14]);
        let f = WsaFile::parse(&b).unwrap();
        assert!(!f.has_palette && f.palette().is_none());
        assert_eq!(f.frame(1), Some([2u8; 6].as_slice()));
        assert_eq!(f.loop_data(), Some([2u8; 4].as_slice()));
    }

    #[test]
    fn zero_first_offset_marks_a_chained_clip() {
        let mut b = header(2, 0, 100);
        for v in [0u32, 34, 38, 0] {
            b.extend(v.to_le_bytes());
        }
        b.extend([3u8; 8]);
        let f = WsaFile::parse(&b).unwrap();
        assert!(!f.has_initial_state);
        assert!(f.frame(0).is_none());
        assert_eq!(f.frame(1), Some([3u8; 4].as_slice()));
    }

    #[test]
    fn chained_frames_cannot_read_header_table_or_palette_bytes() {
        for flags in [0, WSA_FLAG_HAS_PALETTE] {
            for start in 0..30u32 {
                let mut data = header(2, flags, 100);
                for offset in [0, start, 38, 0] {
                    data.extend(offset.to_le_bytes());
                }
                if flags != 0 {
                    data.extend([7; WSA_PALETTE_BYTES]);
                }
                data.extend([3; 8]);
                assert!(
                    matches!(WsaFile::parse(&data), Err(WsaError::Index)),
                    "chained start {start}, flags {flags} overlaps metadata"
                );
            }
        }
    }

    #[test]
    fn explicit_loop_frame_must_have_payload() {
        for flags in [0, WSA_FLAG_HAS_PALETTE] {
            let mut data = header(2, flags, 100);
            for offset in [30u32, 34, 38, 38] {
                data.extend(offset.to_le_bytes());
            }
            if flags != 0 {
                data.extend([7; WSA_PALETTE_BYTES]);
            }
            data.extend([3; 8]);
            assert!(matches!(WsaFile::parse(&data), Err(WsaError::Index)));
        }
    }

    #[test]
    fn valid_packet_ranges_stay_outside_metadata_for_all_container_modes() {
        for chained in [false, true] {
            for flags in [0, WSA_FLAG_HAS_PALETTE] {
                for looped in [false, true] {
                    for gap in [0u32, 4] {
                        if !chained && gap != 0 {
                            continue;
                        }
                        let palette_len = if flags == 0 { 0 } else { WSA_PALETTE_BYTES };
                        let first = if chained { 0 } else { 30 };
                        let second = if chained { 30 + gap } else { 34 };
                        let end = second + 4;
                        let eof = if looped { end + 4 } else { end };
                        let mut data = header(2, flags, 100);
                        for offset in [first, second, end, if looped { eof } else { 0 }] {
                            data.extend(offset.to_le_bytes());
                        }
                        data.extend(vec![7; palette_len]);
                        data.extend((30..eof).map(|n| n as u8));
                        let clip = WsaFile::parse(&data).unwrap();
                        assert_eq!(clip.has_initial_state, !chained);
                        assert_eq!(clip.frame(0).is_none(), chained);
                        assert_eq!(
                            clip.frame(1),
                            Some(
                                data[second as usize + palette_len..end as usize + palette_len]
                                    .as_ref()
                            )
                        );
                        assert_eq!(
                            clip.loop_data(),
                            looped
                                .then(|| &data
                                    [end as usize + palette_len..eof as usize + palette_len])
                        );
                        for packet in clip
                            .frames
                            .iter()
                            .filter(|f| f.size != 0)
                            .chain(clip.loop_frame.iter())
                        {
                            assert!(packet.offset >= 30 + palette_len);
                            assert!(packet.size > 0);
                            assert!(packet.offset + packet.size <= data.len());
                        }
                        for prefix in 0..data.len() {
                            assert!(WsaFile::parse(&data[..prefix]).is_err());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn hostile_headers_tables_and_offsets_are_rejected_before_allocation() {
        let b = fixture();
        for i in 0..b.len() {
            assert!(WsaFile::parse(&b[..i]).is_err(), "truncated at {i}");
        }
        // Frame count whose table cannot fit the input.
        let mut bad = b.clone();
        bad[0..2].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(WsaFile::parse(&bad).is_err());
        // Zero and oversized dimensions.
        for (at, v) in [(6u16, 0u16), (8, 0), (6, 8193), (8, 8193)] {
            let mut bad = b.clone();
            bad[at as usize..at as usize + 2].copy_from_slice(&v.to_le_bytes());
            assert!(WsaFile::parse(&bad).is_err(), "dimension at {at}");
        }
        // Zero frame count.
        let mut bad = b.clone();
        bad[0..2].copy_from_slice(&0u16.to_le_bytes());
        assert!(WsaFile::parse(&bad).is_err());
        // Non-monotonic offsets and an end that misses EOF.
        for (at, v) in [(22u32, 30u32), (26, 999)] {
            let mut bad = b.clone();
            bad[at as usize..at as usize + 4].copy_from_slice(&v.to_le_bytes());
            assert!(WsaFile::parse(&bad).is_err(), "offset at {at}");
        }
    }
}
