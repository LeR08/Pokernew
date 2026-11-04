//! Trainer and hand analysis against solver solutions (see `docs/trainer.md`).
//!
//! * [`drill`]: questions dealt from a solved postflop game (random line, hero
//!   combo drawn from the range), answers graded by EV loss;
//! * [`analysis`]: a played hand replayed in the solved spot, EV lost at each of
//!   hero's postflop decisions;
//! * [`grade`]: from EV loss (% of pot) to best / correct / inaccuracy / mistake / blunder.

pub mod analysis;
pub mod drill;
pub mod grade;

/// Small seedable generator (SplitMix64).
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Index drawn with probability proportional to `w` (None when all are 0).
    pub fn pick(&mut self, w: &[f64]) -> Option<usize> {
        let total: f64 = w.iter().filter(|x| **x > 0.0).sum();
        if total <= 0.0 {
            return None;
        }
        let mut r = self.unit() * total;
        for (i, &x) in w.iter().enumerate() {
            if x > 0.0 {
                if r < x {
                    return Some(i);
                }
                r -= x;
            }
        }
        w.iter().rposition(|x| *x > 0.0)
    }
}
