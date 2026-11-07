//! `RND` and `RANDOMIZE`: the 24-bit linear congruential generator of QuickBASIC 4.5 and
//! QBasic 1.1.

/// The generator state.
#[derive(Clone, Debug)]
pub struct Rnd {
    seed: u32,
}

impl Default for Rnd {
    fn default() -> Self {
        // The state QBasic starts with: the same sequence on every run without RANDOMIZE.
        Self { seed: 327_680 }
    }
}

impl Rnd {
    /// `RANDOMIZE n`: mixes the high word pair of `n` as a double into the middle 16 bits of
    /// the state and keeps the low byte.
    pub fn randomize(&mut self, n: f64) {
        let hi = (n.to_bits() >> 32) as u32;
        let m = ((hi ^ (hi >> 16)) & 0xFFFF) << 8;
        self.seed = (self.seed & 0xFF) | m;
    }

    /// `RND(1)`: the next value in `[0, 1)`, single precision.
    pub fn rnd(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(16_598_013).wrapping_add(12_820_163) & 0x00FF_FFFF;
        self.seed as f32 / 16_777_216.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sequence_starts_like_qbasic() {
        // PRINT RND in a fresh QBasic session prints .7055475.
        let mut r = Rnd::default();
        assert!((r.rnd() - 0.705_547_5).abs() < 1e-6);
    }
}
