use std::array;

mod backend;
mod guts;
#[cfg(test)]
mod tests;

use arrayref::array_ref;
pub use backend::Backend;

pub struct ChaCha8 {
    seed: [u32; 8],
    i: usize,
    buf: [u32; 256],
    backend: Backend,
}

pub struct Seed([u32; 8]);

impl From<[u8; 32]> for Seed {
    fn from(bytes: [u8; 32]) -> Self {
        Self(array::from_fn(|i| {
            u32::from_le_bytes(*array_ref![bytes, 4 * i, 4])
        }))
    }
}

impl From<&[u8; 32]> for Seed {
    fn from(bytes: &[u8; 32]) -> Self {
        Self::from(*bytes)
    }
}

fn backend_detect() -> Backend {
    if let Some(avx2) = Backend::avx2() {
        return avx2;
    }
    if cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64") {
        // These targets always have 128 bit SIMD available
        return Backend::simd128();
    }
    if cfg!(target_arch = "x86") && cfg!(target_feature = "sse2") {
        // The case for the x4 impl is less obvious for 32-bit x86 SIMD because there we only have
        // eight XMM registers, but it's probably no worse than the scalar implementation. TODO:
        // benchmark it.
        return Backend::simd128();
    }
    // Fallback if we don't know for sure that we have SIMD:
    Backend::scalar()
}

impl ChaCha8 {
    pub fn new(seed: Seed) -> Self {
        Self::with_backend(seed, backend_detect())
    }

    pub fn with_backend(seed: Seed, backend: Backend) -> Self {
        let mut this = Self {
            seed: seed.0,
            i: 0,
            buf: [0; 256],
            backend,
        };
        backend.refill(&this.seed, &mut this.buf);
        this
    }

    pub fn next_u32(&mut self) -> u32 {
        let new_seed_start = self.buf.len() - self.seed.len();
        if self.i >= new_seed_start {
            self.seed.copy_from_slice(&self.buf[new_seed_start..]);
            self.backend.refill(&self.seed, &mut self.buf);
            self.i = 0;
        }
        let result = self.buf[self.i];
        self.i += 1;
        result
    }
}
