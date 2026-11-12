#![allow(dead_code)]
//! Deterministic SplitMix64 PRNG so every long-horizon marathon episode and
//! failure reproduction is bit-for-bit reproducible from a single `u64` seed
//! with zero external crate dependencies.

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        // Avoid a zero-ish fixed point by mixing in a constant offset.
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            return 0;
        }
        (self.next_u64() % (upper_exclusive as u64)) as usize
    }

    pub fn range_inclusive(&mut self, low: usize, high: usize) -> usize {
        if high <= low {
            return low;
        }
        low + self.next_usize(high - low + 1)
    }

    /// True with probability `numerator / denominator`.
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        if numerator == 0 || denominator == 0 {
            return false;
        }
        if numerator >= denominator {
            return true;
        }
        (self.next_u64() % u64::from(denominator)) < u64::from(numerator)
    }

    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.next_usize(items.len());
        &items[idx]
    }
}
