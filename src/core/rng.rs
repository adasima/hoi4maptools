use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

/// アプリ全体で共有する決定論的RNG。
pub struct WorldRng {
    inner: Xoshiro256PlusPlus,
    seed: u64,
}

impl WorldRng {
    pub fn new(seed: u64) -> Self {
        let inner = Xoshiro256PlusPlus::seed_from_u64(seed);
        Self { inner, seed }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.inner = Xoshiro256PlusPlus::seed_from_u64(seed);
        self.seed = seed;
    }

    pub fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }

    pub fn next_f32(&mut self) -> f32 {
        // 0.0..1.0 の範囲の乱数
        let v = self.next_u64();
        const SCALE: f64 = 1.0 / (u64::MAX as f64);
        (v as f64 * SCALE) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::WorldRng;

    #[test]
    fn same_seed_produces_same_sequence() {
        let mut rng1 = WorldRng::new(42);
        let mut rng2 = WorldRng::new(42);

        let seq1: Vec<u64> = (0..10).map(|_| rng1.next_u64()).collect();
        let seq2: Vec<u64> = (0..10).map(|_| rng2.next_u64()).collect();

        assert_eq!(seq1, seq2);
    }
}

