//! Deterministic RNG (SplitMix64). The state is part of the sim state
//! and of the state hash; there is no other source of randomness.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Value in `0..n` (n > 0) via modulo; bias is acceptable for game
    /// scatter and keeps the mapping trivially deterministic.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_known_answer() {
        // Published first output for seed 0.
        assert_eq!(Rng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
        let mut r = Rng::new(123);
        assert!(r.below(10) < 10);
    }
}
