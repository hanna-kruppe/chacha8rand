use rand_core::{RngCore, SeedableRng};

use crate::ChaCha8Rand;

/// Integration with rand_core v0.6 / rand v0.8. Requires crate feature `rand_core_0_6`.
///
/// The trait methods simply delegate to the equivalent inherent methods. `next_u32` maps to
/// [`ChaCha8Rand::read_u32`], and so on. `try_fill_bytes` never returns an error.
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

/// Integration with rand_core v0.6 / rand v0.8. Requires crate feature `rand_core_0_6`.
///
/// `from_seed` is equivalent to [`ChaCha8Rand::new`] except that it takes the seed by value instead
/// of by reference.
impl SeedableRng for ChaCha8Rand {
    type Seed = [u8; 32];

    #[inline]
    fn from_seed(seed: [u8; 32]) -> Self {
        Self::new(&seed)
    }
}
