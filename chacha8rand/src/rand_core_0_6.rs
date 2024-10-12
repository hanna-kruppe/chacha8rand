use rand_core::{RngCore, SeedableRng};

use crate::ChaCha8Rand;

impl RngCore for ChaCha8Rand {
    fn next_u32(&mut self) -> u32 {
        self.read_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.read_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.read_bytes(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.read_bytes(dest);
        Ok(())
    }
}

impl SeedableRng for ChaCha8Rand {
    type Seed = [u8; 32];

    fn from_seed(seed: [u8; 32]) -> Self {
        Self::new(seed)
    }
}
