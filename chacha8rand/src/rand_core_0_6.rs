use rand_core::{RngCore, SeedableRng};

use crate::ChaCha8;

impl RngCore for ChaCha8 {
    fn next_u32(&mut self) -> u32 {
        ChaCha8::next_u32(self)
    }

    fn next_u64(&mut self) -> u64 {
        ChaCha8::next_u64(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // TODO: fill_via_u32_chunks might be faster, but is also more fiddly to implement
        rand_core::impls::fill_bytes_via_next(self, dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl SeedableRng for ChaCha8 {
    type Seed = [u8; 32];

    fn from_seed(seed: [u8; 32]) -> Self {
        Self::new(crate::Seed::from(seed))
    }
}
