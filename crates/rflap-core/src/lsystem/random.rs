//! `java.util.Random`, so seeded expansions and games repeat what JFLAP's generator would do.

use serde::{Deserialize, Serialize};

const MULTIPLIER: i64 = 0x5DEECE66D;
const MASK: i64 = (1 << 48) - 1;

/// The linear congruential generator of `java.util.Random`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaRandom {
    seed: i64,
}

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        JavaRandom {
            seed: (seed ^ MULTIPLIER) & MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(0xB) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// `nextInt(bound)`, `bound` must be positive.
    pub fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next(31);
        let m = bound - 1;
        if bound & m == 0 {
            ((i64::from(bound) * i64::from(r)) >> 31) as i32
        } else {
            let mut u = r;
            loop {
                r = u % bound;
                if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                    return r;
                }
                u = self.next(31);
            }
        }
    }

    /// `nextBoolean()`.
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::JavaRandom;

    #[test]
    fn matches_the_values_java_prints_for_seed_zero() {
        // new Random(0).nextInt() == -1155484576 and nextInt(2) takes the top bit.
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next(32), -1155484576);
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int(2), 1);
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int(10), 0);
        assert_eq!(r.next_int(10), 3);
    }
}
