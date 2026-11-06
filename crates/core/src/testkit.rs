//! Reproducible randomness for the robustness tests of every crate
//! (S0-T07, S3-05): a tiny xorshift generator and byte mutators, with no
//! dependencies, so the tests run on stable Rust on every OS in CI.

/// xorshift64*: small, fast, reproducible.
pub struct Rng(pub u64);

impl Rng {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    /// A float spanning the cases parsers meet: zero, negatives, huge, tiny, NaN, infinities.
    pub fn float(&mut self) -> f64 {
        match self.below(10) {
            0 => 0.0,
            1 => f64::NAN,
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => -((self.next() % 1000) as f64),
            5 => 1e300,
            6 => 1e-300,
            7 => (self.next() % 100_000) as f64 / 7.0,
            _ => (self.next() % 400) as f64 / 3.0,
        }
    }

    pub fn bytes(&mut self, max: usize) -> Vec<u8> {
        let n = self.below(max + 1);
        (0..n).map(|_| self.next() as u8).collect()
    }

    /// `base` with a few bytes flipped, inserted or cut.
    pub fn mutate(&mut self, base: &[u8]) -> Vec<u8> {
        let mut v = base.to_vec();
        for _ in 0..1 + self.below(8) {
            match self.below(4) {
                0 if !v.is_empty() => {
                    let i = self.below(v.len());
                    v[i] = self.next() as u8;
                }
                1 => {
                    let i = self.below(v.len() + 1);
                    v.insert(i, self.next() as u8);
                }
                2 if !v.is_empty() => {
                    let cut = self.below(v.len());
                    v.truncate(cut);
                }
                _ => {
                    if let Some(i) = v.len().checked_sub(1).map(|l| self.below(l + 1)) {
                        v[i] = [0x00, 0xFF, 0x7F, 0x80][self.below(4)];
                    }
                }
            }
        }
        v
    }

    /// `base` as text, mutated (invalid UTF-8 is replaced, as a lossy reader would).
    pub fn mutate_text(&mut self, base: &str) -> String {
        String::from_utf8_lossy(&self.mutate(base.as_bytes())).into_owned()
    }
}
