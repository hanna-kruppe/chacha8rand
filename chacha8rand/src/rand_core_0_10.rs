use rand_core_0_10::{SeedableRng, TryRng};

use crate::ChaCha8Rand;

/// Integration with rand_core v0.10 / rand v0.10. Requires crate feature `rand_core_0_10`.
///
/// The trait methods simply delegate to the equivalent inherent methods. `next_u32` maps to
/// [`ChaCha8Rand::read_u32`], and so on.
impl TryRng for ChaCha8Rand {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.read_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.read_u64())
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
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
