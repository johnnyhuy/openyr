//! Little-endian bounds-checked byte cursor shared by the binary parsers.
//!
//! Every read returns `None` on overrun so parsers can map truncation to their
//! own error type with `.ok_or(...)`.

#[derive(Clone)]
pub(crate) struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Cursor { data, pos: 0 }
    }

    pub fn at(data: &'a [u8], pos: usize) -> Self {
        Cursor { data, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }

    pub fn skip(&mut self, n: usize) -> Option<()> {
        self.take(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    pub fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }

    pub fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }

    pub fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N).map(|b| b.try_into().unwrap())
    }

    pub fn f32s<const N: usize>(&mut self) -> Option<[f32; N]> {
        let mut out = [0f32; N];
        for v in &mut out {
            *v = self.f32()?;
        }
        Some(out)
    }
}

/// NUL-terminated ASCII name from a fixed-width field.
pub(crate) fn fixed_name(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    raw[..end].iter().map(|&b| b as char).collect()
}
