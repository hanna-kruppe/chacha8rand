use rand_core::{impls::fill_via_u32_chunks, RngCore, SeedableRng};

use crate::ChaCha8;

impl RngCore for ChaCha8 {
    fn next_u32(&mut self) -> u32 {
        ChaCha8::next_u32(self)
    }

    fn next_u64(&mut self) -> u64 {
        ChaCha8::next_u64(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut total_filled = 0;
        while total_filled < dest.len() {
            let buffered = self.buf.output();
            if self.i >= buffered.len() {
                self.refill();
            }
            let src = &self.buf.output()[self.i..];
            let (consumed_u32, filled_u8) = fill_via_u32_chunks(src, &mut dest[total_filled..]);
            self.i += consumed_u32;
            total_filled += filled_u8;
        }
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
