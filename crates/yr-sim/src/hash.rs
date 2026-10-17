//! FNV-1a 64-bit state hasher (deterministic across platforms/runs).

pub struct Hasher {
    h: u64,
}

impl Hasher {
    pub fn new() -> Hasher {
        Hasher { h: 0xcbf2_9ce4_8422_2325 }
    }

    pub fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.h ^= u64::from(b);
            self.h = self.h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    pub fn finish(&self) -> u64 {
        self.h
    }
}
