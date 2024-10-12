use rand_core::{RngCore, SeedableRng};

use crate::ChaCha8Rand;

impl RngCore for ChaCha8Rand {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.read_u32()
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.read_u64()
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.read_bytes(dest);
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.read_bytes(dest);
        Ok(())
    }
}

impl SeedableRng for ChaCha8Rand {
    type Seed = [u8; 32];

    #[inline]
    fn from_seed(seed: [u8; 32]) -> Self {
        Self::new(seed)
    }
}
