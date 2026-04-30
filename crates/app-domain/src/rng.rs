use rand::SeedableRng;
use rand::rngs::SmallRng;

/// A seeded RNG wrapper. Wraps `SmallRng` (fast, non-crypto) with an explicit
/// seed so that any combat sequence can be replayed deterministically by
/// re-seeding from the stored `rng_seed` in the Snapshot.
pub struct SeededRng {
    inner: SmallRng,
    seed: u64,
}

impl SeededRng {
    pub fn from_seed(seed: u64) -> Self {
        Self { inner: SmallRng::seed_from_u64(seed), seed }
    }

    /// Generate a random seed from the OS entropy source (for new combats).
    pub fn new_random() -> Self {
        use rand::TryRngCore;
        let mut buf = [0u8; 8];
        rand::rngs::OsRng.try_fill_bytes(&mut buf).expect("os rng");
        let seed = u64::from_le_bytes(buf);
        Self::from_seed(seed)
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn inner_mut(&mut self) -> &mut SmallRng {
        &mut self.inner
    }
}
