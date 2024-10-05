use rand_core::{
    block::{BlockRng, BlockRngCore},
    RngCore, SeedableRng,
};

use crate::{Backend, Buffer};

pub struct ChaCha8Rng(BlockRng<ChaCha8Core>);

impl RngCore for ChaCha8Rng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.0.try_fill_bytes(dest)
    }
}

impl SeedableRng for ChaCha8Rng {
    type Seed = [u8; 32];

    fn from_seed(seed: Self::Seed) -> Self {
        Self(BlockRng::new(ChaCha8Core {
            key: crate::Seed::from(seed).0,
            backend: Backend::detect_best(),
        }))
    }
}

struct ChaCha8Core {
    key: [u32; 8],
    backend: Backend,
}

struct U32x248([u32; 248]);

impl AsRef<[u32]> for U32x248 {
    fn as_ref(&self) -> &[u32] {
        &self.0
    }
}

impl AsMut<[u32]> for U32x248 {
    fn as_mut(&mut self) -> &mut [u32] {
        &mut self.0
    }
}

impl Default for U32x248 {
    fn default() -> Self {
        Self([0; 248])
    }
}

impl BlockRngCore for ChaCha8Core {
    type Item = u32;
    type Results = U32x248;

    fn generate(&mut self, results: &mut Self::Results) {
        let mut buf = Buffer { words: [0; 256] };
        self.backend.refill(&self.key, &mut buf);
        results.0 = *buf.output();
        self.key = *buf.new_key();
    }
}

#[test]
fn test_block_rng() {
    use crate::tests::{test_sample_output, SAMPLE_SEED};

    let mut rng = ChaCha8Rng::from_seed(SAMPLE_SEED);
    test_sample_output(&mut || rng.next_u64());
}
